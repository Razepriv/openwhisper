//! Insights / Voice Profile — analytics computed from the local
//! transcription history.
//!
//! Phase 8.1 of the OpenWhisper roadmap (research/11-phase-1-tasks.md §8).
//!
//! ## What gets surfaced
//!
//! Per Wispr Flow's Voice Profile dashboard:
//! - Total words dictated
//! - Words per minute estimate
//! - Most-used words ("catch phrases")
//! - Daily / weekly streaks
//! - Activity heatmap (entries per day)
//! - Hour-of-day breakdown ("peak dictation hours")
//!
//! ## Privacy
//!
//! Everything here reads from the LOCAL SQLite history database
//! (`managers/history.rs`). No data is aggregated to a server.
//! No transcription text leaves the machine. The frontend can render
//! the dashboard 100% offline. Matches the local-only guarantee
//! enforced in Phase 1.10.
//!
//! ## Design choices
//!
//! - Pure functions over `&[HistoryEntry]` — trivially testable, no
//!   AppHandle dependency. The Tauri command in `commands/mod.rs`
//!   handles the I/O of fetching the entries.
//! - WPM is an estimate, not measured. Wispr Flow has the actual
//!   recording duration per entry; Handy's HistoryEntry doesn't store
//!   it. We approximate as `words / estimated_seconds` where
//!   `estimated_seconds` is computed from a fixed words-per-second
//!   floor. Future Phase 8.1b: extend HistoryEntry with
//!   `duration_seconds` and switch to real WPM.
//! - All timestamp arithmetic uses the `chrono` crate (already a dep)
//!   for tz-aware day boundaries. The user's local timezone defines
//!   "today" — necessary for streaks to feel right.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use specta::Type;

use super::history::HistoryEntry;

/// Estimated dictation speed cap. Used to derive a `duration_seconds`
/// proxy when the entry doesn't carry a real duration. 3 wps =
/// 180 wpm, slightly above the typical 150 wpm fast dictation rate;
/// this floor avoids divide-by-tiny-number WPM spikes on short entries.
const ESTIMATED_WORDS_PER_SECOND: f64 = 3.0;

/// Threshold below which a word is considered too short to be a
/// "catch phrase". Drops articles + stop-words from the top-words list.
const CATCH_PHRASE_MIN_LEN: usize = 4;

/// How many top entries to return in summary fields (top apps, top
/// words, peak hours).
const TOP_N: usize = 5;

