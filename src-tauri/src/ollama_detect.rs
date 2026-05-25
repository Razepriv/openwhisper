//! Ollama auto-detect.
//!
//! Phase 1.8 of the OpenWhisper roadmap. Probes whether the user has
//! [Ollama](https://ollama.com) installed locally so the BackendResolver
//! can upgrade the cleanup routing from the bundled llama.cpp sidecar to
//! the user's Ollama install (more models, often better tuning).
//!
//! ## Detection strategy
//!
//! Two independent signals, OR'd together:
//!
//! 1. **Binary on PATH** — `which ollama` / `where ollama`. Most reliable on
//!    Linux + macOS Homebrew installs. PowerShell `Get-Command` on Windows.
//! 2. **Default install path** — `/usr/local/bin/ollama` (Mac), `/usr/bin/ollama`
//!    (Linux package managers), `%LOCALAPPDATA%\Programs\Ollama\ollama.exe`
//!    (Windows Squirrel installer). Catches users who didn't add Ollama to
//!    PATH.
//!
//! We do NOT probe the HTTP endpoint (`http://localhost:11434`). That would
//! catch only a *running* Ollama, but Ollama autostarts on demand — the
//! binary's existence is the right signal.
//!
//! ## Cost
//!
//! ~10–50 ms per call (a single filesystem stat or a process spawn). Cheap
//! enough to run on every auto-provisioner kickoff without caching.

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use specta::Type;

/// Result of an Ollama detection probe.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct OllamaDetection {
    /// True if Ollama appears to be installed on this machine.
    pub installed: bool,
    /// Absolute path to the Ollama binary we found, if any. Useful for the
    /// frontend "Detected at: ..." disclosure in Settings.
    pub binary_path: Option<String>,
}

/// Probe for Ollama. Always returns a result (never errors) — at worst the
/// detection is a no-op that reports `installed: false`. Designed to be
/// safe to call repeatedly.
pub fn detect() -> OllamaDetection {
    if let Some(path) = find_on_path() {
        return OllamaDetection {
            installed: true,
            binary_path: Some(path.to_string_lossy().into_owned()),
        };
    }

    for candidate in default_install_paths() {
        if candidate.is_file() {
            return OllamaDetection {
                installed: true,
                binary_path: Some(candidate.to_string_lossy().into_owned()),
            };
        }
    }

    OllamaDetection {
        installed: false,
        binary_path: None,
    }
}

/// Run the platform's `which` equivalent to find `ollama` on PATH.
fn find_on_path() -> Option<PathBuf> {
    #[cfg(target_family = "unix")]
    let (cmd, arg) = ("which", "ollama");
    #[cfg(target_family = "windows")]
    let (cmd, arg) = ("where", "ollama");

    let output = Command::new(cmd).arg(arg).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?.trim();
    if first_line.is_empty() {
        None
    } else {
        Some(PathBuf::from(first_line))
    }
}

/// Default install locations to check. Order matters — we return the first
/// match. Per-OS paths are gated by `#[cfg]` to keep the loop short.
fn default_install_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    #[cfg(target_os = "macos")]
    {
        paths.push(PathBuf::from("/usr/local/bin/ollama"));
        paths.push(PathBuf::from("/opt/homebrew/bin/ollama"));
        paths.push(PathBuf::from("/Applications/Ollama.app/Contents/Resources/ollama"));
    }

    #[cfg(target_os = "linux")]
    {
        paths.push(PathBuf::from("/usr/local/bin/ollama"));
        paths.push(PathBuf::from("/usr/bin/ollama"));
        paths.push(PathBuf::from("/snap/bin/ollama"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
            paths.push(
                PathBuf::from(local_appdata)
                    .join("Programs")
                    .join("Ollama")
                    .join("ollama.exe"),
            );
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            paths.push(PathBuf::from(program_files).join("Ollama").join("ollama.exe"));
        }
    }

    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `detect()` never panics, never returns an error, and the
    /// `installed`/`binary_path` invariant holds: when installed is true,
    /// binary_path must be Some.
    #[test]
    fn detect_returns_well_formed_result() {
        let result = detect();
        if result.installed {
            assert!(
                result.binary_path.is_some(),
                "installed=true should always come with a binary_path"
            );
        }
    }

    /// Even on machines without Ollama, the function is safe to call.
    /// Specifically guards against the cmd-not-found path in `find_on_path`.
    #[test]
    fn detect_handles_missing_ollama_gracefully() {
        // We can't reliably simulate "Ollama not installed" without
        // mocking; this test just exercises the function under the actual
        // host environment. The interesting assertion is that it doesn't
        // panic.
        let _ = detect();
    }

    /// `default_install_paths()` returns at least one candidate per
    /// supported platform (sanity that the cfg gates aren't all empty).
    #[test]
    fn default_install_paths_is_non_empty_on_supported_platforms() {
        let paths = default_install_paths();
        #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
        assert!(!paths.is_empty(), "expected at least one candidate path");
        let _ = paths;
    }
}
