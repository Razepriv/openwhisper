//! Vibe Coding — IDE-aware dictation behaviour for code editors and CLI
//! coding assistants.
//!
//! Phase 6 of the OpenWhisper roadmap (research/11-phase-1-tasks.md §6).
//! User directive (2026-05-25): add explicit support for **Claude Code**
//! and **Codex** alongside the IDEs originally in scope.
//!
//! ## Supported environments
//!
//! Two distinct families with different capabilities:
//!
//! **A. Graphical IDEs** — full Vibe Coding treatment:
//! - Cursor (`cursor`, `cursor.exe`)
//! - VS Code (`code`, `code.exe`, also `Code` on macOS)
//! - Windsurf (`windsurf`, `windsurf.exe`)
//! - JetBrains family is best-effort: same heuristic catches IntelliJ /
//!   PyCharm / WebStorm / GoLand by process name.
//!
//! For these, when the user dictates we:
//! 1. Wrap recognised code identifiers in backticks
//!    (`"the user settings object"` → `the \`userSettings\` object`).
//! 2. Recognise file-tagging cues (`"tag main.py"`, `"at main.py"`,
//!    `"@main.py"`, bare `"main.py"`) and produce `@main.py` in the
//!    text — Cursor / Windsurf chat panels treat that as a file ref.
//! 3. Preserve `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`
//!    casing on identifiers we explicitly know about.
//!
//! **B. CLI coding assistants in a terminal** — lighter touch:
//! - **Claude Code** — Anthropic's CLI, detected by:
//!   - process tree containing `claude` or `claude-code`, OR
//!   - terminal window title containing `claude code` (case insensitive)
//! - **Codex** — OpenAI's CLI coding agent, detected by process /
//!   title containing `codex`.
//! - Plus the terminals these typically run in: Warp, iTerm2,
//!   Terminal.app, Windows Terminal, kitty, alacritty, gnome-terminal.
//!
//! For terminal contexts we apply file tagging (Claude Code understands
//! `@filename` references too) but skip backtick-wrapping (the shell
//! interprets backticks as command substitution — wrapping there would
//! be actively harmful).
//!
//! ## Why heuristic detection
//!
//! No public API tells us "Claude Code is running in this terminal".
//! Process tree walking is more robust than parsing window titles —
//! titles get clobbered by `screen` / `tmux` — but neither is bullet-
//! proof. The matcher is generous about false positives (recognising a
//! shell where Claude Code happens to be installed but isn't running)
//! because the worst-case is "we run file tagging where the user didn't
//! want it", which produces innocuous `@filename` strings that the
//! shell shrugs at.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::active_app::ActiveApp;

/// Which Vibe Coding mode applies to the currently-focused app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum VibeContext {
    /// Cursor / VS Code / Windsurf / JetBrains — full treatment.
    GraphicalIde,
    /// Claude Code / Codex / generic shell running a coding agent.
    /// File tagging applies; backtick wrapping does not.
    AgentTerminal,
    /// Not a coding environment — Vibe Coding is a no-op.
    None,
}

impl VibeContext {
    /// True when file-tagging substitutions (`tag main.py` → `@main.py`)
    /// should run.
    pub fn supports_file_tagging(self) -> bool {
        matches!(self, VibeContext::GraphicalIde | VibeContext::AgentTerminal)
    }

    /// True when identifier backtick-wrapping should run. Terminals
    /// must not — the shell evaluates backticks.
    pub fn supports_backtick_wrapping(self) -> bool {
        matches!(self, VibeContext::GraphicalIde)
    }
}

/// Substring markers we look for in `ActiveApp::matchable()`. Order
/// doesn't matter — first hit wins by category.
const GRAPHICAL_IDE_MARKERS: &[&str] = &[
    // VS Code family
    "code.exe",
    " code ",
    "/code",
    "\\code",
    "vscode",
    "code - insiders",
    // VS Code AI forks
    "cursor",
    "windsurf",
    "trae", // Bytedance's AI IDE
    "void",  // Open-source AI fork of VS Code
    "zed",   // Zed.dev — fast collaborative editor with AI
    // JetBrains family
    "intellij",
    "pycharm",
    "webstorm",
    "goland",
    "rustrover",
    "rider",
    "phpstorm",
    "clion",
    "rubymine",
    "datagrip",
    "appcode",
    "androidstudio",
    "android studio",
    // Other widely used editors
    "sublime_text",
    "sublime text",
    "notepad++",
    "atom",
    "brackets",
    "geany",
    // Modern JetBrains-adjacent
    "fleet", // JetBrains Fleet
];

const AGENT_TERMINAL_MARKERS: &[&str] = &[
    "claude code",
    "claude-code",
    "claude_code",
    "claude.cli",
    "codex", // OpenAI Codex CLI
    "aider", // Bonus: aider is another agentic CLI users care about.
];

