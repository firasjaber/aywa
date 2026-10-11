use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::{Update, Updater, UpdaterBuilder, UpdaterExt};

const UPDATE_EVENT: &str = "app-update-changed";
const BUSY_MESSAGE: &str =
    "Finish recording, transcription, or model work before installing the update.";

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    pub revision: u64,
    pub current_version: String,
    pub available_version: Option<String>,
    pub status: &'static str,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub download_ready: bool,
    pub error: Option<String>,
}

struct UpdateInner {
    snapshot: UpdateSnapshot,
    update: Option<Update>,
    downloaded: Option<Vec<u8>>,
    active: bool,
}

#[derive(Default)]
struct Activity {
    work: usize,
    installing: bool,
}

#[derive(Clone, Default)]
pub struct OperationGate(Arc<Mutex<Activity>>);

pub struct OperationPermit {
    gate: OperationGate,
    installing: bool,
}

impl OperationGate {
    pub fn begin_work(&self) -> Result<OperationPermit, String> {
        let mut activity = self.0.lock().map_err(|_| "Update guard is unavailable")?;
        if activity.installing {
            return Err("Aywa is installing an update. Wait for it to restart.".to_string());
        }
        activity.work += 1;
        Ok(OperationPermit {
            gate: self.clone(),
            installing: false,
        })
    }

    fn ensure_idle(&self) -> Result<(), String> {
        let activity = self.0.lock().map_err(|_| "Update guard is unavailable")?;
        if activity.work != 0 || activity.installing {
            return Err(BUSY_MESSAGE.to_string());
        }
        Ok(())
    }

    fn begin_install(&self) -> Result<OperationPermit, String> {
        let mut activity = self.0.lock().map_err(|_| "Update guard is unavailable")?;
        if activity.work != 0 || activity.installing {
            return Err(BUSY_MESSAGE.to_string());
        }
        activity.installing = true;
        Ok(OperationPermit {
            gate: self.clone(),
            installing: true,
        })
    }
}

impl Drop for OperationPermit {
    fn drop(&mut self) {
        if let Ok(mut activity) = self.gate.0.lock() {
            if self.installing {
                activity.installing = false;
            } else {
                activity.work -= 1;
            }
        }
    }
}

pub struct UpdaterState {
    inner: Mutex<UpdateInner>,
    pub gate: OperationGate,
}

impl Default for UpdaterState {
    fn default() -> Self {
        Self {
            inner: Mutex::new(UpdateInner {
                snapshot: UpdateSnapshot {
                    revision: 0,
                    current_version: env!("CARGO_PKG_VERSION").to_string(),
                    available_version: None,
                    status: "idle",
                    downloaded_bytes: 0,
                    total_bytes: None,
                    download_ready: false,
                    error: None,
                },
                update: None,
                downloaded: None,
                active: false,
            }),
            gate: OperationGate::default(),
        }
    }
}

impl UpdaterState {
    pub fn snapshot(&self) -> UpdateSnapshot {
        self.inner
            .lock()
            .expect("updater mutex poisoned")
            .snapshot
            .clone()
    }

