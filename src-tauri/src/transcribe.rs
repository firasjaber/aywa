use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::{
    app_data_dir,
    model_manager::{
        registry,
        service::{registry_path, ModelManagerState, MODEL_MANAGER_EVENT},
        types::{ModelId, WorkerCudaStatus, WorkerRuntimeBootstrapStatus},
    },
    runtime_profile::{provision, types::RuntimeProfileCatalogEntry},
    RuntimeHandles,
};

const WORKER_MAIN_SOURCE: &str = include_str!("../python/worker_main.py");
const WORKER_PROTOCOL_SOURCE: &str = include_str!("../python/worker_protocol.py");
const RUNTIME_SMOKE_TEST_SOURCE: &str = include_str!("../python/runtime_smoke_test.py");
const TRANSCRIBER_SOURCE: &str = include_str!("../python/transcriber.py");
const REQUIREMENTS_SOURCE: &str = include_str!("../python/requirements.txt");

pub const WORKER_START_TIMEOUT: Duration = Duration::from_secs(60);
pub const TRANSCRIBE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerRequest {
    Ping,
    Shutdown,
    Transcribe {
        request_id: String,
        audio_path: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WorkerResponse {
    Ready {
        model_id: String,
        model_path: String,
        #[serde(default)]
        device: Option<String>,
    },
    Pong,
    TranscribeResult {
        request_id: String,
        text: String,
        duration_ms: u64,
    },
    Error {
        request_id: Option<String>,
        message: String,
    },
    Fatal {
        message: String,
    },
}

impl WorkerResponse {
    pub fn is_terminal_failure(&self) -> bool {
        matches!(self, Self::Fatal { .. })
    }

    pub fn matches_request(&self, request_id: &str) -> bool {
        match self {
            Self::TranscribeResult {
                request_id: current, ..
            } => current == request_id,
            Self::Error {
                request_id: Some(current),
                ..
            } => current == request_id,
            _ => false,
        }
    }
}

#[derive(Debug)]
struct WorkerFailure {
    message: String,
    terminal: bool,
}

pub struct WorkerHandle {
    child: Child,
    stdin: ChildStdin,
    responses: mpsc::Receiver<Result<WorkerResponse, String>>,
    loaded_model_id: ModelId,
}

pub struct TranscriptionResult {
    pub text: String,
    pub duration_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct WorkerRuntimeProbeResult {
    pub(crate) dependencies_ready: bool,
    pub(crate) dependency_error: Option<String>,
    pub(crate) cuda_available: bool,
    pub(crate) reason: Option<String>,
    pub(crate) gpu_name: Option<String>,
    pub(crate) torch_version: Option<String>,
    pub(crate) cuda_version: Option<String>,
}

pub fn encode_json_line<T: Serialize>(message: &T) -> Result<String, String> {
    let mut line = serde_json::to_string(message).map_err(|error| error.to_string())?;
    line.push('\n');
    Ok(line)
}

fn decode_worker_response_line(line: &str) -> Result<WorkerResponse, String> {
    serde_json::from_str::<WorkerResponse>(line).map_err(|error| {
        format!(
            "worker emitted non-JSON stdout: {} ({error})",
            summarize_worker_line(line)
        )
    })
}

fn summarize_worker_line(line: &str) -> String {
    const MAX_LEN: usize = 160;

    if line.len() <= MAX_LEN {
        return line.to_string();
    }

    format!("{}...", &line[..MAX_LEN])
}

pub fn python_runtime_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join("runtime").join("python")
}

fn embedded_python_executable(app_data_root: &Path) -> PathBuf {
    python_runtime_dir(app_data_root).join("python.exe")
}

pub fn worker_runtime_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join("runtime").join("worker")
}

pub fn worker_entrypoint(app_data_root: &Path) -> PathBuf {
    worker_runtime_dir(app_data_root).join("worker_main.py")
}

pub(crate) fn worker_smoke_test_path(app_data_root: &Path) -> PathBuf {
    worker_runtime_dir(app_data_root).join("runtime_smoke_test.py")
}

fn worker_protocol_path(app_data_root: &Path) -> PathBuf {
    worker_runtime_dir(app_data_root).join("worker_protocol.py")
}

fn worker_transcriber_path(app_data_root: &Path) -> PathBuf {
    worker_runtime_dir(app_data_root).join("transcriber.py")
}

pub(crate) fn worker_requirements_path(app_data_root: &Path) -> PathBuf {
    worker_runtime_dir(app_data_root).join("requirements.txt")
}

pub(crate) fn worker_cuda_status_from_probe(probe: WorkerRuntimeProbeResult) -> WorkerCudaStatus {
    WorkerCudaStatus {
        cuda_available: probe.cuda_available,
        reason: probe.reason.or(probe.dependency_error),
        gpu_name: probe.gpu_name,
        torch_version: probe.torch_version,
        cuda_version: probe.cuda_version,
    }
}

pub fn bootstrap_runtime(app_data_root: &Path) -> Result<(), String> {
    fs::create_dir_all(worker_runtime_dir(app_data_root)).map_err(|error| error.to_string())?;
    fs::create_dir_all(python_runtime_dir(app_data_root)).map_err(|error| error.to_string())?;
    write_runtime_file(&worker_entrypoint(app_data_root), WORKER_MAIN_SOURCE)?;
    write_runtime_file(&worker_smoke_test_path(app_data_root), RUNTIME_SMOKE_TEST_SOURCE)?;
    write_runtime_file(&worker_protocol_path(app_data_root), WORKER_PROTOCOL_SOURCE)?;
    write_runtime_file(&worker_transcriber_path(app_data_root), TRANSCRIBER_SOURCE)?;
    write_runtime_file(&worker_requirements_path(app_data_root), REQUIREMENTS_SOURCE)?;
    Ok(())
}

pub fn verify_runtime_layout(app_data_root: &Path) -> Result<PathBuf, String> {
    let python_exe = resolve_python_executable(app_data_root)?;
    let worker_script = worker_entrypoint(app_data_root);

    if !worker_script.exists() {
        return Err(format!("worker entrypoint missing: {}", worker_script.display()));
    }

    Ok(python_exe)
}

fn ensure_worker_python_ready<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    crate::runtime_profile::service::ensure_worker_python_ready(app)
}

