//! Tauri commands that expose the OpenWhisper-added backend modules to
//! the frontend.
//!
//! Phase 9.bridge of the OpenWhisper roadmap. Bundling all the
//! "wire-up" commands in one module keeps `commands/mod.rs` from
//! growing past its already-substantial size and gives the audit
//! reviewer a single place to scan for "what does the OpenWhisper
//! frontend get to call?".
//!
//! Each section maps to one of the modules added in Phases 1.x – 8.2.
//! Commands are intentionally thin: every one is a few lines that
//! either read settings + transforms / appends / writes back, or
//! delegates to a pure backend function. All business logic lives in
//! the module being wrapped — the commands are just FFI plumbing.

use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::managers::dictionary::{DictionaryEntry, DictionaryEntrySource};
use crate::managers::history::HistoryManager;
use crate::managers::insights::{compute as compute_voice_profile, VoiceProfile};
use crate::managers::notes::{Note, NotesStore};
use crate::managers::snippets::{Snippet, MAX_BULK_IMPORT};
use crate::settings::{get_settings, write_settings};
use crate::transforms::{
    apply_template as apply_transform_template, find_by_id as find_transform_by_id, new as new_transform,
    TransformBinding, MAX_TRANSFORMS,
};

// ============================================================================
// Insights / Voice Profile (Phase 8.1)
// ============================================================================

/// Compute the Voice Profile dashboard data from the local history DB.
/// Pure read — does not mutate any state. Safe to call repeatedly.
#[specta::specta]
#[tauri::command]
pub async fn get_voice_profile(app: AppHandle) -> Result<VoiceProfile, String> {
    let history = app.state::<Arc<HistoryManager>>().inner().clone();
    let entries = history
        .get_all_entries()
        .map_err(|e| format!("failed to load history for voice profile: {}", e))?;
    Ok(compute_voice_profile(&entries))
}

// ============================================================================
// Notes / Scratchpad (Phase 8.2)
// ============================================================================

fn notes_store(app: &AppHandle) -> Result<NotesStore, String> {
    let dir = crate::portable::app_data_dir(app)
        .map_err(|e| format!("failed to resolve app data dir: {}", e))?;
    NotesStore::new(&dir).map_err(|e| format!("failed to open notes store: {}", e))
}

#[specta::specta]
#[tauri::command]
pub fn list_notes(app: AppHandle) -> Result<Vec<Note>, String> {
    notes_store(&app)?
        .list()
        .map_err(|e| format!("failed to list notes: {}", e))
}

#[specta::specta]
#[tauri::command]
pub fn get_note(app: AppHandle, id: String) -> Result<Note, String> {
    notes_store(&app)?
        .get(&id)
        .map_err(|e| format!("failed to load note '{}': {}", id, e))
}

#[specta::specta]
#[tauri::command]
pub fn create_note(app: AppHandle, title: String, body: String) -> Result<Note, String> {
    notes_store(&app)?
        .create(title, body)
        .map_err(|e| format!("failed to create note: {}", e))
}

#[specta::specta]
#[tauri::command]
pub fn update_note(
    app: AppHandle,
    id: String,
    title: String,
    body: String,
) -> Result<Note, String> {
    notes_store(&app)?
        .update(&id, title, body)
        .map_err(|e| format!("failed to update note '{}': {}", id, e))
}

#[specta::specta]
#[tauri::command]
pub fn delete_note(app: AppHandle, id: String) -> Result<(), String> {
    notes_store(&app)?
        .delete(&id)
        .map_err(|e| format!("failed to delete note '{}': {}", id, e))
}

// ============================================================================
// Snippets (Phase 3.1 / 3.2)
// ============================================================================

#[specta::specta]
#[tauri::command]
pub fn list_snippets(app: AppHandle) -> Result<Vec<Snippet>, String> {
    Ok(get_settings(&app).snippets)
}

#[specta::specta]
#[tauri::command]
pub fn add_snippet(
    app: AppHandle,
    id: String,
    trigger: String,
    expansion: String,
) -> Result<Snippet, String> {
    let snippet =
        Snippet::new(id, trigger, expansion).map_err(|e| format!("invalid snippet: {}", e))?;
    let mut settings = get_settings(&app);
    if settings.snippets.iter().any(|s| s.id == snippet.id) {
        return Err(format!("snippet with id '{}' already exists", snippet.id));
    }
    settings.snippets.push(snippet.clone());
    write_settings(&app, settings);
    Ok(snippet)
}

