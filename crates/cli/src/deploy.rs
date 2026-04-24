use std::{
    fs::{self, File},
    path::Path,
};

use anyhow::{bail, Context};
use rune_core::{
    DeploymentManifest, FunctionMeta, DEPLOYMENTS_MANIFEST_PATH, FUNCTIONS_DIR, RUNE_STATE_DIR,
};

pub fn deploy_function(id: &str, route: &str, source_wasm: &Path) -> anyhow::Result<FunctionMeta> {
    if id.trim().is_empty() {
        bail!("function id cannot be empty");
    }

    if !route.starts_with('/') {
        bail!("route must start with '/'");
    }

    if !source_wasm.is_file() {
        bail!("wasm artifact not found: {}", source_wasm.display());
    }

    fs::create_dir_all(RUNE_STATE_DIR)
        .with_context(|| format!("failed to create state dir '{}'", RUNE_STATE_DIR))?;
    fs::create_dir_all(FUNCTIONS_DIR)
        .with_context(|| format!("failed to create functions dir '{}'", FUNCTIONS_DIR))?;

    let mut manifest = load_manifest()?;
    let target_path = Path::new(FUNCTIONS_DIR).join(format!("{id}.wasm"));

    fs::copy(source_wasm, &target_path).with_context(|| {
        format!(
            "failed to copy '{}' to '{}'",
            source_wasm.display(),
            target_path.display()
        )
    })?;

    let func = FunctionMeta {
        id: id.to_string(),
        route: route.to_string(),
        wasm_path: target_path.to_string_lossy().into_owned(),
    };

    manifest
        .upsert(func.clone())
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    save_manifest(&manifest)?;

    Ok(func)
}

fn load_manifest() -> anyhow::Result<DeploymentManifest> {
    let manifest_path = Path::new(DEPLOYMENTS_MANIFEST_PATH);
    if !manifest_path.exists() {
        return Ok(DeploymentManifest::default());
    }

    let file = File::open(manifest_path).with_context(|| {
        format!(
            "failed to open deployment manifest '{}'",
            manifest_path.display()
        )
    })?;

    serde_json::from_reader(file).with_context(|| {
        format!(
            "failed to parse deployment manifest '{}'",
            manifest_path.display()
        )
    })
}

fn save_manifest(manifest: &DeploymentManifest) -> anyhow::Result<()> {
    let manifest_path = Path::new(DEPLOYMENTS_MANIFEST_PATH);
    let file = File::create(manifest_path).with_context(|| {
        format!(
            "failed to create deployment manifest '{}'",
            manifest_path.display()
        )
    })?;

    serde_json::to_writer_pretty(file, manifest).with_context(|| {
        format!(
            "failed to write deployment manifest '{}'",
            manifest_path.display()
        )
    })
}
