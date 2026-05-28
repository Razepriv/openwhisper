#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use crate::apple_intelligence;
use crate::audio_feedback::{play_feedback_sound, play_feedback_sound_blocking, SoundType};
use crate::audio_toolkit::{is_microphone_access_denied, is_no_input_device_error};
use crate::managers::audio::AudioRecordingManager;
use crate::managers::history::HistoryManager;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, AppSettings, APPLE_INTELLIGENCE_PROVIDER_ID};
use crate::shortcut;
use crate::tray::{change_tray_icon, TrayIconState};
use crate::utils::{
    self, show_processing_overlay, show_recording_overlay, show_transcribing_overlay,
};
use crate::TranscriptionCoordinator;
use ferrous_opencc::{config::BuiltinConfig, OpenCC};
use log::{debug, error, info, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::Manager;
use tauri::{AppHandle, Emitter};

/// What the next finished dictation should be processed as.
///
/// Phase Finalize.A — both Transforms and Command Mode reuse the
/// standard recording / transcription pipeline; they only diverge at
/// the post-transcription step. Rather than thread a strategy enum
/// through every layer (the recording state machine doesn't care), we
/// stash the strategy in this process-wide cell at start() time and
/// read it back when the dictation finishes.
///
/// The cell is reset to `Standard` after every read so a stray hotkey
/// press can never accidentally route a normal dictation through a
/// Transform's prompt. The mutex is uncontended in practice (set once
/// on press, read once on release) but exists so multiple shortcut
/// threads can't race on it.
#[derive(Debug, Clone)]
pub(crate) enum NextProcessing {
    Standard,
    Transform { transform_id: String },
    CommandMode { selection: String },
}

static NEXT_PROCESSING: Lazy<Mutex<NextProcessing>> =
    Lazy::new(|| Mutex::new(NextProcessing::Standard));

pub(crate) fn set_next_processing(next: NextProcessing) {
    if let Ok(mut guard) = NEXT_PROCESSING.lock() {
        *guard = next;
    }
}

pub(crate) fn take_next_processing() -> NextProcessing {
    NEXT_PROCESSING
        .lock()
        .map(|mut g| std::mem::replace(&mut *g, NextProcessing::Standard))
        .unwrap_or(NextProcessing::Standard)
}

#[derive(Clone, serde::Serialize)]
struct RecordingErrorEvent {
    error_type: String,
    detail: Option<String>,
}

/// Drop guard that notifies the [`TranscriptionCoordinator`] when the
/// transcription pipeline finishes — whether it completes normally or panics.
struct FinishGuard(AppHandle);
impl Drop for FinishGuard {
    fn drop(&mut self) {
        if let Some(c) = self.0.try_state::<TranscriptionCoordinator>() {
            c.notify_processing_finished();
        }
    }
}

// Shortcut Action Trait
pub trait ShortcutAction: Send + Sync {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
}

// Transcribe Action
struct TranscribeAction {
    post_process: bool,
}

/// Field name for structured output JSON schema
const TRANSCRIPTION_FIELD: &str = "transcription";

/// Strip invisible Unicode characters that some LLMs may insert
fn strip_invisible_chars(s: &str) -> String {
    s.replace(['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'], "")
}

/// Build a system prompt from the user's prompt template.
/// Removes `${output}` placeholder since the transcription is sent as the user message.
fn build_system_prompt(prompt_template: &str) -> String {
    prompt_template.replace("${output}", "").trim().to_string()
}

/// Compose the final system prompt sent to the LLM by prepending the
/// active style-preset fragment (when present) to the user's prompt
/// template. The style fragment goes FIRST because LLMs anchor on the
/// opening of the system prompt; putting the tone instruction up
/// front gives it more weight without burying the user's prompt.
///
/// Returns the base prompt unchanged when no style fragment is
/// supplied — preserving exact behaviour for users who haven't
/// triggered the Phase Finalize.D code path yet.
fn compose_system_prompt(prompt_template: &str, style_fragment: Option<&str>) -> String {
    let base = build_system_prompt(prompt_template);
    match style_fragment {
        Some(frag) if !frag.trim().is_empty() => format!("{}\n\n{}", frag.trim(), base),
        _ => base,
    }
}

async fn post_process_transcription(
    settings: &AppSettings,
    transcription: &str,
    style_fragment: Option<&str>,
) -> Option<String> {
    let provider = match settings.active_post_process_provider().cloned() {
        Some(provider) => provider,
        None => {
            debug!("Post-processing enabled but no provider is selected");
            return None;
        }
    };

    let model = settings
        .post_process_models
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    if model.trim().is_empty() {
        debug!(
            "Post-processing skipped because provider '{}' has no model configured",
            provider.id
        );
        return None;
    }

    let selected_prompt_id = match &settings.post_process_selected_prompt_id {
        Some(id) => id.clone(),
        None => {
            debug!("Post-processing skipped because no prompt is selected");
            return None;
        }
    };

    let prompt = match settings
        .post_process_prompts
        .iter()
        .find(|prompt| prompt.id == selected_prompt_id)
    {
        Some(prompt) => prompt.prompt.clone(),
        None => {
            debug!(
                "Post-processing skipped because prompt '{}' was not found",
                selected_prompt_id
            );
            return None;
        }
    };

    if prompt.trim().is_empty() {
        debug!("Post-processing skipped because the selected prompt is empty");
        return None;
    }

    debug!(
        "Starting LLM post-processing with provider '{}' (model: {})",
        provider.id, model
    );

    let api_key = settings
        .post_process_api_keys
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    // Disable reasoning for providers where post-processing rarely benefits from it.
    // - custom: top-level reasoning_effort (works for local OpenAI-compat servers)
    // - openrouter: nested reasoning object; exclude:true also keeps reasoning text
    //   out of the response so it can't pollute structured-output JSON parsing
    let (reasoning_effort, reasoning) = match provider.id.as_str() {
        "custom" => (Some("none".to_string()), None),
        "openrouter" => (
            None,
            Some(crate::llm_client::ReasoningConfig {
                effort: Some("none".to_string()),
                exclude: Some(true),
            }),
        ),
        _ => (None, None),
    };

    if provider.supports_structured_output {
        debug!("Using structured outputs for provider '{}'", provider.id);

        // Phase Finalize.D — splice the active app's style preset into
        // the system prompt so the LLM matches tone to context (formal
        // for email, casual for chat, concise for code, etc.).
        let system_prompt = compose_system_prompt(&prompt, style_fragment);
        let user_content = transcription.to_string();

        // Handle Apple Intelligence separately since it uses native Swift APIs
        if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
            #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
            {
                if !apple_intelligence::check_apple_intelligence_availability() {
                    debug!(
                        "Apple Intelligence selected but not currently available on this device"
                    );
                    return None;
                }

                let token_limit = model.trim().parse::<i32>().unwrap_or(0);
                return match apple_intelligence::process_text_with_system_prompt(
                    &system_prompt,
                    &user_content,
                    token_limit,
                ) {
                    Ok(result) => {
                        if result.trim().is_empty() {
                            debug!("Apple Intelligence returned an empty response");
                            None
                        } else {
                            let result = strip_invisible_chars(&result);
                            debug!(
                                "Apple Intelligence post-processing succeeded. Output length: {} chars",
                                result.len()
                            );
                            Some(result)
                        }
                    }
                    Err(err) => {
                        error!("Apple Intelligence post-processing failed: {}", err);
                        None
                    }
                };
            }

            #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
            {
                debug!("Apple Intelligence provider selected on unsupported platform");
                return None;
            }
        }

        // Define JSON schema for transcription output
        let json_schema = serde_json::json!({
            "type": "object",
            "properties": {
                (TRANSCRIPTION_FIELD): {
                    "type": "string",
                    "description": "The cleaned and processed transcription text"
                }
            },
            "required": [TRANSCRIPTION_FIELD],
            "additionalProperties": false
        });

        match crate::llm_client::send_chat_completion_with_schema(
            &provider,
            api_key.clone(),
            &model,
            user_content,
            Some(system_prompt),
            Some(json_schema),
            reasoning_effort.clone(),
            reasoning.clone(),
        )
        .await
        {
            Ok(Some(content)) => {
                // Parse the JSON response to extract the transcription field
                match serde_json::from_str::<serde_json::Value>(&content) {
                    Ok(json) => {
                        if let Some(transcription_value) =
                            json.get(TRANSCRIPTION_FIELD).and_then(|t| t.as_str())
                        {
                            let result = strip_invisible_chars(transcription_value);
                            debug!(
                                "Structured output post-processing succeeded for provider '{}'. Output length: {} chars",
                                provider.id,
                                result.len()
                            );
                            return Some(result);
                        } else {
                            error!("Structured output response missing 'transcription' field");
                            return Some(strip_invisible_chars(&content));
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to parse structured output JSON: {}. Returning raw content.",
                            e
                        );
                        return Some(strip_invisible_chars(&content));
                    }
                }
            }
            Ok(None) => {
                error!("LLM API response has no content");
                return None;
            }
            Err(e) => {
                warn!(
                    "Structured output failed for provider '{}': {}. Falling back to legacy mode.",
                    provider.id, e
                );
                // Fall through to legacy mode below
            }
        }
    }

    // Legacy mode: Replace ${output} variable in the prompt with the actual text.
    // Phase Finalize.D — also prepend the style fragment so legacy
    // providers benefit from per-app tone calibration too.
    let processed_prompt = prompt.replace("${output}", transcription);
    let processed_prompt = match style_fragment {
        Some(frag) if !frag.trim().is_empty() => {
            format!("{}\n\n{}", frag.trim(), processed_prompt)
        }
        _ => processed_prompt,
    };
    debug!("Processed prompt length: {} chars", processed_prompt.len());

    match crate::llm_client::send_chat_completion(
        &provider,
        api_key,
        &model,
        processed_prompt,
        reasoning_effort,
        reasoning,
    )
    .await
    {
        Ok(Some(content)) => {
            let content = strip_invisible_chars(&content);
            debug!(
                "LLM post-processing succeeded for provider '{}'. Output length: {} chars",
                provider.id,
                content.len()
            );
            Some(content)
        }
        Ok(None) => {
            error!("LLM API response has no content");
            None
        }
        Err(e) => {
            error!(
                "LLM post-processing failed for provider '{}': {}. Falling back to original transcription.",
                provider.id,
                e
            );
            None
        }
    }
}

