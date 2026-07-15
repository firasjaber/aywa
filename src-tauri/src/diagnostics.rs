use std::{
    collections::VecDeque,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tauri::Manager;

static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();
static LOG_DIRECTORY: OnceLock<PathBuf> = OnceLock::new();
static RECENT_EVENTS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

pub const MAX_MANAGED_LOG_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_RECENT_EVENTS: usize = 80;
const SESSION_LOG_PREFIX: &str = "aywa-session-";
const SESSION_LOG_SUFFIX: &str = ".log";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssueReport {
    pub preview: String,
}

pub struct IssueReportInput {
    pub app_version: String,
    pub operating_system: String,
    pub selected_model_id: String,
    pub loaded_model_id: Option<String>,
    pub execution_device: Option<String>,
}

pub fn diagnostics_log_path(app_data_root: &Path) -> PathBuf {
    session_log_path(&app_data_root.join("logs"))
}

pub fn init(app_data_root: &Path) -> Result<PathBuf, String> {
    let path = diagnostics_log_path(app_data_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        enforce_log_cap(parent, MAX_MANAGED_LOG_BYTES, None)?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| error.to_string())?;

    let _ = LOG_PATH.set(path.clone());
    let _ = LOG_DIRECTORY.set(path.parent().unwrap_or(app_data_root).to_path_buf());
    let _ = LOG_FILE.set(Mutex::new(file));
    log("diagnostics initialized");
    Ok(path)
}

pub fn init_for_app<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?;
    init(&root)
}

pub fn log_path_for_app<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Option<PathBuf> {
    LOG_PATH.get().cloned().or_else(|| {
        app.path()
            .app_local_data_dir()
            .ok()
            .map(|root| diagnostics_log_path(&root))
    })
}

pub fn operating_system_label() -> String {
    match std::env::consts::OS {
        "windows" => "Windows".to_string(),
        "macos" => "macOS".to_string(),
        "linux" => "Linux".to_string(),
        other => other.to_string(),
    }
}

pub fn explorer_select_arg(path: &Path) -> String {
    format!("/select,{}", path.display())
}