pub fn prepare_worker_runtime<R, F>(app: &AppHandle<R>, progress: F) -> Result<WorkerCudaStatus, String>
where
    R: Runtime,
    F: FnMut(WorkerRuntimeBootstrapStatus, Option<String>, Option<WorkerCudaStatus>),
{
    crate::runtime_profile::service::prepare_worker_runtime(app, progress)
}

pub(crate) fn ensure_pip_available<F>(python_exe: &Path, progress: &mut F) -> Result<(), String>
where
    F: FnMut(WorkerRuntimeBootstrapStatus, Option<String>, Option<WorkerCudaStatus>),
{
    progress(
        WorkerRuntimeBootstrapStatus::UpgradingPip,
        Some("Checking Python package installer".to_string()),
        None,
    );
    if python_command_succeeds(python_exe, &["-m", "pip", "--version"]) {
        crate::diagnostics::log(format!(
            "[transcribe] pip is already available python_exe={}",
            python_exe.display()
        ));
        return Ok(());
    }

    run_python_command(
        python_exe,
        &["-m", "ensurepip", "--upgrade"],
        "bootstrap pip with ensurepip",
    )?;
    upgrade_pip(python_exe)
}

fn upgrade_pip(python_exe: &Path) -> Result<(), String> {
    run_python_command(
        python_exe,
        &[
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            "--upgrade",
            "pip",
        ],
        "upgrade pip",
    )
}

pub(crate) fn install_runtime_profile_packages(
    python_exe: &Path,
    profile: &RuntimeProfileCatalogEntry,
) -> Result<(), String> {
    let args = provision::build_pip_install_args(profile);
    let borrowed = args.iter().map(String::as_str).collect::<Vec<_>>();
    run_python_command(
        python_exe,
        &borrowed,
        &format!("install runtime profile {}", profile.profile_id),
    )
}

pub(crate) fn install_worker_requirements(python_exe: &Path, requirements_path: &Path) -> Result<(), String> {
    let requirements = requirements_path
        .to_str()
        .ok_or_else(|| "requirements path is not valid UTF-8".to_string())?;
    run_python_command(
        python_exe,
        &[
            "-m",
            "pip",
            "install",
            "--disable-pip-version-check",
            "--no-input",
            "--no-warn-script-location",
            "--prefer-binary",
            "--only-binary",
            "editdistance",
            "--requirement",
            requirements,
        ],
        "install worker requirements",
    )
}

pub(crate) fn run_python_command(python_exe: &Path, args: &[&str], description: &str) -> Result<(), String> {
    crate::diagnostics::run_logged_command(python_exe, args, description)
}

