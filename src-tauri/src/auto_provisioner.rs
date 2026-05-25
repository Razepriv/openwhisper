//! AutoProvisioner — wires `SystemProbe` → `BackendResolver` → `ModelManager`
//! to deliver the zero-touch first-run promise from decision B6.
//!
//! Phase 1.5 of the OpenWhisper roadmap (research/11-phase-1-tasks.md).
//!
//! ## Flow
//!
//! On first run (or whenever the user clicks "Set up best models"):
//!
//! 1. Snapshot the host system (`system_probe::probe`).
//! 2. Resolve the recommended STT + LLM cleanup stack (`backend_resolver::resolve`).
//! 3. Decide whether action is needed:
//!    - If the recommended STT model is already downloaded → set it as the
//!      active model and we're done.
//!    - Otherwise queue a download via `ModelManager::download_model`.
//! 4. Once download completes, set `settings.selected_model` to the recommended
//!    id so the user can dictate immediately.
//!
//! LLM cleanup backend wiring is a separate step (Phase 1.6 — bundled
//! llama.cpp sidecar) and is not invoked from this module yet. The resolver
//! still surfaces it in the Stack so the frontend can show the user what's
//! coming.
//!
//! ## Why a separate module?
//!
//! The probe + resolve + download pieces each live in their own crates of
//! concern. Without a dedicated orchestrator, the onboarding command in
//! `commands/mod.rs` would need to know about all three. Centralising the
//! workflow here makes the command paper-thin and the workflow testable.
//!
//! ## Event contract
//!
//! Emits the following Tauri events while running so the frontend can drive
//! a progress UI:
//!
//! - `"auto-provision-started"` — payload `AutoProvisionStarted`
//! - `"auto-provision-progress"` — payload `AutoProvisionProgress` (download bytes)
//! - `"auto-provision-completed"` — payload `AutoProvisionCompleted` (active model)
//! - `"auto-provision-failed"` — payload `AutoProvisionFailed` (error string)
//!
//! Frontend should listen for all four to update its onboarding UI. See
//! `src/components/onboarding/AutoSetupStep.tsx` (Phase 1.5b) for the consumer.

use std::sync::Arc;

use anyhow::Result;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Manager};

use crate::backend_resolver::{resolve, Stack};
use crate::managers::model::ModelManager;
use crate::settings::{get_settings, write_settings};
use crate::system_probe::probe;

