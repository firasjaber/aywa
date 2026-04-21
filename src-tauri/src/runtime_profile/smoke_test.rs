use std::{path::Path, process::Command, time::{SystemTime, UNIX_EPOCH}};

use serde::Deserialize;

use crate::runtime_profile::types::RuntimeSmokeTestResult;

#[derive(Debug, Deserialize)]
struct SmokeTestPayload {
    success: bool,
    device: String,
    torch_version: Option<String>,
    cuda_version: Option<String>,
    gpu_name: Option<String>,
    error: Option<String>,
}

pub fn run_smoke_test(
    python_exe: &Path,
    smoke_test_script: &Path,
    profile_id: &str,
    expect_gpu: bool,
) -> Result<RuntimeSmokeTestResult, String> {
    let output = Command::new(python_exe)
        .arg(smoke_test_script)
        .arg("--profile-id")
        .arg(profile_id)
        .arg("--expect-gpu")
        .arg(if expect_gpu { "true" } else { "false" })
        .output()
        .map_err(|error| format!("failed to run runtime smoke test: {error}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "runtime smoke test exited unsuccessfully".to_string()
        } else {
            format!("runtime smoke test failed: {stderr}")
        });
    }

    parse_smoke_test_output(profile_id, String::from_utf8_lossy(&output.stdout).trim())
}

fn parse_smoke_test_output(
    profile_id: &str,
    stdout: &str,
) -> Result<RuntimeSmokeTestResult, String> {
    let payload = serde_json::from_str::<SmokeTestPayload>(stdout)
        .map_err(|error| format!("failed to parse runtime smoke test output: {error}"))?;

    Ok(RuntimeSmokeTestResult {
        profile_id: profile_id.to_string(),
        success: payload.success,
        device: payload.device,
        torch_version: payload.torch_version,
        cuda_version: payload.cuda_version,
        gpu_name: payload.gpu_name,
        tested_at_epoch_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        error: payload.error,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_smoke_test_output;

    #[test]
    fn parses_success_payload() {
        let result = parse_smoke_test_output(
            "nvidia-current",
            r#"{"success":true,"device":"gpu","torch_version":"2.6.0","cuda_version":"12.4","gpu_name":"RTX 4090","error":null}"#,
        )
        .unwrap();

        assert!(result.success);
        assert_eq!(result.device, "gpu");
        assert_eq!(result.profile_id, "nvidia-current");
    }

    #[test]
    fn preserves_failure_error_text() {
        let result = parse_smoke_test_output(
            "nvidia-latest",
            r#"{"success":false,"device":"gpu","torch_version":"2.7.0","cuda_version":"12.8","gpu_name":"RTX 5090","error":"CUDA kernel launch failed"}"#,
        )
        .unwrap();

        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("CUDA kernel launch failed"));
    }
}
