use std::{path::Path, process::Command};

use libloading::Library;

use crate::model_manager::types::{PreparedModel, RuntimePreflight};

pub fn parse_nvidia_smi_line(line: &str) -> (Option<String>, Option<String>) {
    let mut parts = line.split(',').map(str::trim);
    (
        parts.next().map(str::to_string),
        parts.next().map(str::to_string),
    )
}

pub fn preflight() -> RuntimePreflight {
    let cuda_driver_available = unsafe { Library::new("nvcuda.dll") }.is_ok();
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=name,driver_version", "--format=csv,noheader"])
        .output();

    match output {
        Ok(output) if output.status.success() && cuda_driver_available => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let first_line = stdout.lines().next().unwrap_or_default();
            let (gpu_name, driver_version) = parse_nvidia_smi_line(first_line);
            RuntimePreflight {
                supported: gpu_name.is_some(),
                reason: None,
                gpu_name,
                driver_version,
            }
        }
        Ok(_) => RuntimePreflight {
            supported: false,
            reason: Some("nvidia-smi failed or CUDA driver DLL was unavailable".to_string()),
            gpu_name: None,
            driver_version: None,
        },
        Err(error) => RuntimePreflight {
            supported: false,
            reason: Some(format!("preflight failed: {error}")),
            gpu_name: None,
            driver_version: None,
        },
    }
}

pub fn load_check(model: &PreparedModel) -> anyhow::Result<()> {
    if !model.model_path.exists() {
        anyhow::bail!("installed model file is missing");
    }

    if model.model_path.extension().and_then(|ext| ext.to_str()) != Some("nemo") {
        anyhow::bail!("installed model file must be a .nemo payload");
    }

    Ok(())
}

pub fn health_check(model_path: &Path) -> anyhow::Result<()> {
    if !model_path.exists() {
        anyhow::bail!("model file disappeared from disk");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_nvidia_smi_line;

    #[test]
    fn parser_extracts_gpu_name_and_driver_version() {
        let (gpu, driver) = parse_nvidia_smi_line("NVIDIA GeForce RTX 4090, 576.80");
        assert_eq!(gpu.as_deref(), Some("NVIDIA GeForce RTX 4090"));
        assert_eq!(driver.as_deref(), Some("576.80"));
    }
}
