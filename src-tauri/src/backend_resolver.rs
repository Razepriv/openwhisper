//! BackendResolver — maps a [`SystemProfile`] to the optimal STT + cleanup
//! stack for the host machine.
//!
//! Phase 1.3 of the OpenWhisper roadmap (research/11-phase-1-tasks.md).
//!
//! ## Design
//!
//! The resolver is intentionally a pair of **pure functions** so it's trivial
//! to test against every documented hardware tier:
//!
//! 1. [`classify_tier`] takes the system profile (plus whether Ollama is
//!    installed) and returns one of ten tiers `S` through `Z`.
//! 2. [`resolve`] composes [`classify_tier`] with a table lookup to produce
//!    the concrete [`Stack`] (STT model id + LLM cleanup backend).
//!
//! Both functions are deterministic and free of side effects. The decision
//! matrix mirrors the table in
//! `research/09-auto-provisioning.md §BackendResolver`. When that document
//! changes, this file changes too — keep them in sync.
//!
//! ## Why not VRAM / GPU details?
//!
//! Some tier decisions in the spec mention `NVIDIA RTX (≥6 GB VRAM)`, but we
//! intentionally **don't** probe VRAM at this layer. Reasons:
//!
//! - Detecting VRAM portably is heavy (wgpu adapter enumeration, ~30 MB compile cost).
//! - The realistic distinction is binary: "do we have a usable discrete GPU?"
//!   `transcribe-rs::whisper_cpp::gpu::list_gpu_devices()` already answers that
//!   at runtime via the loaded accelerator, and Handy pre-warms it on startup.
//! - For users with truly underpowered GPUs we route to `whisper.cpp + CPU`
//!   anyway via the BackendResolver's RAM fallback.
//!
//! Phase 1.3 keeps the resolver coarse-grained. Phase 1.7+ refines it once we
//! have telemetry from real users.
//!
//! ## Conservative defaults
//!
//! When in doubt, we route DOWN (to a smaller model) rather than UP. A
//! too-small model annoys the user; a too-big model bricks the dictation
//! experience. The Z tier is the universal fallback: works on any 32-bit
//! Pentium that can build the project.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::system_probe::SystemProfile;

/// Provisioning tier. Each tier maps to a single [`Stack`] in [`resolve`].
/// Ordering is "highest capability first" by convention; do not rely on it for
/// numeric comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum Tier {
    /// Apple Silicon M3+, ≥16 GB RAM, macOS 26+. Uses Apple Foundation Models
    /// (zero-disk LLM) and WhisperKit Turbo on the Neural Engine.
    S,
    /// Apple Silicon (any M-series), 8–16 GB RAM. Uses whisper.cpp + Metal +
    /// Large-v3 Turbo with a bundled llama.cpp cleanup sidecar.
    A,
    /// Intel Mac, ≥8 GB RAM. Whisper Medium + bundled llama.cpp.
    B,
    /// Win 11 24H2+ on a Copilot+ NPU machine. Whisper Turbo (Vulkan) + Phi
    /// Silica via Windows AI APIs (zero-disk LLM).
    C,
    /// Windows + discrete NVIDIA GPU, ≥8 GB RAM. Whisper Turbo (CUDA) +
    /// bundled llama.cpp with Gemma 3 4B.
    D,
    /// Windows + AMD/Intel GPU, ≥8 GB RAM. Whisper Medium (Vulkan) + bundled
    /// llama.cpp with Phi-4 mini.
    E,
    /// Windows CPU-only, ≥8 GB RAM. Whisper Small + bundled llama.cpp with
    /// Llama 3.2 3B.
    F,
    /// Linux + NVIDIA CUDA. Mirror of D.
    G,
    /// Linux CPU / AMD / Intel. Mirror of E/F.
    H,
    /// Low-spec fallback (< 8 GB RAM or insufficient disk). Moonshine Tiny,
    /// no LLM cleanup. Works on anything.
    Z,
}

