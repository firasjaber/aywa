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
pub struct RuntimePreflight {
    pub supported: bool,
    pub reason: Option<String>,
    pub gpu_name: Option<String>,
    pub driver_version: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
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