async fn maybe_convert_chinese_variant(
    settings: &AppSettings,
    transcription: &str,
) -> Option<String> {
    // Check if language is set to Simplified or Traditional Chinese
    let is_simplified = settings.selected_language == "zh-Hans";
    let is_traditional = settings.selected_language == "zh-Hant";

    if !is_simplified && !is_traditional {
        debug!("selected_language is not Simplified or Traditional Chinese; skipping translation");
        return None;
    }

    debug!(
        "Starting Chinese translation using OpenCC for language: {}",
        settings.selected_language
    );

    // Use OpenCC to convert based on selected language
    let config = if is_simplified {
        // Convert Traditional Chinese to Simplified Chinese
        BuiltinConfig::Tw2sp
    } else {
        // Convert Simplified Chinese to Traditional Chinese
        BuiltinConfig::S2tw
    };

    match OpenCC::from_config(config) {
        Ok(converter) => {
            let converted = converter.convert(transcription);
            debug!(
                "OpenCC translation completed. Input length: {}, Output length: {}",
                transcription.len(),
                converted.len()
            );
            Some(converted)
        }
        Err(e) => {
            error!("Failed to initialize OpenCC converter: {}. Falling back to original transcription.", e);
            None
        }
    }
}

pub(crate) struct ProcessedTranscription {
    pub final_text: String,
    pub post_processed_text: Option<String>,
    pub post_process_prompt: Option<String>,
}

