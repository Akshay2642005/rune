use std::{fs::File, path::Path};

use anyhow::Context;
use rune_core::{DeploymentManifest, FunctionStore, DEPLOYMENTS_MANIFEST_PATH};

pub fn load_deployments(store: &dyn FunctionStore) -> anyhow::Result<usize> {
    let manifest_path = Path::new(DEPLOYMENTS_MANIFEST_PATH);
    if !manifest_path.exists() {
        return Ok(0);
    }

    let file = File::open(manifest_path).with_context(|| {
        format!(
            "failed to open deployment manifest '{}'",
            manifest_path.display()
        )
    })?;

    let manifest: DeploymentManifest = serde_json::from_reader(file).with_context(|| {
        format!(
            "failed to parse deployment manifest '{}'",
            manifest_path.display()
        )
    })?;

    let mut loaded = 0;
    for function in manifest.functions {
        store
            .register(function)
            .map_err(|err| anyhow::anyhow!("failed to load deployed function: {err}"))?;
        loaded += 1;
    }

    Ok(loaded)
}