const TERMINAL_HOST_MARKERS: &[&str] = &[
    "warp",
    "iterm",
    "terminal.app",
    "windowsterminal",
    "windows terminal",
    "wt.exe",
    "kitty",
    "alacritty",
    "gnome-terminal",
    "konsole",
    "powershell",
    "cmd.exe",
    "bash",
    "zsh",
    "fish",
];

/// Classify which Vibe Coding mode applies to the given app snapshot.
pub fn classify(app: &ActiveApp) -> VibeContext {
    if app.is_empty() {
        return VibeContext::None;
    }
    let m = app.matchable();

    // Agent terminals win over generic IDE matching: a user running
    // Claude Code inside VS Code's integrated terminal should get the
    // terminal treatment, not the IDE treatment.
    if AGENT_TERMINAL_MARKERS.iter().any(|marker| m.contains(marker)) {
        return VibeContext::AgentTerminal;
    }

    // Standalone terminal apps default to AgentTerminal — better to
    // run file-tagging where the user might want it and have them
    // turn it off, than to silently no-op in every shell session.
    if TERMINAL_HOST_MARKERS.iter().any(|marker| m.contains(marker)) {
        return VibeContext::AgentTerminal;
    }

    if GRAPHICAL_IDE_MARKERS.iter().any(|marker| m.contains(marker)) {
        return VibeContext::GraphicalIde;
    }

    VibeContext::None
}

/// Recognise spoken file-tagging cues and rewrite them to `@filename`
/// form. Handles:
/// - `"tag main.py"` / `"tagged main.py"` / `"at main.py"`
/// - `"@main.py"` (already correct — left alone)
/// - bare `"main.py"` — left alone (would generate too many false
///   positives; the user must say "tag" or "at" to opt in).
///
/// Multi-segment paths like `"src slash main dot py"` are out of scope
/// for v1 — the LLM cleanup pass tends to handle dots/slashes well
/// enough. Phase 6.1b can add an explicit pass if users ask.
pub fn apply_file_tagging(text: &str) -> String {
    use std::borrow::Cow;

    // Trigger words that introduce a filename. Order matters: longer
    // first so "tagged" matches before "tag".
    const TRIGGERS: &[&str] = &["tagged ", "tag ", "at "];

    let mut result = Cow::Borrowed(text);
    for trigger in TRIGGERS {
        // Case-insensitive scan from the start. We rewrite in place
        // each time we hit a "<trigger><filename>" pattern.
        let mut out = String::with_capacity(result.len());
        let lower = result.to_lowercase();
        let mut cursor = 0;
        while cursor < result.len() {
            if let Some(found) = lower[cursor..].find(trigger) {
                let abs = cursor + found;
                // Carry forward the segment before the trigger.
                out.push_str(&result[cursor..abs]);

                // Skip the trigger word itself.
                let after = abs + trigger.len();
                // Grab the next whitespace-bounded token.
                let token_end = result[after..]
                    .find(|c: char| c.is_whitespace())
                    .map(|i| after + i)
                    .unwrap_or(result.len());
                let token = &result[after..token_end];

                if looks_like_filename(token) {
                    out.push('@');
                    out.push_str(token);
                } else {
                    // No filename followed the trigger — pass the
                    // trigger through unchanged.
                    out.push_str(&result[abs..token_end]);
                }
                cursor = token_end;
            } else {
                out.push_str(&result[cursor..]);
                break;
            }
        }
        result = Cow::Owned(out);
    }
    result.into_owned()
}

/// True if the token looks like a filename worth tagging. Conservative
/// heuristic: must contain a dot, must not start with `@` (already
/// tagged), must not be a pure number or URL.
fn looks_like_filename(token: &str) -> bool {
    if token.is_empty() || token.starts_with('@') {
        return false;
    }
    if !token.contains('.') {
        return false;
    }
    // URLs end up matching this otherwise — `http://x.com`.
    if token.contains("://") {
        return false;
    }
    // Allow leading dot for dotfiles (`.env`).
    let after_dot = token
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .unwrap_or("");
    if after_dot.is_empty() || !after_dot.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    true
}

