use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum number of history entries to keep.
const MAX_HISTORY: usize = 200;

/// How much of the input/output to store in each entry.
const PREVIEW_LEN: usize = 100;

static HISTORY_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The type of operation that was performed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoryOp {
    Encode,
    Decode,
}

impl HistoryOp {
    pub fn icon(&self) -> &'static str {
        match self {
            HistoryOp::Encode => "⬆",
            HistoryOp::Decode => "⬇",
        }
    }
}

/// A single history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Stable identifier for selection and deletion.
    #[serde(default = "next_history_id")]
    pub id: String,
    /// Unix timestamp (seconds since epoch) when the operation occurred.
    pub timestamp: u64,
    /// Encode or Decode.
    pub op: HistoryOp,
    /// Full input used for reliable reloads.
    #[serde(default)]
    pub input_value: String,
    /// First PREVIEW_LEN chars of the input.
    pub input_preview: String,
    /// First PREVIEW_LEN chars of the output.
    pub output_preview: String,
    /// Base64 variant used (e.g. "standard", "url-safe").
    pub variant: String,
}

impl HistoryEntry {
    pub fn new(op: HistoryOp, input: &str, output: &str, variant: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let timestamp = now.as_secs();
        Self {
            id: format!("{}-{}", now.as_nanos(), next_counter()),
            timestamp,
            op,
            input_value: input.to_string(),
            input_preview: truncate(input, PREVIEW_LEN),
            output_preview: truncate(output, PREVIEW_LEN),
            variant: variant.to_string(),
        }
    }

    /// Format the timestamp as a human-readable relative string.
    pub fn relative_time(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let diff = now.saturating_sub(self.timestamp);
        if diff < 60 {
            "just now".into()
        } else if diff < 3600 {
            format!("{}m ago", diff / 60)
        } else if diff < 86400 {
            format!("{}h ago", diff / 3600)
        } else {
            format!("{}d ago", diff / 86400)
        }
    }

    pub fn reload_input(&self) -> &str {
        if self.input_value.is_empty() {
            &self.input_preview
        } else {
            &self.input_value
        }
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let mut end = s.len();
        for (count, (idx, _)) in s.char_indices().enumerate() {
            if count == max_len {
                end = idx;
                break;
            }
        }
        format!("{}…", &s[..end])
    }
}

/// Persistent store for history entries, backed by a JSON file.
pub struct HistoryStore {
    entries: Vec<HistoryEntry>,
    path: PathBuf,
    private_mode: bool,
}

impl HistoryStore {
    /// Load the store from disk, or create a new empty store.
    pub fn load(path: PathBuf, private_mode: bool) -> Self {
        let entries = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        Self {
            entries,
            path,
            private_mode,
        }
    }

    /// Append a new entry. No-op if private_mode is active.
    pub fn append(&mut self, entry: HistoryEntry) {
        if self.private_mode {
            return;
        }
        self.entries.push(entry);
        // FIFO eviction
        if self.entries.len() > MAX_HISTORY {
            self.entries.remove(0);
        }
        self.save();
    }

    /// Search entries by a query string (case-insensitive, matches input or output preview).
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        if query.is_empty() {
            return self.entries.iter().collect();
        }
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.input_preview.to_lowercase().contains(&q)
                    || e.output_preview.to_lowercase().contains(&q)
            })
            .collect()
    }

    /// Remove all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.save();
    }

    pub fn set_private_mode(&mut self, private_mode: bool) {
        self.private_mode = private_mode;
    }

    /// Remove a single entry by id.
    pub fn remove_by_id(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.id != id);
        let removed = self.entries.len() != before;
        if removed {
            self.save();
        }
        removed
    }

    /// All entries (for rendering the full list).
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn get_by_id(&self, id: &str) -> Option<&HistoryEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    fn save(&self) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(&self.entries) {
            let _ = fs::write(&self.path, text);
        }
    }
}

fn next_counter() -> u64 {
    HISTORY_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn next_history_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}-{}", now.as_nanos(), next_counter())
}

/// Resolve the default path for the history file in the OS config directory.
pub fn history_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "basie64", "basie-64").map(|d| d.config_dir().join("history.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (HistoryStore, tempfile::NamedTempFile) {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        let store = HistoryStore::load(file.path().to_path_buf(), false);
        (store, file)
    }

    #[test]
    fn append_and_evict() {
        let (mut store, _file) = temp_store();
        // Fill up to MAX_HISTORY + 10
        for i in 0..(MAX_HISTORY + 10) {
            store.append(HistoryEntry::new(
                HistoryOp::Encode,
                &format!("input {}", i),
                &format!("output {}", i),
                "standard",
            ));
        }
        assert_eq!(store.entries().len(), MAX_HISTORY);
        // Oldest entries should have been evicted
        assert_eq!(store.entries()[0].input_preview, "input 10");
    }

    #[test]
    fn private_mode_no_append() {
        let file = tempfile::NamedTempFile::new().expect("temp file");
        let mut store = HistoryStore::load(file.path().to_path_buf(), true);
        store.append(HistoryEntry::new(
            HistoryOp::Decode,
            "test",
            "test",
            "standard",
        ));
        assert!(store.entries().is_empty());
    }

    #[test]
    fn search_filters() {
        let (mut store, _file) = temp_store();
        store.append(HistoryEntry::new(
            HistoryOp::Encode,
            "Hello world",
            "SGVsbG8=",
            "standard",
        ));
        store.append(HistoryEntry::new(
            HistoryOp::Decode,
            "SGVsbG8=",
            "Hello world",
            "url-safe",
        ));
        store.append(HistoryEntry::new(
            HistoryOp::Encode,
            "foo bar",
            "Zm9v",
            "standard",
        ));

        let results = store.search("hello");
        assert_eq!(results.len(), 2);

        let results = store.search("notfound");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn remove_entry_by_id() {
        let (mut store, _file) = temp_store();
        let first = HistoryEntry::new(HistoryOp::Encode, "a", "b", "standard");
        let second = HistoryEntry::new(HistoryOp::Decode, "c", "d", "standard");
        let second_id = second.id.clone();
        store.append(first);
        store.append(second);
        assert_eq!(store.entries().len(), 2);
        assert!(store.remove_by_id(&second_id));
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.entries()[0].input_preview, "a");
    }

    #[test]
    fn remove_duplicate_previews_removes_exact_entry() {
        let (mut store, _file) = temp_store();
        let first = HistoryEntry::new(HistoryOp::Encode, "same", "out-1", "standard");
        let second = HistoryEntry::new(HistoryOp::Encode, "same", "out-2", "standard");
        let second_id = second.id.clone();
        store.append(first);
        store.append(second);

        assert!(store.remove_by_id(&second_id));
        assert_eq!(store.entries().len(), 1);
        assert_eq!(store.entries()[0].output_preview, "out-1");
    }

    #[test]
    fn relative_time() {
        let entry = HistoryEntry::new(HistoryOp::Encode, "x", "y", "standard");
        // Entry was just created, so it should be "just now"
        assert_eq!(entry.relative_time(), "just now");
    }

    #[test]
    fn truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long_string() {
        let long = "a".repeat(200);
        let result = truncate(&long, 10);
        assert_eq!(result.chars().count(), 11); // 10 chars + "…"
        assert!(result.ends_with("…"));
    }

    #[test]
    fn reload_input_prefers_full_value() {
        let entry = HistoryEntry::new(HistoryOp::Decode, &"a".repeat(150), "out", "standard");
        assert_eq!(entry.reload_input().len(), 150);
    }
}
