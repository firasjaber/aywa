use std::path::PathBuf;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub enum ModelId {
    #[serde(rename = "parakeet-v0.2-en", alias = "parakeet-v02-en")]
    ParakeetV02En,
    #[serde(rename = "parakeet-v0.3-multilingual", alias = "parakeet-v03-multilingual")]
    ParakeetV03Multilingual,
}

impl ModelId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ParakeetV02En => "parakeet-v0.2-en",
            Self::ParakeetV03Multilingual => "parakeet-v0.3-multilingual",
        }
    }

    pub fn from_registry_key(value: &str) -> Option<Self> {
        match value {
            "parakeet-v0.2-en" | "parakeet-v02-en" => Some(Self::ParakeetV02En),
            "parakeet-v0.3-multilingual" | "parakeet-v03-multilingual" => {
                Some(Self::ParakeetV03Multilingual)
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum ModelStatus {
    NotDownloaded,
    Downloading,
    Downloaded,
    Loading,
    Loaded,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePreflight {
    pub supported: bool,
    pub reason: Option<String>,
    pub gpu_name: Option<String>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WorkerCudaStatus {
    pub cuda_available: bool,
    pub reason: Option<String>,
    pub gpu_name: Option<String>,
    pub torch_version: Option<String>,
    pub cuda_version: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerRuntimeBootstrapStatus {
    Idle,
    Checking,
    InstallingPythonRuntime,
    UpgradingPip,
    InstallingCudaTorch,
    InstallingWorkerRequirements,
    FallingBackToCpu,
    SmokeTesting,
    Ready,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct WorkerRuntimeBootstrap {
    pub status: WorkerRuntimeBootstrapStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct InstalledModel {
    pub version: String,
    pub sha256: String,
    pub model_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PreparedModel {
    pub model_id: ModelId,
    pub version: String,
    pub model_path: PathBuf,
    pub runtime: &'static str,
}

#[cfg(test)]
mod tests {
    use super::{RuntimePreflight, WorkerCudaStatus};

    #[test]
    fn diagnostic_types_serialize_with_frontend_field_names() {
        let preflight = serde_json::to_value(RuntimePreflight {
            supported: true,
            reason: None,
            gpu_name: Some("NVIDIA GPU".to_string()),
            driver_version: Some("576.80".to_string()),
        })
        .unwrap();
        let cuda = serde_json::to_value(WorkerCudaStatus {
            cuda_available: true,
            reason: None,
            gpu_name: Some("NVIDIA GPU".to_string()),
            torch_version: Some("2.7.0+cu128".to_string()),
            cuda_version: Some("12.8".to_string()),
        })
        .unwrap();

        assert_eq!(preflight["gpuName"], "NVIDIA GPU");
        assert_eq!(preflight["driverVersion"], "576.80");
        assert!(preflight.get("gpu_name").is_none());
        assert_eq!(cuda["cudaAvailable"], true);
        assert_eq!(cuda["torchVersion"], "2.7.0+cu128");
        assert_eq!(cuda["cudaVersion"], "12.8");
        assert!(cuda.get("cuda_available").is_none());
    }
}
