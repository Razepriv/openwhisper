//! Personal Dictionary — structured replacement for the flat `custom_words`.
//!
//! Phase 2.1 of the OpenWhisper roadmap. Wispr Flow's Personal Dictionary
//! supports `(term, optional_replacement, starred)` triples plus
//! auto-learn-from-corrections. Handy's original `custom_words: Vec<String>`
//! only carried the term. This module adds the richer structure while
//! preserving backward compatibility with existing user settings.
//!
//! ## Migration contract
//!
//! - **Read path:** legacy `custom_words: Vec<String>` deserialises as
//!   before. On manager init we check if `dictionary_entries` is empty
//!   while `custom_words` is not, and copy each string over as
//!   `DictionaryEntry { term, replacement: None, starred: false, source: Migrated }`.
//! - **Write path:** new code writes to `dictionary_entries` only. We do
//!   NOT continue to mirror writes into `custom_words` — that would risk
//!   data loss if a string-only client deletes an entry that the
//!   structured client just edited.
//! - **Bias build:** when constructing the Whisper `initial_prompt`, the
//!   manager prefers structured entries; falls back to `custom_words` only
//!   if the migration hasn't run yet (defensive belt-and-suspenders).
//!
//! ## Whisper `initial_prompt` length guard
//!
//! Whisper's prompt has a hard ~244-token limit. Past that, terms at the
//! end are silently truncated. Phase 2.2 (separate ticket) will implement
//! starred-first prioritisation; for Phase 2.1 we cap at the first
//! `MAX_BIAS_TERMS` entries in dictionary order. The cap is conservative
//! (200) to leave headroom for non-Latin scripts that tokenise more
//! densely.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Hard cap on the number of dictionary terms passed to Whisper as bias.
/// Conservative — see module docs.
pub const MAX_BIAS_TERMS: usize = 200;

/// Where this dictionary entry came from. Drives UX hints in the settings
/// table ("auto-learned from your corrections") and is used by analytics
/// in a future Insights phase.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "lowercase")]
pub enum DictionaryEntrySource {
    /// User typed it into the dictionary settings UI.
    #[default]
    Manual,
    /// Promoted from `custom_words: Vec<String>` by the migration step.
    Migrated,
    /// Auto-learned from a user correction of a previous transcription.
    /// (Implementation lands in a later phase.)
    Learned,
    /// Imported from a JSON file (bulk import).
    Imported,
}

/// Structured dictionary entry. Mirrors the Wispr Flow shape closely
/// enough to make the UI work straightforward.
///
/// `term` is what the user wants Whisper to recognise. `replacement` is
/// the optional canonical form Flow substitutes after transcription when
/// the heard text matches `term` fuzzily. For pure ASR-bias entries
/// `replacement` is `None`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Type)]
pub struct DictionaryEntry {
    /// The term as Whisper should recognise / produce it (e.g. "Sarbanes-Oxley").
    pub term: String,
    /// Optional canonical replacement applied as a post-transcription
    /// substitution when Whisper produces a fuzzy match for the term.
    /// `None` means "use the term as-is" — biasing only.
    #[serde(default)]
    pub replacement: Option<String>,
    /// Starred entries get priority in the bias-prompt assembly when the
    /// dictionary exceeds the Whisper prompt budget.
    #[serde(default)]
    pub starred: bool,
    /// Where this entry came from. Read-only metadata.
    #[serde(default)]
    pub source: DictionaryEntrySource,
}

impl DictionaryEntry {
    /// Construct a plain manual entry. Most common path from the settings UI.
    pub fn manual(term: impl Into<String>) -> Self {
        Self {
            term: term.into(),
            replacement: None,
            starred: false,
            source: DictionaryEntrySource::Manual,
        }
    }

    /// Construct an entry migrated from a legacy `custom_words` string.
    /// Marked as `Migrated` so the UI can disclose its provenance.
    pub fn from_legacy(term: impl Into<String>) -> Self {
        Self {
            term: term.into(),
            replacement: None,
            starred: false,
            source: DictionaryEntrySource::Migrated,
        }
    }
}