/// Wrap recognised code identifiers in backticks. Looks for tokens
/// matching the configured `known_symbols` list (case-insensitive) and
/// rewrites the matched substring to its canonical casing wrapped in
/// backticks.
///
/// `known_symbols` should come from an extractor that reads the focused
/// editor's open file (Phase 6.1b will add that). For Phase 6 we accept
/// any caller-supplied list, which keeps the function unit-testable
/// without needing to mock the accessibility API.
///
/// Already-backticked code is preserved verbatim (no double-wrapping).
pub fn apply_variable_recognition(text: &str, known_symbols: &[String]) -> String {
    if known_symbols.is_empty() {
        return text.to_string();
    }

    // Build a lookup: lowercase token => canonical form.
    let by_lower: std::collections::HashMap<String, &str> = known_symbols
        .iter()
        .map(|s| (s.to_lowercase(), s.as_str()))
        .collect();

    let mut out = String::with_capacity(text.len());
    let mut in_backtick = false;
    let mut buf = String::new();

    let flush = |buf: &mut String, out: &mut String, by_lower: &std::collections::HashMap<String, &str>| {
        if buf.is_empty() {
            return;
        }
        // Strip punctuation only at the END for the lookup but keep
        // it in the output (so "userSettings," ends up as
        // "`userSettings`,").
        let trailing: String = buf
            .chars()
            .rev()
            .take_while(|c| matches!(c, ',' | '.' | ';' | ':' | '!' | '?' | ')' | ']'))
            .collect();
        let trailing: String = trailing.chars().rev().collect();
        let core = &buf[..buf.len() - trailing.len()];

        // Keep alphanumerics AND underscores — underscores are
        // structural in `SCREAMING_SNAKE_CASE` and `snake_case`
        // identifiers; dropping them prevents lookups from hitting
        // those entries.
        let lookup_key = core
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase();
        match by_lower.get(&lookup_key) {
            Some(canonical) => {
                out.push('`');
                out.push_str(canonical);
                out.push('`');
                out.push_str(&trailing);
            }
            None => out.push_str(buf),
        }
        buf.clear();
    };

    for ch in text.chars() {
        if ch == '`' {
            // Pass through backticks untouched + flip the "inside
            // pre-existing code span" flag so we don't double-wrap.
            flush(&mut buf, &mut out, &by_lower);
            out.push(ch);
            in_backtick = !in_backtick;
        } else if in_backtick {
            out.push(ch);
        } else if ch.is_whitespace() {
            flush(&mut buf, &mut out, &by_lower);
            out.push(ch);
        } else {
            buf.push(ch);
        }
    }
    flush(&mut buf, &mut out, &by_lower);
    out
}