fn python_command_succeeds(python_exe: &Path, args: &[&str]) -> bool {
    Command::new(python_exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn probe_worker_cuda<R: Runtime>(app: &AppHandle<R>) -> Result<WorkerCudaStatus, String> {
    let python_exe = ensure_worker_python_ready(app)?;
    probe_worker_cuda_with_python(&python_exe)
}

fn probe_worker_cuda_with_python(python_exe: &Path) -> Result<WorkerCudaStatus, String> {
    let parsed = probe_worker_runtime_with_python(python_exe)?;
    Ok(WorkerCudaStatus {
        cuda_available: parsed.cuda_available,
        reason: parsed.reason.or(parsed.dependency_error),
        gpu_name: parsed.gpu_name,
        torch_version: parsed.torch_version,
        cuda_version: parsed.cuda_version,
    })
}

pub(crate) fn probe_worker_runtime_with_python(python_exe: &Path) -> Result<WorkerRuntimeProbeResult, String> {
    const PROBE_SCRIPT: &str = r#"import json
result = {
    'dependencies_ready': False,
    'dependency_error': None,
    'cuda_available': False,
    'reason': None,
    'gpu_name': None,
    'torch_version': None,
    'cuda_version': None,
}

try:
    import torch
    result['torch_version'] = getattr(torch, '__version__', None)
    result['cuda_version'] = getattr(getattr(torch, 'version', None), 'cuda', None)
    try:
        result['cuda_available'] = bool(torch.cuda.is_available())
    except Exception as exc:
        result['reason'] = f'torch.cuda probe failed: {exc}'

    if result['cuda_available']:
        try:
            result['gpu_name'] = torch.cuda.get_device_name(0)
        except Exception as exc:
            result['reason'] = f'CUDA available but GPU query failed: {exc}'
    elif result['cuda_version'] is None:
        result['reason'] = 'Installed torch build does not include CUDA support'
    elif torch.cuda.device_count() == 0:
        result['reason'] = 'Torch CUDA build is present but no CUDA devices are available'
    else:
        result['reason'] = 'torch.cuda.is_available() returned false'
except Exception as exc:
    result['dependency_error'] = f'torch import failed: {exc}'

try:
    import nemo.collections.asr.models  # noqa: F401
except Exception as exc:
    if result['dependency_error']:
        result['dependency_error'] += f'; nemo import failed: {exc}'
    else:
        result['dependency_error'] = f'nemo import failed: {exc}'

result['dependencies_ready'] = result['dependency_error'] is None
print(json.dumps(result))"#;

    let output = Command::new(python_exe)
        .arg("-c")
        .arg(PROBE_SCRIPT)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| error.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Ok(WorkerRuntimeProbeResult {
            dependencies_ready: false,
            dependency_error: Some(if stderr.is_empty() {
                "Python worker probe exited unsuccessfully".to_string()
            } else {
                format!("Python worker probe failed: {stderr}")
            }),
            cuda_available: false,
            reason: None,
            gpu_name: None,
            torch_version: None,
            cuda_version: None,
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(stdout.trim()).map_err(|error| {
        format!(
            "failed to parse Python worker probe output: {} ({error})",
            summarize_worker_line(stdout.trim())
        )
    })
}

pub fn spawn_worker(
    python_exe: &Path,
    worker_script: &Path,
    model_id: ModelId,
    model_path: &Path,
) -> Result<WorkerHandle, String> {
    let mut child = Command::new(python_exe)
        .arg(worker_script)
        .arg("--model-id")
        .arg(model_id.as_str())
        .arg("--model-path")
        .arg(model_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| "worker stdin unavailable".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "worker stdout unavailable".to_string())?;
    let stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let item = line
                .map_err(|error| error.to_string())
                .and_then(|line| decode_worker_response_line(&line));
            if tx.send(item).is_err() {
                return;
            }
        }

        let _ = tx.send(Err("worker stdout closed".to_string()));
    });

    if let Some(stderr) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => eprintln!("[transcribe-worker] {line}"),
                    Err(error) => {
                        eprintln!("[transcribe-worker] stderr read failed: {error}");
                        break;
                    }
                }
            }
        });
    }

    Ok(WorkerHandle {
        child,
        stdin,
        responses: rx,
        loaded_model_id: model_id,
    })
}

impl WorkerHandle {
    pub fn wait_for_ready(&self, timeout: Duration) -> Result<(), String> {
        match self.responses.recv_timeout(timeout) {
            Ok(Ok(WorkerResponse::Ready { .. })) => Ok(()),
            Ok(Ok(WorkerResponse::Fatal { message })) => Err(message),
            Ok(Ok(other)) => Err(format!("unexpected worker startup message: {other:?}")),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(format!(
                "worker setup timed out after {}s",
                timeout.as_secs()
            )),
        }
    }