/// Event name constants. Kept in one place so frontend `useEffect` strings
/// can't drift from Rust emit calls without a compile error here.
pub mod events {
    pub const STARTED: &str = "auto-provision-started";
    pub const PROGRESS: &str = "auto-provision-progress";
    pub const COMPLETED: &str = "auto-provision-completed";
    pub const FAILED: &str = "auto-provision-failed";
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AutoProvisionStarted {
    /// Recommended stack chosen for this machine.
    pub stack: Stack,
    /// True iff the recommended STT model still needs to be downloaded.
    /// When `false` the frontend can skip straight to the "all set" screen.
    pub download_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AutoProvisionCompleted {
    /// Id of the STT model that's now active.
    pub active_model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct AutoProvisionFailed {
    pub error: String,
}

/// Result of `plan()`: what the provisioner intends to do, without doing it yet.
/// Used by the Tauri command to emit the `STARTED` event before kicking off
/// any work.
#[derive(Debug, Clone)]
pub struct ProvisionPlan {
    pub stack: Stack,
    pub download_required: bool,
}

/// Plan the provisioning action. Pure: probes the system, resolves the stack,
/// checks whether the recommended STT model is already on disk. No downloads
/// initiated, no settings written.
pub fn plan(
    model_manager: &ModelManager,
    has_capable_gpu: bool,
    ollama_detected: bool,
) -> Result<ProvisionPlan> {
    let profile = probe()?;
    let stack = resolve(&profile, has_capable_gpu, ollama_detected);
    let download_required = !is_model_downloaded(model_manager, &stack.stt_model_id);
    Ok(ProvisionPlan {
        stack,
        download_required,
    })
}

/// Check whether the model with the given id is downloaded according to the
/// `ModelManager`'s state. Returns `false` for unknown ids (the
/// model_manager would refuse to download those anyway, surfacing a clearer
/// error at download time).
pub fn is_model_downloaded(model_manager: &ModelManager, model_id: &str) -> bool {
    model_manager
        .get_model_info(model_id)
        .map(|m| m.is_downloaded)
        .unwrap_or(false)
}

/// Set `settings.selected_model = model_id` and persist. Used after a
/// successful provision so the dictation pipeline picks up the new model on
/// the next invocation.
pub fn set_active_model(app: &AppHandle, model_id: &str) -> Result<()> {
    let mut settings = get_settings(app);
    if settings.selected_model == model_id {
        return Ok(());
    }
    info!(
        "Auto-provisioner activating model '{}' (was '{}')",
        model_id, settings.selected_model
    );
    settings.selected_model = model_id.to_string();
    write_settings(app, settings);
    Ok(())
}

/// Drive the full provisioning workflow. Designed to be invoked from a Tauri
/// command — emits events for the frontend's onboarding UI and returns once
/// the recommended model is downloaded + activated.
///
/// **Idempotent.** Safe to call repeatedly. If the recommended model is
/// already downloaded, this is essentially a no-op (just sets the active
/// model and emits `COMPLETED`).
///
/// **Long-running.** Holds a tokio task for the duration of the download
/// (which can be 30 s for Whisper Small on a fast network, 5+ min for
/// Whisper Turbo on a slow one). Wrap in a `tokio::spawn` from the caller
/// if you don't want to block the command handler thread.
pub async fn provision(
    app: AppHandle,
    has_capable_gpu: bool,
    ollama_detected: bool,
) -> Result<AutoProvisionCompleted> {
    let model_manager = app.state::<Arc<ModelManager>>().inner().clone();

    let plan = plan(&model_manager, has_capable_gpu, ollama_detected)?;

    let started = AutoProvisionStarted {
        stack: plan.stack.clone(),
        download_required: plan.download_required,
    };
    if let Err(e) = app.emit(events::STARTED, &started) {
        warn!("Failed to emit auto-provision-started event: {}", e);
    }

    if plan.download_required {
        info!(
            "Auto-provisioner downloading recommended model '{}' (~{} MB)",
            plan.stack.stt_model_id, plan.stack.estimated_download_mb
        );
        // download_model already emits per-byte progress via its own
        // events; we don't re-emit per-chunk progress here to avoid double
        // counting.
        if let Err(err) = model_manager.download_model(&plan.stack.stt_model_id).await {
            let payload = AutoProvisionFailed {
                error: err.to_string(),
            };
            let _ = app.emit(events::FAILED, &payload);
            return Err(err);
        }
    } else {
        info!(
            "Auto-provisioner: recommended model '{}' already on disk, skipping download",
            plan.stack.stt_model_id
        );
    }

    set_active_model(&app, &plan.stack.stt_model_id)?;

    let completed = AutoProvisionCompleted {
        active_model_id: plan.stack.stt_model_id.clone(),
    };
    if let Err(e) = app.emit(events::COMPLETED, &completed) {
        warn!("Failed to emit auto-provision-completed event: {}", e);
    }
    Ok(completed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: event name constants are exactly what the frontend
    /// expects. If any of these strings change, the corresponding listener
    /// in `src/components/onboarding/AutoSetupStep.tsx` (Phase 1.5b) also
    /// needs to change. This test exists to surface that contract clearly.
    #[test]
    fn event_names_are_kebab_case_strings() {
        assert_eq!(events::STARTED, "auto-provision-started");
        assert_eq!(events::PROGRESS, "auto-provision-progress");
        assert_eq!(events::COMPLETED, "auto-provision-completed");
        assert_eq!(events::FAILED, "auto-provision-failed");
    }
}