/// Drive the configured LLM with explicit prompts. Shared between
/// Auto Cleanup (configured prompt), Transforms (transform's own
/// prompt), and Command Mode (selection + instruction prompt).
///
/// Returns `None` if the provider is misconfigured (no model, no
/// selected provider) or the LLM call errors. Callers fall back to the
/// raw text so a broken LLM never silently swallows a dictation.
///
/// Local-only enforcement (Phase 1.10) still applies — the URL
/// whitelist in `llm_client::validate_local_url` runs before any
/// request leaves the process.
async fn run_llm_with_prompts(
    settings: &AppSettings,
    system_prompt: String,
    user_content: String,
) -> Option<String> {
    let provider = settings.active_post_process_provider().cloned()?;
    let model = settings.post_process_models.get(&provider.id).cloned()?;
    if model.trim().is_empty() {
        debug!(
            "Skipping LLM call: provider '{}' has no model configured",
            provider.id
        );
        return None;
    }

    let api_key = settings
        .post_process_api_keys
        .get(&provider.id)
        .cloned()
        .unwrap_or_default();

    // Match `post_process_transcription`'s reasoning policy — disable
    // reasoning where it's costly and adds no quality for short
    // rewrite tasks.
    let (reasoning_effort, reasoning) = match provider.id.as_str() {
        "custom" => (Some("none".to_string()), None),
        "openrouter" => (
            None,
            Some(crate::llm_client::ReasoningConfig {
                effort: Some("none".to_string()),
                exclude: Some(true),
            }),
        ),
        _ => (None, None),
    };

    // Apple Intelligence path (macOS aarch64) — uses the native
    // FoundationModels framework instead of an HTTP call.
    if provider.id == APPLE_INTELLIGENCE_PROVIDER_ID {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            if !apple_intelligence::check_apple_intelligence_availability() {
                debug!("Apple Intelligence not available; skipping LLM call");
                return None;
            }
            let token_limit = model.trim().parse::<i32>().unwrap_or(0);
            return match apple_intelligence::process_text_with_system_prompt(
                &system_prompt,
                &user_content,
                token_limit,
            ) {
                Ok(result) if !result.trim().is_empty() => {
                    Some(strip_invisible_chars(&result))
                }
                _ => None,
            };
        }
        #[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
        {
            debug!("Apple Intelligence selected on unsupported platform");
            return None;
        }
    }

    match crate::llm_client::send_chat_completion_with_schema(
        &provider,
        api_key.clone(),
        &model,
        user_content.clone(),
        Some(system_prompt.clone()),
        // No structured schema for transforms / command mode — they
        // want free-form rewrites, not a JSON envelope.
        None,
        reasoning_effort.clone(),
        reasoning.clone(),
    )
    .await
    {
        Ok(Some(content)) if !content.trim().is_empty() => Some(strip_invisible_chars(&content)),
        Ok(_) => {
            warn!("LLM returned empty content for '{}'", provider.id);
            None
        }
        Err(e) => {
            // Fall back to the legacy /chat/completions path: some
            // local servers (older llama.cpp builds) don't accept the
            // schema-aware endpoint.
            warn!(
                "Structured chat completion failed ({}); retrying legacy endpoint",
                e
            );
            let legacy_prompt =
                format!("{}\n\n{}", system_prompt.trim(), user_content.trim());
            match crate::llm_client::send_chat_completion(
                &provider,
                api_key,
                &model,
                legacy_prompt,
                reasoning_effort,
                reasoning,
            )
            .await
            {
                Ok(Some(content)) if !content.trim().is_empty() => {
                    Some(strip_invisible_chars(&content))
                }
                Ok(_) => None,
                Err(err) => {
                    error!("Legacy LLM call also failed: {}", err);
                    None
                }
            }
        }
    }
}

