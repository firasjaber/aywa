use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use tauri::Manager;

pub const PYTHON_RUNTIME_VERSION: &str = "3.12.10-embed-amd64";
pub const PYTHON_RUNTIME_ZIP_NAME: &str = "python-3.12.10-embed-amd64.zip";
pub const GET_PIP_RESOURCE_NAME: &str = "get-pip.py";
pub const PYTHON_RUNTIME_MARKER_FILE: &str = "ok-python-runtime.json";
pub const PYTHON_RUNTIME_MODE_ENV: &str = "OK_PYTHON_RUNTIME";
pub const PYTHON_RUNTIME_MODE_MANAGED: &str = "managed";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PythonRuntimeMarker {
    pub version: String,
}

pub fn python_runtime_dir(app_data_root: &Path) -> PathBuf {
    app_data_root.join("runtime").join("python")
}

pub fn python_executable(app_data_root: &Path) -> PathBuf {
    python_runtime_dir(app_data_root).join("python.exe")
}

pub fn ensure_python_runtime<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    app_data_root: &Path,
) -> Result<PathBuf, String> {
    let force_managed = forced_managed_runtime();

    if !force_managed {
        if let Some(override_path) = configured_python_override() {
            if is_usable_python(&override_path) {
                return Ok(override_path);
            }

            return Err(format!("OK_PYTHON_EXE is not usable: {}", override_path.display()));
        }
    }

    let runtime_dir = python_runtime_dir(app_data_root);
    let python_exe = python_executable(app_data_root);
    if marker_matches(&runtime_dir) && is_usable_python(&python_exe) {
        return Ok(python_exe);
    }

    if let Some((zip_path, get_pip_path)) = find_runtime_resources(app)? {
        install_managed_runtime(&runtime_dir, &python_exe, &zip_path, &get_pip_path)?;
        return Ok(python_exe);
    }

    if force_managed {
        return Err(
            "OK_PYTHON_RUNTIME=managed was requested, but bundled Python resources were not found. Run npm run prepare:python-runtime."
                .to_string(),
        );
    }

    let fallback = path_python();
    if is_usable_python(&fallback) {
        return Ok(fallback);
    }

    Err("No usable Python runtime found. Set OK_PYTHON_EXE or run npm run prepare:python-runtime before building."
        .to_string())
}

fn install_managed_runtime(
    runtime_dir: &Path,
    python_exe: &Path,
    zip_path: &Path,
    get_pip_path: &Path,
) -> Result<(), String> {
    if runtime_dir.exists() {
        fs::remove_dir_all(runtime_dir).map_err(|error| error.to_string())?;
    }

    extract_zip(zip_path, runtime_dir)?;
    patch_embedded_python_pth(runtime_dir)?;
    bootstrap_pip(python_exe, get_pip_path)?;
    write_marker(runtime_dir)
}

fn configured_python_override() -> Option<PathBuf> {
    std::env::var_os("OK_PYTHON_EXE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn forced_managed_runtime() -> bool {
    std::env::var_os(PYTHON_RUNTIME_MODE_ENV)
        .and_then(|value| value.into_string().ok())
        .map(|value| value.eq_ignore_ascii_case(PYTHON_RUNTIME_MODE_MANAGED))
        .unwrap_or(false)
}

fn path_python() -> PathBuf {
    PathBuf::from("python")
}

fn python_resource_dirs<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Vec<PathBuf>, String> {
    let resource_dir = app
        .path()
        .resource_dir()
        .map_err(|error| error.to_string())?
        .join("python");

    #[cfg(debug_assertions)]
    {
        let mut dirs = vec![resource_dir];
        if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
            dirs.push(PathBuf::from(manifest_dir).join("resources").join("python"));
        }
        Ok(dirs)
    }

    #[cfg(not(debug_assertions))]
    {
        Ok(vec![resource_dir])
    }
}

fn find_runtime_resources<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<Option<(PathBuf, PathBuf)>, String> {
    for resource_dir in python_resource_dirs(app)? {
        let zip_path = resource_dir.join(PYTHON_RUNTIME_ZIP_NAME);
        let get_pip_path = resource_dir.join(GET_PIP_RESOURCE_NAME);
        if zip_path.exists() && get_pip_path.exists() {
            return Ok(Some((zip_path, get_pip_path)));
        }
    }

    Ok(None)
}

fn marker_path(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(PYTHON_RUNTIME_MARKER_FILE)
}

fn write_marker(runtime_dir: &Path) -> Result<(), String> {
    let marker = PythonRuntimeMarker {
        version: PYTHON_RUNTIME_VERSION.to_string(),
    };
    let json = serde_json::to_string_pretty(&marker).map_err(|error| error.to_string())?;
    fs::write(marker_path(runtime_dir), json).map_err(|error| error.to_string())
}

fn marker_matches(runtime_dir: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(marker_path(runtime_dir)) else {
        return false;
    };

    serde_json::from_str::<PythonRuntimeMarker>(&contents)
        .map(|marker| marker.version == PYTHON_RUNTIME_VERSION)
        .unwrap_or(false)
}

fn extract_zip(zip_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;

    fs::create_dir_all(target_dir).map_err(|error| error.to_string())?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|error| error.to_string())?;
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| format!("zip entry has unsafe path: {}", entry.name()))?;
        let output_path = target_dir.join(enclosed);

        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|error| error.to_string())?;
            continue;
        }

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let mut output = fs::File::create(&output_path).map_err(|error| error.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn patch_embedded_python_pth(runtime_dir: &Path) -> Result<(), String> {
    let pth_path = fs::read_dir(runtime_dir)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "_pth"))
        .ok_or_else(|| "embedded Python ._pth file missing".to_string())?;

    let contents = fs::read_to_string(&pth_path).map_err(|error| error.to_string())?;
    let patched = enable_site_in_pth(&contents);
    if patched != contents {
        fs::write(&pth_path, patched).map_err(|error| error.to_string())?;
    }

    Ok(())
}

fn enable_site_in_pth(contents: &str) -> String {
    contents
        .lines()
        .map(|line| if line.trim() == "#import site" { "import site" } else { line })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn is_usable_python(candidate: &Path) -> bool {
    Command::new(candidate)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn run_python_command(python_exe: &Path, args: &[&str], description: &str) -> Result<(), String> {
    let output = Command::new(python_exe)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("failed to {description}: {error}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(if stderr.is_empty() {
        format!("failed to {description}")
    } else {
        format!("failed to {description}: {stderr}")
    })
}

fn bootstrap_pip(python_exe: &Path, get_pip_path: &Path) -> Result<(), String> {
    if Command::new(python_exe)
        .args(["-m", "pip", "--version"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
    {
        return Ok(());
    }

    run_python_command(
        python_exe,
        &[
            get_pip_path
                .to_str()
                .ok_or_else(|| "get-pip path is not valid UTF-8".to_string())?,
            "--no-warn-script-location",
        ],
        "bootstrap pip",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_matches_current_runtime_version() {
        let temp = tempfile::tempdir().unwrap();
        write_marker(temp.path()).unwrap();

        assert!(marker_matches(temp.path()));
    }

    #[test]
    fn marker_rejects_different_runtime_version() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            marker_path(temp.path()),
            r#"{"version":"3.11.9-embed-amd64"}"#,
        )
        .unwrap();

        assert!(!marker_matches(temp.path()));
    }

    #[test]
    fn pth_patch_enables_import_site() {
        let patched = enable_site_in_pth("python312.zip\n.\n#import site\n");

        assert_eq!(patched, "python312.zip\n.\nimport site\n");
    }
}
