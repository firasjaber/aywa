use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use sha2::{Digest, Sha256};

use crate::model_manager::manifest::RemoteModelAsset;

fn temp_path_for(path: &Path) -> anyhow::Result<PathBuf> {
    let file_name = path
        .file_name()
        .context("temporary path requires a file name")?
        .to_string_lossy();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX_EPOCH")?
        .as_nanos();

    Ok(path.with_file_name(format!("{file_name}.{unique}.tmp")))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn verify_sha256(path: &Path, expected: &str) -> anyhow::Result<()> {
    let actual = sha256_hex(&fs::read(path)?);
    if actual != expected {
        return Err(anyhow!("checksum mismatch: expected {expected}, got {actual}"));
    }
    Ok(())
}

pub fn download_to_cache(
    client: &reqwest::blocking::Client,
    asset: &RemoteModelAsset,
    cache_dir: &Path,
) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(cache_dir)?;
    let target = cache_dir.join(format!("{}.part", asset.file_name));
    let temp_target = temp_path_for(&target)?;
    eprintln!(
        "[model_manager] download_to_cache start url={} temp_target={} expected_sha={} expected_size={}",
        asset.download_url,
        temp_target.display(),
        asset.sha256,
        asset.size_bytes
    );
    let mut response = client.get(&asset.download_url).send()?.error_for_status()?;
    eprintln!(
        "[model_manager] download_to_cache response received temp_target={}",
        temp_target.display()
    );
    let mut output = fs::File::create(&temp_target)?;
    let mut buffer = [0_u8; 64 * 1024];
    let mut total_bytes = 0_u64;

    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        total_bytes += read as u64;
    }

    output.flush()?;
    drop(output);
    eprintln!(
        "[model_manager] download_to_cache stream complete temp_target={} bytes_written={}",
        temp_target.display(),
        total_bytes
    );

    eprintln!(
        "[model_manager] download_to_cache verifying checksum temp_target={}",
        temp_target.display()
    );
    verify_sha256(&temp_target, &asset.sha256)?;
    eprintln!(
        "[model_manager] download_to_cache checksum ok temp_target={}",
        temp_target.display()
    );
    if target.exists() {
        eprintln!(
            "[model_manager] download_to_cache removing existing target={} before rename",
            target.display()
        );
        fs::remove_file(&target)?;
    }
    fs::rename(&temp_target, &target)?;
    eprintln!(
        "[model_manager] download_to_cache finish target={}",
        target.display()
    );
    Ok(target)
}

pub fn promote_verified_model(
    downloaded_file: &Path,
    asset: &RemoteModelAsset,
    install_dir: &Path,
) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(install_dir)?;
    let final_path = install_dir.join(&asset.file_name);
    let temp_final_path = temp_path_for(&final_path)?;
    eprintln!(
        "[model_manager] promote_verified_model copy start source={} temp_final_path={}",
        downloaded_file.display(),
        temp_final_path.display()
    );
    fs::copy(downloaded_file, &temp_final_path)?;

    if !temp_final_path.exists() {
        return Err(anyhow!("verified model file was not promoted"));
    }

    if final_path.exists() {
        eprintln!(
            "[model_manager] promote_verified_model removing existing final_path={} before rename",
            final_path.display()
        );
        fs::remove_file(&final_path)?;
    }
    fs::rename(&temp_final_path, &final_path)?;
    eprintln!(
        "[model_manager] promote_verified_model finish final_path={}",
        final_path.display()
    );
    Ok(final_path)
}

#[cfg(test)]
mod tests {
    use super::{promote_verified_model, sha256_hex, verify_sha256};
    use crate::model_manager::manifest::RemoteModelAsset;

    #[test]
    fn sha256_hex_matches_known_value() {
        assert_eq!(
            sha256_hex(b"ok"),
            "2689367b205c16ce32ed4200942b8b8b1e262dfc70d9bc9fbc77c49699a4f1df"
        );
    }

    #[test]
    fn verified_file_is_promoted_into_the_install_directory() {
        let root = std::env::temp_dir().join(format!(
            "ok-model-manager-download-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);

        let cache_file = root.join("cache").join("model.part");
        std::fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        std::fs::write(&cache_file, b"ok").unwrap();
        verify_sha256(
            &cache_file,
            "2689367b205c16ce32ed4200942b8b8b1e262dfc70d9bc9fbc77c49699a4f1df",
        )
        .unwrap();

        let asset = RemoteModelAsset {
            version: "sha256-2689367b205c".to_string(),
            file_name: "parakeet.nemo".to_string(),
            download_url: "https://example.invalid/model".to_string(),
            sha256: "2689367b205c16ce32ed4200942b8b8b1e262dfc70d9bc9fbc77c49699a4f1df"
                .to_string(),
            size_bytes: 2,
        };

        let final_path = promote_verified_model(&cache_file, &asset, &root.join("models")).unwrap();
        assert!(final_path.ends_with("parakeet.nemo"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