pub(crate) async fn process_transcription_output(
    app: &AppHandle,
    transcription: &str,
    post_process: bool,
) -> ProcessedTranscription {
    // Phase Finalize.A — Transforms and Command Mode set this cell
    // during their start() handler so we know to take a different
    // post-processing path. Default is `Standard` (the regular
    // auto-cleanup pipeline below).
    let next = take_next_processing();

    let settings = get_settings(app);
    let mut final_text = transcription.to_string();
    let mut post_processed_text: Option<String> = None;
    let mut post_process_prompt: Option<String> = None;

    if let Some(converted_text) = maybe_convert_chinese_variant(&settings, transcription).await {
        final_text = converted_text;
    }

    // Phase 3.2: snippet expansion happens BEFORE the LLM cleanup pass
    // so that the LLM sees the expanded text and can polish grammar
    // around the substitution. No-op when the snippets list is empty.
    // Snippets run for Standard + Transform paths but NOT Command
    // Mode (the dictated instruction is meta-text, not user content).
    let run_snippets = matches!(
        next,
        NextProcessing::Standard | NextProcessing::Transform { .. }
    );
    if run_snippets && !settings.snippets.is_empty() {
        final_text = crate::managers::snippets::expand(&final_text, &settings.snippets);
    }

    // Phase 6 (Vibe Coding) — per-dictation per-app routing. We snapshot
    // the focused app exactly once here, before any LLM call, so the
    // routing decision is consistent for the rest of this dictation.
    // `active_app::detect()` is cheap (≤30 ms) and never panics.
    //
    // When the focused app is a known IDE or coding-agent terminal,
    // `vibe_coding::apply` rewrites the text in place — file-tagging in
    // both contexts, backtick wrapping of identifiers in IDEs only. On
    // every other app (browsers, mail, docs…) `apply` is a no-op.
    //
    // Phase Finalize.C — `known_symbols` is filled by the
    // accessibility-tree probe when supported (Windows UI Automation
    // today; macOS / Linux still empty). The function tolerates an
    // empty list and simply skips the variable-recognition pass.
    //
    // Vibe Coding does NOT run for Transforms or Command Mode — the
    // user explicitly opted into a rewrite there, and applying
    // identifier markup on top would be surprising.
    if matches!(next, NextProcessing::Standard) && settings.vibe_coding_enabled {
        let active_app = crate::active_app::detect();
        if !active_app.is_empty() {
            let symbols = crate::symbols::extract_visible_symbols(&active_app);
            let rewritten = crate::vibe_coding::apply(&final_text, &active_app, &symbols);
            if rewritten != final_text {
                debug!(
                    "Vibe Coding rewrote transcription for app '{}' ({} → {} chars, {} symbols)",
                    active_app.process_name,
                    final_text.len(),
                    rewritten.len(),
                    symbols.len()
                );
                final_text = rewritten;
            }
        }
    }

    // Phase Finalize.A — Transform and Command Mode short-circuit the
    // standard cleanup pipeline below. They run their own LLM call
    // with an explicit prompt and return.
    match next {
        NextProcessing::Transform { transform_id } => {
            if let Some(transform) =
                crate::transforms::find_by_id(&settings.transforms, &transform_id)
            {
                info!(
                    "Applying transform '{}' to {} char dictation",
                    transform.name,
                    final_text.len()
                );
                let user_content = crate::transforms::apply_template(transform, &final_text);
                // System prompt is intentionally minimal — the
                // transform's body already encodes the instruction.
                let system =
                    "You apply the user's prompt to the text they wrote. \
                     Output ONLY the rewritten text — no commentary, no quotes.".to_string();
                if let Some(rewritten) =
                    run_llm_with_prompts(&settings, system, user_content).await
                {
                    post_processed_text = Some(rewritten.clone());
                    post_process_prompt = Some(transform.prompt.clone());
                    final_text = rewritten;
                } else {
                    warn!(
                        "Transform '{}' LLM call returned nothing; pasting raw dictation",
                        transform.name
                    );
                }
            } else {
                warn!(
                    "Transform '{}' not found (deleted?); pasting raw dictation",
                    transform_id
                );
            }
            return ProcessedTranscription {
                final_text,
                post_processed_text,
                post_process_prompt,
            };
        }
        NextProcessing::CommandMode { selection } => {
            match crate::command_mode::prepare(selection.clone(), final_text.clone()) {
                Ok(request) => {
                    let (system, user) = crate::command_mode::build_prompts(&request);
                    if let Some(rewritten) =
                        run_llm_with_prompts(&settings, system, user).await
                    {
                        if crate::command_mode::is_no_op(&request.selection, &rewritten) {
                            info!("Command Mode: rewrite identical to selection; no-op");
                            // Final text matches selection so paste is
                            // effectively idempotent.
                            final_text = request.selection.clone();
                        } else {
                            final_text = rewritten.clone();
                        }
                        post_processed_text = Some(final_text.clone());
                        post_process_prompt = Some(format!(
                            "[Command Mode] Apply instruction \"{}\" to selection",
                            request.instruction
                        ));
                    } else {
                        warn!(
                            "Command Mode LLM call returned nothing; restoring selection unchanged"
                        );
                        final_text = request.selection.clone();
                    }
                }
                Err(e) => {
                    warn!("Command Mode validation rejected request: {}", e);
                    // Surface the error to the user via the existing
                    // recording-error event channel.
                    let _ = app.emit(
                        "command-mode-error",
                        RecordingErrorEvent {
                            error_type: "validation".to_string(),
                            detail: Some(e.to_string()),
                        },
                    );
                    // Restore selection so the user's clipboard / paste
                    // doesn't accidentally end up as the dictated text.
                    final_text = selection;
                }
            }
            return ProcessedTranscription {
                final_text,
                post_processed_text,
                post_process_prompt,
            };
        }
        NextProcessing::Standard => {} // fall through to the existing pipeline
    }

    if post_process {
        // Phase Finalize.D — compute the per-app style fragment for
        // the cleanup LLM. Snapshotting the active app HERE (separate
        // from the Vibe Coding snapshot above) keeps the style call
        // and the routing call decoupled, but in practice both look at
        // the same focused window.
        let style_fragment = {
            let app_snap = crate::active_app::detect();
            let preset = crate::style_presets::effective_preset(
                &app_snap,
                &settings.style_overrides,
            );
            preset.system_fragment().to_string()
        };
        if let Some(processed_text) = post_process_transcription(
            &settings,
            &final_text,
            Some(&style_fragment),
        )
        .await
        {
            post_processed_text = Some(processed_text.clone());
            final_text = processed_text;

            if let Some(prompt_id) = &settings.post_process_selected_prompt_id {
                if let Some(prompt) = settings
                    .post_process_prompts
                    .iter()
                    .find(|prompt| &prompt.id == prompt_id)
                {
                    post_process_prompt = Some(prompt.prompt.clone());
                }
            }
        }
    } else if final_text != transcription {
        post_processed_text = Some(final_text.clone());
    }

    ProcessedTranscription {
        final_text,
        post_processed_text,
        post_process_prompt,
    }
}

