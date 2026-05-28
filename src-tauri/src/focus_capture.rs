//! Focus capture & restore — keeps text pasting at the right place when
//! the user triggers dictation via a UI affordance (floating widget,
//! tray icon) instead of a global hotkey.
//!
//! ## The bug this fixes
//!
//! When the user holds Ctrl+Space in their text editor, OS focus stays
//! in the editor for the whole dictation — paste lands correctly. But
//! when they CLICK the floating widget, the click shifts focus from
//! the editor to our window. After transcription, the paste then lands
//! in our window (or wherever else focus drifted) instead of the editor.
//!
//! ## How we fix it
//!
//! 1. **Capture** the foreground window HWND BEFORE the click shifts
//!    focus. `trigger_dictation_from_widget` calls this immediately on
//!    entry, while the click event is still mid-flight on the OS side.
//! 2. **Stash** it in a process-wide `Mutex<Option<HWND>>` cell.
//! 3. **Restore** focus to that HWND right before paste runs. The
//!    paste path in `clipboard::paste` (and the Command Mode paste
//!    path) calls this; if there's a stashed HWND, we `SetForegroundWindow`
//!    + small sleep so the OS settles the focus before we type.
//! 4. **Clear** the stash after each restore so a stale HWND from an
//!    earlier dictation doesn't redirect the next one.
//!
//! ## Why a global cell vs Tauri state
//!
//! The capture happens from `trigger_dictation_from_widget` (a Tauri
//! command, has AppHandle access). The restore happens deep in the
//! paste pipeline, which already runs without easy access to AppHandle
//! state. A `Lazy<Mutex<...>>` static keeps the API call-sites tiny —
//! `capture()` and `take_and_restore()` are zero-arg.

#[cfg(target_os = "windows")]
use std::sync::Mutex;

#[cfg(target_os = "windows")]
use once_cell::sync::Lazy;

/// The HWND captured at widget-click time. `None` means either no
/// dictation is in flight, or it was triggered by a hotkey (which
/// doesn't shift focus and doesn't need restoration).
///
/// Storing as `usize` instead of `HWND` because `HWND` is a non-Send
/// raw-pointer wrapper and we need a `'static + Send` cell. usize is
/// the same width as HWND on Windows and survives a round-trip just
/// fine for the SetForegroundWindow call.
#[cfg(target_os = "windows")]
static CAPTURED_HWND: Lazy<Mutex<Option<usize>>> = Lazy::new(|| Mutex::new(None));

/// Snapshot the current foreground window. Called from
/// `trigger_dictation_from_widget` BEFORE we kick off recording. On
/// non-Windows platforms this is a no-op (Tauri's Wayland support is
/// limited, and we don't have an equivalent click-flow yet).
#[cfg(target_os = "windows")]
pub fn capture() {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.0.is_null() {
        return;
    }
    if let Ok(mut guard) = CAPTURED_HWND.lock() {
        *guard = Some(hwnd.0 as usize);
        log::debug!("focus_capture: captured HWND {:?}", hwnd.0);
    }
}

#[cfg(not(target_os = "windows"))]
pub fn capture() {}

/// Restore focus to the captured HWND (if any) and clear the cell.
/// Called from the paste path RIGHT BEFORE typing/pasting begins, so
/// any text we send lands in the originally-focused control.
///
/// Returns `true` if a window was restored, `false` if there was
/// nothing to restore (no widget click in flight). Callers can use
/// the return to decide whether to add a short sleep before paste.
#[cfg(target_os = "windows")]
pub fn take_and_restore() -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{IsWindow, SetForegroundWindow};

    let captured = match CAPTURED_HWND.lock() {
        Ok(mut g) => g.take(),
        Err(_) => return false,
    };
    let Some(hwnd_raw) = captured else {
        return false;
    };
    let hwnd = HWND(hwnd_raw as *mut _);
    let valid = unsafe { IsWindow(Some(hwnd)) }.as_bool();
    if !valid {
        log::debug!("focus_capture: captured HWND no longer valid; skipping restore");
        return false;
    }
    let ok = unsafe { SetForegroundWindow(hwnd) }.as_bool();
    if ok {
        log::debug!("focus_capture: restored focus to HWND {:?}", hwnd_raw);
    } else {
        // SetForegroundWindow has well-known restrictions (foreground
        // lock timeout, no input recently received) — failure here is
        // not fatal; the user's focus might already be where they
        // want it, or the OS will deliver paste to whatever window
        // happens to have focus at the moment, which usually matches.
        log::debug!("focus_capture: SetForegroundWindow returned false (lock timeout?)");
    }
    ok
}

#[cfg(not(target_os = "windows"))]
pub fn take_and_restore() -> bool {
    false
}

/// Drop any captured HWND without restoring. Useful for cancellation
/// paths — if the user cancels the dictation before it produces text,
/// we don't want a stale capture redirecting the NEXT dictation.
#[cfg(target_os = "windows")]
pub fn clear() {
    if let Ok(mut guard) = CAPTURED_HWND.lock() {
        *guard = None;
    }
}

#[cfg(not(target_os = "windows"))]
pub fn clear() {}
