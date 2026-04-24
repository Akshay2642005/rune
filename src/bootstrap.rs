use std::{
    collections::HashSet,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
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

    validate_deployments(&manifest)?;

    let mut loaded = 0;
    for function in manifest.functions {
        store
            .register(function)
            .map_err(|err| anyhow::anyhow!("failed to load deployed function: {err}"))?;
        loaded += 1;
    }

    Ok(loaded)
}

fn validate_deployments(manifest: &DeploymentManifest) -> anyhow::Result<()> {
    let mut seen_routes = HashSet::new();

    for function in &manifest.functions {
        if !seen_routes.insert(function.route.as_str()) {
            bail!("duplicate deployed route in manifest: {}", function.route);
        }

        let wasm_path = PathBuf::from(&function.wasm_path);
        if !wasm_path.is_file() {
            bail!(
                "deployed function '{}' references missing wasm artifact '{}'",
                function.id,
                wasm_path.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env, fs,
        path::{Path, PathBuf},
    };

    use rune_core::{FunctionMeta, RUNE_STATE_DIR};
    use rune_registry::InMemoryFunctionStore;
    use tempfile::TempDir;

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
        let _lock = crate::TEST_CWD_LOCK.blocking_lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let _guard = CurrentDirGuard::enter(temp_dir.path());
        test(&temp_dir);
    }

    fn write_manifest(functions: Vec<FunctionMeta>) {
        fs::create_dir_all(RUNE_STATE_DIR).unwrap();
        let file = File::create(DEPLOYMENTS_MANIFEST_PATH).unwrap();
        serde_json::to_writer_pretty(file, &DeploymentManifest { functions }).unwrap();
    }

    #[test]
    fn load_deployments_rejects_missing_wasm_without_partial_registration() {
        with_temp_cwd(|_| {
            write_manifest(vec![FunctionMeta {
                id: "hello".into(),
                route: "/hello".into(),
                wasm_path: ".rune/functions/hello.wasm".into(),
            }]);

            let store = InMemoryFunctionStore::new();
            let err = load_deployments(&store).unwrap_err();

            assert!(err.to_string().contains("missing wasm artifact"));
            assert!(store.get_by_route("/hello").unwrap().is_none());
        });
    }

    #[test]
    fn load_deployments_rejects_duplicate_routes_without_partial_registration() {
        with_temp_cwd(|_| {
            fs::create_dir_all(".rune/functions").unwrap();
            fs::write(".rune/functions/hello.wasm", [0]).unwrap();
            fs::write(".rune/functions/goodbye.wasm", [1]).unwrap();

            write_manifest(vec![
                FunctionMeta {
                    id: "hello".into(),
                    route: "/hello".into(),
                    wasm_path: ".rune/functions/hello.wasm".into(),
                },
                FunctionMeta {
                    id: "goodbye".into(),
                    route: "/hello".into(),
                    wasm_path: ".rune/functions/goodbye.wasm".into(),
                },
            ]);

            let store = InMemoryFunctionStore::new();
            let err = load_deployments(&store).unwrap_err();

            assert!(err.to_string().contains("duplicate deployed route"));
            assert!(store.get_by_route("/hello").unwrap().is_none());
        });
    }
}
