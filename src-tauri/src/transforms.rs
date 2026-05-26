//! Transforms — hotkey-bound, user-defined post-dictation rewrites.
//!
//! Phase 7.2 of the OpenWhisper roadmap (research/11-phase-1-tasks.md §7).
//!
//! ## What's a transform?
//!
//! After dictating a passage, the user presses a hotkey (default
//! `Win+Alt+1` = "Polish", `Win+Alt+2` = "Prompt Engineer") and the
//! transcribed text is rewritten in place by an LLM using the
//! transform's prompt. Differs from the regular Auto Cleanup pass
//! (Phase 1.9) in three ways:
//!
//! 1. Triggered by an explicit user hotkey, not automatically.
//! 2. Multiple transforms can be defined and bound to different
//!    hotkeys — the user picks which transformation to apply per
//!    dictation.
//! 3. The prompt is user-supplied (with sensible defaults shipped),
//!    not a fixed system prompt.
//!
//! Differs from Command Mode (Phase 7.1) in that there's no selection —
//! the transform operates on the text that was JUST dictated. Wispr
//! Flow calls these "Transforms"; we keep the same name.
//!
//! ## Defaults shipped
//!
//! - **Polish** — "improve clarity and conciseness without changing
//!   meaning".
//! - **Prompt Engineer** — "rewrite this as a high-quality LLM prompt".
//!
//! Both can be edited or deleted by the user. Custom transforms can be
//! added through the settings UI (Phase 7.2b, queued).
//!
//! ## Persistence
//!
//! Transforms live in `AppSettings::transforms: Vec<TransformBinding>`
//! (added in this commit). The list is part of the regular tauri-store
//! settings JSON, no separate database.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Maximum number of hotkey-bound transforms a user can have. High
/// enough that no normal user will hit it; low enough that the
/// hotkey config UI doesn't degenerate.
pub const MAX_TRANSFORMS: usize = 20;

/// A single user-defined transform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct TransformBinding {
    /// Stable identifier (UUID in production; tests use any string).
    pub id: String,
    /// Display name shown in the settings UI ("Polish", "Make casual",
    /// "Convert to bullet points", etc.).
    pub name: String,
    /// The prompt body. May reference `{TEXT}` as the placeholder for
    /// the dictated text — `apply_template` substitutes it. When the
    /// placeholder is absent the dictated text is appended at the end
    /// after a blank line.
    pub prompt: String,
    /// Optional global shortcut. `None` means the transform exists in
    /// the library but can't be triggered from a hotkey (only from a
    /// menu / future tray submenu). The string format is the same as
    /// Handy's `current_binding` field — interpreted by the hotkey
    /// layer, not by this module.
    #[serde(default)]
    pub hotkey: Option<String>,
}

/// Errors produced when validating a transform at create / import time.
#[derive(Debug, PartialEq, Eq)]
pub enum TransformValidationError {
    EmptyName,
    EmptyPrompt,
    NameTooLong { len: usize, max: usize },
    PromptTooLong { len: usize, max: usize },
}

impl std::fmt::Display for TransformValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "transform name must not be empty"),
            Self::EmptyPrompt => write!(f, "transform prompt must not be empty"),
            Self::NameTooLong { len, max } => {
                write!(f, "transform name too long ({} chars, max {})", len, max)
            }
            Self::PromptTooLong { len, max } => {
                write!(f, "transform prompt too long ({} chars, max {})", len, max)
            }
        }
    }
}

impl std::error::Error for TransformValidationError {}

/// Maximum length of a transform name. 80 chars matches Wispr Flow's
/// UI affordance.
pub const MAX_NAME_LEN: usize = 80;

/// Maximum length of a transform prompt. 4000 chars matches the
/// snippets cap for symmetry.
pub const MAX_PROMPT_LEN: usize = 4000;

/// Construct a transform with validation. Use this rather than the
/// struct literal so the input cleanup is consistent.
pub fn new(
    id: impl Into<String>,
    name: impl Into<String>,
    prompt: impl Into<String>,
) -> Result<TransformBinding, TransformValidationError> {
    let id = id.into();
    let name = name.into();
    let prompt = prompt.into();

    if name.trim().is_empty() {
        return Err(TransformValidationError::EmptyName);
    }
    if prompt.trim().is_empty() {
        return Err(TransformValidationError::EmptyPrompt);
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(TransformValidationError::NameTooLong {
            len: name.chars().count(),
            max: MAX_NAME_LEN,
        });
    }
    if prompt.chars().count() > MAX_PROMPT_LEN {
        return Err(TransformValidationError::PromptTooLong {
            len: prompt.chars().count(),
            max: MAX_PROMPT_LEN,
        });
    }

    Ok(TransformBinding {
        id,
        name,
        prompt,
        hotkey: None,
    })
}

