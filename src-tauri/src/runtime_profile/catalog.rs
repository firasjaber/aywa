use sha2::{Digest, Sha256};

use crate::runtime_profile::types::{RuntimeProfileCapabilityRange, RuntimeProfileManifest};

const MANIFEST_SOURCE: &str = include_str!("manifest.runtime-profiles.json");

pub fn load_manifest() -> anyhow::Result<RuntimeProfileManifest> {
    let manifest = serde_json::from_str::<RuntimeProfileManifest>(MANIFEST_SOURCE)?;

    for profile in &manifest.profiles {
        if let Some(RuntimeProfileCapabilityRange {
            min: Some(min),
            max: Some(max),
        }) = &profile.capability_range
        {
            if min > max {
                anyhow::bail!(
                    "profile {} has invalid capability_range min={} max={}",
                    profile.profile_id,
                    min,
                    max
                );
            }
        }
    }

    Ok(manifest)
}

pub fn manifest_version() -> anyhow::Result<String> {
    let manifest = load_manifest()?;
    let mut hasher = Sha256::new();
    hasher.update(manifest.version.as_bytes());
    hasher.update(MANIFEST_SOURCE.as_bytes());
    Ok(format!("{}-{:x}", manifest.version, hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::{load_manifest, manifest_version};

    #[test]
    fn bundled_manifest_contains_expected_initial_profiles() {
        let manifest = load_manifest().unwrap();
        let profile_ids = manifest
            .profiles
            .iter()
            .map(|profile| profile.profile_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            profile_ids,
            vec!["cpu", "nvidia-legacy", "nvidia-current", "nvidia-latest"]
        );
        assert!(manifest.profiles[0].auto_eligible);
    }

    #[test]
    fn manifest_version_includes_declared_version_prefix() {
        let version = manifest_version().unwrap();
        assert!(version.starts_with("2026-04-21-runtime-profiles-"));
    }
}
