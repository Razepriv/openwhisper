//! Active-app detection — figures out which application currently owns
//! the foreground window so the dictation pipeline can specialise its
//! behaviour (per-app style presets, Vibe Coding for IDEs, terminal
//! handling for Claude Code / Codex, etc.).
//!
//! Phase 5 of the OpenWhisper roadmap (research/11-phase-1-tasks.md §5).
//!
//! ## Platform implementations
//!
//! - **Windows**: Win32 `GetForegroundWindow` + `GetWindowTextW` +
//!   `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`. Already
//!   pulls only deps that are in our `windows` crate features. No shell
//!   out, no extra subprocesses — fast and reliable.
//! - **Linux (X11)**: Shells out to `xprop` + `xdotool`. These are
//!   standard on every X11 distro. Wayland is a known gap (see below).
//! - **macOS**: Currently a stub returning an empty profile. A full
//!   implementation needs Cocoa via `objc2` or AppleScript via
//!   `osascript`; planned for a follow-up phase. The Vibe Coding +
//!   per-app routing paths degrade gracefully when the profile is empty
//!   (they just don't apply per-app rules), so this is safe.
//!
//! ## Wayland gap
//!
//! Wayland doesn't expose the focused window's process / class to
//! arbitrary clients. Compositor-specific protocols exist (wlroots
//! foreign-toplevel, KDE plasma-window-management) but they are not
//! universal. Phase 5 returns an empty profile on Wayland. Phase 9
//! tracks adding compositor-specific detection.
//!
//! ## Performance
//!
//! `detect()` is called once per dictation, never in the hot audio
//! path. Even the shell-out path on Linux completes in ~30 ms. We do
//! NOT cache the result — focus can change between dictations and a
//! stale cache would corrupt per-app routing.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Snapshot of the focused application at dictation time.
///
/// All fields are best-effort — any of them may be empty depending on
/// the platform's API capabilities. Consumers must handle empty values
/// gracefully (treat as "no special handling for this app").
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
pub struct ActiveApp {
    /// Process executable name without path, e.g. `"cursor.exe"`,
    /// `"Code"`, `"firefox"`. Lowercased on Windows for matching
    /// convenience. Empty when probing failed.
    pub process_name: String,
    /// Foreground window title, exactly as the OS reports it. Useful
    /// for distinguishing "Cursor — file.tsx" from "Cursor — main".
    pub window_title: String,
    /// Window class / bundle identifier — `WM_CLASS` on X11, bundle id
    /// on macOS (when implemented). Empty on Windows (the concept
    /// doesn't exist there in the same way).
    pub window_class: String,
}

impl ActiveApp {
    /// Concatenated lowercase identifier used by simple substring
    /// matching in `vibe_coding.rs`. Combines process name + window
    /// title + class so a single `contains()` call can pick up any of
    /// them. Empty when nothing was detected.
    pub fn matchable(&self) -> String {
        let mut s = String::with_capacity(
            self.process_name.len() + self.window_title.len() + self.window_class.len() + 4,
        );
        s.push_str(&self.process_name.to_lowercase());
        s.push(' ');
        s.push_str(&self.window_title.to_lowercase());
        s.push(' ');
        s.push_str(&self.window_class.to_lowercase());
        s
    }

    /// True when we couldn't detect anything at all. Per-app routing
    /// callers should branch on this and fall back to default behaviour.
    pub fn is_empty(&self) -> bool {
        self.process_name.is_empty()
            && self.window_title.is_empty()
            && self.window_class.is_empty()
    }
}

/// Probe the OS for the active foreground app. Never panics, never
/// errors — returns a default `ActiveApp` if the probe fails. Designed
/// to be called once per dictation kick-off (~10–30 ms cost).
pub fn detect() -> ActiveApp {
    #[cfg(target_os = "windows")]
    {
        windows_detect()
    }
    #[cfg(target_os = "linux")]
    {
        linux_detect()
    }
    #[cfg(target_os = "macos")]
    {
        macos_detect()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        ActiveApp::default()
    }
}

#[cfg(target_os = "windows")]
fn windows_detect() -> ActiveApp {
    use windows::Win32::Foundation::{HWND, MAX_PATH};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    };

    // SAFETY: All `unsafe` calls here are bog-standard Win32 reads with
    // no state mutation, no FFI lifetime hazards. Each handle is closed
    // by `windows`-crate Drop impls.
    let hwnd: HWND = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return ActiveApp::default();
    }

    let mut profile = ActiveApp::default();

    // Window title — query length first, then allocate exactly that.
    let title_len = unsafe { GetWindowTextLengthW(hwnd) };
    if title_len > 0 {
        let mut buf = vec![0u16; title_len as usize + 1];
        let written = unsafe { GetWindowTextW(hwnd, &mut buf) };
        if written > 0 {
            profile.window_title = String::from_utf16_lossy(&buf[..written as usize]);
        }
    }

    // Process name via PID -> handle -> QueryFullProcessImageName.
    let mut pid: u32 = 0;
    let _tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid != 0 {
        if let Ok(handle) = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) } {
            let mut buf = vec![0u16; MAX_PATH as usize];
            let mut size: u32 = buf.len() as u32;
            if unsafe {
                QueryFullProcessImageNameW(handle, PROCESS_NAME_FORMAT(0), windows::core::PWSTR(buf.as_mut_ptr()), &mut size)
            }
            .is_ok()
            {
                let full = String::from_utf16_lossy(&buf[..size as usize]);
                // Strip path: "C:\\Program Files\\Cursor\\Cursor.exe" -> "cursor.exe"
                profile.process_name = full
                    .rsplit(['\\', '/'])
                    .next()
                    .unwrap_or(&full)
                    .to_lowercase();
            }
        }
    }

    profile
}