    pub fn wait_for_ready_device(&self, timeout: Duration) -> Result<Option<String>, String> {
        match self.responses.recv_timeout(timeout) {
            Ok(Ok(WorkerResponse::Ready { device, .. })) => Ok(device),
            Ok(Ok(WorkerResponse::Fatal { message })) => Err(message),
            Ok(Ok(other)) => Err(format!("unexpected worker startup message: {other:?}")),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(format!(
                "worker setup timed out after {}s",
                timeout.as_secs()
            )),
        }
    }

    pub fn send_request(&mut self, request: &WorkerRequest) -> Result<(), String> {
        let line = encode_json_line(request)?;
        self.stdin
            .write_all(line.as_bytes())
            .map_err(|error| error.to_string())?;
        self.stdin.flush().map_err(|error| error.to_string())
    }

    fn wait_for_transcribe_result(
        &self,
        request_id: &str,
        timeout: Duration,
    ) -> Result<(String, u64), WorkerFailure> {
        let start = Instant::now();

        loop {
            let Some(remaining) = timeout.checked_sub(start.elapsed()) else {
                return Err(WorkerFailure {
                    message: "transcription timed out".to_string(),
                    terminal: true,
                });
            };

            match self.responses.recv_timeout(remaining) {
                Ok(Ok(WorkerResponse::TranscribeResult {
                    request_id: current,
                    text,
                    duration_ms,
                })) if current == request_id => return Ok((text, duration_ms)),
                Ok(Ok(WorkerResponse::Error {
                    request_id: Some(current),
                    message,
                })) if current == request_id => {
                    return Err(WorkerFailure {
                        message,
                        terminal: false,
                    });
                }
                Ok(Ok(WorkerResponse::Fatal { message })) => {
                    return Err(WorkerFailure {
                        message,
                        terminal: true,
                    });
                }
                Ok(Ok(other)) => {
                    return Err(WorkerFailure {
                        message: format!("unexpected worker response: {other:?}"),
                        terminal: true,
                    });
                }
                Ok(Err(error)) => {
                    return Err(WorkerFailure {
                        message: error,
                        terminal: true,
                    });
                }
                Err(_) => {
                    return Err(WorkerFailure {
                        message: "transcription timed out".to_string(),
                        terminal: true,
                    });
                }
            }
        }
    }

    pub fn has_exited(&mut self) -> Result<bool, String> {
        self.child
            .try_wait()
            .map(|status| status.is_some())
            .map_err(|error| error.to_string())
    }