#[specta::specta]
#[tauri::command]
pub fn update_snippet(
    app: AppHandle,
    id: String,
    trigger: String,
    expansion: String,
) -> Result<Snippet, String> {
    let mut settings = get_settings(&app);
    let snippet = settings
        .snippets
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("snippet '{}' not found", id))?;
    // Reuse the validator for trigger + expansion limits.
    let validated = Snippet::new(snippet.id.clone(), trigger, expansion)
        .map_err(|e| format!("invalid snippet update: {}", e))?;
    snippet.trigger = validated.trigger;
    snippet.expansion = validated.expansion;
    let result = snippet.clone();
    write_settings(&app, settings);
    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub fn delete_snippet(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let before = settings.snippets.len();
    settings.snippets.retain(|s| s.id != id);
    if settings.snippets.len() == before {
        return Err(format!("snippet '{}' not found", id));
    }
    write_settings(&app, settings);
    Ok(())
}

/// Bulk replace the entire snippets list — used by the JSON import
/// flow in the settings UI. Enforces the documented `MAX_BULK_IMPORT`
/// cap.
#[specta::specta]
#[tauri::command]
pub fn bulk_set_snippets(app: AppHandle, snippets: Vec<Snippet>) -> Result<(), String> {
    if snippets.len() > MAX_BULK_IMPORT {
        return Err(format!(
            "too many snippets in import: {} (max {})",
            snippets.len(),
            MAX_BULK_IMPORT
        ));
    }
    // Re-validate each entry so the import boundary can't smuggle past
    // the trigger/expansion length caps.
    for s in &snippets {
        Snippet::new(s.id.clone(), s.trigger.clone(), s.expansion.clone())
            .map_err(|e| format!("invalid snippet '{}' in import: {}", s.id, e))?;
    }
    let mut settings = get_settings(&app);
    settings.snippets = snippets;
    write_settings(&app, settings);
    Ok(())
}

// ============================================================================
// Personal Dictionary (Phase 2.1)
// ============================================================================

#[specta::specta]
#[tauri::command]
pub fn list_dictionary_entries(app: AppHandle) -> Result<Vec<DictionaryEntry>, String> {
    Ok(get_settings(&app).dictionary_entries)
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DictionaryEntryInput {
    pub term: String,
    #[serde(default)]
    pub replacement: Option<String>,
    #[serde(default)]
    pub starred: bool,
}

#[specta::specta]
#[tauri::command]
pub fn add_dictionary_entry(
    app: AppHandle,
    input: DictionaryEntryInput,
) -> Result<DictionaryEntry, String> {
    let term = input.term.trim().to_string();
    if term.is_empty() {
        return Err("dictionary term must not be empty".to_string());
    }
    let entry = DictionaryEntry {
        term,
        replacement: input.replacement.filter(|r| !r.trim().is_empty()),
        starred: input.starred,
        source: DictionaryEntrySource::Manual,
    };
    let mut settings = get_settings(&app);
    if settings.dictionary_entries.iter().any(|e| e.term == entry.term) {
        return Err(format!("dictionary entry '{}' already exists", entry.term));
    }
    settings.dictionary_entries.push(entry.clone());
    write_settings(&app, settings);
    Ok(entry)
}

#[specta::specta]
#[tauri::command]
pub fn update_dictionary_entry(
    app: AppHandle,
    term: String,
    input: DictionaryEntryInput,
) -> Result<DictionaryEntry, String> {
    let mut settings = get_settings(&app);
    let entry = settings
        .dictionary_entries
        .iter_mut()
        .find(|e| e.term == term)
        .ok_or_else(|| format!("dictionary entry '{}' not found", term))?;
    let new_term = input.term.trim().to_string();
    if new_term.is_empty() {
        return Err("dictionary term must not be empty".to_string());
    }
    entry.term = new_term;
    entry.replacement = input.replacement.filter(|r| !r.trim().is_empty());
    entry.starred = input.starred;
    let result = entry.clone();
    write_settings(&app, settings);
    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub fn delete_dictionary_entry(app: AppHandle, term: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let before = settings.dictionary_entries.len();
    settings.dictionary_entries.retain(|e| e.term != term);
    if settings.dictionary_entries.len() == before {
        return Err(format!("dictionary entry '{}' not found", term));
    }
    write_settings(&app, settings);
    Ok(())
}

/// Bulk replace the entire dictionary — used by the JSON import flow.
/// Matches the snippets cap of `MAX_BULK_IMPORT` entries.
#[specta::specta]
#[tauri::command]
pub fn bulk_set_dictionary_entries(
    app: AppHandle,
    entries: Vec<DictionaryEntryInput>,
) -> Result<(), String> {
    if entries.len() > MAX_BULK_IMPORT {
        return Err(format!(
            "too many entries in import: {} (max {})",
            entries.len(),
            MAX_BULK_IMPORT
        ));
    }
    let mut converted = Vec::with_capacity(entries.len());
    for input in entries {
        let term = input.term.trim().to_string();
        if term.is_empty() {
            return Err("dictionary term must not be empty".to_string());
        }
        converted.push(DictionaryEntry {
            term,
            replacement: input.replacement.filter(|r| !r.trim().is_empty()),
            starred: input.starred,
            source: DictionaryEntrySource::Imported,
        });
    }
    let mut settings = get_settings(&app);
    settings.dictionary_entries = converted;
    write_settings(&app, settings);
    Ok(())
}

// ============================================================================
// Transforms (Phase 7.2)
// ============================================================================

#[specta::specta]
#[tauri::command]
pub fn list_transforms(app: AppHandle) -> Result<Vec<TransformBinding>, String> {
    Ok(get_settings(&app).transforms)
}

#[specta::specta]
#[tauri::command]
pub fn add_transform(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
    hotkey: Option<String>,
) -> Result<TransformBinding, String> {
    let mut transform =
        new_transform(id, name, prompt).map_err(|e| format!("invalid transform: {}", e))?;
    transform.hotkey = hotkey;
    let mut settings = get_settings(&app);
    if settings.transforms.len() >= MAX_TRANSFORMS {
        return Err(format!(
            "cannot add transform: already at {} (cap {})",
            settings.transforms.len(),
            MAX_TRANSFORMS
        ));
    }
    if settings.transforms.iter().any(|t| t.id == transform.id) {
        return Err(format!("transform with id '{}' already exists", transform.id));
    }
    settings.transforms.push(transform.clone());
    write_settings(&app, settings);
    Ok(transform)
}

#[specta::specta]
#[tauri::command]
pub fn update_transform(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
    hotkey: Option<String>,
) -> Result<TransformBinding, String> {
    let mut settings = get_settings(&app);
    let transform = settings
        .transforms
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("transform '{}' not found", id))?;
    // Re-validate via the constructor to enforce the name + prompt caps.
    let validated = new_transform(id.clone(), name, prompt)
        .map_err(|e| format!("invalid transform update: {}", e))?;
    transform.name = validated.name;
    transform.prompt = validated.prompt;
    transform.hotkey = hotkey;
    let result = transform.clone();
    write_settings(&app, settings);
    Ok(result)
}

#[specta::specta]
#[tauri::command]
pub fn delete_transform(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);
    let before = settings.transforms.len();
    settings.transforms.retain(|t| t.id != id);
    if settings.transforms.len() == before {
        return Err(format!("transform '{}' not found", id));
    }
    write_settings(&app, settings);
    Ok(())
}

