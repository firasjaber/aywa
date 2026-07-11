use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use tauri::Manager;

static LOG_FILE: OnceLock<Mutex<File>> = OnceLock::new();
static LOG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn diagnostics_log_path(app_data_root: &Path) -> PathBuf {
    app_data_root.join("logs").join("ok-debug.log")
}

pub fn init(app_data_root: &Path) -> Result<PathBuf, String> {
    let path = diagnostics_log_path(app_data_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|error| error.to_string())?;

    let _ = LOG_PATH.set(path.clone());
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
    let line = format!("[{}] {}", timestamp_ms(), message.as_ref());
    eprintln!("{line}");

    if let Some(file) = LOG_FILE.get() {
        if let Ok(mut file) = file.lock() {
            let _ = writeln!(file, "{line}");
            let _ = file.flush();
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
    log(format!(
        "[command] start description=\"{}\" command={}",
        description,
        render_command(program, args)
    ));

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
        "[command] failed description=\"{}\" status={} output={}",
        description, status, summary
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
                    log(format!("[command:{stream}] {description}: {line}"));
                    lines.push(format!("{stream}: {line}"));
                }
                Err(error) => {
                    let message = format!("{stream}: failed to read command output: {error}");
                    log(format!("[command:{stream}] {description}: {message}"));
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

    format!("{}...", &value[..MAX_LEN])
}

fn render_command(program: &Path, args: &[&str]) -> String {
    std::iter::once(program.display().to_string())
        .chain(args.iter().map(|arg| quote_arg(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_arg(arg: &str) -> String {
    if arg.chars().any(char::is_whitespace) {
        format!("\"{}\"", arg.replace('"', "\\\""))
    } else {
        arg.to_string()
    }
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

    use super::{explorer_select_arg, operating_system_label};

    #[test]
    fn operating_system_label_is_human_readable() {
        let label = operating_system_label();
        assert!(!label.is_empty());
        assert_ne!(label, "windows");
    }

    #[test]
    fn explorer_select_arg_targets_the_log_file() {
        let path = Path::new(r"C:\Users\Test User\AppData\Local\ok\logs\ok-debug.log");
        assert_eq!(
            explorer_select_arg(path),
            r"/select,C:\Users\Test User\AppData\Local\ok\logs\ok-debug.log"
        );
    }
}