    pub fn shutdown(mut self) {
        let _ = self.send_request(&WorkerRequest::Shutdown);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn ensure_loaded<R: Runtime>(
    app: &AppHandle<R>,
    model_id: ModelId,
    model_path: &Path,
) -> Result<Option<String>, String> {
    let root = app_data_dir(app)?;
    let python_exe = ensure_worker_python_ready(app)?;

    let handles: State<'_, RuntimeHandles> = app.state();
    let mut slot = handles
        .transcription
        .lock()
        .map_err(|_| "transcription mutex poisoned".to_string())?;

    if let Some(existing) = slot.as_mut() {
        if existing.loaded_model_id == model_id && !existing.has_exited()? {
            return Ok(None);
        }
    }

    if let Some(existing) = slot.take() {
        existing.shutdown();
    }

    let worker_script = worker_entrypoint(&root);
    eprintln!(
        "[transcribe] ensure_loaded model_id={} python_exe={} worker_script={} model_path={}",
        model_id.as_str(),
        python_exe.display(),
        worker_script.display(),
        model_path.display()
    );
    let handle = spawn_worker(&python_exe, &worker_script, model_id, model_path)?;
    let ready_device = match handle.wait_for_ready_device(WORKER_START_TIMEOUT) {
        Ok(device) => device,
        Err(error) => {
            handle.shutdown();
            return Err(error);
        }
    };
    *slot = Some(handle);
    Ok(ready_device)
}

pub fn transcribe_file<R: Runtime>(
    app: &AppHandle<R>,
    audio_path: &Path,
) -> Result<TranscriptionResult, String> {
    enum Outcome {
        Success(TranscriptionResult),
        Error {
            message: String,
            terminal: bool,
            model_id: ModelId,
        },
    }

    let handles: State<'_, RuntimeHandles> = app.state();
    let mut slot = handles
        .transcription
        .lock()
        .map_err(|_| "transcription mutex poisoned".to_string())?;

    let request_id = build_request_id();
    eprintln!(
        "[transcribe] start request_id={} audio_path={}",
        request_id,
        audio_path.display()
    );
    let outcome = {
        let worker = slot
            .as_mut()
            .ok_or_else(|| "No loaded transcription model".to_string())?;

        if worker.has_exited()? {
            Outcome::Error {
                message: "Loaded transcription worker is no longer running".to_string(),
                terminal: true,
                model_id: worker.loaded_model_id,
            }
        } else {
            match worker.send_request(&WorkerRequest::Transcribe {
                request_id: request_id.clone(),
                audio_path: audio_path.display().to_string(),
            }) {
                Ok(()) => match worker.wait_for_transcribe_result(&request_id, TRANSCRIBE_TIMEOUT) {
                    Ok((text, duration_ms)) => {
                        eprintln!(
                            "[transcribe] success request_id={} chars={} duration_ms={}",
                            request_id,
                            text.chars().count(),
                            duration_ms
                        );
                        Outcome::Success(TranscriptionResult { text, duration_ms })
                    }
                    Err(failure) => Outcome::Error {
                        message: failure.message,
                        terminal: failure.terminal,
                        model_id: worker.loaded_model_id,
                    },
                },
                Err(error) => Outcome::Error {
                    message: error,
                    terminal: true,
                    model_id: worker.loaded_model_id,
                },
            }
        }
    };

    match outcome {
        Outcome::Success(result) => Ok(result),
        Outcome::Error {
            message,
            terminal: false,
            ..
        } => {
            eprintln!("[transcribe] request failed: {message}");
            Err(message)
        }
        Outcome::Error {
            message,
            terminal: true,
            model_id,
        } => {
            eprintln!("[transcribe] terminal worker failure model_id={} error={message}", model_id.as_str());
            if let Some(existing) = slot.take() {
                existing.shutdown();
            }
            drop(slot);
            clear_loaded_model_state(app, model_id, Some(message.clone()))?;
            Err(message)
        }
    }
}

pub fn unload<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let handles: State<'_, RuntimeHandles> = app.state();
    let mut slot = handles
        .transcription
        .lock()
        .map_err(|_| "transcription mutex poisoned".to_string())?;

    if let Some(existing) = slot.take() {
        existing.shutdown();
    }

    Ok(())
}

fn build_request_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("req-{nanos}")
}

fn resolve_python_executable(app_data_root: &Path) -> Result<PathBuf, String> {
    let candidates = python_command_candidates(app_data_root, configured_python_override());
    for candidate in &candidates {
        if is_usable_python_command(candidate) {
            return Ok(candidate.clone());
        }
    }

    let embedded = embedded_python_executable(app_data_root);
    Err(format!(
        "embedded python missing: {}. No usable Python interpreter found. Set OK_PYTHON_EXE or install python on PATH.",
        embedded.display()
    ))
}

fn configured_python_override() -> Option<PathBuf> {
    std::env::var_os("OK_PYTHON_EXE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn python_command_candidates(app_data_root: &Path, override_path: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(override_path) = override_path {
        candidates.push(override_path);
    }

    candidates.push(embedded_python_executable(app_data_root));
    candidates.push(PathBuf::from("python"));
    candidates
}