    fn change<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        change: impl FnOnce(&mut UpdateInner),
    ) -> UpdateSnapshot {
        let snapshot = {
            let mut inner = self.inner.lock().expect("updater mutex poisoned");
            change(&mut inner);
            inner.snapshot.revision += 1;
            inner.snapshot.clone()
        };
        let _ = app.emit(UPDATE_EVENT, &snapshot);
        snapshot
    }

    fn claim_check(&self) -> bool {
        let mut inner = self.inner.lock().expect("updater mutex poisoned");
        if inner.active {
            return false;
        }
        inner.active = true;
        true
    }

    fn claim_install(&self) -> Result<Option<(Update, Option<Vec<u8>>)>, String> {
        let mut inner = self.inner.lock().map_err(|_| "Updater is unavailable")?;
        if inner.active {
            return Ok(None);
        }
        self.gate.ensure_idle()?;
        let update = inner
            .update
            .clone()
            .ok_or("Check for an update before installing it.")?;
        inner.active = true;
        Ok(Some((update, inner.downloaded.take())))
    }

    fn ready_to_install<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        bytes: Vec<u8>,
    ) -> Option<(OperationPermit, Vec<u8>)> {
        match self.gate.begin_install() {
            Ok(permit) => Some((permit, bytes)),
            Err(error) => {
                self.change(app, |inner| {
                    inner.active = false;
                    inner.downloaded = Some(bytes);
                    inner.snapshot.download_ready = true;
                    inner.snapshot.status = "available";
                    inner.snapshot.error = Some(error);
                });
                None
            }
        }
    }

    fn finish_check<R: Runtime>(
        &self,
        app: &AppHandle<R>,
        manual: bool,
        result: Result<Option<Update>, String>,
        previous: UpdateSnapshot,
    ) -> UpdateSnapshot {
        self.change(app, |inner| {
            inner.active = false;
            match result {
                Ok(update) => {
                    let version = update.as_ref().map(|update| update.version.clone());
                    if version != inner.snapshot.available_version {
                        inner.downloaded = None;
                        inner.snapshot.download_ready = false;
                        inner.snapshot.downloaded_bytes = 0;
                        inner.snapshot.total_bytes = None;
                    }
                    inner.update = update;
                    inner.snapshot.available_version = version;
                    inner.snapshot.status = if inner.update.is_some() {
                        "available"
                    } else {
                        "upToDate"
                    };
                    inner.snapshot.error = None;
                }
                Err(error) if manual => {
                    crate::diagnostics::log(format!("[updater] manual check failed: {error}"));
                    inner.snapshot.status = "error";
                    inner.snapshot.error = Some(
                        "Could not check for updates. Check your connection and try again."
                            .to_string(),
                    );
                }
                Err(error) => {
                    crate::diagnostics::log(format!("[updater] automatic check failed: {error}"));
                    inner.snapshot.status = previous.status;
                    inner.snapshot.error = previous.error;
                }
            }
        })
    }
}

pub fn begin_work<R: Runtime>(app: &AppHandle<R>) -> Result<OperationPermit, String> {
    app.state::<UpdaterState>().gate.begin_work()
}

fn configured_updater(builder: UpdaterBuilder) -> Result<Updater, String> {
    builder
        .timeout(Duration::from_secs(15))
        .version_comparator(|current, release| {
            release.version.pre.is_empty() && release.version > current
        })
        .build()
        .map_err(|error| error.to_string())
}

async fn run_check<R: Runtime>(
    app: &AppHandle<R>,
    manual: bool,
    updater: Result<Updater, String>,
) -> UpdateSnapshot {
    let state = app.state::<UpdaterState>();
    if !state.claim_check() {
        return state.snapshot();
    }
    let previous = state.snapshot();
    if manual {
        state.change(app, |inner| {
            inner.snapshot.status = "checking";
            inner.snapshot.error = None;
        });
    }
    let result = match updater {
        Ok(updater) => updater.check().await.map_err(|error| error.to_string()),
        Err(error) => Err(error),
    };
    state.finish_check(app, manual, result, previous)
}

pub async fn check<R: Runtime>(app: &AppHandle<R>, manual: bool) -> UpdateSnapshot {
    run_check(app, manual, configured_updater(app.updater_builder())).await
}

pub fn start_schedule<R: Runtime>(app: AppHandle<R>) {
    if cfg!(debug_assertions) {
        return;
    }
    std::thread::spawn(move || loop {
        tauri::async_runtime::block_on(check(&app, false));
        std::thread::sleep(Duration::from_secs(86_400));
    });
}