impl Tier {
    /// Stable string code (single uppercase letter). Used in logs, telemetry,
    /// and the JSON returned to the frontend.
    pub fn code(self) -> &'static str {
        match self {
            Tier::S => "S",
            Tier::A => "A",
            Tier::B => "B",
            Tier::C => "C",
            Tier::D => "D",
            Tier::E => "E",
            Tier::F => "F",
            Tier::G => "G",
            Tier::H => "H",
            Tier::Z => "Z",
        }
    }
}

/// Which LLM-cleanup backend to route to. The order in this enum reflects
/// preference: try Apple FM first if available, then Phi Silica, then Ollama,
/// then the bundled llama.cpp sidecar, then nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CleanupBackend {
    /// macOS 26+ on Apple Silicon. OS-provided, zero disk cost, ~50 ms cleanup.
    AppleFM,
    /// Windows 11 24H2+ on Copilot+ NPU. OS-provided, zero disk cost.
    PhiSilica,
    /// User has Ollama pre-installed. Use it as an upgrade path over the
    /// bundled sidecar (more models, better perf if user has it tuned).
    Ollama,
    /// OpenWhisper-bundled `llama-server` child process. Default for systems
    /// without an OS-native LLM.
    LlamaSidecar,
    /// No LLM cleanup. Raw Whisper transcription only. Tier Z fallback.
    None,
}

/// The resolved stack for the host system.
///
/// Frontend onboarding UI reads this to show the user what's about to be
/// downloaded ("we picked Whisper Turbo for your M3 Pro, ~620 MB").
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct Stack {
    /// Which provisioning tier the host falls into.
    pub tier: Tier,
    /// `id` of the STT model entry in `models_manifest.json` that the resolver
    /// recommends downloading on first run.
    pub stt_model_id: String,
    /// Which LLM cleanup backend to wire up for the post-process pass.
    pub cleanup_backend: CleanupBackend,
    /// Estimated total disk footprint after auto-provisioning completes.
    /// Used by the onboarding UI to show "Downloading ~3.2 GB" before
    /// committing to the download.
    pub estimated_download_mb: u64,
    /// Human-readable summary, e.g. "Whisper Turbo + Apple Foundation Models".
    /// The frontend can localise the formatting but the model names are stable.
    pub summary: String,
}

/// Minimum RAM (in MB) below which we drop to Tier Z regardless of other
/// signals. 8 GB - some safety margin. Below this, downloading a 3 GB LLM
/// just to dictate is a bad experience.
const TIER_Z_RAM_FLOOR_MB: u64 = 7000;

/// Minimum free disk (in MB) below which we drop to Tier Z. We need room for
/// the LLM cleanup model plus the Whisper model plus some headroom.
const TIER_Z_DISK_FLOOR_MB: u64 = 5000;

/// macOS version (major.minor) at which Apple Foundation Models become
/// available. Anything older falls back to llama.cpp.
const APPLE_FOUNDATION_MODELS_MIN_MAJOR: u32 = 26;

/// Classify which provisioning tier a system falls into. The `ollama_detected`
/// parameter comes from a separate detection step (Phase 1.8); the resolver
/// upgrades cleanup routing to Ollama when both Ollama is present AND the OS
/// doesn't ship a native NPU LLM.
pub fn classify_tier(profile: &SystemProfile, has_capable_gpu: bool) -> Tier {
    // Universal floor: if there isn't enough RAM or disk for any real model,
    // bail out early to Tier Z. Better to give the user something working than
    // to fail-then-fall-back at download time.
    if profile.ram_total_mb < TIER_Z_RAM_FLOOR_MB {
        return Tier::Z;
    }
    if let Some(free) = profile.disk_free_mb {
        if free < TIER_Z_DISK_FLOOR_MB {
            return Tier::Z;
        }
    }

    match profile.os_family.as_str() {
        "macos" => classify_macos(profile),
        "windows" => classify_windows(profile, has_capable_gpu),
        "linux" => classify_linux(profile, has_capable_gpu),
        // FreeBSD / Android / iOS / unknown — degrade gracefully.
        _ => Tier::Z,
    }
}

