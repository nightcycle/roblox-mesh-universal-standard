pub mod assets;
pub mod exchange;
pub mod model;
pub mod roblox;
pub mod schema;

use anyhow::{Context, Result};
use std::path::Path;

pub fn import(
    input: &str,
    assets: &assets::Assets,
    scale: f32,
) -> Result<(schema::Document, Option<model::Model>)> {
    let bytes = if assets::asset_id(input).is_some() {
        assets.load(input)?
    } else {
        std::fs::read(input).with_context(|| format!("reading {input}"))?
    };
    let mut result = if bytes.starts_with(b"<roblox") {
        let model = model::Model::read(&bytes)?;
        let mesh = model.mesh()?;
        let mut document = roblox::decode(&assets.load(&mesh.content)?)?;
        document.part = Some(mesh.part);
        model.transform_special(&mut document)?;
        document
            .metadata
            .insert("SourceMesh".into(), mesh.content.into());
        (document, Some(model))
    } else if bytes.starts_with(b"version ") {
        (roblox::decode(&bytes)?, None)
    } else if Path::new(input).extension().is_some_and(|e| e == "json") {
        (serde_json::from_slice(&bytes)?, None)
    } else {
        (exchange::decode(&bytes, input, scale)?, None)
    };
    result.0.metadata.insert("Source".into(), input.into());
    result.0.validate()?;
    Ok(result)
}
