use crate::model_manager::types::ModelId;

pub struct ModelDefinition {
    pub id: ModelId,
    pub label: &'static str,
    pub hugging_face_repo: &'static str,
    pub expected_file_name: &'static str,
}

const MODELS: [ModelDefinition; 2] = [
    ModelDefinition {
        id: ModelId::ParakeetV02En,
        label: "Parakeet English v0.2",
        hugging_face_repo: "nvidia/parakeet-tdt-0.6b-v2",
        expected_file_name: "parakeet-tdt-0.6b-v2.nemo",
    },
    ModelDefinition {
        id: ModelId::ParakeetV03Multilingual,
        label: "Parakeet Multilingual v0.3",
        hugging_face_repo: "nvidia/parakeet-tdt-0.6b-v3",
        expected_file_name: "parakeet-tdt-0.6b-v3.nemo",
    },
];

pub fn default_model_id() -> ModelId {
    ModelId::ParakeetV02En
}

pub fn all_model_ids() -> &'static [ModelId] {
    &[ModelId::ParakeetV02En, ModelId::ParakeetV03Multilingual]
}

pub fn get(model_id: ModelId) -> &'static ModelDefinition {
    MODELS
        .iter()
        .find(|definition| definition.id == model_id)
        .expect("supported model must exist")
}

#[cfg(test)]
mod tests {
    use super::{all_model_ids, default_model_id, get};
    use crate::model_manager::types::ModelId;

    #[test]
    fn default_model_is_english() {
        assert_eq!(default_model_id(), ModelId::ParakeetV02En);
    }

    #[test]
    fn catalog_contains_exactly_two_supported_models() {
        assert_eq!(
            all_model_ids(),
            &[ModelId::ParakeetV02En, ModelId::ParakeetV03Multilingual]
        );
    }

    #[test]
    fn multilingual_entry_points_at_the_v3_repo() {
        let model = get(ModelId::ParakeetV03Multilingual);
        assert_eq!(model.hugging_face_repo, "nvidia/parakeet-tdt-0.6b-v3");
        assert_eq!(model.expected_file_name, "parakeet-tdt-0.6b-v3.nemo");
    }
}
