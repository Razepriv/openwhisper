//! Model manifest loader.
//!
//! Phase 1.1: replaces the hardcoded `HashMap<String, ModelInfo>` in
//! `model.rs::ModelManager::new()` with a JSON manifest bundled at compile time
//! via `include_str!`. The manifest lives at
//! `src-tauri/resources/models_manifest.json`.
//!
//! ## Why bundle the manifest at compile time?
//!
//! - The manifest is an integral part of the app — without it the model picker
//!   would be empty. We don't want to ship an installer that can fail at runtime
//!   if `resources/models_manifest.json` is missing or corrupted.
//! - `include_str!` validates the file exists at build time.
//! - Loading is infallible after compile: no I/O, no parsing surprises.
//!
//! ## Schema
//!
//! See `src-tauri/resources/models_manifest.json` for the live schema. Briefly:
//!
//! ```jsonc
//! {
//!   "$schema_version": 1,
//!   "language_groups": { "whisper_99": ["en", ...], ... },
//!   "models": [
//!     {
//!       "id": "turbo",
//!       "name": "Whisper Turbo",
//!       "engine_type": "Whisper",
//!       "quant": "f16",
//!       "tier_recommendation": ["A", "C", "D", "G"],
//!       "requires_gpu": false,
//!       "requires_npu": false,
//!       "supported_languages_ref": "whisper_99",
//!       // ... + all the other fields ModelInfo has
//!     }
//!   ]
//! }
//! ```
//!
//! Models reference a language group via `supported_languages_ref` to avoid
//! repeating the 99-language Whisper list four times. The loader resolves the
//! reference into `ModelInfo::supported_languages` at load time.

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use super::model::{EngineType, ModelInfo};

/// Compile-time bundled manifest JSON.
const MANIFEST_JSON: &str = include_str!("../../resources/models_manifest.json");

/// The schema version of the manifest format. Bump if we make a breaking change
/// to the manifest layout (loader-incompatible).
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// Top-level manifest file shape.
#[derive(Debug, Deserialize)]
struct ManifestFile {
    #[serde(rename = "$schema_version")]
    schema_version: u32,
    #[serde(default)]
    language_groups: HashMap<String, Vec<String>>,
    models: Vec<ManifestEntry>,
}

/// A single model entry as stored in the JSON manifest.
///
/// This is intentionally close to `ModelInfo` but distinct so that:
/// - Runtime-only fields (`is_downloaded`, `is_downloading`, `partial_size`,
///   `is_custom`) are not present here.
/// - Language lists are referenced by name (`supported_languages_ref`) rather
///   than duplicated inline.
#[derive(Debug, Deserialize)]
struct ManifestEntry {
    id: String,
    name: String,
    description: String,
    filename: String,
    url: Option<String>,
    sha256: Option<String>,
    size_mb: u64,
    is_directory: bool,
    engine_type: EngineType,
    #[serde(default)]
    quant: Option<String>,
    #[serde(default)]
    tier_recommendation: Vec<String>,
    #[serde(default)]
    requires_gpu: bool,
    #[serde(default)]
    requires_npu: bool,
    accuracy_score: f32,
    speed_score: f32,
    supports_translation: bool,
    #[serde(default)]
    is_recommended: bool,
    supported_languages_ref: String,
    supports_language_selection: bool,
}

/// Load the bundled manifest at startup. Returns the list of model entries in
/// the order they appear in the JSON file.
///
/// Returns an error if the bundled manifest is malformed — this is treated as a
/// build-time bug since the manifest is `include_str!`'d at compile time. Tests
/// in this module catch any schema regressions before they hit production.
pub fn load_bundled() -> Result<Vec<ModelInfo>> {
    let manifest: ManifestFile = serde_json::from_str(MANIFEST_JSON)
        .context("Failed to parse bundled models_manifest.json")?;

    if manifest.schema_version != SUPPORTED_SCHEMA_VERSION {
        bail!(
            "Unsupported manifest schema version: got {}, expected {}",
            manifest.schema_version,
            SUPPORTED_SCHEMA_VERSION
        );
    }

    manifest
        .models
        .into_iter()
        .map(|entry| entry_to_model_info(entry, &manifest.language_groups))
        .collect()
}

