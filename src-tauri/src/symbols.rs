//! Symbol extraction — pulls visible identifiers out of the focused
//! editor so Vibe Coding's backtick-wrapping pass has something to
//! recognise.
//!
//! Phase Finalize.C of the OpenWhisper roadmap.
//!
//! ## Platform implementations
//!
//! - **Windows**: UI Automation tree walk on the focused HWND. Looks
//!   for `Edit` / `Document` controls and extracts identifier-shaped
//!   tokens (`[A-Za-z_][A-Za-z0-9_]*` longer than 3 chars).
//! - **macOS**: Stub. The proper implementation goes through the AX
//!   API (`AXUIElement` + `AXSelectedText` + `AXVisibleCharacterRange`)
//!   and needs an `objc2` dependency we don't pull in yet. Returns an
//!   empty `Vec` so Vibe Coding falls back to "no known symbols".
//! - **Linux X11**: Stub. AT-SPI (`atk` bridge) is the equivalent but
//!   not every distro ships it; deferred until we know what users have.
//!
//! ## Why "best effort"
//!
//! Variable recognition is a quality-of-life pass — it's nice when it
//! fires but the product still works without it. Missing or wrong
//! symbols produce, at worst, no change to the dictated text. So this
//! module is allowed to fail silently: any extraction error returns an
//! empty list instead of bubbling up.
//!
//! ## Caching
//!
//! Not cached. `extract_visible_symbols` is called once per dictation
//! at process_transcription_output time and the focused app may have
//! changed since the last call. The Windows UIA walk runs in ~10–30 ms
//! on a typical editor — well below the budget for the LLM call that
//! follows.

use crate::active_app::ActiveApp;

/// Maximum number of symbols returned. The variable-recognition pass
/// builds a HashMap from this list; keeping it bounded prevents
/// pathological "edit a 100k-line file" cases from blowing memory or
/// stalling the dictation pipeline.
pub const MAX_SYMBOLS: usize = 1024;

/// Minimum length of a token to be considered an identifier worth
/// wrapping. Short tokens (1–3 chars) generate too many false
/// positives — common English words like "if", "or", "and" would
/// otherwise get backtick-wrapped.
pub const MIN_SYMBOL_LEN: usize = 4;

/// Pull visible identifiers from the focused editor. Returns an empty
/// `Vec` for unsupported platforms / unrecognised apps / any failure.
///
/// Caller is `process_transcription_output` in `actions.rs`. It feeds
/// the result into `vibe_coding::apply` as the `known_symbols` slice.
pub fn extract_visible_symbols(app: &ActiveApp) -> Vec<String> {
    // Only fire for editors / agent terminals — there's no point
    // walking the UI tree of a browser. `vibe_coding::classify` will
    // make the final call but we cheap-check here so we don't spend
    // 30 ms on every Slack dictation.
    let context = crate::vibe_coding::classify(app);
    if matches!(context, crate::vibe_coding::VibeContext::None) {
        return Vec::new();
    }

    #[cfg(target_os = "windows")]
    {
        windows_extract(app).unwrap_or_default()
    }
    #[cfg(not(target_os = "windows"))]
    {
        // macOS / Linux: stub until AX API / AT-SPI wiring lands.
        let _ = app;
        Vec::new()
    }
}