pub fn start_install<R: Runtime>(app: AppHandle<R>) -> Result<UpdateSnapshot, String> {
    let state = app.state::<UpdaterState>();
    let Some((mut update, downloaded)) = state.claim_install()? else {
        return Ok(state.snapshot());
    };
    update.timeout = Some(Duration::from_secs(300));
    let snapshot = state.change(&app, |inner| {
        inner.snapshot.status = if downloaded.is_some() {
            "installing"
        } else {
            "downloading"
        };
        inner.snapshot.error = None;
        if downloaded.is_none() {
            inner.snapshot.downloaded_bytes = 0;
            inner.snapshot.total_bytes = None;
        }
    });
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<UpdaterState>();
        let bytes = match downloaded {
            Some(bytes) => Ok(bytes),
            None => update
                .download(
                    |chunk, total| {
                        state.change(&app, |inner| {
                            inner.snapshot.downloaded_bytes += chunk as u64;
                            inner.snapshot.total_bytes = total;
                        });
                    },
                    || {},
                )
                .await
                .map_err(|error| error.to_string()),
        };
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(error) => {
                crate::diagnostics::log(format!("[updater] download failed: {error}"));
                state.change(&app, |inner| {
                    inner.active = false;
                    inner.snapshot.status = "error";
                    inner.snapshot.download_ready = false;
                    inner.snapshot.error = Some(
                        "The update download or signature check failed. Try again.".to_string(),
                    );
                });
                return;
            }
        };
        let Some((permit, bytes)) = state.ready_to_install(&app, bytes) else {
            return;
        };
        state.change(&app, |inner| {
            inner.snapshot.status = "installing";
        });
        // Worker shutdown and installer launch are blocking operations.
        let result = tauri::async_runtime::spawn_blocking({
            let app = app.clone();
            move || {
                let _permit = permit;
                prepare_for_install(&app)
                    .and_then(|_| update.install(&bytes).map_err(|error| error.to_string()))
                    .map_err(|error| (error, bytes))
            }
        })
        .await;
        let (error, bytes) = match result {
            Ok(Ok(())) => return, // On Windows, successful handoff exits the app.
            Ok(Err(failure)) => failure,
            Err(error) => (error.to_string(), Vec::new()),
        };
        crate::diagnostics::log(format!("[updater] installation failed: {error}"));
        state.change(&app, |inner| {
            inner.active = false;
            inner.snapshot.status = "error";
            inner.snapshot.download_ready = !bytes.is_empty();
            inner.downloaded = if bytes.is_empty() { None } else { Some(bytes) };
            inner.snapshot.error = Some("Could not start the update installer. Try again or download it from GitHub Releases.".to_string());
        });
    });
    Ok(snapshot)
}

