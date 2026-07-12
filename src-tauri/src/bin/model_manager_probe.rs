use std::path::PathBuf;

use anyhow::{anyhow, Context};
use aywa_lib::model_manager::types::ModelId;
use aywa_lib::model_manager::{catalog, manifest, runtime, types::PreparedModel};

fn parse_model_id(value: &str) -> anyhow::Result<ModelId> {
    match value {
        "parakeet-v0.2-en" => Ok(ModelId::ParakeetV02En),
        "parakeet-v0.3-multilingual" => Ok(ModelId::ParakeetV03Multilingual),
        _ => Err(anyhow!("unsupported model id: {value}")),
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).context("expected a probe command")?;

    match command.as_str() {
        "preflight" => {
            println!("{}", serde_json::to_string_pretty(&runtime::preflight())?);
        }
        "manifest" => {
            let model_id = parse_model_id(args.get(2).context("expected model id")?)?;
            let client = reqwest::blocking::Client::new();
            println!(
                "{}",
                serde_json::to_string_pretty(&manifest::fetch_model_asset(&client, model_id)?)?
            );
        }
        "load-check" => {
            let model_path = PathBuf::from(args.get(2).context("expected model path")?);
            let model = PreparedModel {
                model_id: catalog::default_model_id(),
                version: "manual".to_string(),
                model_path,
                runtime: "probe",
            };
            runtime::load_check(&model)?;
            println!("load check passed");
        }
        _ => return Err(anyhow!("unknown command: {command}")),
    }
    Ok(())
}