/// macOS-specific classification. Apple Silicon + macOS 26+ ≥16 GB → S;
/// otherwise A (Apple Silicon any RAM) or B (Intel Mac).
fn classify_macos(profile: &SystemProfile) -> Tier {
    let is_apple_silicon = profile.has_apple_neural_engine; // ANE iff Apple Silicon

    if is_apple_silicon {
        let supports_foundation_models = parse_macos_major_version(profile.os_version.as_deref())
            .map(|major| major >= APPLE_FOUNDATION_MODELS_MIN_MAJOR)
            .unwrap_or(false);
        if supports_foundation_models && profile.ram_total_mb >= 16_000 {
            Tier::S
        } else {
            Tier::A
        }
    } else {
        // Intel Mac.
        Tier::B
    }
}

/// Windows-specific classification. Copilot+ NPU → C; discrete GPU detected →
/// D (NVIDIA assumption — we can refine later); other GPU → E; CPU-only → F.
///
/// `has_capable_gpu` is a Boolean hint from `transcribe-rs`'s GPU enumeration.
/// We don't try to distinguish NVIDIA vs AMD vs Intel here — that requires
/// loading more vendor APIs than is worth it for an already-simple tier
/// classification. Both NVIDIA-D and AMD-E end up downloading the same
/// `turbo` model anyway.
fn classify_windows(profile: &SystemProfile, has_capable_gpu: bool) -> Tier {
    if profile.has_windows_npu {
        return Tier::C;
    }
    if has_capable_gpu {
        // Treat any usable GPU as Tier D (the resolver downloads `turbo`
        // either way; the runtime accelerator binding picks CUDA / Vulkan /
        // DirectML appropriately).
        Tier::D
    } else {
        // Distinguish E (8 GB+) vs F (lower). Both ≥ TIER_Z_RAM_FLOOR_MB at
        // this point so just check whether we have enough headroom for the
        // Medium model + Phi-4 mini cleanup.
        if profile.ram_total_mb >= 12_000 {
            Tier::E
        } else {
            Tier::F
        }
    }
}

/// Linux mirrors Windows: GPU → G, otherwise H. We don't try to detect
/// Copilot+ NPUs on Linux (Snapdragon X Linux laptops exist but the LLM
/// drivers do not). Sidecar llama.cpp is the universal cleanup backend.
fn classify_linux(profile: &SystemProfile, has_capable_gpu: bool) -> Tier {
    if has_capable_gpu {
        Tier::G
    } else {
        let _ = profile; // Reserved for future RAM-tier refinement.
        Tier::H
    }
}

/// Parse the major version number from a macOS version string ("15.2", "26.0").
fn parse_macos_major_version(version: Option<&str>) -> Option<u32> {
    version?.split('.').next()?.parse().ok()
}

/// Resolve the full stack for the given system. This is the entry point most
/// callers will use; [`classify_tier`] is exposed for unit testing.
///
/// `has_capable_gpu` should come from
/// `transcribe-rs::whisper_cpp::gpu::list_gpu_devices()`; pass `false` if the
/// caller hasn't probed yet (the resolver will pick a CPU-friendly model).
///
/// `ollama_detected` should come from the Ollama auto-detect flow in
/// Phase 1.8; pass `false` until that lands and the resolver will route to the
/// bundled llama.cpp sidecar instead.
pub fn resolve(profile: &SystemProfile, has_capable_gpu: bool, ollama_detected: bool) -> Stack {
    let tier = classify_tier(profile, has_capable_gpu);
    let (stt_model_id, native_cleanup, estimated_mb, summary) = stack_for_tier(tier);
    let cleanup_backend = pick_cleanup(native_cleanup, ollama_detected);
    Stack {
        tier,
        stt_model_id: stt_model_id.to_string(),
        cleanup_backend,
        estimated_download_mb: estimated_mb,
        summary: summary.to_string(),
    }
}

