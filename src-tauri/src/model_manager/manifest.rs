use anyhow::{anyhow, Context};
use serde::Deserialize;

use crate::model_manager::{catalog, types::ModelId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RemoteModelAsset {
    pub version: String,
    pub file_name: String,
    pub download_url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceTreeEntry {
    path: String,
    lfs: Option<HuggingFaceLfs>,
}

#[derive(Debug, Deserialize)]
struct HuggingFaceLfs {
    oid: String,
    size: u64,
}

pub fn manifest_url(model_id: ModelId) -> String {
    format!(
        "https://huggingface.co/api/models/{}/tree/main",
        catalog::get(model_id).hugging_face_repo
    )
}

pub fn parse_model_asset(model_id: ModelId, body: &str) -> anyhow::Result<RemoteModelAsset> {
    let entries: Vec<HuggingFaceTreeEntry> = serde_json::from_str(body)?;
    let definition = catalog::get(model_id);
    let entry = entries
        .into_iter()
        .find(|entry| entry.path == definition.expected_file_name)
        .ok_or_else(|| {
            anyhow!(
                "missing {} in Hugging Face tree",
                definition.expected_file_name
            )
        })?;
    let lfs = entry.lfs.context("expected LFS metadata for .nemo asset")?;

    Ok(RemoteModelAsset {
        version: format!("sha256-{}", &lfs.oid[..12]),
        file_name: entry.path.clone(),
        download_url: format!(
            "https://huggingface.co/{}/resolve/main/{}?download=1",
            definition.hugging_face_repo, entry.path
        ),
        sha256: lfs.oid,
        size_bytes: lfs.size,
    })
}

pub fn fetch_model_asset(
    client: &reqwest::blocking::Client,
    model_id: ModelId,
) -> anyhow::Result<RemoteModelAsset> {
    let url = manifest_url(model_id);
    eprintln!(
        "[model_manager] fetch_model_asset request model_id={} url={}",
        model_id.as_str(),
        url
    );
    let body = client.get(&url).send()?.error_for_status()?.text()?;

    eprintln!(
        "[model_manager] fetch_model_asset response model_id={} body_len={}",
        model_id.as_str(),
        body.len()
    );

    parse_model_asset(model_id, &body)
}

#[cfg(test)]
mod tests {
    use super::parse_model_asset;
    use crate::model_manager::types::ModelId;

    #[test]
    fn parse_model_asset_extracts_the_expected_v2_nemo_file() {
        let body = r#"[{"path":"parakeet-tdt-0.6b-v2.nemo","lfs":{"oid":"d99e39955c9d3d0350d8fb7c75e40c64a2b2eaeb003883d7c941fd2e8747b28c","size":2472222720}}]"#;
        let asset = parse_model_asset(ModelId::ParakeetV02En, body).unwrap();
        assert_eq!(asset.file_name, "parakeet-tdt-0.6b-v2.nemo");
        assert_eq!(asset.version, "sha256-d99e39955c9d");
        assert_eq!(
            asset.sha256,
            "d99e39955c9d3d0350d8fb7c75e40c64a2b2eaeb003883d7c941fd2e8747b28c"
        );
    }
}
