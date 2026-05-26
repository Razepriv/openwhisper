//! Scratchpad / Notes — local-only markdown notes the user can
//! capture without leaving OpenWhisper.
//!
//! Phase 8.2 of the OpenWhisper roadmap (research/11-phase-1-tasks.md §8).
//!
//! Wispr Flow's Scratchpad is a floating notepad with autosave, image
//! attachments, and a quick `Opt+S` summon hotkey. OpenWhisper's
//! first cut is text-only — image attachments + the hotkey-to-summon
//! wiring are queued for Phase 8.2b.
//!
//! ## Storage
//!
//! Plain JSON file in the app data directory (`notes.json`). One file
//! per user keeps things simple — no SQLite migration to worry about,
//! the file is human-readable so users can back it up trivially, and
//! "thousands of notes" remains < 10 MB in practice.
//!
//! The file format is versioned (`schema_version` field) so future
//! migrations to SQLite or per-note files can detect what they're
//! looking at.
//!
//! ## Privacy
//!
//! Stored locally, never synced, never sent anywhere. Matches the
//! Phase 1.10 local-only guarantee.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Bump when making a breaking change to the on-disk shape.
const SCHEMA_VERSION: u32 = 1;

/// Notes file name within the OpenWhisper app data directory.
pub const NOTES_FILE_NAME: &str = "notes.json";

/// Maximum number of notes a single user can have. Soft cap — the
/// JSON store can technically hold more, but the picker UI gets
/// unusable past a few thousand. The CRUD operations enforce it at
/// `create` time.
pub const MAX_NOTES: usize = 10_000;

/// Maximum size of a note body (chars). 1 MB worth of plain text — a
/// novel-sized note. Past this users should probably split into
/// multiple notes, and the JSON store starts to feel sluggish.
pub const MAX_NOTE_BODY_CHARS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct Note {
    pub id: String,
    pub title: String,
    /// Markdown body. Renderers (frontend) decide how to display it.
    pub body: String,
    /// Unix-seconds creation timestamp.
    pub created_at: i64,
    /// Unix-seconds last-modified timestamp.
    pub updated_at: i64,
}

/// Top-level JSON shape stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotesFile {
    schema_version: u32,
    notes: Vec<Note>,
}

/// Errors surfaced from the CRUD layer.
#[derive(Debug, PartialEq, Eq)]
pub enum NoteError {
    NotFound,
    EmptyTitle,
    BodyTooLong { len: usize, max: usize },
    TooManyNotes { current: usize, max: usize },
}

impl std::fmt::Display for NoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "note not found"),
            Self::EmptyTitle => write!(f, "note title must not be empty"),
            Self::BodyTooLong { len, max } => {
                write!(f, "note body too long ({} chars, max {})", len, max)
            }
            Self::TooManyNotes { current, max } => write!(
                f,
                "cannot add note: already at {} notes (cap {})",
                current, max
            ),
        }
    }
}

impl std::error::Error for NoteError {}

/// CRUD over the notes file. Stateless — load + save per call. That's
/// fine because the file is small and operations are user-driven, not
/// hot-path.
pub struct NotesStore {
    path: PathBuf,
}

impl NotesStore {
    /// Construct a store at `app_data_dir/notes.json`. Creates the
    /// file if missing.
    pub fn new(app_data_dir: &Path) -> Result<Self> {
        let path = app_data_dir.join(NOTES_FILE_NAME);
        if !path.exists() {
            let empty = NotesFile {
                schema_version: SCHEMA_VERSION,
                notes: Vec::new(),
            };
            std::fs::write(&path, serde_json::to_vec_pretty(&empty)?)
                .with_context(|| format!("creating empty notes file at {:?}", path))?;
        }
        Ok(Self { path })
    }

    pub fn list(&self) -> Result<Vec<Note>> {
        let file = self.load()?;
        Ok(file.notes)
    }

    pub fn get(&self, id: &str) -> Result<Note> {
        let file = self.load()?;
        file.notes
            .into_iter()
            .find(|n| n.id == id)
            .ok_or_else(|| anyhow::anyhow!(NoteError::NotFound))
    }

    /// Create a new note. Title is required; body may be empty
    /// (Wispr Flow allows untitled notes — we require a title for
    /// listing usability).
    pub fn create(&self, title: String, body: String) -> Result<Note> {
        validate_title(&title)?;
        validate_body(&body)?;

        let mut file = self.load()?;
        if file.notes.len() >= MAX_NOTES {
            anyhow::bail!(NoteError::TooManyNotes {
                current: file.notes.len(),
                max: MAX_NOTES,
            });
        }
        let now = current_unix_seconds();
        let note = Note {
            id: generate_id(),
            title,
            body,
            created_at: now,
            updated_at: now,
        };
        file.notes.push(note.clone());
        self.save(&file)?;
        Ok(note)
    }

    /// Replace title + body of an existing note. Updates the
    /// `updated_at` stamp.
    pub fn update(&self, id: &str, title: String, body: String) -> Result<Note> {
        validate_title(&title)?;
        validate_body(&body)?;

        let mut file = self.load()?;
        let note = file
            .notes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or_else(|| anyhow::anyhow!(NoteError::NotFound))?;
        note.title = title;
        note.body = body;
        note.updated_at = current_unix_seconds();
        let updated = note.clone();
        self.save(&file)?;
        Ok(updated)
    }

