use std::sync::atomic::{AtomicBool, Ordering};

use crate::runtime_profile::types::RuntimeProfileCatalogEntry;

#[derive(Debug)]
pub struct RuntimeOperationGuard<'a> {
    flag: &'a AtomicBool,
}

impl<'a> Drop for RuntimeOperationGuard<'a> {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::SeqCst);
    }
}

pub fn try_start_runtime_operation(flag: &AtomicBool) -> Result<RuntimeOperationGuard<'_>, String> {
    match flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
        Ok(_) => Ok(RuntimeOperationGuard { flag }),
        Err(_) => Err("a runtime-profile operation is already in progress".to_string()),
    }
}

pub fn build_pip_install_args(profile: &RuntimeProfileCatalogEntry) -> Vec<String> {
    let mut args = vec![
        "-m".to_string(),
        "pip".to_string(),
        "install".to_string(),
        "--disable-pip-version-check".to_string(),
        "--no-input".to_string(),
        "--no-warn-script-location".to_string(),
        "--upgrade".to_string(),
        "--index-url".to_string(),
        profile.install_source.clone(),
    ];
    args.extend(profile.packages.iter().cloned());
    args
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use crate::runtime_profile::{catalog::load_manifest, provision::{build_pip_install_args, try_start_runtime_operation}};

    #[test]
    fn pip_install_args_use_profile_install_source_and_exact_packages() {
        let manifest = load_manifest().unwrap();
        let profile = manifest
            .profiles
            .iter()
            .find(|profile| profile.profile_id == "nvidia-current")
            .unwrap();

        assert_eq!(
            build_pip_install_args(profile),
            vec![
                "-m",
                "pip",
                "install",
                "--disable-pip-version-check",
                "--no-input",
                "--no-warn-script-location",
                "--upgrade",
                "--index-url",
                "https://download.pytorch.org/whl/cu124",
                "torch==2.6.0",
                "torchaudio==2.6.0"
            ]
        );
    }

    #[test]
    fn runtime_operation_guard_rejects_concurrent_work() {
        let flag = AtomicBool::new(false);
        let first = try_start_runtime_operation(&flag).unwrap();
        let second = try_start_runtime_operation(&flag).unwrap_err();

        assert!(second.contains("already in progress"));
        drop(first);
        assert!(try_start_runtime_operation(&flag).is_ok());
    }
}