impl ShortcutAction for TranscribeAction {
    fn start(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        let start_time = Instant::now();
        debug!("TranscribeAction::start called for binding: {}", binding_id);

        // Load model in the background
        let tm = app.state::<Arc<TranscriptionManager>>();
        let rm = app.state::<Arc<AudioRecordingManager>>();

        // Load ASR model and VAD model in parallel
        tm.initiate_model_load();
        let rm_clone = Arc::clone(&rm);
        std::thread::spawn(move || {
            if let Err(e) = rm_clone.preload_vad() {
                debug!("VAD pre-load failed: {}", e);
            }
        });

        let binding_id = binding_id.to_string();
        change_tray_icon(app, TrayIconState::Recording);
        show_recording_overlay(app);

        // Get the microphone mode to determine audio feedback timing
        let settings = get_settings(app);
        let is_always_on = settings.always_on_microphone;
        debug!("Microphone mode - always_on: {}", is_always_on);

        let mut recording_error: Option<String> = None;
        if is_always_on {
            // Always-on mode: Play audio feedback immediately, then apply mute after sound finishes
            debug!("Always-on mode: Playing audio feedback immediately");
            let rm_clone = Arc::clone(&rm);
            let app_clone = app.clone();
            // The blocking helper exits immediately if audio feedback is disabled,
            // so we can always reuse this thread to ensure mute happens right after playback.
            std::thread::spawn(move || {
                play_feedback_sound_blocking(&app_clone, SoundType::Start);
                rm_clone.apply_mute();
            });

            if let Err(e) = rm.try_start_recording(&binding_id) {
                debug!("Recording failed: {}", e);
                recording_error = Some(e);
            }
        } else {
            // On-demand mode: Start recording first, then play audio feedback, then apply mute
            // This allows the microphone to be activated before playing the sound
            debug!("On-demand mode: Starting recording first, then audio feedback");
            let recording_start_time = Instant::now();
            match rm.try_start_recording(&binding_id) {
                Ok(()) => {
                    debug!("Recording started in {:?}", recording_start_time.elapsed());
                    // Small delay to ensure microphone stream is active
                    let app_clone = app.clone();
                    let rm_clone = Arc::clone(&rm);
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        debug!("Handling delayed audio feedback/mute sequence");
                        // Helper handles disabled audio feedback by returning early, so we reuse it
                        // to keep mute sequencing consistent in every mode.
                        play_feedback_sound_blocking(&app_clone, SoundType::Start);
                        rm_clone.apply_mute();
                    });
                }
                Err(e) => {
                    debug!("Failed to start recording: {}", e);
                    recording_error = Some(e);
                }
            }
        }

        if recording_error.is_none() {
            // Dynamically register the cancel shortcut in a separate task to avoid deadlock
            shortcut::register_cancel_shortcut(app);
        } else {
            // Starting failed (for example due to blocked microphone permissions).
            // Revert UI state so we don't stay stuck in the recording overlay.
            utils::hide_recording_overlay(app);
            change_tray_icon(app, TrayIconState::Idle);
            if let Some(err) = recording_error {
                let error_type = if is_microphone_access_denied(&err) {
                    "microphone_permission_denied"
                } else if is_no_input_device_error(&err) {
                    "no_input_device"
                } else {
                    "unknown"
                };
                let _ = app.emit(
                    "recording-error",
                    RecordingErrorEvent {
                        error_type: error_type.to_string(),
                        detail: Some(err),
                    },
                );
            }
        }

        debug!(
            "TranscribeAction::start completed in {:?}",
            start_time.elapsed()
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        // Unregister the cancel shortcut when transcription stops
        shortcut::unregister_cancel_shortcut(app);

        let stop_time = Instant::now();
        debug!("TranscribeAction::stop called for binding: {}", binding_id);

        let ah = app.clone();
        let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
        let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());
        let hm = Arc::clone(&app.state::<Arc<HistoryManager>>());

        change_tray_icon(app, TrayIconState::Transcribing);
        show_transcribing_overlay(app);

        // Unmute before playing audio feedback so the stop sound is audible
        rm.remove_mute();

        // Play audio feedback for recording stop
        play_feedback_sound(app, SoundType::Stop);

        let binding_id = binding_id.to_string(); // Clone binding_id for the async task
        let post_process = self.post_process;

        tauri::async_runtime::spawn(async move {
            let _guard = FinishGuard(ah.clone());
            debug!(
                "Starting async transcription task for binding: {}",
                binding_id
            );

            let stop_recording_time = Instant::now();
            if let Some(samples) = rm.stop_recording(&binding_id) {
                debug!(
                    "Recording stopped and samples retrieved in {:?}, sample count: {}",
                    stop_recording_time.elapsed(),
                    samples.len()
                );

                if samples.is_empty() {
                    debug!("Recording produced no audio samples; skipping persistence");
                    utils::hide_recording_overlay(&ah);
                    change_tray_icon(&ah, TrayIconState::Idle);
                } else {
                    // Save WAV concurrently with transcription
                    let sample_count = samples.len();
                    let file_name = format!("handy-{}.wav", chrono::Utc::now().timestamp());
                    let wav_path = hm.recordings_dir().join(&file_name);
                    let wav_path_for_verify = wav_path.clone();
                    let samples_for_wav = samples.clone();
                    let wav_handle = tauri::async_runtime::spawn_blocking(move || {
                        crate::audio_toolkit::save_wav_file(&wav_path, &samples_for_wav)
                    });

                    // Transcribe concurrently with WAV save
                    let transcription_time = Instant::now();
                    let transcription_result = tm.transcribe(samples);

                    // Await WAV save and verify
                    let wav_saved = match wav_handle.await {
                        Ok(Ok(())) => {
                            match crate::audio_toolkit::verify_wav_file(
                                &wav_path_for_verify,
                                sample_count,
                            ) {
                                Ok(()) => true,
                                Err(e) => {
                                    error!("WAV verification failed: {}", e);
                                    false
                                }
                            }
                        }
                        Ok(Err(e)) => {
                            error!("Failed to save WAV file: {}", e);
                            false
                        }
                        Err(e) => {
                            error!("WAV save task panicked: {}", e);
                            false
                        }
                    };

                    match transcription_result {
                        Ok(transcription) => {
                            debug!(
                                "Transcription completed in {:?}: '{}'",
                                transcription_time.elapsed(),
                                transcription
                            );

                            if post_process {
                                show_processing_overlay(&ah);
                            }
                            let processed =
                                process_transcription_output(&ah, &transcription, post_process)
                                    .await;

                            // Save to history if WAV was saved
                            if wav_saved {
                                if let Err(err) = hm.save_entry(
                                    file_name,
                                    transcription,
                                    post_process,
                                    processed.post_processed_text.clone(),
                                    processed.post_process_prompt.clone(),
                                ) {
                                    error!("Failed to save history entry: {}", err);
                                }
                            }

                            if processed.final_text.is_empty() {
                                utils::hide_recording_overlay(&ah);
                                change_tray_icon(&ah, TrayIconState::Idle);
                            } else {
                                let ah_clone = ah.clone();
                                let paste_time = Instant::now();
                                let final_text = processed.final_text;
                                ah.run_on_main_thread(move || {
                                    match utils::paste(final_text, ah_clone.clone()) {
                                        Ok(()) => debug!(
                                            "Text pasted successfully in {:?}",
                                            paste_time.elapsed()
                                        ),
                                        Err(e) => {
                                            error!("Failed to paste transcription: {}", e);
                                            let _ = ah_clone.emit("paste-error", ());
                                        }
                                    }
                                    utils::hide_recording_overlay(&ah_clone);
                                    change_tray_icon(&ah_clone, TrayIconState::Idle);
                                })
                                .unwrap_or_else(|e| {
                                    error!("Failed to run paste on main thread: {:?}", e);
                                    utils::hide_recording_overlay(&ah);
                                    change_tray_icon(&ah, TrayIconState::Idle);
                                });
                            }
                        }
                        Err(err) => {
                            debug!("Global Shortcut Transcription error: {}", err);
                            // Save entry with empty text so user can retry
                            if wav_saved {
                                if let Err(save_err) = hm.save_entry(
                                    file_name,
                                    String::new(),
                                    post_process,
                                    None,
                                    None,
                                ) {
                                    error!("Failed to save failed history entry: {}", save_err);
                                }
                            }
                            utils::hide_recording_overlay(&ah);
                            change_tray_icon(&ah, TrayIconState::Idle);
                        }
                    }
                }
            } else {
                debug!("No samples retrieved from recording stop");
                utils::hide_recording_overlay(&ah);
                change_tray_icon(&ah, TrayIconState::Idle);
            }
        });

        debug!(
            "TranscribeAction::stop completed in {:?}",
            stop_time.elapsed()
        );
    }
}