/// Two defaults shipped with every fresh install. Mirrors Wispr Flow's
/// "Polish" + "Prompt Engineer" defaults.
pub fn default_transforms() -> Vec<TransformBinding> {
    vec![
        TransformBinding {
            id: "default_polish".to_string(),
            name: "Polish".to_string(),
            prompt:
                "Improve the clarity and conciseness of the following text without changing \
                 its meaning. Keep the speaker's voice and tone. Output only the rewritten \
                 text.\n\n{TEXT}"
                    .to_string(),
            hotkey: None,
        },
        TransformBinding {
            id: "default_prompt_engineer".to_string(),
            name: "Prompt Engineer".to_string(),
            prompt:
                "Rewrite the following as a high-quality LLM prompt. Add structure, be explicit \
                 about the task and the desired output format, and preserve the speaker's \
                 intent. Output only the rewritten prompt.\n\n{TEXT}"
                    .to_string(),
            hotkey: None,
        },
    ]
}

/// Substitute the dictated text into the transform's prompt template.
/// Replaces every `{TEXT}` placeholder; if the placeholder is absent
/// (a hand-crafted prompt that forgot it), appends the text after the
/// prompt with a blank line separator so the LLM still sees both.
pub fn apply_template(transform: &TransformBinding, dictated_text: &str) -> String {
    if transform.prompt.contains("{TEXT}") {
        transform.prompt.replace("{TEXT}", dictated_text)
    } else {
        format!("{}\n\n{}", transform.prompt.trim_end(), dictated_text)
    }
}

/// Look up a transform by id. Returns `None` for unknown ids; the
/// caller surfaces this as "this hotkey isn't bound to a transform
/// any more" so users who delete a transform without rebinding the
/// hotkey get a clear error instead of silent failure.
pub fn find_by_id<'a>(
    transforms: &'a [TransformBinding],
    id: &str,
) -> Option<&'a TransformBinding> {
    transforms.iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_name() {
        assert_eq!(
            new("a", "", "prompt"),
            Err(TransformValidationError::EmptyName),
        );
    }

    #[test]
    fn new_rejects_empty_prompt() {
        assert_eq!(
            new("a", "name", ""),
            Err(TransformValidationError::EmptyPrompt),
        );
    }

    #[test]
    fn new_enforces_name_cap() {
        let too_long = "x".repeat(MAX_NAME_LEN + 1);
        match new("a", &too_long, "prompt").unwrap_err() {
            TransformValidationError::NameTooLong { len, max } => {
                assert_eq!(len, MAX_NAME_LEN + 1);
                assert_eq!(max, MAX_NAME_LEN);
            }
            other => panic!("expected NameTooLong, got {:?}", other),
        }
    }

    #[test]
    fn new_enforces_prompt_cap() {
        let too_long = "x".repeat(MAX_PROMPT_LEN + 1);
        match new("a", "name", &too_long).unwrap_err() {
            TransformValidationError::PromptTooLong { len, max } => {
                assert_eq!(len, MAX_PROMPT_LEN + 1);
                assert_eq!(max, MAX_PROMPT_LEN);
            }
            other => panic!("expected PromptTooLong, got {:?}", other),
        }
    }

    #[test]
    fn defaults_ship_polish_and_prompt_engineer() {
        let d = default_transforms();
        assert_eq!(d.len(), 2);
        assert!(d.iter().any(|t| t.name == "Polish"));
        assert!(d.iter().any(|t| t.name == "Prompt Engineer"));
    }

    #[test]
    fn defaults_contain_text_placeholder() {
        for t in default_transforms() {
            assert!(
                t.prompt.contains("{TEXT}"),
                "default transform '{}' missing {{TEXT}} placeholder",
                t.name,
            );
        }
    }

    #[test]
    fn apply_template_substitutes_placeholder() {
        let t = TransformBinding {
            id: "x".into(),
            name: "x".into(),
            prompt: "Make this casual: {TEXT}".into(),
            hotkey: None,
        };
        assert_eq!(apply_template(&t, "Hello world"), "Make this casual: Hello world");
    }

    #[test]
    fn apply_template_substitutes_multiple_placeholders() {
        let t = TransformBinding {
            id: "x".into(),
            name: "x".into(),
            prompt: "Original: {TEXT}\n---\nRevise: {TEXT}".into(),
            hotkey: None,
        };
        let out = apply_template(&t, "hi");
        assert!(out.contains("Original: hi"));
        assert!(out.contains("Revise: hi"));
    }

    #[test]
    fn apply_template_appends_when_no_placeholder() {
        let t = TransformBinding {
            id: "x".into(),
            name: "x".into(),
            prompt: "Make it shorter.".into(),
            hotkey: None,
        };
        let out = apply_template(&t, "Hello world");
        assert!(out.contains("Make it shorter."));
        assert!(out.contains("Hello world"));
        // Blank line separates prompt from text so the LLM doesn't
        // run them together.
        assert!(out.contains("\n\nHello world"));
    }

    #[test]
    fn find_by_id_returns_matching_transform() {
        let d = default_transforms();
        let found = find_by_id(&d, "default_polish");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "Polish");
    }

    #[test]
    fn find_by_id_returns_none_for_unknown_id() {
        let d = default_transforms();
        assert!(find_by_id(&d, "nonexistent").is_none());
    }
}
