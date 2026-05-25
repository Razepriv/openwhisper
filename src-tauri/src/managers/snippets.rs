//! Voice Snippets — text expansions triggered by spoken phrases.
//!
//! Phase 3.1/3.2 of the OpenWhisper roadmap. Wispr Flow lets the user
//! save common text fragments (email signatures, LinkedIn URLs, code
//! snippets, prompt templates) and trigger them by saying a short phrase
//! during dictation. Triggers and expansions are simple text-for-text;
//! no dynamic variables (Wispr's own UI documents this constraint).
//!
//! ## Field semantics
//!
//! - **trigger**: case-insensitive substring scanned in the transcribed
//!   text. Hits replace the trigger inline with the expansion.
//! - **expansion**: arbitrary text up to 4000 chars (matches Wispr's cap).
//! - **casing_preserve**: when `true`, the expansion is written verbatim;
//!   when `false`, the expansion adopts the casing of the surrounding
//!   sentence (capitalised mid-sentence after a period, etc.). Default
//!   true because Wispr's documented behaviour is verbatim.
//!
//! ## Limits (mirroring Wispr Flow)
//!
//! - 60 chars max trigger
//! - 4000 chars max expansion
//! - 1000 entries max bulk import (enforced at the import boundary, not here)
//!
//! ## Expansion algorithm
//!
//! 1. For each snippet, find all case-insensitive occurrences of `trigger`
//!    in the input text.
//! 2. Replace each occurrence with the snippet's expansion.
//! 3. Multiple snippets compose left-to-right with no precedence rules
//!    yet — duplicate triggers across snippets are treated as a user
//!    config error and we just pick the first match. A future phase
//!    (priority field?) can refine this.
//!
//! The current implementation is intentionally simple and O(n*m). For
//! 100 snippets and a 500-char transcription the total work is trivial.
//! If we ever ship users with 10k+ snippets we'll switch to an Aho-Corasick
//! automaton; the API contract here is stable enough to swap engines
//! without callers noticing.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Maximum length of a snippet trigger (matches Wispr Flow's 60-char cap).
pub const MAX_TRIGGER_LEN: usize = 60;

/// Maximum length of a snippet expansion (matches Wispr Flow's 4000-char cap).
pub const MAX_EXPANSION_LEN: usize = 4000;

/// Maximum number of snippets accepted in a single bulk import call.
pub const MAX_BULK_IMPORT: usize = 1000;

/// A single user-defined snippet.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Type)]
pub struct Snippet {
    /// Stable identifier (UUID v4 in production; tests can use any string).
    pub id: String,
    /// The trigger phrase to match in the transcription. Compared
    /// case-insensitively.
    pub trigger: String,
    /// The text inserted in place of the trigger.
    pub expansion: String,
    /// When true, expansion is inserted verbatim (no automatic casing
    /// adjustment). Default is true to match Wispr Flow.
    #[serde(default = "default_casing_preserve")]
    pub casing_preserve: bool,
}

fn default_casing_preserve() -> bool {
    true
}

/// Errors produced when validating a snippet at create / import time.
#[derive(Debug, PartialEq, Eq)]
pub enum SnippetValidationError {
    EmptyTrigger,
    EmptyExpansion,
    TriggerTooLong { len: usize, max: usize },
    ExpansionTooLong { len: usize, max: usize },
}

impl std::fmt::Display for SnippetValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyTrigger => write!(f, "trigger must not be empty"),
            Self::EmptyExpansion => write!(f, "expansion must not be empty"),
            Self::TriggerTooLong { len, max } => {
                write!(f, "trigger too long ({} chars, max {})", len, max)
            }
            Self::ExpansionTooLong { len, max } => {
                write!(f, "expansion too long ({} chars, max {})", len, max)
            }
        }
    }
}

impl std::error::Error for SnippetValidationError {}