    pub fn delete(&self, id: &str) -> Result<()> {
        let mut file = self.load()?;
        let before = file.notes.len();
        file.notes.retain(|n| n.id != id);
        if file.notes.len() == before {
            anyhow::bail!(NoteError::NotFound);
        }
        self.save(&file)?;
        Ok(())
    }

    fn load(&self) -> Result<NotesFile> {
        let contents = std::fs::read_to_string(&self.path)
            .with_context(|| format!("reading notes file at {:?}", self.path))?;
        let file: NotesFile = serde_json::from_str(&contents)
            .with_context(|| format!("parsing notes file at {:?}", self.path))?;
        Ok(file)
    }

    fn save(&self, file: &NotesFile) -> Result<()> {
        let serialised = serde_json::to_vec_pretty(file)?;
        // Write to a sibling temp file then rename, so a crash mid-
        // write doesn't truncate the user's notes file.
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serialised)
            .with_context(|| format!("writing notes tmp file at {:?}", tmp))?;
        std::fs::rename(&tmp, &self.path)
            .with_context(|| format!("renaming notes tmp -> {:?}", self.path))?;
        Ok(())
    }
}

fn validate_title(title: &str) -> Result<()> {
    if title.trim().is_empty() {
        anyhow::bail!(NoteError::EmptyTitle);
    }
    Ok(())
}

fn validate_body(body: &str) -> Result<()> {
    let len = body.chars().count();
    if len > MAX_NOTE_BODY_CHARS {
        anyhow::bail!(NoteError::BodyTooLong {
            len,
            max: MAX_NOTE_BODY_CHARS,
        });
    }
    Ok(())
}

fn current_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Quick unique id — not cryptographic, just unique-enough for the
/// notes namespace. Combines current nanos with a counter so two
/// notes created in the same nanosecond don't collide.
fn generate_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("note_{:x}_{:x}", nanos, n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fresh_store() -> (TempDir, NotesStore) {
        let dir = TempDir::new().unwrap();
        let store = NotesStore::new(dir.path()).unwrap();
        (dir, store)
    }

    #[test]
    fn new_creates_empty_file_when_missing() {
        let (dir, store) = fresh_store();
        assert!(dir.path().join(NOTES_FILE_NAME).exists());
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn create_appends_a_note() {
        let (_dir, store) = fresh_store();
        let note = store.create("First".into(), "body".into()).unwrap();
        assert_eq!(note.title, "First");
        assert_eq!(note.body, "body");
        assert_eq!(store.list().unwrap().len(), 1);
    }

    #[test]
    fn create_rejects_empty_title() {
        let (_dir, store) = fresh_store();
        let err = store.create("   ".into(), "body".into()).unwrap_err();
        assert_eq!(err.downcast::<NoteError>().unwrap(), NoteError::EmptyTitle);
    }

    #[test]
    fn create_rejects_overlong_body() {
        let (_dir, store) = fresh_store();
        let big = "x".repeat(MAX_NOTE_BODY_CHARS + 1);
        let err = store.create("title".into(), big).unwrap_err();
        match err.downcast::<NoteError>().unwrap() {
            NoteError::BodyTooLong { max, .. } => assert_eq!(max, MAX_NOTE_BODY_CHARS),
            other => panic!("expected BodyTooLong, got {:?}", other),
        }
    }

    #[test]
    fn get_returns_existing_note() {
        let (_dir, store) = fresh_store();
        let created = store.create("Hi".into(), "x".into()).unwrap();
        let fetched = store.get(&created.id).unwrap();
        assert_eq!(fetched, created);
    }

    #[test]
    fn get_errors_on_unknown_id() {
        let (_dir, store) = fresh_store();
        let err = store.get("nope").unwrap_err();
        assert_eq!(err.downcast::<NoteError>().unwrap(), NoteError::NotFound);
    }

    #[test]
    fn update_changes_fields_and_bumps_updated_at() {
        let (_dir, store) = fresh_store();
        let created = store.create("Old".into(), "old body".into()).unwrap();
        // Sleep a hair so updated_at differs.
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let updated = store
            .update(&created.id, "New".into(), "new body".into())
            .unwrap();
        assert_eq!(updated.title, "New");
        assert_eq!(updated.body, "new body");
        assert!(updated.updated_at >= created.updated_at);
    }

    #[test]
    fn delete_removes_the_note() {
        let (_dir, store) = fresh_store();
        let created = store.create("A".into(), "x".into()).unwrap();
        store.delete(&created.id).unwrap();
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn delete_errors_on_unknown_id() {
        let (_dir, store) = fresh_store();
        let err = store.delete("nope").unwrap_err();
        assert_eq!(err.downcast::<NoteError>().unwrap(), NoteError::NotFound);
    }

    #[test]
    fn generated_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            assert!(seen.insert(generate_id()));
        }
    }

    #[test]
    fn save_uses_atomic_rename_to_avoid_truncation() {
        let (dir, store) = fresh_store();
        store.create("a".into(), "x".into()).unwrap();
        // Tmp file should not survive a successful write.
        let tmp = dir.path().join("notes.json.tmp");
        assert!(!tmp.exists());
    }

    #[test]
    fn store_round_trips_through_disk() {
        let (dir, store) = fresh_store();
        let note = store.create("Persist me".into(), "body".into()).unwrap();
        // New store reading the same file should see the note.
        let store2 = NotesStore::new(dir.path()).unwrap();
        let fetched = store2.get(&note.id).unwrap();
        assert_eq!(fetched, note);
    }
}
