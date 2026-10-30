use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use roblox_mesh_universal_standard::{assets::Assets, model::Model, schema::Document};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(
    name = "rmus",
    version,
    about = "Export Roblox EditableMesh data to versioned JSON"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    /// Export an RBXM/RBXMX, rbxassetid:// ID, Roblox mesh, OBJ, FBX, or RMUS JSON.
    Export {
        input: String,
        #[arg(short, long)]
        output: PathBuf,
        /// Indent exported JSON for readability (compact by default).
        #[arg(long)]
        pretty: bool,
        /// Preserve RBXM children and part properties as a Rojo Configuration.
        #[arg(long)]
        children: Option<PathBuf>,
        #[arg(long, default_value = ".rmus-cache")]
        cache: PathBuf,
        #[arg(long)]
        offline: bool,
        /// JSON map from asset content IDs to local files (relative to the map).
        #[arg(long)]
        asset_map: Option<PathBuf>,
        /// OBJ/FBX source units to Roblox studs.
        #[arg(long, default_value_t = 1.0)]
        scale: f32,
        /// Merge developer metadata from a JSON object.
        #[arg(long)]
        metadata: Option<PathBuf>,
    },
    /// Validate an RMUS JSON export without network access.
    Validate { input: PathBuf },
    /// Count/list mesh instances without downloading referenced assets.
    Inspect { inputs: Vec<PathBuf> },
}
fn write(path: &Path, bytes: impl AsRef<[u8]>) -> Result<()> {
    if let Some(p) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(p)?;
    }
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}
fn json(value: &impl serde::Serialize, pretty: bool) -> Result<Vec<u8>> {
    Ok(if pretty {
        serde_json::to_vec_pretty(value)?
    } else {
        serde_json::to_vec(value)?
    })
}
fn run() -> Result<()> {
    match Cli::parse().command {
        Command::Inspect { inputs } => {
            for input in inputs {
                let m = Model::read(&fs::read(&input)?)?;
                println!(
                    "{}: {} mesh instance(s)",
                    input.display(),
                    m.mesh_instances().len()
                );
                for i in m.mesh_instances() {
                    println!(
                        "  {} {} {:?}",
                        i.class,
                        i.name,
                        roblox_mesh_universal_standard::model::content(
                            i,
                            &["MeshContent", "MeshId"]
                        )
                    );
                }
            }
        }
        Command::Validate { input } => {
            let d: Document = serde_json::from_slice(&fs::read(input)?)?;
            d.validate()?;
            println!(
                "Valid RMUS {}.{}: {} vertices, {} faces, {} bones, {} FACS poses",
                d.version.major,
                d.version.minor,
                d.vertices.len(),
                d.faces.len(),
                d.bones.len(),
                d.facs.len()
            );
        }
        Command::Export {
            input,
            output,
            pretty,
            children,
            cache,
            offline,
            asset_map,
            scale,
            metadata,
        } => {
            let mut paths = vec![&output];
            if let Some(p) = &children {
                paths.push(p);
            }
            let resolved = paths
                .iter()
                .map(|p| fs::canonicalize(p).or_else(|_| std::path::absolute(p)))
                .collect::<std::io::Result<Vec<_>>>()?;
            let source_path = fs::canonicalize(&input).ok();
            for (i, p) in resolved.iter().enumerate() {
                ensure!(
                    Some(p) != source_path.as_ref(),
                    "output would overwrite input"
                );
                ensure!(!resolved[..i].contains(p), "output paths must be distinct");
            }
            let assets = Assets::new(cache, offline, asset_map.as_deref())?;
            let (mut d, m) = roblox_mesh_universal_standard::import(&input, &assets, scale)?;
            if let Some(path) = metadata {
                let values: std::collections::BTreeMap<String, serde_json::Value> =
                    serde_json::from_slice(&fs::read(path)?)?;
                d.metadata.extend(values);
            }
            let sidecar = if children.is_some() {
                Some(
                    m.context("--children requires an RBXM/RBXMX source")?
                        .sidecar()?,
                )
            } else {
                None
            };
            d.validate()?;
            let mesh_json = json(&d, pretty)?;
            let child_json = sidecar.map(|v| json(&v, pretty)).transpose()?;
            write(&output, mesh_json)?;
            if let (Some(path), Some(json)) = (children, child_json) {
                write(&path, json)?;
            }
            println!(
                "Exported {} vertices, {} faces, {} bones, {} FACS poses to {}",
                d.vertices.len(),
                d.faces.len(),
                d.bones.len(),
                d.facs.len(),
                output.display()
            );
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("rmus: {e:#}");
        std::process::exit(1);
    }
}