fn is_usable_python_command(candidate: &Path) -> bool {
    if path_requires_existing_file(candidate) && !candidate.exists() {
        return false;
    }

    Command::new(candidate)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn path_requires_existing_file(candidate: &Path) -> bool {
    candidate.is_absolute() || candidate.parent().is_some_and(|parent| parent != Path::new(""))
}

fn write_runtime_file(path: &Path, contents: &str) -> Result<(), String> {
    let needs_write = match fs::read_to_string(path) {
        Ok(existing) => existing != contents,
        Err(_) => true,
    };

    if needs_write {
        fs::write(path, contents).map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn clear_loaded_model_state<R: Runtime>(
    app: &AppHandle<R>,
    model_id: ModelId,
    error_message: Option<String>,
) -> Result<(), String> {
    let path = registry_path(app).map_err(|error| error.to_string())?;
    let state: State<'_, ModelManagerState> = app.state();
    let mut registry = state
        .registry
        .lock()
        .map_err(|_| "model registry mutex poisoned".to_string())?;

    if registry.loaded_model_id == Some(model_id) {
        registry.loaded_model_id = None;
    }

    if let Ok(mut live_device) = state.live_execution_device.lock() {
        *live_device = None;
    }

    if let Some(error_message) = error_message {
        registry
            .model_errors
            .insert(model_id.as_str().to_string(), error_message);
    }

    registry::save(&path, &registry).map_err(|error| error.to_string())?;
    drop(registry);

    emit_model_manager_snapshot(app)
}

fn emit_model_manager_snapshot<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let state: State<'_, ModelManagerState> = app.state();
    let snapshot = state.snapshot(app);
    app.emit(MODEL_MANAGER_EVENT, &snapshot)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        decode_worker_response_line, embedded_python_executable, probe_worker_cuda_with_python,
        python_command_candidates, python_runtime_dir, worker_entrypoint, worker_runtime_dir,
        WorkerRequest, WorkerResponse,
    };

    #[test]
    fn transcribe_request_serializes_as_json_line() {
        let request = WorkerRequest::Transcribe {
            request_id: "req-1".to_string(),
            audio_path: "C:/temp/recording.wav".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert_eq!(
            json,
            r#"{"type":"transcribe","request_id":"req-1","audio_path":"C:/temp/recording.wav"}"#
        );
    }

    #[test]
    fn ready_response_deserializes() {
        let response: WorkerResponse = serde_json::from_str(
            r#"{"type":"ready","model_id":"parakeet-v0.2-en","model_path":"C:/models/model.nemo","device":"cuda"}"#,
        )
        .unwrap();

        assert!(matches!(response, WorkerResponse::Ready { .. }));
    }

    #[test]
    fn fatal_response_marks_worker_unusable() {
        let response = WorkerResponse::Fatal {
            message: "model load failed".to_string(),
        };

        assert!(response.is_terminal_failure());
    }

    #[test]
    fn protocol_parse_error_is_terminal_failure() {
        let parsed = serde_json::from_str::<WorkerResponse>("not-json");
        assert!(parsed.is_err());
    }

    #[test]
    fn protocol_parse_error_reports_stdout_line() {
        let error = decode_worker_response_line("[NeMo W] noisy stdout").unwrap_err();
        assert!(error.contains("worker emitted non-JSON stdout"));
        assert!(error.contains("[NeMo W] noisy stdout"));
    }

    #[test]
    fn transcribe_result_matches_request_id() {
        let response = WorkerResponse::TranscribeResult {
            request_id: "req-123".to_string(),
            text: "hello".to_string(),
            duration_ms: 10,
        };

        assert!(response.matches_request("req-123"));
        assert!(!response.matches_request("req-999"));
    }

    #[test]
    fn error_response_without_request_id_does_not_match() {
        let response = WorkerResponse::Error {
            request_id: None,
            message: "bad".to_string(),
        };

        assert!(!response.matches_request("req-1"));
    }

    #[test]
    fn worker_runtime_dir_is_under_runtime_worker() {
        let root = Path::new("C:/AppData/OK");
        assert_eq!(worker_runtime_dir(root), root.join("runtime").join("worker"));
    }

    #[test]
    fn embedded_python_dir_is_under_runtime_python() {
        let root = Path::new("C:/AppData/OK");
        assert_eq!(python_runtime_dir(root), root.join("runtime").join("python"));
    }

    #[test]
    fn embedded_python_executable_points_to_python_exe() {
        let root = Path::new("C:/AppData/OK");
        assert_eq!(
            embedded_python_executable(root),
            root.join("runtime").join("python").join("python.exe")
        );
    }

    #[test]
    fn python_command_candidates_prefer_override_then_embedded_then_path_python() {
        let root = Path::new("C:/AppData/OK");
        let candidates = python_command_candidates(root, Some(Path::new("D:/custom/python.exe").to_path_buf()));

        assert_eq!(candidates[0], Path::new("D:/custom/python.exe"));
        assert_eq!(candidates[1], root.join("runtime").join("python").join("python.exe"));
        assert_eq!(candidates[2], Path::new("python"));
    }

    #[test]
    fn worker_entrypoint_points_to_worker_main() {
        let root = Path::new("C:/AppData/OK");
        assert_eq!(
            worker_entrypoint(root),
            root.join("runtime").join("worker").join("worker_main.py")
        );
    }

    #[test]
    fn worker_cuda_probe_reports_missing_torch() {
        let status = probe_worker_cuda_with_python(Path::new("python-not-installed")).unwrap_err();
        assert!(!status.is_empty());
    }
}