/// Windows UI Automation extraction. Walks the focused window's UIA
/// tree, grabs the value of `Edit` / `Document` controls, and tokenises
/// out anything that looks like a programming identifier.
///
/// Failure mode: returns `None`. Callers map to empty.
#[cfg(target_os = "windows")]
fn windows_extract(_app: &ActiveApp) -> Option<Vec<String>> {
    use std::collections::HashSet;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, IUIAutomation, IUIAutomationElement, TreeScope_Descendants, UIA_NamePropertyId,
        UIA_ValueValuePropertyId,
    };
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    };

    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        // Best-effort COM init — already initialised on most threads.
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);

        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let root: IUIAutomationElement = automation.ElementFromHandle(hwnd).ok()?;

        // We don't filter by control type because the actual editor
        // surface in VS Code / Cursor is a custom WebView2 control
        // that doesn't report as `Edit`. Pulling `Name` and `Value` of
        // every descendant and tokenising gives broad coverage.
        let condition = automation.CreateTrueCondition().ok()?;
        let descendants = root
            .FindAll(TreeScope_Descendants, &condition)
            .ok()?;

        let mut out: HashSet<String> = HashSet::new();
        let count = descendants.Length().unwrap_or(0);
        // Cap iteration even before we hit MAX_SYMBOLS — pathologically
        // large editor trees (1M+ nodes) would otherwise dominate the
        // pipeline budget.
        let cap = count.min(20_000);
        for i in 0..cap {
            let Ok(el) = descendants.GetElement(i) else {
                continue;
            };
            for prop_id in &[UIA_NamePropertyId, UIA_ValueValuePropertyId] {
                if let Ok(variant) = el.GetCurrentPropertyValue(*prop_id) {
                    // UIA returns a VARIANT — we lean on the Debug
                    // impl to coerce to text (covers BSTR + other
                    // string-shaped variants). Empty / wrong-type
                    // variants fall through to no-op tokenisation.
                    let s = format!("{:?}", variant);
                    tokenize_into(&s, &mut out);
                    if out.len() >= MAX_SYMBOLS {
                        return Some(out.into_iter().collect());
                    }
                }
            }
        }
        Some(out.into_iter().collect())
    }
}

/// Pull identifier-shaped tokens out of arbitrary text and push them
/// into `out`. Splits on anything that isn't `[A-Za-z0-9_]`, then
/// filters by `MIN_SYMBOL_LEN` and rejects all-lowercase words that
/// look like normal English (no camelCase / snake_case / digits).
#[cfg(target_os = "windows")]
fn tokenize_into(text: &str, out: &mut std::collections::HashSet<String>) {
    let mut buf = String::new();
    let flush = |buf: &mut String, out: &mut std::collections::HashSet<String>| {
        if buf.len() >= MIN_SYMBOL_LEN && looks_like_identifier(buf) {
            out.insert(buf.clone());
        }
        buf.clear();
    };
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            buf.push(ch);
        } else {
            flush(&mut buf, out);
        }
    }
    flush(&mut buf, out);
}

/// True when `s` looks more like a programming identifier than a
/// natural-language word: contains an underscore, a digit, or an
/// internal uppercase letter. Pure lowercase letter sequences are
/// rejected — those are almost always normal words.
#[cfg(target_os = "windows")]
fn looks_like_identifier(s: &str) -> bool {
    let has_underscore = s.contains('_');
    let has_digit = s.chars().any(|c| c.is_ascii_digit());
    let has_internal_upper = s
        .chars()
        .skip(1)
        .any(|c| c.is_ascii_uppercase());
    has_underscore || has_digit || has_internal_upper
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn looks_like_identifier_accepts_camel_case() {
        assert!(looks_like_identifier("userSettings"));
        assert!(looks_like_identifier("HttpClient"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn looks_like_identifier_accepts_snake_case() {
        assert!(looks_like_identifier("user_settings"));
        assert!(looks_like_identifier("MAX_RETRIES"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn looks_like_identifier_accepts_digits() {
        assert!(looks_like_identifier("x509"));
        assert!(looks_like_identifier("base64"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn looks_like_identifier_rejects_plain_words() {
        assert!(!looks_like_identifier("hello"));
        assert!(!looks_like_identifier("application"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn tokenize_extracts_identifiers() {
        let mut out = std::collections::HashSet::new();
        tokenize_into("call userSettings.save_state() now", &mut out);
        assert!(out.contains("userSettings"));
        assert!(out.contains("save_state"));
        // 'call' and 'now' are plain words — should be rejected.
        assert!(!out.contains("call"));
        assert!(!out.contains("now"));
    }
}