/// Voice Profile snapshot returned to the frontend dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct VoiceProfile {
    /// Total words dictated, all-time.
    pub total_words: u64,
    /// Number of recorded dictation sessions.
    pub total_sessions: u64,
    /// Estimated words per minute, averaged across sessions.
    /// `None` when there are no sessions.
    pub estimated_wpm: Option<u32>,
    /// Top N most-frequent dictated words (lowercase, stop-words
    /// removed, sorted by frequency descending). Length ≤ `TOP_N`.
    pub top_words: Vec<TopWord>,
    /// Per-day word totals for the trailing 30 days (most recent
    /// first). Used to render the streak heatmap.
    pub daily_word_counts: Vec<DailyWordCount>,
    /// Hour-of-day buckets (0–23) with cumulative word totals.
    /// Used to render "you dictate most at 9am" style insights.
    pub hourly_word_counts: Vec<HourlyWordCount>,
    /// Current consecutive-day dictation streak ending today.
    pub current_streak_days: u32,
    /// Longest consecutive-day streak ever recorded.
    pub longest_streak_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TopWord {
    pub word: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct DailyWordCount {
    /// ISO-8601 calendar date in the user's local timezone (YYYY-MM-DD).
    pub date: String,
    pub words: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct HourlyWordCount {
    /// 0..=23, hour-of-day in the user's local timezone.
    pub hour: u8,
    pub words: u64,
}

/// Compute a fresh `VoiceProfile` from the full history list.
///
/// Pure function — pass in the entries, get a profile back. Designed
/// to be called on demand from the Insights dashboard. Cost: O(N)
/// over the history. For Wispr-scale history (tens of thousands of
/// entries) this is single-digit milliseconds.
pub fn compute(entries: &[HistoryEntry]) -> VoiceProfile {
    let mut total_words: u64 = 0;
    let mut total_session_seconds: f64 = 0.0;
    let mut word_freq: HashMap<String, u64> = HashMap::new();
    let mut daily: HashMap<NaiveDate, u64> = HashMap::new();
    let mut hourly: [u64; 24] = [0; 24];

    for entry in entries {
        // Prefer post-processed text when available — that's what the
        // user actually committed. Falls back to raw transcription.
        let text = entry
            .post_processed_text
            .as_deref()
            .unwrap_or(&entry.transcription_text);
        let words = count_words(text);
        total_words += words as u64;
        total_session_seconds += words as f64 / ESTIMATED_WORDS_PER_SECOND;

        for word in normalised_words(text) {
            *word_freq.entry(word).or_insert(0) += 1;
        }

        if let Some(dt) = local_datetime(entry.timestamp) {
            *daily.entry(dt.date_naive()).or_insert(0) += words as u64;
            hourly[dt.hour() as usize] += words as u64;
        }
    }

    let estimated_wpm = if total_session_seconds > 0.0 {
        Some(((total_words as f64 * 60.0) / total_session_seconds).round() as u32)
    } else {
        None
    };

    let top_words = top_n_words(word_freq, TOP_N);
    let daily_word_counts = trailing_30_day_counts(&daily);
    let hourly_word_counts = hourly_buckets(&hourly);
    let (current_streak_days, longest_streak_days) = compute_streaks(&daily);

    VoiceProfile {
        total_words,
        total_sessions: entries.len() as u64,
        estimated_wpm,
        top_words,
        daily_word_counts,
        hourly_word_counts,
        current_streak_days,
        longest_streak_days,
    }
}

/// Whitespace-tokenized word count. Same definition as
/// `command_mode::word_count` — kept duplicate intentionally so the
/// modules don't have a hidden cross-dependency.
fn count_words(text: &str) -> usize {
    text.split_whitespace().count()
}

/// Lowercase, alphabetic-only word iterator for frequency stats. Drops
/// punctuation, numbers, and very short tokens (typically articles).
fn normalised_words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split_whitespace().filter_map(|raw| {
        let lower: String = raw
            .chars()
            .filter(|c| c.is_alphabetic())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        if lower.len() >= CATCH_PHRASE_MIN_LEN && !is_stop_word(&lower) {
            Some(lower)
        } else {
            None
        }
    })
}

/// Tiny stop-word list — enough to suppress the obvious "the / and / for"
/// from dominating the top-words. Not a full NLP stop list (that would
/// need a per-language dictionary which is out of scope here).
fn is_stop_word(word: &str) -> bool {
    const STOP_WORDS: &[&str] = &[
        "this", "that", "with", "from", "have", "they", "what",
        "your", "will", "would", "could", "should", "their", "there",
        "about", "which", "when", "then", "than", "into", "just",
        "like", "some", "more", "also", "only", "even", "after",
        "before", "because", "between", "through", "really", "still",
        "going", "want", "need", "make", "made", "know", "thing",
        "things",
    ];
    STOP_WORDS.contains(&word)
}

/// Sort a frequency map by count desc + alphabetical tie-break, keep
/// top N.
fn top_n_words(freq: HashMap<String, u64>, n: usize) -> Vec<TopWord> {
    let mut pairs: Vec<(String, u64)> = freq.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    pairs.into_iter().take(n).map(|(word, count)| TopWord { word, count }).collect()
}

/// Convert a Unix-seconds timestamp (or millis — we autodetect) into a
/// local-tz `DateTime`. Returns `None` if conversion fails.
fn local_datetime(ts: i64) -> Option<DateTime<Local>> {
    // Heuristic: if the magnitude is > 10^12, it's likely millis. The
    // year 2001 in seconds is ~10^9; the year 33658 in seconds is the
    // first that crosses 10^12. So this is safe through year ~33000.
    let secs = if ts.abs() > 1_000_000_000_000 {
        ts / 1000
    } else {
        ts
    };
    Local.timestamp_opt(secs, 0).single()
}

/// Trailing 30 days of word counts, most recent first. Missing days
/// get a zero entry so the heatmap renders correctly.
fn trailing_30_day_counts(daily: &HashMap<NaiveDate, u64>) -> Vec<DailyWordCount> {
    let today = Local::now().date_naive();
    (0..30)
        .map(|i| {
            let date = today - Duration::days(i);
            DailyWordCount {
                date: date.format("%Y-%m-%d").to_string(),
                words: *daily.get(&date).unwrap_or(&0),
            }
        })
        .collect()
}

/// Convert the 24-element hourly array into the public vec shape.
fn hourly_buckets(hourly: &[u64; 24]) -> Vec<HourlyWordCount> {
    hourly
        .iter()
        .enumerate()
        .map(|(hour, words)| HourlyWordCount {
            hour: hour as u8,
            words: *words,
        })
        .collect()
}

/// Compute (current_streak, longest_streak) from the daily-counts map.
///
/// A "streak day" is any local-tz day with at least one dictated word.
/// Current streak counts back from today; it's 0 if today has no
/// dictation. Longest streak scans the full history.
fn compute_streaks(daily: &HashMap<NaiveDate, u64>) -> (u32, u32) {
    if daily.is_empty() {
        return (0, 0);
    }

    let today = Local::now().date_naive();

    let mut current = 0u32;
    let mut cursor = today;
    while daily.get(&cursor).copied().unwrap_or(0) > 0 {
        current += 1;
        cursor -= Duration::days(1);
    }

    // Longest streak — walk the sorted distinct days and count
    // consecutive runs.
    let mut days: Vec<NaiveDate> = daily
        .iter()
        .filter(|(_, &words)| words > 0)
        .map(|(d, _)| *d)
        .collect();
    days.sort();
    let mut longest = 0u32;
    let mut run = 0u32;
    let mut prev: Option<NaiveDate> = None;
    for day in days {
        let extends = prev.map(|p| day == p + Duration::days(1)).unwrap_or(false);
        if extends {
            run += 1;
        } else {
            run = 1;
        }
        longest = longest.max(run);
        prev = Some(day);
    }

    (current, longest.max(current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(id: i64, ts: i64, text: &str) -> HistoryEntry {
        HistoryEntry {
            id,
            file_name: format!("{}.wav", id),
            timestamp: ts,
            saved: false,
            title: String::new(),
            transcription_text: text.to_string(),
            post_processed_text: None,
            post_process_prompt: None,
            post_process_requested: false,
        }
    }

    #[test]
    fn compute_empty_history_returns_zero_profile() {
        let p = compute(&[]);
        assert_eq!(p.total_words, 0);
        assert_eq!(p.total_sessions, 0);
        assert_eq!(p.estimated_wpm, None);
        assert!(p.top_words.is_empty());
        assert_eq!(p.daily_word_counts.len(), 30);
        assert_eq!(p.hourly_word_counts.len(), 24);
        assert_eq!(p.current_streak_days, 0);
        assert_eq!(p.longest_streak_days, 0);
    }

    #[test]
    fn compute_counts_total_words_correctly() {
        let now = Local::now().timestamp();
        let entries = vec![
            entry(1, now, "hello world foo bar"),     // 4 words
            entry(2, now, "the quick brown fox jumps"), // 5 words
        ];
        let p = compute(&entries);
        assert_eq!(p.total_words, 9);
        assert_eq!(p.total_sessions, 2);
        assert!(p.estimated_wpm.is_some());
    }

    #[test]
    fn compute_prefers_post_processed_text_when_present() {
        let now = Local::now().timestamp();
        let mut e = entry(1, now, "um hello uh world");
        e.post_processed_text = Some("Hello world.".to_string());
        // "Hello world." = 2 words; raw was 5.
        let p = compute(&[e]);
        assert_eq!(p.total_words, 2);
    }

    #[test]
    fn top_words_drops_stop_words_and_short_tokens() {
        let now = Local::now().timestamp();
        let entries = vec![entry(
            1,
            now,
            "the quick brown brown brown fox the the and a but",
        )];
        let p = compute(&entries);
        // "brown" should make it (5+ chars, not a stop word).
        // "the" / "and" / "a" / "but" should be filtered.
        let top: Vec<&str> = p.top_words.iter().map(|w| w.word.as_str()).collect();
        assert!(top.contains(&"brown"));
        assert!(!top.contains(&"the"));
        assert!(!top.contains(&"and"));
    }

    #[test]
    fn top_words_sorts_by_frequency_desc() {
        let now = Local::now().timestamp();
        let entries = vec![entry(
            1,
            now,
            "alpha alpha alpha beta beta charlie",
        )];
        let p = compute(&entries);
        assert_eq!(p.top_words[0].word, "alpha");
        assert_eq!(p.top_words[0].count, 3);
        assert_eq!(p.top_words[1].word, "beta");
        assert_eq!(p.top_words[1].count, 2);
        assert_eq!(p.top_words[2].word, "charlie");
    }

    #[test]
    fn daily_counts_has_exactly_30_buckets() {
        let p = compute(&[]);
        assert_eq!(p.daily_word_counts.len(), 30);
        // Most recent first.
        let today = Local::now().date_naive().format("%Y-%m-%d").to_string();
        assert_eq!(p.daily_word_counts[0].date, today);
    }

    #[test]
    fn hourly_buckets_cover_full_day() {
        let p = compute(&[]);
        assert_eq!(p.hourly_word_counts.len(), 24);
        assert_eq!(p.hourly_word_counts[0].hour, 0);
        assert_eq!(p.hourly_word_counts[23].hour, 23);
    }

    #[test]
    fn current_streak_is_zero_when_no_dictation_today() {
        let yesterday = Local::now().timestamp() - 60 * 60 * 24;
        let entries = vec![entry(1, yesterday, "hello world")];
        let p = compute(&entries);
        assert_eq!(p.current_streak_days, 0);
        // But longest is 1 (yesterday).
        assert_eq!(p.longest_streak_days, 1);
    }

    #[test]
    fn current_streak_counts_today_only() {
        let now = Local::now().timestamp();
        let entries = vec![entry(1, now, "hello world")];
        let p = compute(&entries);
        assert_eq!(p.current_streak_days, 1);
    }

    #[test]
    fn longest_streak_counts_consecutive_days() {
        // Construct three consecutive days of dictation a year ago.
        let base = Local
            .with_ymd_and_hms(2025, 6, 1, 12, 0, 0)
            .unwrap()
            .timestamp();
        let day = 60 * 60 * 24;
        let entries = vec![
            entry(1, base, "alpha alpha alpha"),
            entry(2, base + day, "beta beta beta"),
            entry(3, base + 2 * day, "gamma gamma gamma"),
        ];
        let p = compute(&entries);
        assert_eq!(p.longest_streak_days, 3);
    }

    #[test]
    fn estimated_wpm_in_sensible_range() {
        let now = Local::now().timestamp();
        let entries = vec![entry(1, now, &"hello ".repeat(100))];
        let p = compute(&entries);
        let wpm = p.estimated_wpm.unwrap();
        // 100 words / (100/3 seconds) * 60 = 180 wpm.
        assert!((175..=185).contains(&wpm), "expected ~180 wpm, got {}", wpm);
    }

    #[test]
    fn local_datetime_handles_millisecond_timestamps() {
        let ms = Local::now().timestamp() * 1000;
        assert!(local_datetime(ms).is_some());
    }
}
