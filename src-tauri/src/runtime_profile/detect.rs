use libloading::Library;

use crate::runtime_profile::types::RuntimeMachineFacts;

pub fn parse_nvidia_smi_line(line: &str) -> RuntimeMachineFacts {
    let mut parts = line.split(',').map(str::trim);
    let gpu_name = parts
        .next()
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let driver_version = parts
        .next()
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let compute_capability = parts.next().and_then(parse_compute_capability);

    RuntimeMachineFacts {
        gpu_vendor: Some("nvidia".to_string()),
        gpu_name,
        compute_capability,
        driver_version,
        detection_error: None,
    }
}

fn parse_compute_capability(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("n/a") {
        return None;
    }

    let digits = trimmed.replace('.', "");
    digits.parse::<u32>().ok()
}

pub fn detect_machine_facts() -> RuntimeMachineFacts {
    let cuda_driver_available = unsafe { Library::new("nvcuda.dll") }.is_ok();
    let output = crate::child_process::internal_command("nvidia-smi")
        .args([
            "--query-gpu=name,driver_version,compute_cap",
            "--format=csv,noheader",
        ])
        .output();

    match output {
        Ok(output) if output.status.success() && cuda_driver_available => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_nvidia_smi_line(stdout.lines().next().unwrap_or_default())
        }
        Ok(_) => RuntimeMachineFacts {
            gpu_vendor: None,
            gpu_name: None,
            compute_capability: None,
            driver_version: None,
            detection_error: Some(
                "nvidia-smi failed or CUDA driver DLL was unavailable".to_string(),
            ),
        },
        Err(error) => RuntimeMachineFacts {
            gpu_vendor: None,
            gpu_name: None,
            compute_capability: None,
            driver_version: None,
            detection_error: Some(format!("machine detection failed: {error}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::parse_nvidia_smi_line;

    #[test]
    fn parser_extracts_nvidia_name_driver_and_capability() {
        let facts = parse_nvidia_smi_line("NVIDIA GeForce RTX 5090, 576.80, 12.0");

        assert_eq!(facts.gpu_vendor.as_deref(), Some("nvidia"));
        assert_eq!(facts.gpu_name.as_deref(), Some("NVIDIA GeForce RTX 5090"));
        assert_eq!(facts.driver_version.as_deref(), Some("576.80"));
        assert_eq!(facts.compute_capability, Some(120));
        assert_eq!(facts.detection_error, None);
    }

    #[test]
    fn parser_leaves_capability_empty_for_missing_value() {
        let facts = parse_nvidia_smi_line("NVIDIA GeForce RTX 4090, 551.61, N/A");

        assert_eq!(facts.compute_capability, None);
    }
}