/// Render the (system, user) prompt pair for a given transform applied
/// to the supplied text. Frontend uses this to surface a "preview the
/// prompt" affordance in the transforms settings page.
#[specta::specta]
#[tauri::command]
pub fn preview_transform_prompt(
    app: AppHandle,
    id: String,
    text: String,
) -> Result<String, String> {
    let settings = get_settings(&app);
    let transform = find_transform_by_id(&settings.transforms, &id)
        .ok_or_else(|| format!("transform '{}' not found", id))?;
    Ok(apply_transform_template(transform, &text))
}

// ============================================================================
// Vibe Coding (Phase 6)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct VibeCodingResult {
    /// Transformed text after file-tagging + variable-recognition passes.
    pub text: String,
    /// Which active-app classification the resolver picked.
    pub context: crate::vibe_coding::VibeContext,
}

/// Detect the currently-focused app and apply the Vibe Coding pipeline
/// to `text`. Frontend can call this for the post-dictation preview
/// even when Vibe Coding isn't part of the live dictation flow.
///
/// `known_symbols` should be the identifier list extracted from the
/// focused editor (queued for the Phase 6.1b accessibility-tree
/// reader). For now any caller can supply a manual list.
#[specta::specta]
#[tauri::command]
pub fn apply_vibe_coding(text: String, known_symbols: Vec<String>) -> Result<VibeCodingResult, String> {
    let app = crate::active_app::detect();
    let context = crate::vibe_coding::classify(&app);
    let transformed = crate::vibe_coding::apply(&text, &app, &known_symbols);
    Ok(VibeCodingResult {
        text: transformed,
        context,
    })
}

/// Light-weight active-app probe exposed to the frontend for the
/// "what is OpenWhisper looking at" debug panel.
#[specta::specta]
#[tauri::command]
pub fn get_active_app() -> Result<crate::active_app::ActiveApp, String> {
    Ok(crate::active_app::detect())
}

// ============================================================================
// Command Mode (Phase 7.1)
// ============================================================================

/// Validate a Command Mode (selection, instruction) pair. The frontend
/// calls this BEFORE dispatching the LLM call so it can show the user
/// a useful error if the selection is too long or empty. The actual
/// LLM dispatch goes through the regular post-process pipeline once
/// validation passes.
#[specta::specta]
#[tauri::command]
pub fn prepare_command_mode(
    selection: String,
    instruction: String,
) -> Result<crate::command_mode::CommandModeRequest, String> {
    crate::command_mode::prepare(selection, instruction).map_err(|e| format!("{}", e))
}
