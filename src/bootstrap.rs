use std::path::Path;

use anyhow::bail;
use sqlx::SqlitePool;

use rune_core::FunctionStore;

/// Seed the in-memory function store from SQLite at server startup.
///
/// Aborts if any persisted function points to a missing WASM artifact — this
/// prevents silent 502s on functions that were never uploaded properly.
pub async fn load_deployments(
    pool: &SqlitePool,
    store: &dyn FunctionStore,
) -> anyhow::Result<usize> {
    let functions = rune_registry::load_all_functions(pool).await?;
    let mut loaded = 0;

    for func in functions {
        // Validate artifact exists.
        if !Path::new(&func.wasm_path).is_file() {
            bail!(
                "deployed function '{}' references missing wasm artifact '{}'",
                func.id,
                func.wasm_path,
            );
        }

        store
            .register(func)
            .map_err(|e| anyhow::anyhow!("failed to register function: {e}"))?;

        loaded += 1;
    }

    Ok(loaded)
}