/// Build the Whisper `initial_prompt` from a dictionary. Starred entries
/// come first, then by insertion order. Capped at `MAX_BIAS_TERMS` to
/// stay under Whisper's ~244-token limit.
///
/// Returns an empty string when the dictionary is empty so the caller can
/// just pass it through to Whisper without a None-check.
pub fn build_initial_prompt(entries: &[DictionaryEntry]) -> String {
    let mut ordered: Vec<&DictionaryEntry> = entries.iter().collect();
    // Stable sort: starred entries float to the top, original order
    // preserved within each group.
    ordered.sort_by(|a, b| b.starred.cmp(&a.starred));
    ordered
        .into_iter()
        .take(MAX_BIAS_TERMS)
        .map(|e| e.term.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

/// One-shot migration: produce `Vec<DictionaryEntry>` from a legacy
/// `Vec<String>` of custom words. Idempotent in the sense that calling
/// it on an empty input returns an empty output; the caller is responsible
/// for only invoking it when `dictionary_entries.is_empty()`.
pub fn migrate_from_custom_words(custom_words: &[String]) -> Vec<DictionaryEntry> {
    custom_words
        .iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| DictionaryEntry::from_legacy(s.trim()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_entry_has_manual_source() {
        let entry = DictionaryEntry::manual("Sarbanes-Oxley");
        assert_eq!(entry.term, "Sarbanes-Oxley");
        assert_eq!(entry.source, DictionaryEntrySource::Manual);
        assert!(!entry.starred);
        assert!(entry.replacement.is_none());
    }

    #[test]
    fn legacy_entry_has_migrated_source() {
        let entry = DictionaryEntry::from_legacy("prima facie");
        assert_eq!(entry.source, DictionaryEntrySource::Migrated);
    }

    #[test]
    fn migrate_drops_empty_strings_and_trims() {
        let custom_words = vec![
            "Foo".to_string(),
            "  ".to_string(),
            "".to_string(),
            "  Bar  ".to_string(),
        ];
        let migrated = migrate_from_custom_words(&custom_words);
        assert_eq!(migrated.len(), 2);
        assert_eq!(migrated[0].term, "Foo");
        assert_eq!(migrated[1].term, "Bar");
    }

    #[test]
    fn build_initial_prompt_joins_with_comma_space() {
        let entries = vec![
            DictionaryEntry::manual("Alpha"),
            DictionaryEntry::manual("Beta"),
            DictionaryEntry::manual("Gamma"),
        ];
        assert_eq!(build_initial_prompt(&entries), "Alpha, Beta, Gamma");
    }

    #[test]
    fn build_initial_prompt_floats_starred_entries_first() {
        let entries = vec![
            DictionaryEntry::manual("Alpha"),
            DictionaryEntry {
                term: "Beta".to_string(),
                replacement: None,
                starred: true,
                source: DictionaryEntrySource::Manual,
            },
            DictionaryEntry::manual("Gamma"),
        ];
        // Starred Beta should appear first, then unstarred entries in
        // insertion order (Alpha, Gamma).
        assert_eq!(build_initial_prompt(&entries), "Beta, Alpha, Gamma");
    }

    #[test]
    fn build_initial_prompt_caps_at_max_bias_terms() {
        let entries: Vec<DictionaryEntry> = (0..MAX_BIAS_TERMS + 50)
            .map(|i| DictionaryEntry::manual(format!("term_{}", i)))
            .collect();
        let prompt = build_initial_prompt(&entries);
        // Count commas + 1 = entries. Cap = MAX_BIAS_TERMS.
        let count = prompt.split(", ").count();
        assert_eq!(count, MAX_BIAS_TERMS);
    }

    #[test]
    fn build_initial_prompt_handles_empty_dictionary() {
        assert_eq!(build_initial_prompt(&[]), "");
    }
}