// Cancel Action
struct CancelAction;

impl ShortcutAction for CancelAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        utils::cancel_current_operation(app);
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // Nothing to do on stop for cancel
    }
}

// Test Action
struct TestAction;

impl ShortcutAction for TestAction {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        log::info!(
            "Shortcut ID '{}': Started - {} (App: {})", // Changed "Pressed" to "Started" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        log::info!(
            "Shortcut ID '{}': Stopped - {} (App: {})", // Changed "Released" to "Stopped" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }
}

// ============================================================================
// Phase Finalize.A — Transform + Command Mode actions
// ============================================================================
//
// Both delegate to the standard recording pipeline via TranscribeAction
// for everything except the post-transcription step. The strategy
// switch happens in `process_transcription_output` via the
// `NEXT_PROCESSING` cell set in the corresponding action's start().

/// Prefix used in binding IDs to identify per-transform hotkeys.
/// A binding with id `"transform:abc123"` runs the transform whose
/// `id == "abc123"`. We use a colon because UUIDs / user-chosen IDs
/// never contain colons.
pub const TRANSFORM_BINDING_PREFIX: &str = "transform:";

/// Binding id reserved for Command Mode.
pub const COMMAND_MODE_BINDING_ID: &str = "command_mode";

fn extract_transform_id(binding_id: &str) -> Option<&str> {
    binding_id.strip_prefix(TRANSFORM_BINDING_PREFIX)
}

/// Hotkey-bound transform — exhibits the same press-to-record /
/// release-to-process flow as the standard Transcribe binding, but the
/// transcribed text is passed through the transform's prompt to the
/// LLM before being pasted.
struct TransformAction {
    transform_id: String,
}

impl ShortcutAction for TransformAction {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        set_next_processing(NextProcessing::Transform {
            transform_id: self.transform_id.clone(),
        });
        // Reuse the standard transcribe path (which runs the recording
        // pipeline). The strategy in NEXT_PROCESSING flips the
        // post-transcription branch when stop() finishes the
        // transcription.
        TranscribeAction { post_process: true }.start(app, binding_id, shortcut_str);
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        TranscribeAction { post_process: true }.stop(app, binding_id, shortcut_str);
    }
}