pub fn open_log_in_file_manager<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<(), String> {
    let path = log_path_for_app(app)
        .ok_or_else(|| "The diagnostics log is not available yet.".to_string())?;

    if !path.exists() {
        return Err("The diagnostics log has not been created yet.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("explorer.exe")
            .arg(explorer_select_arg(&path))
            .spawn()
            .map_err(|error| format!("Could not open Windows Explorer: {error}"))?;
        return Ok(());
    }

    #[cfg(not(target_os = "windows"))]
    Err("Opening the diagnostics log is currently supported on Windows only.".to_string())
}

pub fn log(message: impl AsRef<str>) {
    let event = redact_sensitive_text(message.as_ref());
    let line = format!("[{}] {event}", timestamp_ms());
    eprintln!("{line}");

    if let Some(events) = RECENT_EVENTS
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .ok()
    {
        let mut events = events;
        events.push_back(event);
        if events.len() > MAX_RECENT_EVENTS {
            events.pop_front();
        }
    }

    if let Some(file) = LOG_FILE.get() {
        if let Ok(mut file) = file.lock() {
            let line_bytes = line.len() as u64 + 1;
            let can_write = match (LOG_DIRECTORY.get(), LOG_PATH.get()) {
                (Some(directory), Some(active_path)) => {
                    enforce_log_cap(
                        directory,
                        MAX_MANAGED_LOG_BYTES.saturating_sub(line_bytes),
                        Some(active_path),
                    )
                    .is_ok()
                        && fs::metadata(active_path)
                            .map(|metadata| {
                                metadata.len().saturating_add(line_bytes) <= MAX_MANAGED_LOG_BYTES
                            })
                            .unwrap_or(false)
                }
                _ => true,
            };
            if can_write {
                let _ = writeln!(file, "{line}");
                let _ = file.flush();
            }
        }
    }
}

pub fn run_logged_command(program: &Path, args: &[&str], description: &str) -> Result<(), String> {
    run_logged_command_inner(program, args, description, None)
}

pub fn run_logged_command_with_timeout(
    program: &Path,
    args: &[&str],
    description: &str,
    timeout: Duration,
) -> Result<(), String> {
    run_logged_command_inner(program, args, description, Some(timeout))
}

fn run_logged_command_inner(
    program: &Path,
    args: &[&str],
    description: &str,
    timeout: Option<Duration>,
) -> Result<(), String> {
    let _ = program;
    log(format!("[command] start description=\"{}\"", description));

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to {description}: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .map(|pipe| collect_command_pipe(pipe, "stdout", description.to_string()));
    let stderr = child
        .stderr
        .take()
        .map(|pipe| collect_command_pipe(pipe, "stderr", description.to_string()));

    let status = if let Some(timeout) = timeout {
        wait_with_timeout(&mut child, description, timeout)?
    } else {
        child
            .wait()
            .map_err(|error| format!("failed to wait for {description}: {error}"))?
    };
    let mut lines = Vec::new();

    if let Some(handle) = stdout {
        lines.extend(handle.join().unwrap_or_default());
    }
    if let Some(handle) = stderr {
        lines.extend(handle.join().unwrap_or_default());
    }

    if status.success() {
        log(format!(
            "[command] success description=\"{}\" status={}",
            description, status
        ));
        return Ok(());
    }

    let summary = summarize_lines(&lines);
    log(format!(
        "[command] failed description=\"{}\" status={}",
        description, status
    ));
    Err(format!("failed to {description}: {summary}"))
}

fn wait_with_timeout(
    child: &mut std::process::Child,
    description: &str,
    timeout: Duration,
) -> Result<std::process::ExitStatus, String> {
    let start = Instant::now();

    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to wait for {description}: {error}"))?
        {
            return Ok(status);
        }

        if start.elapsed() >= timeout {
            log(format!(
                "[command] timeout description=\"{}\" timeout_seconds={}",
                description,
                timeout.as_secs()
            ));
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "failed to {description}: timed out after {} seconds",
                timeout.as_secs()
            ));
        }

        thread::sleep(Duration::from_millis(250));
    }
}

fn collect_command_pipe<R: Read + Send + 'static>(
    pipe: R,
    stream: &'static str,
    description: String,
) -> thread::JoinHandle<Vec<String>> {
    thread::spawn(move || {
        let reader = BufReader::new(pipe);
        let mut lines = Vec::new();

        for line in reader.lines() {
            match line {
                Ok(line) => {
                    lines.push(format!("{stream}: {line}"));
                }
                Err(error) => {
                    let message = format!("{stream}: failed to read command output: {error}");
                    log(format!(
                        "[command:{stream}] {description}: output read failed"
                    ));
                    lines.push(message);
                    break;
                }
            }
        }

        lines
    })
}

fn summarize_lines(lines: &[String]) -> String {
    if lines.is_empty() {
        return "no output".to_string();
    }

    let joined = lines
        .iter()
        .rev()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join(" | ");

    summarize(&joined)
}

fn summarize(value: &str) -> String {
    const MAX_LEN: usize = 800;

    if value.len() <= MAX_LEN {
        return value.to_string();
    }

    let boundary = value
        .char_indices()
        .take_while(|(index, _)| *index <= MAX_LEN)
        .map(|(index, character)| index + character.len_utf8())
        .last()
        .unwrap_or(0);
    format!("{}...", &value[..boundary])
}

pub fn redact_sensitive_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;

    while let Some(index) = find_path_start(remaining) {
        output.push_str(&remaining[..index]);
        output.push_str("<redacted-path>");
        let path_end = remaining[index..]
            .find(|character: char| {
                character.is_whitespace() || matches!(character, '\"' | '\'' | ')' | ']')
            })
            .unwrap_or(remaining[index..].len());
        remaining = &remaining[index + path_end..];
    }

    output.push_str(remaining);
    output
}

