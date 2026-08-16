use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};

pub const HISTORY_SCHEMA_VERSION: u32 = 1;
pub const MAX_PAGE_SIZE: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryMetadata {
    pub id: String,
    pub created_at_epoch_ms: u64,
    pub duration_ms: u64,
    pub language: Option<String>,
    pub preview: String,
    pub character_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub entries: Vec<HistoryEntryMetadata>,
    pub next_offset: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTranscript {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct HistoryIndex {
    version: u32,
    entries: Vec<HistoryEntryMetadata>,
}

pub struct HistoryRepository {
    root: PathBuf,
}

impl HistoryRepository {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn save(
        &self,
        text: &str,
        duration_ms: u64,
        language: Option<String>,
    ) -> Result<HistoryEntryMetadata, String> {
        let created_at_epoch_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("system clock error: {error}"))?
            .as_millis() as u64;
        let id = format!(
            "transcript-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| format!("system clock error: {error}"))?
                .as_nanos()
        );
        self.save_entry(text, duration_ms, language, created_at_epoch_ms, id)
    }

    pub fn list(&self, offset: usize, limit: usize) -> Result<HistoryPage, String> {
        let index = self.load_index()?;
        let limit = limit.clamp(1, MAX_PAGE_SIZE);
        let total = index.entries.len();
        let entries = index
            .entries
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect::<Vec<_>>();
        let next_offset = if offset.saturating_add(entries.len()) < total {
            Some(offset.saturating_add(entries.len()))
        } else {
            None
        };
        Ok(HistoryPage {
            entries,
            next_offset,
        })
    }

    pub fn read(&self, id: &str) -> Result<HistoryTranscript, String> {
        self.require_entry(id)?;
        let path = self.transcript_path(id);
        let text = fs::read_to_string(&path).map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => "History transcript is unavailable".to_string(),
            _ => format!("failed to read history transcript: {error}"),
        })?;
        Ok(HistoryTranscript {
            id: id.to_string(),
            text,
        })
    }

    pub fn delete(&self, id: &str) -> Result<(), String> {
        let mut index = self.load_index()?;
        let position = index
            .entries
            .iter()
            .position(|entry| entry.id == id)
            .ok_or_else(|| "History entry not found".to_string())?;
        let path = self.transcript_path(id);
        if let Err(error) = fs::remove_file(&path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("failed to delete history transcript: {error}"));
            }
        }
        index.entries.remove(position);
        self.write_index(&index)
    }

    pub fn clear(&self) -> Result<(), String> {
        let transcripts = self.transcripts_dir();
        if transcripts.exists() {
            fs::remove_dir_all(&transcripts)
                .map_err(|error| format!("failed to clear history transcripts: {error}"))?;
        }
        self.write_index(&HistoryIndex {
            version: HISTORY_SCHEMA_VERSION,
            entries: Vec::new(),
        })
    }

    pub fn startup_repair(&self) -> Result<(), String> {
        fs::create_dir_all(self.transcripts_dir())
            .map_err(|error| format!("failed to create history directory: {error}"))?;
        for entry in fs::read_dir(self.transcripts_dir())
            .map_err(|error| format!("failed to inspect history directory: {error}"))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.extension().is_some_and(|extension| extension == "tmp") {
                let _ = fs::remove_file(path);
            }
        }
        if let Err(error) = self.load_index() {
            eprintln!("[history] resetting unreadable index: {error}");
            self.write_index(&HistoryIndex {
                version: HISTORY_SCHEMA_VERSION,
                entries: Vec::new(),
            })?;
        }
        Ok(())
    }

    fn save_entry(
        &self,
        text: &str,
        duration_ms: u64,
        language: Option<String>,
        created_at_epoch_ms: u64,
        id: String,
    ) -> Result<HistoryEntryMetadata, String> {
        fs::create_dir_all(self.transcripts_dir())
            .map_err(|error| format!("failed to create history directory: {error}"))?;
        let entry = HistoryEntryMetadata {
            id: id.clone(),
            created_at_epoch_ms,
            duration_ms,
            language,
            preview: preview(text),
            character_count: text.chars().count(),
        };
        atomic_write(&self.transcript_path(&id), text.as_bytes())?;
        let mut index = self.load_index()?;
        index.entries.push(entry.clone());
        index
            .entries
            .sort_by(|left, right| right.created_at_epoch_ms.cmp(&left.created_at_epoch_ms));
        self.write_index(&index)?;
        Ok(entry)
    }

    fn require_entry(&self, id: &str) -> Result<(), String> {
        if self
            .load_index()?
            .entries
            .iter()
            .any(|entry| entry.id == id)
        {
            Ok(())
        } else {
            Err("History entry not found".to_string())
        }
    }

    fn load_index(&self) -> Result<HistoryIndex, String> {
        let path = self.index_path();
        match fs::read_to_string(path) {
            Ok(contents) => {
                let mut index: HistoryIndex = serde_json::from_str(&contents)
                    .map_err(|error| format!("failed to read history index: {error}"))?;
                if index.version != HISTORY_SCHEMA_VERSION {
                    return Err("unsupported history index version".to_string());
                }
                index.entries.sort_by(|left, right| {
                    right.created_at_epoch_ms.cmp(&left.created_at_epoch_ms)
                });
                Ok(index)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(HistoryIndex {
                version: HISTORY_SCHEMA_VERSION,
                entries: Vec::new(),
            }),
            Err(error) => Err(format!("failed to read history index: {error}")),
        }
    }

    fn write_index(&self, index: &HistoryIndex) -> Result<(), String> {
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("failed to create history root: {error}"))?;
        let contents = serde_json::to_vec_pretty(index).map_err(|error| error.to_string())?;
        atomic_write(&self.index_path(), &contents)
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }
    fn transcripts_dir(&self) -> PathBuf {
        self.root.join("transcripts")
    }
    fn transcript_path(&self, id: &str) -> PathBuf {
        self.transcripts_dir().join(format!("{id}.txt"))
    }

    #[cfg(test)]
    fn save_at(
        &self,
        text: &str,
        duration_ms: u64,
        language: Option<String>,
        created_at_epoch_ms: u64,
    ) -> Result<HistoryEntryMetadata, String> {
        self.save_entry(
            text,
            duration_ms,
            language,
            created_at_epoch_ms,
            format!("test-{created_at_epoch_ms}"),
        )
    }

    #[cfg(test)]
    fn transcript_path_for_test(&self, id: &str) -> PathBuf {
        self.transcript_path(id)
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("tmp")
    ));
    fs::write(&temporary, contents)
        .map_err(|error| format!("failed to write history data: {error}"))?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("failed to replace history data: {error}"))?;
    }
    fs::rename(temporary, path).map_err(|error| format!("failed to finalize history data: {error}"))
}