/// Command Mode — Wispr Flow's headline feature. On press we capture
/// the user's selection via Ctrl/Cmd+C and stash it; on release we
/// transcribe the spoken instruction, build the (selection +
/// instruction) prompt, run it through the LLM, and paste the rewrite
/// over the original selection.
struct CommandModeAction;

impl ShortcutAction for CommandModeAction {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        let selection = capture_selection_via_clipboard(app).unwrap_or_default();
        if selection.trim().is_empty() {
            warn!(
                "Command Mode pressed but no selection captured (clipboard empty). \
                 Continuing — the LLM step will short-circuit with NoSelection."
            );
        } else {
            debug!(
                "Command Mode captured {} chars of selection",
                selection.len()
            );
        }
        set_next_processing(NextProcessing::CommandMode { selection });
        TranscribeAction { post_process: true }.start(app, binding_id, shortcut_str);
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        TranscribeAction { post_process: true }.stop(app, binding_id, shortcut_str);
    }
}

/// Capture the user's current selection by simulating Ctrl/Cmd+C and
/// reading the clipboard. Restores the previous clipboard contents
/// before returning so the user's clipboard isn't trashed.
///
/// Returns `None` only when the Enigo state isn't available (unusual —
/// it's initialised at app startup); an empty selection returns
/// `Some("")` so the caller can decide how to handle it.
fn capture_selection_via_clipboard(app: &AppHandle) -> Option<String> {
    use tauri_plugin_clipboard_manager::ClipboardExt;

    let enigo_state = app.try_state::<crate::input::EnigoState>()?;
    // Save whatever's currently on the clipboard so we can restore it.
    let clipboard = app.clipboard();
    let prior = clipboard.read_text().ok();

    // Empty the clipboard so we can tell whether the Ctrl+C actually
    // produced a copy — some apps swallow Ctrl+C on empty selections.
    let _ = clipboard.write_text(String::new());

    {
        let mut enigo = enigo_state.0.lock().ok()?;
        if let Err(e) = crate::input::send_copy_ctrl_c(&mut *enigo) {
            warn!("Failed to send Ctrl+C for Command Mode capture: {}", e);
            // Restore the prior clipboard and bail.
            if let Some(prev) = prior {
                let _ = clipboard.write_text(prev);
            }
            return Some(String::new());
        }
    }

    // Give the focused app a beat to fulfil the copy. 80 ms matches
    // the safe-default we use elsewhere (clipboard.rs paste path).
    std::thread::sleep(std::time::Duration::from_millis(80));

    let captured = clipboard.read_text().unwrap_or_default();
    // Restore prior clipboard. We do this even when capture succeeded
    // because the user's clipboard belongs to them, not us.
    if let Some(prev) = prior {
        let _ = clipboard.write_text(prev);
    }

    Some(captured)
}