#[cfg(target_os = "linux")]
fn linux_detect() -> ActiveApp {
    use std::process::Command;

    // Wayland detection: WAYLAND_DISPLAY is the canonical signal.
    // xdotool/xprop won't work; bail out with an empty profile.
    if std::env::var("WAYLAND_DISPLAY").is_ok() && std::env::var("DISPLAY").is_err() {
        log::debug!("active_app: Wayland detected, returning empty profile (no compositor support yet)");
        return ActiveApp::default();
    }

    // Get the active window id via xdotool, then ask xprop for its
    // title + class. Both are standard on X11 distros.
    let win_id = match Command::new("xdotool").arg("getactivewindow").output() {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => return ActiveApp::default(),
    };
    if win_id.is_empty() {
        return ActiveApp::default();
    }

    let mut profile = ActiveApp::default();

    if let Ok(out) = Command::new("xprop")
        .args(["-id", &win_id, "WM_NAME", "WM_CLASS", "_NET_WM_PID"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if let Some(val) = line.strip_prefix("WM_NAME(STRING) = ") {
                profile.window_title = val.trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("WM_NAME(UTF8_STRING) = ") {
                profile.window_title = val.trim_matches('"').to_string();
            } else if let Some(val) = line.strip_prefix("WM_CLASS(STRING) = ") {
                // WM_CLASS is two strings: instance + class. Take the second.
                let parts: Vec<&str> = val.split(',').collect();
                if let Some(class) = parts.get(1).or(parts.first()) {
                    profile.window_class = class.trim().trim_matches('"').to_string();
                }
            } else if let Some(val) = line.strip_prefix("_NET_WM_PID(CARDINAL) = ") {
                // Resolve PID to process name via /proc/<pid>/comm.
                if let Ok(pid) = val.trim().parse::<u32>() {
                    if let Ok(comm) = std::fs::read_to_string(format!("/proc/{}/comm", pid)) {
                        profile.process_name = comm.trim().to_lowercase();
                    }
                }
            }
        }
    }

    profile
}

#[cfg(target_os = "macos")]
fn macos_detect() -> ActiveApp {
    use std::process::Command;

    // Cocoa-API access requires Objective-C interop. As a pragmatic
    // first cut we shell out to AppleScript which is always available
    // on macOS and returns the frontmost app's bundle id + name.
    //
    // Cost: ~80 ms per call. Acceptable for a once-per-dictation
    // probe. A native Cocoa version is queued for a follow-up phase.
    let script = r#"
        tell application "System Events"
            set frontApp to first application process whose frontmost is true
            set appName to name of frontApp
            set appBundle to bundle identifier of frontApp
            try
                set windowTitle to name of front window of frontApp
            on error
                set windowTitle to ""
            end try
            return appName & "\t" & appBundle & "\t" & windowTitle
        end tell
    "#;

    let output = match Command::new("osascript").args(["-e", script]).output() {
        Ok(out) if out.status.success() => out.stdout,
        _ => return ActiveApp::default(),
    };
    let line = String::from_utf8_lossy(&output);
    let parts: Vec<&str> = line.trim().split('\t').collect();
    ActiveApp {
        process_name: parts.first().map(|s| s.to_lowercase()).unwrap_or_default(),
        window_class: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
        window_title: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_never_panics() {
        let _ = detect();
    }

    #[test]
    fn matchable_lowercases_everything() {
        let app = ActiveApp {
            process_name: "Cursor.exe".to_string(),
            window_title: "MyProject — App.tsx".to_string(),
            window_class: "Cursor".to_string(),
        };
        let m = app.matchable();
        assert!(m.contains("cursor.exe"));
        assert!(m.contains("myproject"));
        assert!(m.contains("app.tsx"));
        // No uppercase characters survive.
        assert_eq!(m, m.to_lowercase());
    }

    #[test]
    fn is_empty_only_when_all_fields_blank() {
        assert!(ActiveApp::default().is_empty());
        let with_title = ActiveApp {
            window_title: "X".to_string(),
            ..Default::default()
        };
        assert!(!with_title.is_empty());
    }

    #[test]
    fn matchable_is_empty_when_app_is_empty() {
        let app = ActiveApp::default();
        // matchable() still produces a string with 2 spaces between
        // the empty fields, but contains() against any non-empty needle
        // must be false.
        assert!(!app.matchable().contains("cursor"));
    }
}
