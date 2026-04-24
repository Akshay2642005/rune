use std::{
    fs::{self, File},
    path::Path,
};

use anyhow::{Context, bail};
use rune_core::{
    DEPLOYMENTS_MANIFEST_PATH, DeploymentManifest, FUNCTIONS_DIR, FunctionMeta, RUNE_STATE_DIR,
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

    File::open(source_wasm)
        .with_context(|| format!("failed to read wasm artifact '{}'", source_wasm.display()))?;

    let mut manifest = load_manifest()?;
    let target_path = Path::new(FUNCTIONS_DIR).join(format!("{id}.wasm"));

    let func = FunctionMeta {
        id: id.to_string(),
        route: route.to_string(),
        wasm_path: target_path.to_string_lossy().into_owned(),
    };

    manifest
        .upsert(func.clone())
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;

    fs::create_dir_all(RUNE_STATE_DIR)
        .with_context(|| format!("failed to create state dir '{}'", RUNE_STATE_DIR))?;
    fs::create_dir_all(FUNCTIONS_DIR)
        .with_context(|| format!("failed to create functions dir '{}'", FUNCTIONS_DIR))?;

    fs::copy(source_wasm, &target_path).with_context(|| {
        format!(
            "failed to copy '{}' to '{}'",
            source_wasm.display(),
            target_path.display()
        )
    })?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        path::{Path, PathBuf},
        sync::Mutex,
    };

    use tempfile::TempDir;

    static CWD_LOCK: Mutex<()> = Mutex::new(());

    struct CurrentDirGuard {
        previous: PathBuf,
    }

    impl CurrentDirGuard {
        fn enter(path: &Path) -> Self {
            let previous = env::current_dir().unwrap();
            env::set_current_dir(path).unwrap();
            Self { previous }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            env::set_current_dir(&self.previous).unwrap();
        }
    }

    fn with_temp_cwd(test: impl FnOnce(&TempDir)) {
        let _lock = CWD_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        let temp_dir = tempfile::tempdir().unwrap();
        let _guard = CurrentDirGuard::enter(temp_dir.path());
        test(&temp_dir);
    }

    fn write_wasm(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn deploy_rejects_missing_artifact_without_creating_state() {
        with_temp_cwd(|temp_dir| {
            let missing = temp_dir.path().join("missing.wasm");

            let err = deploy_function("hello", "/hello", &missing).unwrap_err();

            assert!(err.to_string().contains("wasm artifact not found"));
            assert!(!temp_dir.path().join(RUNE_STATE_DIR).exists());
        });
    }

    #[test]
    fn redeploying_same_id_updates_manifest_and_artifact() {
        with_temp_cwd(|temp_dir| {
            let source_v1 = temp_dir.path().join("hello-v1.wasm");
            let source_v2 = temp_dir.path().join("hello-v2.wasm");
            write_wasm(&source_v1, b"v1");
            write_wasm(&source_v2, b"v2");

            deploy_function("hello", "/hello", &source_v1).unwrap();
            deploy_function("hello", "/v2/hello", &source_v2).unwrap();

            let manifest = load_manifest().unwrap();
            assert_eq!(manifest.functions.len(), 1);
            assert_eq!(manifest.functions[0].id, "hello");
            assert_eq!(manifest.functions[0].route, "/v2/hello");
            assert_eq!(
                Path::new(&manifest.functions[0].wasm_path),
                Path::new(FUNCTIONS_DIR).join("hello.wasm").as_path()
            );

            let copied = fs::read(Path::new(FUNCTIONS_DIR).join("hello.wasm")).unwrap();
            assert_eq!(copied, b"v2");
        });
    }

    #[test]
    fn deploy_rejects_duplicate_route_before_copying_artifact() {
        with_temp_cwd(|temp_dir| {
            let hello = temp_dir.path().join("hello.wasm");
            let goodbye = temp_dir.path().join("goodbye.wasm");
            write_wasm(&hello, b"hello");
            write_wasm(&goodbye, b"goodbye");

            deploy_function("hello", "/hello", &hello).unwrap();

            let err = deploy_function("goodbye", "/hello", &goodbye).unwrap_err();
            assert!(err.to_string().contains("duplicate route"));

            let manifest = load_manifest().unwrap();
            assert_eq!(manifest.functions.len(), 1);
            assert_eq!(manifest.functions[0].id, "hello");
            assert_eq!(manifest.functions[0].route, "/hello");
            assert!(!Path::new(FUNCTIONS_DIR).join("goodbye.wasm").exists());
        });
    }
}