fn prepare_for_install<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    crate::transcribe::unload(app)?;
    let models = app.state::<crate::model_manager::service::ModelManagerState>();
    let result = {
        let mut registry = models
            .registry
            .lock()
            .map_err(|_| "Model registry is unavailable")?;
        registry.loaded_model_id = None;
        let path =
            crate::model_manager::service::registry_path(app).map_err(|error| error.to_string())?;
        crate::model_manager::registry::save(&path, &registry).map_err(|error| error.to_string())
    };
    *models
        .live_execution_device
        .lock()
        .map_err(|_| "Model status is unavailable")? = None;
    let _ = app.emit(
        crate::model_manager::service::MODEL_MANAGER_EVENT,
        models.snapshot(app),
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::Barrier;
    use tauri::test::{mock_builder, mock_context, noop_assets, MockRuntime};

    fn app() -> tauri::App<MockRuntime> {
        let mut context = mock_context(noop_assets());
        context.package_info_mut().version = env!("CARGO_PKG_VERSION").parse().unwrap();
        context.config_mut().plugins.0.insert(
            "updater".to_string(),
            serde_json::json!({
                "pubkey": public_key(), "endpoints": [], "dangerousInsecureTransportProtocol": true
            }),
        );
        mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .manage(UpdaterState::default())
            .build(context)
            .unwrap()
    }

    fn public_key() -> String {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        config["plugins"]["updater"]["pubkey"]
            .as_str()
            .unwrap()
            .to_string()
    }

    fn server(version: &str, body: Vec<u8>, signature: &str, requests: usize) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let manifest = serde_json::json!({
            "version": version,
            "platforms": { "windows-x86_64": {
                "url": format!("http://{address}/installer"), "signature": signature
            }}
        })
        .to_string()
        .into_bytes();
        listener.set_nonblocking(true).unwrap();
        std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let mut served = 0;
            while served < requests && std::time::Instant::now() < deadline {
                let Ok((mut socket, _)) = listener.accept() else {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                };
                socket.set_nonblocking(false).unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = [0; 4096];
                let size = socket.read(&mut request).unwrap();
                let payload =
                    if String::from_utf8_lossy(&request[..size]).starts_with("GET /installer ") {
                        &body
                    } else {
                        &manifest
                    };
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    payload.len()
                )
                .unwrap();
                socket.write_all(payload).unwrap();
                served += 1;
            }
        });
        format!("http://{address}/latest.json")
    }

    fn updater(app: &AppHandle<MockRuntime>, endpoint: &str) -> Updater {
        configured_updater(
            app.updater_builder()
                .endpoints(vec![endpoint.parse().unwrap()])
                .unwrap(),
        )
        .unwrap()
    }

    fn newer_version(app: &AppHandle<MockRuntime>) -> String {
        format!("{}.0.0", app.package_info().version.major + 1)
    }

    #[test]
    fn stable_release_discovery_excludes_equal_older_and_prerelease_versions() {
        let app = app();
        let newer = newer_version(app.handle());
        let current = app.package_info().version.to_string();
        let prerelease = format!("{newer}-beta.1");
        for (version, available) in [
            (newer.as_str(), true),
            (current.as_str(), false),
            ("0.0.0", false),
            (prerelease.as_str(), false),
        ] {
            let endpoint = server(version, Vec::new(), "fixture", 1);
            let result = tauri::async_runtime::block_on(run_check(
                app.handle(),
                true,
                Ok(updater(app.handle(), &endpoint)),
            ));
            assert_eq!(
                result.available_version.as_deref(),
                available.then_some(version)
            );
            assert_eq!(
                result.status,
                if available { "available" } else { "upToDate" }
            );
        }
    }

    #[test]
    fn checks_preserve_known_updates_on_failure_and_reject_duplicates() {
        let app = app();
        let newer = newer_version(app.handle());
        let endpoint = server(&newer, Vec::new(), "fixture", 1);
        tauri::async_runtime::block_on(run_check(
            app.handle(),
            false,
            Ok(updater(app.handle(), &endpoint)),
        ));
        let failed = tauri::async_runtime::block_on(run_check(
            app.handle(),
            false,
            Err("offline".to_string()),
        ));
        assert_eq!(failed.available_version.as_deref(), Some(newer.as_str()));
        assert_eq!(failed.status, "available");
        assert_eq!(failed.error, None);
        let failed = tauri::async_runtime::block_on(run_check(
            app.handle(),
            true,
            Err("invalid metadata".to_string()),
        ));
        assert!(failed.error.is_some());
        let state = app.state::<UpdaterState>();
        assert!(state.claim_check());
        let duplicate = tauri::async_runtime::block_on(run_check(
            app.handle(),
            true,
            Err("must not run".to_string()),
        ));
        assert_eq!(duplicate.revision, failed.revision);
    }

    #[test]
    fn an_automatic_failure_never_claims_the_app_is_up_to_date() {
        let app = app();
        let snapshot = tauri::async_runtime::block_on(run_check(
            app.handle(),
            false,
            Err("missing feed".to_string()),
        ));
        assert_eq!(snapshot.status, "idle");
        assert!(snapshot.error.is_none());
    }

    #[test]
    fn actual_plugin_rejects_missing_target_and_invalid_metadata() {
        let app = app();
        for manifest in ["{}", r#"{"version":"0.4.0","platforms":{}}"#, "not json"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/latest.json", listener.local_addr().unwrap());
            std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                let _ = socket.read(&mut request);
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    manifest.len(),
                    manifest
                )
                .unwrap();
            });
            let snapshot = tauri::async_runtime::block_on(run_check(
                app.handle(),
                true,
                Ok(updater(app.handle(), &endpoint)),
            ));
            assert_eq!(snapshot.status, "error");
            assert!(snapshot.available_version.is_none());
        }
    }

    #[test]
    fn verified_download_accepts_signed_bytes_and_rejects_changed_bytes() {
        let app = app();
        let (bytes, signature) = match std::env::var_os("AYWA_TEST_SIGNED_INSTALLER") {
            Some(path) => {
                let path = std::path::PathBuf::from(path);
                (
                    std::fs::read(&path).unwrap(),
                    std::fs::read_to_string(format!("{}.sig", path.display())).unwrap(),
                )
            }
            None => (
                include_bytes!("../test-data/update-fixture.txt").to_vec(),
                include_str!("../test-data/update-fixture.txt.sig").to_string(),
            ),
        };
        for changed in [false, true] {
            let mut payload = bytes.clone();
            if changed {
                payload.push(1);
            }
            let endpoint = server(&newer_version(app.handle()), payload, &signature, 2);
            let update = tauri::async_runtime::block_on(updater(app.handle(), &endpoint).check())
                .unwrap()
                .unwrap();
            let result = tauri::async_runtime::block_on(update.download(|_, _| {}, || {}));
            if changed {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap(), bytes);
            }
        }
    }

    #[test]
    fn operation_permits_last_through_background_work_and_release_on_failure() {
        let gate = OperationGate::default();
        let work = gate.begin_work().unwrap();
        assert!(gate.begin_install().is_err());
        let task = std::thread::spawn(move || {
            drop(work);
        });
        task.join().unwrap();
        let install = gate.begin_install().unwrap();
        assert!(gate.begin_work().is_err());
        drop(install);
        assert!(gate.begin_work().is_ok());
    }

    #[test]
    fn busy_download_is_cached_for_retry_and_duplicate_installs_are_rejected() {
        let app = app();
        let endpoint = server(&newer_version(app.handle()), Vec::new(), "fixture", 1);
        tauri::async_runtime::block_on(run_check(
            app.handle(),
            true,
            Ok(updater(app.handle(), &endpoint)),
        ));
        let state = app.state::<UpdaterState>();
        assert!(state.claim_install().unwrap().is_some());
        assert!(state.claim_install().unwrap().is_none());
        let work = state.gate.begin_work().unwrap();
        let bytes = include_bytes!("../test-data/update-fixture.txt").to_vec();
        assert!(state
            .ready_to_install(app.handle(), bytes.clone())
            .is_none());
        assert!(state.snapshot().download_ready);
        assert_eq!(state.snapshot().status, "available");
        assert!(state.claim_install().is_err());
        drop(work);
        let (_, cached) = state.claim_install().unwrap().unwrap();
        assert_eq!(cached, Some(bytes));
        let (permit, _) = state
            .ready_to_install(app.handle(), cached.unwrap())
            .unwrap();
        assert!(state.gate.begin_work().is_err());
        drop(permit);
        assert!(state.gate.begin_work().is_ok());
    }

    #[test]
    fn concurrent_install_and_work_cannot_both_enter() {
        let gate = OperationGate::default();
        let start = Arc::new(Barrier::new(3));
        let finish = Arc::new(Barrier::new(3));
        let mut tasks = Vec::new();
        for installing in [false, true] {
            let (gate, start, finish) = (gate.clone(), start.clone(), finish.clone());
            tasks.push(std::thread::spawn(move || {
                start.wait();
                let permit = if installing {
                    gate.begin_install()
                } else {
                    gate.begin_work()
                };
                let entered = permit.is_ok();
                finish.wait();
                drop(permit);
                entered
            }));
        }
        start.wait();
        finish.wait();
        assert_eq!(
            tasks
                .into_iter()
                .map(|task| task.join().unwrap())
                .filter(|entered| *entered)
                .count(),
            1
        );
    }
}
