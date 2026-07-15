use crate::runtime_profile::types::{
    RuntimeMachineFacts, RuntimeProfileCatalogEntry, RuntimeProfileManifest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRuntimeProfile {
    pub profile_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRuntimeProfile {
    pub profile_id: String,
    pub reason: String,
}

pub fn rank_auto_candidates(
    manifest: &RuntimeProfileManifest,
    facts: &RuntimeMachineFacts,
) -> (Vec<ResolvedRuntimeProfile>, Vec<SkippedRuntimeProfile>) {
    let mut selected = Vec::new();
    let mut skipped = Vec::new();
    let mut cpu = None;

    for profile in &manifest.profiles {
        if !profile.auto_eligible {
            skipped.push(skip(profile, "auto-disabled"));
            continue;
        }

        if profile.profile_id == "cpu" {
            cpu = Some(ResolvedRuntimeProfile {
                profile_id: profile.profile_id.clone(),
                reason: "guaranteed fallback".to_string(),
            });
            continue;
        }

        match profile_matches(profile, facts) {
            Ok(reason) => selected.push(ResolvedRuntimeProfile {
                profile_id: profile.profile_id.clone(),
                reason,
            }),
            Err(reason) => skipped.push(skip(profile, &reason)),
        }
    }

    selected.sort_by_key(|profile| profile_sort_order(&profile.profile_id));

    if let Some(cpu) = cpu {
        selected.push(cpu);
    }

    (selected, skipped)
}

fn profile_matches(
    profile: &RuntimeProfileCatalogEntry,
    facts: &RuntimeMachineFacts,
) -> Result<String, String> {
    if profile.vendor.as_deref() != facts.gpu_vendor.as_deref() {
        return Err(match (&profile.vendor, &facts.gpu_vendor) {
            (Some(expected), Some(actual)) => {
                format!("vendor mismatch expected={expected} actual={actual}")
            }
            (Some(expected), None) => format!("vendor mismatch expected={expected} actual=unknown"),
            _ => "vendor mismatch".to_string(),
        });
    }

    if let Some(range) = &profile.capability_range {
        let capability = facts
            .compute_capability
            .ok_or_else(|| "compute capability unavailable".to_string())?;

        if let Some(min) = range.min {
            if capability < min {
                return Err(format!(
                    "capability too low capability={capability} min={min}"
                ));
            }
        }

        if let Some(max) = range.max {
            if capability > max {
                return Err(format!(
                    "capability too high capability={capability} max={max}"
                ));
            }
        }
    }

    Ok(match facts.compute_capability {
        Some(capability) => format!("capability match capability={capability}"),
        None => "non-gpu fallback match".to_string(),
    })
}

fn profile_sort_order(profile_id: &str) -> usize {
    match profile_id {
        "nvidia-legacy" => 0,
        "nvidia-current" => 1,
        "nvidia-latest" => 2,
        "cpu" => 3,
        _ => 4,
    }
}

fn skip(profile: &RuntimeProfileCatalogEntry, reason: &str) -> SkippedRuntimeProfile {
    SkippedRuntimeProfile {
        profile_id: profile.profile_id.clone(),
        reason: reason.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::rank_auto_candidates;
    use crate::runtime_profile::{catalog::load_manifest, types::RuntimeMachineFacts};

    #[test]
    fn older_gpu_prefers_legacy_profile() {
        let manifest = load_manifest().unwrap();
        let facts = RuntimeMachineFacts {
            gpu_vendor: Some("nvidia".to_string()),
            gpu_name: Some("NVIDIA GeForce RTX 2080".to_string()),
            compute_capability: Some(75),
            driver_version: Some("551.61".to_string()),
            detection_error: None,
        };

        let (selected, _) = rank_auto_candidates(&manifest, &facts);
        assert_eq!(selected[0].profile_id, "nvidia-legacy");
        assert_eq!(selected.last().unwrap().profile_id, "cpu");
    }

    #[test]
    fn mainstream_gpu_prefers_current_profile() {
        let manifest = load_manifest().unwrap();
        let facts = RuntimeMachineFacts {
            gpu_vendor: Some("nvidia".to_string()),
            gpu_name: Some("NVIDIA GeForce RTX 4090".to_string()),
            compute_capability: Some(89),
            driver_version: Some("551.61".to_string()),
            detection_error: None,
        };

        let (selected, _) = rank_auto_candidates(&manifest, &facts);
        assert_eq!(selected[0].profile_id, "nvidia-current");
    }

    #[test]
    fn newest_gpu_prefers_latest_profile() {
        let manifest = load_manifest().unwrap();
        let facts = RuntimeMachineFacts {
            gpu_vendor: Some("nvidia".to_string()),
            gpu_name: Some("NVIDIA GeForce RTX 5090".to_string()),
            compute_capability: Some(120),
            driver_version: Some("576.80".to_string()),
            detection_error: None,
        };

        let (selected, _) = rank_auto_candidates(&manifest, &facts);
        assert_eq!(selected[0].profile_id, "nvidia-latest");
    }

    #[test]
    fn unsupported_machine_falls_back_to_cpu() {
        let manifest = load_manifest().unwrap();
        let facts = RuntimeMachineFacts {
            gpu_vendor: None,
            gpu_name: None,
            compute_capability: None,
            driver_version: None,
            detection_error: Some("nvidia-smi missing".to_string()),
        };

        let (selected, skipped) = rank_auto_candidates(&manifest, &facts);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].profile_id, "cpu");
        assert!(skipped
            .iter()
            .any(|profile| profile.reason.contains("vendor mismatch")));
    }
}