/// Per-tier static configuration. Keep the table in sync with
/// `research/09-auto-provisioning.md §BackendResolver decision matrix`.
fn stack_for_tier(tier: Tier) -> (&'static str, CleanupBackend, u64, &'static str) {
    match tier {
        Tier::S => (
            "turbo",
            CleanupBackend::AppleFM,
            620,
            "Whisper Turbo + Apple Foundation Models",
        ),
        Tier::A => (
            "turbo",
            CleanupBackend::LlamaSidecar,
            3_200,
            "Whisper Turbo + bundled llama.cpp",
        ),
        Tier::B => (
            "medium",
            CleanupBackend::LlamaSidecar,
            2_800,
            "Whisper Medium + bundled llama.cpp",
        ),
        Tier::C => (
            "turbo",
            CleanupBackend::PhiSilica,
            990,
            "Whisper Turbo + Phi Silica (Windows AI)",
        ),
        Tier::D => (
            "turbo",
            CleanupBackend::LlamaSidecar,
            3_500,
            "Whisper Turbo + bundled llama.cpp",
        ),
        Tier::E => (
            "medium",
            CleanupBackend::LlamaSidecar,
            2_800,
            "Whisper Medium + bundled llama.cpp",
        ),
        Tier::F => (
            // handy fix: Windows CPU-only systems get Whisper Tiny by
            // default instead of Small. Small on CPU takes 5–15 s for
            // a 5 s clip — long enough that users assumed the app was
            // hung at "Transcribing…". Tiny ships bundled with the
            // installer (~75 MB) and runs near real-time on a typical
            // x86_64 CPU. Users who want better accuracy can switch
            // to Small / Medium from Settings → Models any time.
            "tiny",
            CleanupBackend::LlamaSidecar,
            2_500,
            "Whisper Tiny + bundled llama.cpp",
        ),
        Tier::G => (
            "turbo",
            CleanupBackend::LlamaSidecar,
            3_500,
            "Whisper Turbo + bundled llama.cpp",
        ),
        Tier::H => (
            // handy fix: same reasoning as Tier F above — Linux CPU
            // builds get Whisper Tiny for instant transcription on
            // first launch.
            "tiny",
            CleanupBackend::LlamaSidecar,
            2_500,
            "Whisper Tiny + bundled llama.cpp",
        ),
        Tier::Z => (
            "moonshine-tiny-streaming-en",
            CleanupBackend::None,
            30,
            "Moonshine Tiny only (low-spec mode)",
        ),
    }
}