/// Convert a `ManifestEntry` (static JSON shape) into a `ModelInfo` (runtime
/// shape with download state).
fn entry_to_model_info(
    entry: ManifestEntry,
    language_groups: &HashMap<String, Vec<String>>,
) -> Result<ModelInfo> {
    let supported_languages = language_groups
        .get(&entry.supported_languages_ref)
        .cloned()
        .ok_or_else(|| {
            anyhow!(
                "Model '{}' references unknown language group '{}'",
                entry.id,
                entry.supported_languages_ref
            )
        })?;

    Ok(ModelInfo {
        id: entry.id,
        name: entry.name,
        description: entry.description,
        filename: entry.filename,
        url: entry.url,
        sha256: entry.sha256,
        size_mb: entry.size_mb,
        is_downloaded: false,
        is_downloading: false,
        partial_size: 0,
        is_directory: entry.is_directory,
        engine_type: entry.engine_type,
        accuracy_score: entry.accuracy_score,
        speed_score: entry.speed_score,
        supports_translation: entry.supports_translation,
        is_recommended: entry.is_recommended,
        supported_languages,
        supports_language_selection: entry.supports_language_selection,
        is_custom: false,
        quant: entry.quant,
        tier_recommendation: entry.tier_recommendation,
        requires_gpu: entry.requires_gpu,
        requires_npu: entry.requires_npu,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Sanity check: the bundled manifest parses without error.
    /// This catches JSON syntax errors and schema mismatches in CI before
    /// they crash the app at startup.
    #[test]
    fn bundled_manifest_parses() {
        let models = load_bundled().expect("bundled manifest should load");
        assert!(!models.is_empty(), "manifest should contain at least one model");
    }

    /// Phase 1.1 acceptance criterion: 17 entries (16 existing + Whisper Tiny).
    #[test]
    fn bundled_manifest_has_expected_model_count() {
        let models = load_bundled().expect("bundled manifest should load");
        assert_eq!(
            models.len(),
            17,
            "expected 17 models (16 from Handy + Whisper Tiny)"
        );
    }

    /// Phase 1.1 acceptance: Whisper Tiny + Small + Large-v3 Turbo all present.
    #[test]
    fn bundled_manifest_contains_required_models() {
        let models = load_bundled().expect("bundled manifest should load");
        let ids: HashSet<_> = models.iter().map(|m| m.id.as_str()).collect();

        for required in &["tiny", "small", "turbo"] {
            assert!(
                ids.contains(required),
                "manifest is missing required model id '{}'",
                required
            );
        }
    }

    /// Every model id must be unique. Duplicate IDs would silently overwrite
    /// each other in the HashMap and create surprising behavior.
    #[test]
    fn bundled_manifest_ids_are_unique() {
        let models = load_bundled().expect("bundled manifest should load");
        let mut seen = HashSet::new();
        for model in &models {
            assert!(
                seen.insert(model.id.clone()),
                "duplicate model id in manifest: '{}'",
                model.id
            );
        }
    }

    /// SHA-256 hashes are 64 hex characters. A typo here means downloads will
    /// fail verification at runtime, so catch it at build time.
    #[test]
    fn bundled_manifest_sha256_format_is_valid() {
        let models = load_bundled().expect("bundled manifest should load");
        for model in &models {
            if let Some(sha) = &model.sha256 {
                assert_eq!(
                    sha.len(),
                    64,
                    "model '{}' has malformed sha256 (length {}, expected 64)",
                    model.id,
                    sha.len()
                );
                assert!(
                    sha.chars().all(|c| c.is_ascii_hexdigit()),
                    "model '{}' has non-hex characters in sha256",
                    model.id
                );
            }
        }
    }

    /// Every URL must be HTTPS — we never want to download model weights over
    /// plain HTTP (MITM risk for binaries that get executed locally).
    #[test]
    fn bundled_manifest_urls_use_https() {
        let models = load_bundled().expect("bundled manifest should load");
        for model in &models {
            if let Some(url) = &model.url {
                assert!(
                    url.starts_with("https://"),
                    "model '{}' has non-HTTPS url: {}",
                    model.id,
                    url
                );
            }
        }
    }

    /// `supported_languages_ref` strings must resolve. The loader already
    /// surfaces this error but having an explicit test means CI fails fast
    /// with a clear message instead of a generic "manifest failed to load".
    #[test]
    fn bundled_manifest_all_language_refs_resolve() {
        let raw: ManifestFile = serde_json::from_str(MANIFEST_JSON)
            .expect("raw parse should succeed");
        for entry in &raw.models {
            assert!(
                raw.language_groups.contains_key(&entry.supported_languages_ref),
                "model '{}' references unknown language group '{}'",
                entry.id,
                entry.supported_languages_ref
            );
        }
    }

    /// Tier recommendation strings, if present, must be one of the documented
    /// auto-provisioning tiers (S/A/B/C/D/E/F/G/H/Z). See
    /// research/09-auto-provisioning.md.
    #[test]
    fn bundled_manifest_tier_recommendations_are_valid() {
        let valid_tiers: HashSet<&str> =
            ["S", "A", "B", "C", "D", "E", "F", "G", "H", "Z"].into_iter().collect();
        let models = load_bundled().expect("bundled manifest should load");
        for model in &models {
            for tier in &model.tier_recommendation {
                assert!(
                    valid_tiers.contains(tier.as_str()),
                    "model '{}' has invalid tier '{}'",
                    model.id,
                    tier
                );
            }
        }
    }

    /// Accuracy and speed scores live in [0.0, 1.0]. Out-of-range scores would
    /// break sort orders and progress bars in the UI.
    #[test]
    fn bundled_manifest_scores_are_in_range() {
        let models = load_bundled().expect("bundled manifest should load");
        for model in &models {
            assert!(
                (0.0..=1.0).contains(&model.accuracy_score),
                "model '{}' accuracy_score {} not in [0.0, 1.0]",
                model.id,
                model.accuracy_score
            );
            assert!(
                (0.0..=1.0).contains(&model.speed_score),
                "model '{}' speed_score {} not in [0.0, 1.0]",
                model.id,
                model.speed_score
            );
        }
    }

    /// Custom models loaded from disk must have `is_custom = true`. Manifest
    /// entries are always `is_custom = false`. This ensures the loader sets
    /// the field correctly regardless of what the JSON says.
    #[test]
    fn bundled_manifest_entries_are_not_custom() {
        let models = load_bundled().expect("bundled manifest should load");
        for model in &models {
            assert!(
                !model.is_custom,
                "model '{}' was incorrectly marked is_custom=true",
                model.id
            );
        }
    }
}