pub fn recent_events() -> Vec<String> {
    RECENT_EVENTS
        .get_or_init(|| Mutex::new(VecDeque::new()))
        .lock()
        .map(|events| events.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn build_issue_report(input: IssueReportInput) -> IssueReport {
    let events = recent_events();
    let event_text = if events.is_empty() {
        "No diagnostic events have been recorded in this session.".to_string()
    } else {
        events
            .iter()
            .map(|event| format!("- {}", redact_sensitive_text(event)))
            .collect::<Vec<_>>()
            .join("\n")
    };

    IssueReport {
        preview: format!(
            "Aywa support report\n\nApp version: {}\nOperating system: {}\nSelected model: {}\nLoaded model: {}\nExecution device: {}\n\nRecent safe diagnostics:\n{}\n\nNot included: recordings, transcripts, file paths, usernames, command arguments, or raw subprocess output.\n",
            input.app_version,
            input.operating_system,
            input.selected_model_id,
            input.loaded_model_id.as_deref().unwrap_or("none"),
            input.execution_device.as_deref().unwrap_or("unknown"),
            event_text,
        ),
    }
}

fn find_path_start(value: &str) -> Option<usize> {
    value.char_indices().find_map(|(index, character)| {
        let suffix = &value[index..];
        (character == '/'
            || (character.is_ascii_alphabetic() && suffix[1..].starts_with(":\\"))
            || (character.is_ascii_alphabetic() && suffix[1..].starts_with(":/")))
        .then_some(index)
    })
}

fn session_log_path(directory: &Path) -> PathBuf {
    directory.join(format!(
        "{SESSION_LOG_PREFIX}{}-{}.log",
        timestamp_ms(),
        std::process::id()
    ))
}

fn managed_session_logs(directory: &Path) -> Result<Vec<(PathBuf, u64)>, String> {
    let mut logs = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?;
            (name.starts_with(SESSION_LOG_PREFIX) && name.ends_with(SESSION_LOG_SUFFIX))
                .then(|| {
                    fs::metadata(&path)
                        .ok()
                        .map(|metadata| (path, metadata.len()))
                })
                .flatten()
        })
        .collect::<Vec<_>>();
    logs.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(logs)
}

pub fn enforce_log_cap(
    directory: &Path,
    cap: u64,
    active_path: Option<&Path>,
) -> Result<u64, String> {
    let logs = managed_session_logs(directory)?;
    let mut total = logs.iter().map(|(_, size)| size).sum::<u64>();
    for (path, size) in logs {
        if total <= cap {
            break;
        }
        if active_path.is_some_and(|active| active == path) {
            continue;
        }
        fs::remove_file(path).map_err(|error| error.to_string())?;
        total = total.saturating_sub(size);
    }
    Ok(total)
}

fn timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod ui_support_tests {
    use std::path::Path;

    use super::{
        enforce_log_cap, explorer_select_arg, operating_system_label, redact_sensitive_text,
    };

    #[test]
    fn operating_system_label_is_human_readable() {
        let label = operating_system_label();
        assert!(!label.is_empty());
        assert_ne!(label, "windows");
    }

    #[test]
    fn explorer_select_arg_targets_the_log_file() {
        let path = Path::new(r"C:\Users\Test User\AppData\Local\aywa\logs\aywa-debug.log");
        assert_eq!(
            explorer_select_arg(path),
            r"/select,C:\Users\Test User\AppData\Local\aywa\logs\aywa-debug.log"
        );
    }

    #[test]
    fn redaction_removes_windows_and_unix_paths() {
        assert_eq!(
            redact_sensitive_text(r"worker failed at C:\Users\Ava\recording-1.wav"),
            "worker failed at <redacted-path>"
        );
        assert_eq!(
            redact_sensitive_text("worker failed at /home/ava/recording-1.wav"),
            "worker failed at <redacted-path>"
        );
    }

    #[test]
    fn log_cap_removes_oldest_session_file_first() {
        let directory = tempfile::tempdir().unwrap();
        let oldest = directory.path().join("aywa-session-001-1.log");
        let newest = directory.path().join("aywa-session-002-1.log");
        std::fs::write(&oldest, b"123456").unwrap();
        std::fs::write(&newest, b"123456").unwrap();

        assert_eq!(enforce_log_cap(directory.path(), 6, None).unwrap(), 6);
        assert!(!oldest.exists());
        assert!(newest.exists());
    }
}
