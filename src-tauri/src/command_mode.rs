//! Command Mode — "highlight some text, speak an instruction, get the
//! text replaced" — the most-loved Wispr Flow power feature.
//!
//! Phase 7.1 of the OpenWhisper roadmap (research/11-phase-1-tasks.md §7).
//!
//! ## Flow
//!
//! 1. User selects text in any app.
//! 2. User presses the Command Mode hotkey (default `Fn+Ctrl` on macOS,
//!    `Ctrl+Win+Alt` on Windows / Linux — wired in `shortcut/mod.rs`).
//! 3. OpenWhisper captures the selection by simulating Cmd/Ctrl+C and
//!    reading the clipboard (while remembering whatever was on the
//!    clipboard before, so we can restore it afterwards).
//! 4. User speaks the instruction ("make this more assertive",
//!    "translate to Polish", "turn this into a bullet list").
//! 5. When the user releases the hotkey, the spoken text becomes the
//!    `instruction`, the captured text is the `selection`, both are
//!    sent to the LLM cleanup backend.
//! 6. The LLM response replaces the selection by writing it to the
//!    clipboard + simulating Cmd/Ctrl+V.
//!
//! ## Why a separate module
//!
//! `actions.rs` already does dictation → cleanup → paste. Command Mode
//! is a different shape (selection + instruction → LLM → replace) and
//! shares almost no code. Keeping it in its own module avoids growing
//! `actions.rs` past its already-substantial size and lets us unit-test
//! the pure helpers (prompt builder, max-words guard, etc.) without
//! the actions-pipeline machinery.
//!
//! ## Hard caps
//!
//! - **Selection cap: 1000 words.** Mirrors Wispr Flow's documented
//!   limit. Larger selections produce a `CommandModeError::SelectionTooLong`
//!   so the UI can surface a clear error instead of mysteriously
//!   silent failure when the LLM truncates.
//! - **Instruction cap: 200 words.** Defensive: prevents a runaway
//!   dictation from soaking up most of the prompt budget.
//!
//! Both caps are word counts (not chars / tokens) because that's the
//! number users intuit about ("a paragraph is ~150 words").

use serde::{Deserialize, Serialize};
use specta::Type;

/// Maximum number of words in the highlighted selection. Larger
/// selections are rejected — see `validate_selection`.
pub const MAX_SELECTION_WORDS: usize = 1000;

/// Maximum number of words in the spoken instruction.
pub const MAX_INSTRUCTION_WORDS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum CommandModeError {
    /// User pressed the hotkey but the clipboard was empty after the
    /// Cmd/Ctrl+C simulation. Probably means no text was selected.
    NoSelection,
    /// The captured selection exceeds the documented word cap.
    SelectionTooLong { words: usize, max: usize },
    /// The spoken instruction exceeds its cap.
    InstructionTooLong { words: usize, max: usize },
    /// The instruction was empty after trimming — user pressed the
    /// hotkey but didn't actually speak.
    EmptyInstruction,
}

impl std::fmt::Display for CommandModeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSelection => {
                write!(f, "no text was selected when Command Mode was triggered")
            }
            Self::SelectionTooLong { words, max } => {
                write!(
                    f,
                    "selection too long: {} words (max {}). Try a smaller selection.",
                    words, max
                )
            }
            Self::InstructionTooLong { words, max } => {
                write!(f, "instruction too long: {} words (max {})", words, max)
            }
            Self::EmptyInstruction => {
                write!(f, "no instruction was spoken")
            }
        }
    }
}

impl std::error::Error for CommandModeError {}

/// Validated inputs to a Command Mode rewrite request.
///
/// Construct with `prepare(selection, instruction)` rather than
/// directly — the constructor runs the word-cap + emptiness checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct CommandModeRequest {
    pub selection: String,
    pub instruction: String,
}

/// Run the input validation contract and return a ready-to-send request.
///
/// `selection` is whatever came back from the clipboard after the
/// Cmd/Ctrl+C simulation. `instruction` is the transcribed text from
/// the spoken instruction.
pub fn prepare(
    selection: impl Into<String>,
    instruction: impl Into<String>,
) -> Result<CommandModeRequest, CommandModeError> {
    let selection = selection.into();
    let instruction = instruction.into();

    if selection.trim().is_empty() {
        return Err(CommandModeError::NoSelection);
    }
    if instruction.trim().is_empty() {
        return Err(CommandModeError::EmptyInstruction);
    }

    let sel_words = word_count(&selection);
    if sel_words > MAX_SELECTION_WORDS {
        return Err(CommandModeError::SelectionTooLong {
            words: sel_words,
            max: MAX_SELECTION_WORDS,
        });
    }

    let ins_words = word_count(&instruction);
    if ins_words > MAX_INSTRUCTION_WORDS {
        return Err(CommandModeError::InstructionTooLong {
            words: ins_words,
            max: MAX_INSTRUCTION_WORDS,
        });
    }

    Ok(CommandModeRequest {
        selection,
        instruction,
    })
}