/// Upgrade `LlamaSidecar` to `Ollama` when the user has Ollama pre-installed.
/// `AppleFM`, `PhiSilica`, and `None` are not overridden — those are OS-tied
/// or explicit no-op decisions that shouldn't change based on Ollama presence.
fn pick_cleanup(native: CleanupBackend, ollama_detected: bool) -> CleanupBackend {
    match (native, ollama_detected) {
        (CleanupBackend::LlamaSidecar, true) => CleanupBackend::Ollama,
        (backend, _) => backend,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Construct a minimal SystemProfile for tier-classification tests.
    fn make_profile(
        os_family: &str,
        os_version: Option<&str>,
        cpu_arch: &str,
        ram_mb: u64,
        has_apple_ne: bool,
        has_windows_npu: bool,
    ) -> SystemProfile {
        SystemProfile {
            os_family: os_family.to_string(),
            os_version: os_version.map(|s| s.to_string()),
            cpu_arch: cpu_arch.to_string(),
            cpu_brand: "test cpu".to_string(),
            cpu_cores: 8,
            ram_total_mb: ram_mb,
            ram_available_mb: ram_mb / 2,
            disk_free_mb: Some(50_000),
            has_apple_neural_engine: has_apple_ne,
            has_windows_npu,
            locale: "en-US".to_string(),
        }
    }

    // --- Tier classification: the 10 documented tiers ---

    #[test]
    fn tier_s_for_apple_silicon_macos_26_plus_with_16gb() {
        let p = make_profile("macos", Some("26.1"), "aarch64", 32_000, true, false);
        assert_eq!(classify_tier(&p, true), Tier::S);
    }

    #[test]
    fn tier_a_for_apple_silicon_below_macos_26() {
        let p = make_profile("macos", Some("15.2"), "aarch64", 16_000, true, false);
        assert_eq!(classify_tier(&p, true), Tier::A);
    }

    #[test]
    fn tier_a_for_apple_silicon_macos_26_below_16gb() {
        let p = make_profile("macos", Some("26.0"), "aarch64", 8_000, true, false);
        assert_eq!(classify_tier(&p, true), Tier::A);
    }

    #[test]
    fn tier_b_for_intel_mac() {
        let p = make_profile("macos", Some("14.4"), "x86_64", 16_000, false, false);
        assert_eq!(classify_tier(&p, false), Tier::B);
    }

    #[test]
    fn tier_c_for_windows_with_copilot_npu() {
        let p = make_profile("windows", Some("10.0.26100"), "x86_64", 16_000, false, true);
        assert_eq!(classify_tier(&p, false), Tier::C);
    }

    #[test]
    fn tier_d_for_windows_with_gpu_no_npu() {
        let p = make_profile("windows", Some("10.0.22631"), "x86_64", 16_000, false, false);
        assert_eq!(classify_tier(&p, true), Tier::D);
    }

    #[test]
    fn tier_e_for_windows_no_gpu_12gb_ram() {
        let p = make_profile("windows", Some("10.0.22631"), "x86_64", 16_000, false, false);
        assert_eq!(classify_tier(&p, false), Tier::E);
    }

    #[test]
    fn tier_f_for_windows_no_gpu_low_ram() {
        let p = make_profile("windows", Some("10.0.19044"), "x86_64", 8_000, false, false);
        assert_eq!(classify_tier(&p, false), Tier::F);
    }

    #[test]
    fn tier_g_for_linux_with_gpu() {
        let p = make_profile("linux", Some("22.04"), "x86_64", 16_000, false, false);
        assert_eq!(classify_tier(&p, true), Tier::G);
    }

    #[test]
    fn tier_h_for_linux_cpu_only() {
        let p = make_profile("linux", Some("22.04"), "x86_64", 16_000, false, false);
        assert_eq!(classify_tier(&p, false), Tier::H);
    }

    // --- Tier Z fallback paths ---

    #[test]
    fn tier_z_for_low_ram_regardless_of_os() {
        let p = make_profile("macos", Some("26.0"), "aarch64", 4_000, true, false);
        assert_eq!(classify_tier(&p, true), Tier::Z);
    }

    #[test]
    fn tier_z_for_insufficient_disk() {
        let mut p = make_profile("windows", None, "x86_64", 16_000, false, false);
        p.disk_free_mb = Some(2_000); // Below TIER_Z_DISK_FLOOR_MB.
        assert_eq!(classify_tier(&p, true), Tier::Z);
    }

    #[test]
    fn tier_z_for_unknown_os() {
        let p = make_profile("freebsd", None, "x86_64", 16_000, false, false);
        assert_eq!(classify_tier(&p, false), Tier::Z);
    }

    // --- Resolver tests: each tier maps to a sensible stack ---

    #[test]
    fn resolve_tier_s_uses_apple_fm_and_turbo() {
        let p = make_profile("macos", Some("26.0"), "aarch64", 32_000, true, false);
        let stack = resolve(&p, true, false);
        assert_eq!(stack.tier, Tier::S);
        assert_eq!(stack.stt_model_id, "turbo");
        assert_eq!(stack.cleanup_backend, CleanupBackend::AppleFM);
        assert!(stack.estimated_download_mb < 1_000);
    }

    #[test]
    fn resolve_tier_c_uses_phi_silica() {
        let p = make_profile("windows", Some("10.0.26100"), "x86_64", 16_000, false, true);
        let stack = resolve(&p, false, false);
        assert_eq!(stack.tier, Tier::C);
        assert_eq!(stack.cleanup_backend, CleanupBackend::PhiSilica);
    }

    #[test]
    fn resolve_tier_z_disables_cleanup() {
        let p = make_profile("linux", Some("22.04"), "x86_64", 4_000, false, false);
        let stack = resolve(&p, false, false);
        assert_eq!(stack.tier, Tier::Z);
        assert_eq!(stack.cleanup_backend, CleanupBackend::None);
        assert_eq!(stack.stt_model_id, "moonshine-tiny-streaming-en");
        assert!(stack.estimated_download_mb < 100);
    }

    // --- Ollama upgrade path ---

    #[test]
    fn ollama_replaces_bundled_sidecar_when_detected() {
        let p = make_profile("linux", Some("22.04"), "x86_64", 16_000, false, false);
        let stack = resolve(&p, false, true);
        assert_eq!(stack.tier, Tier::H);
        assert_eq!(stack.cleanup_backend, CleanupBackend::Ollama);
    }

    #[test]
    fn ollama_does_not_override_apple_fm() {
        let p = make_profile("macos", Some("26.0"), "aarch64", 32_000, true, false);
        let stack = resolve(&p, true, true);
        assert_eq!(stack.tier, Tier::S);
        assert_eq!(stack.cleanup_backend, CleanupBackend::AppleFM);
    }

    #[test]
    fn ollama_does_not_override_phi_silica() {
        let p = make_profile("windows", Some("10.0.26100"), "x86_64", 16_000, false, true);
        let stack = resolve(&p, false, true);
        assert_eq!(stack.tier, Tier::C);
        assert_eq!(stack.cleanup_backend, CleanupBackend::PhiSilica);
    }

    #[test]
    fn ollama_does_not_revive_cleanup_on_tier_z() {
        let p = make_profile("linux", Some("22.04"), "x86_64", 4_000, false, false);
        let stack = resolve(&p, false, true);
        assert_eq!(stack.tier, Tier::Z);
        assert_eq!(stack.cleanup_backend, CleanupBackend::None);
    }

    // --- Sanity: every recommended stt_model_id exists in the manifest ---

    #[test]
    fn all_resolved_stt_models_exist_in_manifest() {
        let manifest = crate::managers::model_manifest::load_bundled()
            .expect("manifest should load");
        let ids: std::collections::HashSet<_> =
            manifest.iter().map(|m| m.id.clone()).collect();

        for tier in [
            Tier::S, Tier::A, Tier::B, Tier::C, Tier::D, Tier::E, Tier::F, Tier::G, Tier::H,
            Tier::Z,
        ] {
            let (id, _, _, _) = stack_for_tier(tier);
            assert!(
                ids.contains(id),
                "tier {:?} maps to stt_model_id '{}' which is not in models_manifest.json",
                tier,
                id
            );
        }
    }

    // --- Tier code stability (logs / telemetry contract) ---

    #[test]
    fn tier_codes_are_single_uppercase_letters() {
        for tier in [
            Tier::S, Tier::A, Tier::B, Tier::C, Tier::D, Tier::E, Tier::F, Tier::G, Tier::H,
            Tier::Z,
        ] {
            let code = tier.code();
            assert_eq!(code.len(), 1, "tier {:?} code '{}' not length 1", tier, code);
            assert!(
                code.chars().next().unwrap().is_ascii_uppercase(),
                "tier {:?} code '{}' not uppercase",
                tier,
                code
            );
        }
    }

    #[test]
    fn parse_macos_version_handles_common_formats() {
        assert_eq!(parse_macos_major_version(Some("26.1")), Some(26));
        assert_eq!(parse_macos_major_version(Some("15.2.1")), Some(15));
        assert_eq!(parse_macos_major_version(Some("10.15.7")), Some(10));
        assert_eq!(parse_macos_major_version(Some("")), None);
        assert_eq!(parse_macos_major_version(None), None);
        assert_eq!(parse_macos_major_version(Some("alpha.beta")), None);
    }
}
