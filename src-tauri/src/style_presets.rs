//! Style presets — per-app-category cleanup tone (Formal, Casual, etc.)
//!
//! Phase Finalize.D of the OpenWhisper roadmap.
//!
//! ## What this does
//!
//! Wispr Flow lets you say "always use formal English in Gmail, but
//! casual in Slack". We do the same thing here: classify the focused
//! app into a coarse `AppCategory`, look up the user's preferred
//! `StylePreset` for that category, and inject a short style fragment
//! into the cleanup LLM's system prompt.
//!
//! ## Why this is a *pure* module
//!
//! Apart from `effective_preset` (which takes an `ActiveApp` snapshot),
//! everything here is data + functions over enums. That makes the
//! prompt fragments unit-testable and the classifier easy to extend
//! without touching the runtime pipeline.
//!
//! ## Sensible defaults out of the box
//!
//! The defaults map common business apps to `Formal`, chat apps to
//! `Casual`, and everything else to `Neutral`. The user can override
//! any category through the settings UI. This means a fresh install
//! "just works" — Email gets capitalized punctuation, Slack gets
//! contractions, Cursor gets concise prose.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::active_app::ActiveApp;

/// The styling buckets we ship. Wispr-flow-compatible — same labels,
/// same semantic intent, mapped to local-LLM prompt fragments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum StylePreset {
    /// "Polite, complete sentences, no contractions." Default for
    /// email + document apps.
    Formal,
    /// "Natural conversational tone, contractions OK, no formal
    /// salutations." Default for chat apps.
    Casual,
    /// "Loose, conversational, abbreviations OK." Default off — opt-in
    /// for users who actively want it.
    VeryCasual,
    /// "Concise + technical. Preserve code-y terms verbatim." Default
    /// for IDEs and agent terminals (alongside Vibe Coding).
    Concise,
    /// "Standard cleanup — fix punctuation and grammar; leave tone
    /// alone." This is the no-personality default for anything that
    /// doesn't match a known category.
    Neutral,
}

impl Default for StylePreset {
    fn default() -> Self {
        Self::Neutral
    }
}

impl StylePreset {
    /// The prompt fragment we splice into the cleanup system prompt.
    /// Short on purpose: every token is paid for in latency + context.
    pub fn system_fragment(self) -> &'static str {
        match self {
            Self::Formal => {
                "Write in a formal, professional tone. Complete sentences. \
                 No contractions. No slang."
            }
            Self::Casual => {
                "Write in a friendly, conversational tone. Contractions are fine. \
                 Avoid formal salutations."
            }
            Self::VeryCasual => {
                "Write casually, like a quick chat message. \
                 Short sentences and contractions are encouraged."
            }
            Self::Concise => {
                "Be concise and technical. Preserve identifiers, file paths, and \
                 code-shaped terms exactly as dictated."
            }
            Self::Neutral => {
                "Fix punctuation and grammar without changing tone or wording."
            }
        }
    }

    /// Display label for the settings UI. Pulled out so the frontend
    /// doesn't have to hard-code per-locale labels — though i18n keys
    /// still own translation in practice; this is the source-of-truth
    /// English copy used for the EN translation file.
    pub fn label(self) -> &'static str {
        match self {
            Self::Formal => "Formal",
            Self::Casual => "Casual",
            Self::VeryCasual => "Very Casual",
            Self::Concise => "Concise",
            Self::Neutral => "Neutral",
        }
    }
}

/// Coarse buckets the active app falls into. Matched by simple
/// substring inspection of `ActiveApp::matchable()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AppCategory {
    /// Gmail desktop client, Outlook, Apple Mail, Thunderbird, etc.
    Email,
    /// Slack, Discord, Teams, Telegram, Messenger, WhatsApp, etc.
    Chat,
    /// IDEs and agent terminals (Cursor, VS Code, JetBrains, Claude
    /// Code, Codex). Note: Vibe Coding ALSO runs on these — the style
    /// preset adds a tone hint on top of file-tagging / backticks.
    Code,
    /// Word / Google Docs / Notion / Obsidian — long-form writing.
    Document,
    /// Anything else.
    Other,
}

impl Default for AppCategory {
    fn default() -> Self {
        Self::Other
    }
}

impl AppCategory {
    /// Default style preset for this category when the user hasn't
    /// explicitly overridden it.
    pub fn default_preset(self) -> StylePreset {
        match self {
            Self::Email => StylePreset::Formal,
            Self::Chat => StylePreset::Casual,
            Self::Code => StylePreset::Concise,
            Self::Document => StylePreset::Formal,
            Self::Other => StylePreset::Neutral,
        }
    }
}