/// Whitespace-split word count. Good enough for the user-facing cap;
/// not a tokenizer, doesn't try to handle CJK character-based counts
/// (those users are unlikely to hit a 1000-word cap with normal
/// selections anyway).
pub fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Build the system + user prompt pair that goes to the LLM. Returns
/// `(system_prompt, user_prompt)`.
///
/// Separated out as a pure function so the prompt design can be unit-
/// tested and tweaked without spinning up the full pipeline. Changes
/// here should be discussed in the roadmap before shipping — the
/// prompt is part of the product surface, not an implementation detail.
pub fn build_prompts(request: &CommandModeRequest) -> (String, String) {
    let system_prompt = "You are a precise text editor. The user will give you a passage of \
                         text and an instruction. Apply the instruction to the passage and \
                         return ONLY the rewritten text — no commentary, no quotes, no \
                         markdown unless the source already used it. Preserve the source's \
                         language unless explicitly asked to translate.".to_string();

    let user_prompt = format!(
        "INSTRUCTION:\n{}\n\nTEXT TO EDIT:\n{}",
        request.instruction.trim(),
        request.selection.trim()
    );

    (system_prompt, user_prompt)
}

/// If the LLM produced an output that's identical to the input
/// (or differs only in whitespace), return true. Callers surface this
/// as "Your text looks good!" to the user instead of replacing the
/// selection with itself.
pub fn is_no_op(original: &str, rewritten: &str) -> bool {
    original.trim() == rewritten.trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_rejects_empty_selection() {
        assert_eq!(
            prepare("", "make it more concise"),
            Err(CommandModeError::NoSelection),
        );
        assert_eq!(
            prepare("   \n   ", "make it more concise"),
            Err(CommandModeError::NoSelection),
        );
    }

    #[test]
    fn prepare_rejects_empty_instruction() {
        assert_eq!(
            prepare("hello world", ""),
            Err(CommandModeError::EmptyInstruction),
        );
    }

    #[test]
    fn prepare_rejects_selection_over_cap() {
        let big = (0..MAX_SELECTION_WORDS + 5)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");
        match prepare(big, "do a thing").unwrap_err() {
            CommandModeError::SelectionTooLong { words, max } => {
                assert_eq!(words, MAX_SELECTION_WORDS + 5);
                assert_eq!(max, MAX_SELECTION_WORDS);
            }
            other => panic!("expected SelectionTooLong, got {:?}", other),
        }
    }

    #[test]
    fn prepare_rejects_instruction_over_cap() {
        let big_instruction = (0..MAX_INSTRUCTION_WORDS + 1)
            .map(|i| format!("word{}", i))
            .collect::<Vec<_>>()
            .join(" ");
        match prepare("selection", big_instruction).unwrap_err() {
            CommandModeError::InstructionTooLong { words, max } => {
                assert_eq!(words, MAX_INSTRUCTION_WORDS + 1);
                assert_eq!(max, MAX_INSTRUCTION_WORDS);
            }
            other => panic!("expected InstructionTooLong, got {:?}", other),
        }
    }

    #[test]
    fn prepare_accepts_valid_input() {
        let req = prepare("hello world", "translate to french").unwrap();
        assert_eq!(req.selection, "hello world");
        assert_eq!(req.instruction, "translate to french");
    }

    #[test]
    fn build_prompts_includes_both_instruction_and_selection() {
        let req = prepare("the cat sat on the mat", "make it more dramatic").unwrap();
        let (system, user) = build_prompts(&req);
        assert!(!system.is_empty());
        assert!(user.contains("make it more dramatic"));
        assert!(user.contains("the cat sat on the mat"));
        // Must distinguish the two sections so the LLM doesn't mix them.
        assert!(user.contains("INSTRUCTION"));
        assert!(user.contains("TEXT TO EDIT"));
    }

    #[test]
    fn build_prompts_trims_whitespace_from_inputs() {
        let req = CommandModeRequest {
            selection: "  hi  ".to_string(),
            instruction: "  rewrite  ".to_string(),
        };
        let (_system, user) = build_prompts(&req);
        // Inputs go in trimmed, otherwise the LLM sees the noise.
        assert!(user.contains("INSTRUCTION:\nrewrite"));
        assert!(user.contains("TEXT TO EDIT:\nhi"));
    }

    #[test]
    fn is_no_op_treats_identical_text_as_no_op() {
        assert!(is_no_op("hello", "hello"));
        assert!(is_no_op("hello", "  hello  ")); // whitespace differences
    }

    #[test]
    fn is_no_op_is_false_when_text_changed() {
        assert!(!is_no_op("hello", "hi"));
    }

    #[test]
    fn word_count_basics() {
        assert_eq!(word_count(""), 0);
        assert_eq!(word_count("hello"), 1);
        assert_eq!(word_count("hello world"), 2);
        assert_eq!(word_count("  hello   world  "), 2);
    }
}