/// Dynamic action dispatcher used by the shortcut handler + the
/// transcription coordinator. Returns an `Arc<dyn ShortcutAction>`:
///
/// - for the static IDs in `ACTION_MAP` — returns the cached instance.
/// - for `"command_mode"` — returns a fresh `CommandModeAction`.
/// - for `"transform:<id>"` — returns a fresh `TransformAction` keyed
///   to the trailing id (no validation against `settings.transforms`
///   here; the post-processing step handles a missing transform
///   gracefully).
///
/// Returning `None` only when the binding id is unknown — callers log
/// and ignore.
pub fn lookup_action(binding_id: &str) -> Option<Arc<dyn ShortcutAction>> {
    if let Some(action) = ACTION_MAP.get(binding_id) {
        return Some(Arc::clone(action));
    }
    if binding_id == COMMAND_MODE_BINDING_ID {
        return Some(Arc::new(CommandModeAction) as Arc<dyn ShortcutAction>);
    }
    if let Some(transform_id) = extract_transform_id(binding_id) {
        return Some(Arc::new(TransformAction {
            transform_id: transform_id.to_string(),
        }) as Arc<dyn ShortcutAction>);
    }
    None
}

// Static Action Map
pub static ACTION_MAP: Lazy<HashMap<String, Arc<dyn ShortcutAction>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "transcribe".to_string(),
        Arc::new(TranscribeAction {
            post_process: false,
        }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_with_post_process".to_string(),
        Arc::new(TranscribeAction { post_process: true }) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "cancel".to_string(),
        Arc::new(CancelAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "test".to_string(),
        Arc::new(TestAction) as Arc<dyn ShortcutAction>,
    );
    map
});
