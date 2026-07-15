use std::{
    fs,
    path::{Path, PathBuf},
};

/// Owns a temporary WAV recording and removes it when the transcription flow ends.
pub struct TemporaryRecording {
    path: PathBuf,
}

impl TemporaryRecording {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TemporaryRecording {
    fn drop(&mut self) {
        if let Err(error) = fs::remove_file(&self.path) {
            if error.kind() != std::io::ErrorKind::NotFound {
                eprintln!("[recordings] failed to remove temporary recording: {error}");
            }
        }
    }
}

/// Removes WAV recordings left by an interrupted previous session.
pub fn cleanup_abandoned_recordings(directory: &Path) -> Result<usize, String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.to_string()),
    };

    let mut removed = 0;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            continue;
        }

        let path = entry.path();
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
        {
            continue;
        }

        fs::remove_file(path).map_err(|error| error.to_string())?;
        removed += 1;
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{cleanup_abandoned_recordings, TemporaryRecording};

    #[test]
    fn temporary_recording_deletes_its_wav_on_drop() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("recording-1.wav");
        fs::write(&path, b"wav").unwrap();

        drop(TemporaryRecording::new(path.clone()));

        assert!(!path.exists());
    }

    #[test]
    fn cleanup_abandoned_recordings_removes_only_wav_files() {
        let directory = tempfile::tempdir().unwrap();
        let wav = directory.path().join("recording-1.wav");
        let text = directory.path().join("keep.txt");
        fs::write(&wav, b"wav").unwrap();
        fs::write(&text, b"keep").unwrap();

        assert_eq!(cleanup_abandoned_recordings(directory.path()).unwrap(), 1);
        assert!(!wav.exists());
        assert!(text.exists());
    }

    #[test]
    fn cleanup_allows_a_missing_recordings_directory() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing");

        assert_eq!(cleanup_abandoned_recordings(&missing).unwrap(), 0);
    }
}