fn preview(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(160)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::HistoryRepository;
    use std::fs;

    #[test]
    fn save_lists_metadata_without_loading_transcript_bodies() {
        let directory = tempfile::tempdir().unwrap();
        let repository = HistoryRepository::new(directory.path().join("history"));
        let saved = repository
            .save_at("First line\nsecond line", 1_000, None, 20)
            .unwrap();
        let page = repository.list(0, 50).unwrap();
        assert_eq!(page.entries, vec![saved.clone()]);
        assert_eq!(page.next_offset, None);
        assert_eq!(
            fs::read_to_string(repository.transcript_path_for_test(&saved.id)).unwrap(),
            "First line\nsecond line"
        );
    }

    #[test]
    fn list_is_newest_first_and_respects_offsets() {
        let directory = tempfile::tempdir().unwrap();
        let repository = HistoryRepository::new(directory.path().join("history"));
        let old = repository.save_at("old", 500, None, 1).unwrap();
        let new = repository.save_at("new", 600, None, 2).unwrap();
        assert_eq!(repository.list(0, 1).unwrap().entries, vec![new]);
        assert_eq!(repository.list(0, 1).unwrap().next_offset, Some(1));
        assert_eq!(repository.list(1, 1).unwrap().entries, vec![old]);
    }

    #[test]
    fn delete_and_clear_remove_bodies_and_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let repository = HistoryRepository::new(directory.path().join("history"));
        let first = repository.save_at("one", 1, None, 1).unwrap();
        repository.save_at("two", 1, None, 2).unwrap();
        repository.delete(&first.id).unwrap();
        assert!(!repository.transcript_path_for_test(&first.id).exists());
        repository.clear().unwrap();
        assert!(repository.list(0, 50).unwrap().entries.is_empty());
    }

    #[test]
    fn repair_removes_temporary_files_and_recovers_bad_index() {
        let directory = tempfile::tempdir().unwrap();
        let repository = HistoryRepository::new(directory.path().join("history"));
        fs::create_dir_all(repository.transcripts_dir()).unwrap();
        fs::write(repository.index_path(), "not json").unwrap();
        let temporary = repository.transcripts_dir().join("orphan.txt.tmp");
        fs::write(&temporary, "temporary").unwrap();
        repository.startup_repair().unwrap();
        assert!(!temporary.exists());
        assert!(repository.list(0, 50).unwrap().entries.is_empty());
    }

    #[test]
    fn missing_body_does_not_hide_other_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let repository = HistoryRepository::new(directory.path().join("history"));
        let missing = repository.save_at("missing", 1, None, 1).unwrap();
        let present = repository.save_at("present", 1, None, 2).unwrap();
        fs::remove_file(repository.transcript_path_for_test(&missing.id)).unwrap();

        assert!(repository
            .read(&missing.id)
            .unwrap_err()
            .contains("unavailable"));
        assert_eq!(repository.list(0, 50).unwrap().entries.len(), 2);
        assert_eq!(repository.read(&present.id).unwrap().text, "present");
    }
}