/// Convenience pipeline used by `actions.rs` after transcription.
/// Composes file tagging and variable recognition based on the detected
/// context. No-op when the context is `None`.
pub fn apply(text: &str, app: &ActiveApp, known_symbols: &[String]) -> String {
    let context = classify(app);
    let mut current = text.to_string();
    if context.supports_file_tagging() {
        current = apply_file_tagging(&current);
    }
    if context.supports_backtick_wrapping() {
        current = apply_variable_recognition(&current, known_symbols);
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_app(process: &str, title: &str) -> ActiveApp {
        ActiveApp {
            process_name: process.to_string(),
            window_title: title.to_string(),
            window_class: String::new(),
        }
    }

    // ---- classify() ----

    #[test]
    fn classify_cursor_as_graphical_ide() {
        assert_eq!(
            classify(&make_app("cursor.exe", "MyProject — App.tsx")),
            VibeContext::GraphicalIde
        );
    }

    #[test]
    fn classify_vscode_as_graphical_ide() {
        assert_eq!(
            classify(&make_app("code.exe", "openwhisper - Visual Studio Code")),
            VibeContext::GraphicalIde
        );
    }

    #[test]
    fn classify_windsurf_as_graphical_ide() {
        assert_eq!(
            classify(&make_app("windsurf", "")),
            VibeContext::GraphicalIde
        );
    }

    #[test]
    fn classify_claude_code_as_agent_terminal() {
        assert_eq!(
            classify(&make_app("wt.exe", "claude code — main")),
            VibeContext::AgentTerminal
        );
        assert_eq!(
            classify(&make_app("claude-code", "")),
            VibeContext::AgentTerminal
        );
    }

    #[test]
    fn classify_codex_as_agent_terminal() {
        assert_eq!(
            classify(&make_app("codex", "")),
            VibeContext::AgentTerminal
        );
        assert_eq!(
            classify(&make_app("iterm.app", "codex — repo")),
            VibeContext::AgentTerminal
        );
    }

    #[test]
    fn classify_agent_marker_wins_over_ide() {
        // VS Code with an integrated terminal running Claude Code.
        // The title says both — agent terminal should win because
        // file-tagging fits but backticks would break the shell.
        let app = make_app("code.exe", "claude code — main");
        assert_eq!(classify(&app), VibeContext::AgentTerminal);
    }

    #[test]
    fn classify_terminal_host_as_agent_terminal() {
        assert_eq!(
            classify(&make_app("warp", "")),
            VibeContext::AgentTerminal
        );
        assert_eq!(
            classify(&make_app("powershell.exe", "")),
            VibeContext::AgentTerminal
        );
    }

    #[test]
    fn classify_chrome_as_none() {
        assert_eq!(
            classify(&make_app("chrome.exe", "OpenWhisper — GitHub")),
            VibeContext::None
        );
    }

    #[test]
    fn classify_empty_app_as_none() {
        assert_eq!(classify(&ActiveApp::default()), VibeContext::None);
    }

    // ---- VibeContext capabilities ----

    #[test]
    fn graphical_ide_supports_both_features() {
        assert!(VibeContext::GraphicalIde.supports_file_tagging());
        assert!(VibeContext::GraphicalIde.supports_backtick_wrapping());
    }

    #[test]
    fn agent_terminal_disables_backticks() {
        assert!(VibeContext::AgentTerminal.supports_file_tagging());
        assert!(!VibeContext::AgentTerminal.supports_backtick_wrapping());
    }

    #[test]
    fn none_disables_everything() {
        assert!(!VibeContext::None.supports_file_tagging());
        assert!(!VibeContext::None.supports_backtick_wrapping());
    }

    // ---- File tagging ----

    #[test]
    fn file_tagging_rewrites_tag_filename() {
        assert_eq!(apply_file_tagging("tag main.py"), "@main.py");
    }

    #[test]
    fn file_tagging_rewrites_at_filename() {
        assert_eq!(apply_file_tagging("at main.py"), "@main.py");
    }

    #[test]
    fn file_tagging_rewrites_tagged_filename() {
        assert_eq!(apply_file_tagging("tagged index.tsx"), "@index.tsx");
    }

    #[test]
    fn file_tagging_handles_dotfile_extension() {
        assert_eq!(apply_file_tagging("tag .env"), "@.env");
    }

    #[test]
    fn file_tagging_ignores_bare_filename_without_trigger() {
        // No "tag" / "at" — leave bare filenames alone to avoid
        // false-positive tagging of regular conversational text.
        assert_eq!(apply_file_tagging("main.py"), "main.py");
    }

    #[test]
    fn file_tagging_ignores_already_tagged() {
        assert_eq!(apply_file_tagging("tag @main.py"), "tag @main.py");
    }

    #[test]
    fn file_tagging_ignores_urls() {
        assert_eq!(
            apply_file_tagging("at http://example.com/x.py"),
            "at http://example.com/x.py"
        );
    }

    #[test]
    fn file_tagging_handles_multiple_in_one_sentence() {
        assert_eq!(
            apply_file_tagging("compare tag a.ts with tag b.ts"),
            "compare @a.ts with @b.ts"
        );
    }

    // ---- Variable recognition ----

    #[test]
    fn variable_recognition_wraps_known_identifier() {
        let symbols = vec!["userSettings".to_string()];
        let out = apply_variable_recognition("check the usersettings object", &symbols);
        assert_eq!(out, "check the `userSettings` object");
    }

    #[test]
    fn variable_recognition_preserves_canonical_casing() {
        let symbols = vec!["MAX_RETRIES".to_string(), "kSnakeCase".to_string()];
        let out = apply_variable_recognition("set max_retries and ksnakecase", &symbols);
        assert!(out.contains("`MAX_RETRIES`"));
        assert!(out.contains("`kSnakeCase`"));
    }

    #[test]
    fn variable_recognition_does_not_double_wrap_existing_backticks() {
        let symbols = vec!["userSettings".to_string()];
        let out = apply_variable_recognition("the `userSettings` field", &symbols);
        // Already backticked — pass through.
        assert_eq!(out, "the `userSettings` field");
    }

    #[test]
    fn variable_recognition_preserves_trailing_punctuation() {
        let symbols = vec!["userSettings".to_string()];
        let out = apply_variable_recognition("set usersettings, then save.", &symbols);
        assert!(out.contains("`userSettings`,"));
    }

    #[test]
    fn variable_recognition_noop_when_no_symbols() {
        let out = apply_variable_recognition("hello world", &[]);
        assert_eq!(out, "hello world");
    }

    // ---- Composed pipeline ----

    #[test]
    fn apply_runs_both_passes_in_ide() {
        let app = make_app("cursor.exe", "");
        let symbols = vec!["userSettings".to_string()];
        let out = apply("tag config.ts and check usersettings", &app, &symbols);
        assert!(out.contains("@config.ts"));
        assert!(out.contains("`userSettings`"));
    }

    #[test]
    fn apply_skips_backticks_in_agent_terminal() {
        let app = make_app("warp", "claude code");
        let symbols = vec!["userSettings".to_string()];
        let out = apply("tag config.ts and check usersettings", &app, &symbols);
        assert!(out.contains("@config.ts"));
        // Backticks would break the shell — must NOT be added.
        assert!(!out.contains('`'));
    }

    #[test]
    fn apply_is_noop_outside_coding_contexts() {
        let app = make_app("chrome.exe", "");
        let symbols = vec!["userSettings".to_string()];
        let input = "tag main.py and check usersettings";
        let out = apply(input, &app, &symbols);
        assert_eq!(out, input);
    }
}