/// Classify the active app. Generous about false positives — when in
/// doubt, fall back to `Other` so the user sees the safe `Neutral`
/// preset rather than an inappropriate `Formal` or `VeryCasual` style.
pub fn classify_app(app: &ActiveApp) -> AppCategory {
    let m = app.matchable();
    if m.is_empty() {
        return AppCategory::Other;
    }

    for marker in CHAT_MARKERS {
        if m.contains(marker) {
            return AppCategory::Chat;
        }
    }
    for marker in EMAIL_MARKERS {
        if m.contains(marker) {
            return AppCategory::Email;
        }
    }
    for marker in CODE_MARKERS {
        if m.contains(marker) {
            return AppCategory::Code;
        }
    }
    for marker in DOCUMENT_MARKERS {
        if m.contains(marker) {
            return AppCategory::Document;
        }
    }
    AppCategory::Other
}

const CHAT_MARKERS: &[&str] = &[
    "slack",
    "discord",
    "teams",
    "telegram",
    "whatsapp",
    "messenger",
    "messages",
    "line.exe",
    "linelauncher",
    "signal",
    "matrix",
    "element",
    "skype",
];

const EMAIL_MARKERS: &[&str] = &[
    "outlook",
    "gmail",
    "mail.app",
    "thunderbird",
    "apple mail",
    "spark",
    "airmail",
];

const CODE_MARKERS: &[&str] = &[
    // IDEs (same list as vibe_coding for consistency)
    "cursor",
    "code.exe",
    " code ",
    "/code",
    "\\code",
    "windsurf",
    "vscode",
    "intellij",
    "pycharm",
    "webstorm",
    "goland",
    "rustrover",
    "rider",
    "phpstorm",
    "android studio",
    "xcode",
    "sublime_text",
    "sublime text",
    // Agent terminals
    "claude code",
    "codex",
    "aider",
];

const DOCUMENT_MARKERS: &[&str] = &[
    "winword",
    "word.exe",
    "microsoft word",
    "notion",
    "obsidian",
    "google docs",
    "docs.google.com",
    "pages",
    "scrivener",
    "ulysses",
    "bear.app",
    "bear ",
];

/// Pick the right style preset for this dictation. Checks the user's
/// per-category overrides first; falls back to the category's default.
///
/// `overrides` typically comes from `AppSettings.style_overrides` —
/// passing an empty map gives stock defaults.
pub fn effective_preset(
    app: &ActiveApp,
    overrides: &std::collections::HashMap<AppCategory, StylePreset>,
) -> StylePreset {
    let category = classify_app(app);
    overrides
        .get(&category)
        .copied()
        .unwrap_or_else(|| category.default_preset())
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

    #[test]
    fn classify_empty_app_is_other() {
        assert_eq!(classify_app(&ActiveApp::default()), AppCategory::Other);
    }

    #[test]
    fn classify_slack_as_chat() {
        assert_eq!(classify_app(&make_app("slack.exe", "")), AppCategory::Chat);
    }

    #[test]
    fn classify_outlook_as_email() {
        assert_eq!(
            classify_app(&make_app("outlook.exe", "Inbox")),
            AppCategory::Email
        );
    }

    #[test]
    fn classify_cursor_as_code() {
        assert_eq!(
            classify_app(&make_app("cursor.exe", "MyProject")),
            AppCategory::Code
        );
    }

    #[test]
    fn classify_word_as_document() {
        assert_eq!(
            classify_app(&make_app("winword.exe", "Document1 - Microsoft Word")),
            AppCategory::Document
        );
    }

    #[test]
    fn classify_unknown_is_other() {
        assert_eq!(
            classify_app(&make_app("randomthing.exe", "Weird Window")),
            AppCategory::Other
        );
    }

    #[test]
    fn default_presets_match_intent() {
        assert_eq!(AppCategory::Email.default_preset(), StylePreset::Formal);
        assert_eq!(AppCategory::Chat.default_preset(), StylePreset::Casual);
        assert_eq!(AppCategory::Code.default_preset(), StylePreset::Concise);
        assert_eq!(AppCategory::Document.default_preset(), StylePreset::Formal);
        assert_eq!(AppCategory::Other.default_preset(), StylePreset::Neutral);
    }

    #[test]
    fn effective_preset_uses_override_when_provided() {
        use std::collections::HashMap;
        let mut overrides = HashMap::new();
        overrides.insert(AppCategory::Chat, StylePreset::Formal);
        let preset = effective_preset(&make_app("slack.exe", ""), &overrides);
        assert_eq!(preset, StylePreset::Formal);
    }

    #[test]
    fn effective_preset_falls_back_to_default() {
        use std::collections::HashMap;
        let overrides: HashMap<AppCategory, StylePreset> = HashMap::new();
        let preset = effective_preset(&make_app("slack.exe", ""), &overrides);
        assert_eq!(preset, StylePreset::Casual);
    }

    #[test]
    fn fragments_are_non_empty() {
        for p in [
            StylePreset::Formal,
            StylePreset::Casual,
            StylePreset::VeryCasual,
            StylePreset::Concise,
            StylePreset::Neutral,
        ] {
            assert!(!p.system_fragment().is_empty());
            assert!(!p.label().is_empty());
        }
    }
}
