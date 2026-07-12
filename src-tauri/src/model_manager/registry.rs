use std::{collections::BTreeMap, fs, path::Path};

use crate::model_manager::types::{InstalledModel, ModelId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ModelRegistry {
    pub selected_model_id: ModelId,
    pub installed: BTreeMap<String, InstalledModel>,
    pub model_errors: BTreeMap<String, String>,
    pub loaded_model_id: Option<ModelId>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self {
            selected_model_id: ModelId::ParakeetV02En,
            installed: BTreeMap::new(),
            model_errors: BTreeMap::new(),
            loaded_model_id: None,
        }
    }
}

pub fn default_selected_model_id() -> &'static str {
    "parakeet-v0.2-en"
}

pub fn load(path: &Path) -> anyhow::Result<ModelRegistry> {
    if !path.exists() {
        return Ok(ModelRegistry::default());
    }

    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

pub fn save(path: &Path, registry: &ModelRegistry) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, serde_json::to_vec_pretty(registry)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{default_selected_model_id, load, save, ModelRegistry};

    #[test]
    fn registry_defaults_to_the_english_model() {
        assert_eq!(default_selected_model_id(), "parakeet-v0.2-en");
    }

    #[test]
    fn registry_round_trips_on_disk() {
        let temp = std::env::temp_dir().join(format!(
            "aywa-model-manager-registry-test-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let expected = ModelRegistry::default();
        save(&temp, &expected).unwrap();
        let actual = load(&temp).unwrap();
        std::fs::remove_file(&temp).unwrap();
        assert_eq!(actual, expected);
    }
}
