#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimeProfileCapabilityRange {
    pub min: Option<u32>,
    pub max: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimeProfileCatalogEntry {
    pub profile_id: String,
    pub label: String,
    pub stability: String,
    pub install_source: String,
    pub packages: Vec<String>,
    pub auto_eligible: bool,
    pub vendor: Option<String>,
    pub capability_range: Option<RuntimeProfileCapabilityRange>,
    pub min_driver_version: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimeProfileManifest {
    pub version: String,
    pub profiles: Vec<RuntimeProfileCatalogEntry>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimeMachineFacts {
    pub gpu_vendor: Option<String>,
    pub gpu_name: Option<String>,
    pub compute_capability: Option<u32>,
    pub driver_version: Option<String>,
    pub detection_error: Option<String>,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSelectionMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProfileOperationStatus {
    Idle,
    Detecting,
    Resolving,
    Provisioning,
    SmokeTesting,
    Ready,
    Error,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimeSmokeTestResult {
    pub profile_id: String,
    pub success: bool,
    pub device: String,
    pub torch_version: Option<String>,
    pub cuda_version: Option<String>,
    pub gpu_name: Option<String>,
    pub tested_at_epoch_ms: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RuntimeProfileState {
    pub selection_mode: RuntimeSelectionMode,
    pub selected_profile_id: Option<String>,
    pub resolved_auto_profile_id: Option<String>,
    pub active_profile_id: Option<String>,
    pub last_successful_profile_id: Option<String>,
    pub manifest_version: Option<String>,
    pub actual_execution_device: Option<String>,
    pub last_test_result: Option<RuntimeSmokeTestResult>,
    pub last_error: Option<String>,
}

impl Default for RuntimeProfileState {
    fn default() -> Self {
        Self {
            selection_mode: RuntimeSelectionMode::Auto,
            selected_profile_id: None,
            resolved_auto_profile_id: None,
            active_profile_id: None,
            last_successful_profile_id: None,
            manifest_version: None,
            actual_execution_device: None,
            last_test_result: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProfileCatalogOption {
    pub profile_id: String,
    pub label: String,
    pub stability: String,
    pub auto_eligible: bool,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProfileSnapshot {
    pub selection_mode: RuntimeSelectionMode,
    pub selected_profile_id: Option<String>,
    pub resolved_auto_profile_id: Option<String>,
    pub active_profile_id: Option<String>,
    pub last_successful_profile_id: Option<String>,
    pub manifest_version: Option<String>,
    pub actual_execution_device: Option<String>,
    pub last_test_result: Option<RuntimeSmokeTestResult>,
    pub last_error: Option<String>,
    pub profiles: Vec<RuntimeProfileCatalogOption>,
}