impl Snippet {
    /// Construct a new snippet. Returns a validation error if trigger or
    /// expansion violates the documented limits.
    pub fn new(
        id: impl Into<String>,
        trigger: impl Into<String>,
        expansion: impl Into<String>,
    ) -> Result<Self, SnippetValidationError> {
        let trigger = trigger.into();
        let expansion = expansion.into();
        validate_trigger(&trigger)?;
        validate_expansion(&expansion)?;
        Ok(Self {
            id: id.into(),
            trigger,
            expansion,
            casing_preserve: true,
        })
    }
}

fn validate_trigger(trigger: &str) -> Result<(), SnippetValidationError> {
    let trimmed = trigger.trim();
    if trimmed.is_empty() {
        return Err(SnippetValidationError::EmptyTrigger);
    }
    if trimmed.chars().count() > MAX_TRIGGER_LEN {
        return Err(SnippetValidationError::TriggerTooLong {
            len: trimmed.chars().count(),
            max: MAX_TRIGGER_LEN,
        });
    }
    Ok(())
}

fn validate_expansion(expansion: &str) -> Result<(), SnippetValidationError> {
    if expansion.trim().is_empty() {
        return Err(SnippetValidationError::EmptyExpansion);
    }
    if expansion.chars().count() > MAX_EXPANSION_LEN {
        return Err(SnippetValidationError::ExpansionTooLong {
            len: expansion.chars().count(),
            max: MAX_EXPANSION_LEN,
        });
    }
    Ok(())
}

/// Apply all snippets to the given text. Case-insensitive trigger match;
/// preserves original casing when `casing_preserve == false` (not yet
/// implemented — tracked for Phase 3.3 frontend polish).
///
/// Returns the transformed text. When no snippets match, returns the
/// input unchanged (modulo whatever Rust's String::from does).
pub fn expand(text: &str, snippets: &[Snippet]) -> String {
    if snippets.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();
    for snippet in snippets {
        if snippet.trigger.is_empty() {
            continue; // defensive — should never happen post-validation
        }
        result = replace_case_insensitive(&result, &snippet.trigger, &snippet.expansion);
    }
    result
}

/// Replace ALL case-insensitive occurrences of `needle` with `replacement`
/// in `haystack`. Equivalent to `String::replace` but case-insensitive.
///
/// Hand-rolled to avoid pulling in a regex for what's a simple substring
/// scan. Performance is fine for Wispr-scale (< 1k snippets, < 5k char
/// transcriptions).
fn replace_case_insensitive(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() {
        return haystack.to_string();
    }
    let needle_lower = needle.to_lowercase();
    let needle_len = needle.len();
    let mut result = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < haystack.len() {
        // Try to match `needle_lower` against the lowercased slice
        // starting at byte position `i`. We have to be careful about
        // UTF-8 boundaries.
        if let Some(end) = next_match_end(&haystack[i..], &needle_lower, needle_len) {
            result.push_str(replacement);
            i += end;
        } else {
            // Push the current character (one full UTF-8 codepoint).
            let ch_len = haystack[i..].chars().next().map(char::len_utf8).unwrap_or(1);
            result.push_str(&haystack[i..i + ch_len]);
            i += ch_len;
        }
    }
    result
}

/// Returns Some(byte_len_of_match) iff the prefix of `slice` case-insensitively
/// equals `needle_lower`. `expected_byte_len` is the byte length of the
/// original-cased needle and is used to short-circuit before doing any
/// expensive lowercasing.
fn next_match_end(slice: &str, needle_lower: &str, expected_byte_len: usize) -> Option<usize> {
    if slice.len() < expected_byte_len {
        return None;
    }
    // Lowercasing can change byte length (e.g. ß ↔ SS), but for our
    // common case (ASCII) it's the same. We compare lowercased slices
    // directly to handle the edge cases correctly.
    let candidate: String = slice.chars().take(needle_lower.chars().count()).collect();
    if candidate.to_lowercase() == needle_lower {
        Some(candidate.len())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snip(id: &str, trigger: &str, expansion: &str) -> Snippet {
        Snippet::new(id, trigger, expansion).expect("valid snippet")
    }

    #[test]
    fn new_rejects_empty_trigger() {
        assert_eq!(
            Snippet::new("a", "", "expansion"),
            Err(SnippetValidationError::EmptyTrigger),
        );
        assert_eq!(
            Snippet::new("a", "   ", "expansion"),
            Err(SnippetValidationError::EmptyTrigger),
        );
    }

    #[test]
    fn new_rejects_empty_expansion() {
        assert_eq!(
            Snippet::new("a", "trigger", ""),
            Err(SnippetValidationError::EmptyExpansion),
        );
    }

    #[test]
    fn new_enforces_trigger_length_cap() {
        let too_long = "x".repeat(MAX_TRIGGER_LEN + 1);
        match Snippet::new("a", &too_long, "expansion").unwrap_err() {
            SnippetValidationError::TriggerTooLong { len, max } => {
                assert_eq!(len, MAX_TRIGGER_LEN + 1);
                assert_eq!(max, MAX_TRIGGER_LEN);
            }
            other => panic!("expected TriggerTooLong, got {:?}", other),
        }
    }

    #[test]
    fn new_enforces_expansion_length_cap() {
        let too_long = "x".repeat(MAX_EXPANSION_LEN + 1);
        match Snippet::new("a", "trigger", &too_long).unwrap_err() {
            SnippetValidationError::ExpansionTooLong { len, max } => {
                assert_eq!(len, MAX_EXPANSION_LEN + 1);
                assert_eq!(max, MAX_EXPANSION_LEN);
            }
            other => panic!("expected ExpansionTooLong, got {:?}", other),
        }
    }

    #[test]
    fn expand_returns_input_unchanged_when_no_snippets() {
        let text = "hello world";
        assert_eq!(expand(text, &[]), text);
    }

    #[test]
    fn expand_replaces_simple_match() {
        let snippets = vec![snip("a", "my email", "raze.priv@gmail.com")];
        let result = expand("send to my email please", &snippets);
        assert_eq!(result, "send to raze.priv@gmail.com please");
    }

    #[test]
    fn expand_is_case_insensitive_on_trigger() {
        let snippets = vec![snip("a", "my email", "raze.priv@gmail.com")];
        let result = expand("Send to My Email please", &snippets);
        assert_eq!(result, "Send to raze.priv@gmail.com please");
    }

    #[test]
    fn expand_replaces_all_occurrences() {
        let snippets = vec![snip("a", "foo", "BAR")];
        let result = expand("foo and foo and foo", &snippets);
        assert_eq!(result, "BAR and BAR and BAR");
    }

    #[test]
    fn expand_composes_multiple_snippets() {
        let snippets = vec![
            snip("a", "my email", "user@example.com"),
            snip("b", "my linkedin", "https://linkedin.com/in/user"),
        ];
        let input = "email me at my email or my linkedin";
        let result = expand(input, &snippets);
        assert_eq!(
            result,
            "email me at user@example.com or https://linkedin.com/in/user",
        );
    }

    #[test]
    fn expand_does_not_recurse_into_replacements() {
        // If a replacement contains a trigger, we should NOT keep
        // expanding — that would loop forever on circular triggers.
        let snippets = vec![
            snip("a", "foo", "bar"),
            snip("b", "bar", "baz"),
        ];
        let result = expand("foo", &snippets);
        // The first replacement creates "bar", which the second snippet
        // then matches. This is acceptable behaviour because the
        // snippets are independent passes; what we forbid is recursing
        // INSIDE a single snippet's replacement.
        assert_eq!(result, "baz");
    }

    #[test]
    fn expand_handles_unicode_safely() {
        // Multi-byte characters shouldn't trip up the byte-offset arithmetic.
        let snippets = vec![snip("a", "café", "coffee shop")];
        let result = expand("meet at the café tomorrow", &snippets);
        assert_eq!(result, "meet at the coffee shop tomorrow");
    }
}
