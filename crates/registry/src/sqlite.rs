use anyhow::Context;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool, sqlite::SqlitePoolOptions};

use rune_core::FunctionMeta;

// ── Pool bootstrap ────────────────────────────────────────────────────────────

/// Open (or create) the SQLite database at `db_path` and run all pending
/// migrations embedded in `migrations/`.
pub async fn open(db_path: &str) -> anyhow::Result<SqlitePool> {
    // SQLite URL: file path or `sqlite::memory:` for tests.
    let url = if db_path == ":memory:" {
        "sqlite::memory:".to_string()
    } else {
        format!("sqlite:{db_path}?mode=rwc")
    };

    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect(&url)
        .await
        .with_context(|| format!("failed to open SQLite database at '{db_path}'"))?;

    // Enable WAL for better concurrent read performance.
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys=ON;")
        .execute(&pool)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(include_str!("../migrations/001_init.sql"))
        .execute(pool)
        .await
        .context("failed to run database migrations")?;
    Ok(())
}

// ── Function persistence ──────────────────────────────────────────────────────

/// Upsert a function record. Called on every `rune deploy`.
pub async fn upsert_function(pool: &SqlitePool, meta: &FunctionMeta) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO functions (id, subdomain, route, wasm_path, updated_at)
         VALUES (?1, ?2, ?3, ?4, unixepoch())
         ON CONFLICT(id) DO UPDATE SET
             subdomain  = excluded.subdomain,
             route      = excluded.route,
             wasm_path  = excluded.wasm_path,
             updated_at = unixepoch()",
    )
    .bind(&meta.id)
    .bind(&meta.subdomain)
    .bind(&meta.route)
    .bind(&meta.wasm_path)
    .execute(pool)
    .await
    .context("failed to upsert function")?;
    Ok(())
}

/// Delete a function record by id.
pub async fn delete_function(pool: &SqlitePool, id: &str) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM functions WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .context("failed to delete function")?;
    Ok(result.rows_affected() > 0)
}

/// Load all function records (used at server startup to seed InMemoryFunctionStore).
pub async fn load_all_functions(pool: &SqlitePool) -> anyhow::Result<Vec<FunctionMeta>> {
    let rows =
        sqlx::query("SELECT id, subdomain, route, wasm_path FROM functions ORDER BY created_at")
            .fetch_all(pool)
            .await
            .context("failed to load functions")?;

    let functions = rows
        .into_iter()
        .map(|row| FunctionMeta {
            id: row.get("id"),
            subdomain: row.get("subdomain"),
            route: row.get("route"),
            wasm_path: row.get("wasm_path"),
        })
        .collect();

    Ok(functions)
}

/// List all function records (control plane list endpoint).
pub async fn list_functions(pool: &SqlitePool) -> anyhow::Result<Vec<FunctionMeta>> {
    load_all_functions(pool).await
}

// ── API key management ────────────────────────────────────────────────────────

pub const KEY_PREFIX: &str = "rune_sk_";

/// A newly generated API key. `raw` is printed once and never stored.
pub struct NewApiKey {
    pub id: String,
    pub name: String,
    pub raw: String, // rune_sk_<32 random hex bytes>
}

/// Generate a new API key, persist its hash, and return the raw key.
pub async fn create_api_key(pool: &SqlitePool, name: &str) -> anyhow::Result<NewApiKey> {
    use rand::RngCore;

    let id = uuid::Uuid::new_v4().to_string();

    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let raw = format!("{KEY_PREFIX}{}", hex::encode(bytes));

    let key_hash = hash_key(&raw);

    sqlx::query("INSERT INTO api_keys (id, name, key_hash) VALUES (?1, ?2, ?3)")
        .bind(&id)
        .bind(name)
        .bind(&key_hash)
        .execute(pool)
        .await
        .context("failed to insert api key")?;

    Ok(NewApiKey {
        id,
        name: name.to_string(),
        raw,
    })
}

/// Verify a raw key. Returns the key's `id` if valid.
pub async fn verify_api_key(pool: &SqlitePool, raw: &str) -> anyhow::Result<Option<String>> {
    let hash = hash_key(raw);
    let row = sqlx::query("SELECT id FROM api_keys WHERE key_hash = ?1")
        .bind(&hash)
        .fetch_optional(pool)
        .await
        .context("failed to verify api key")?;
    Ok(row.map(|r| r.get("id")))
}

/// List all keys (ids + names, never hashes).
pub async fn list_api_keys(pool: &SqlitePool) -> anyhow::Result<Vec<ApiKeyRecord>> {
    let rows = sqlx::query("SELECT id, name, created_at FROM api_keys ORDER BY created_at")
        .fetch_all(pool)
        .await
        .context("failed to list api keys")?;

    Ok(rows
        .into_iter()
        .map(|r| ApiKeyRecord {
            id: r.get("id"),
            name: r.get("name"),
            created_at: r.get("created_at"),
        })
        .collect())
}

/// Revoke an API key by id. Returns `true` if it existed.
pub async fn revoke_api_key(pool: &SqlitePool, id: &str) -> anyhow::Result<bool> {
    let result = sqlx::query("DELETE FROM api_keys WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
        .context("failed to revoke api key")?;
    Ok(result.rows_affected() > 0)
}

#[derive(Debug, serde::Serialize)]
pub struct ApiKeyRecord {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn hash_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_pool() -> SqlitePool {
        open(":memory:").await.unwrap()
    }

    #[tokio::test]
    async fn upsert_and_load_function() {
        let pool = test_pool().await;
        let meta = FunctionMeta {
            id: "hello".into(),
            subdomain: Some("hello".into()),
            route: "/hello".into(),
            wasm_path: ".rune/functions/hello.wasm".into(),
        };
        upsert_function(&pool, &meta).await.unwrap();

        let all = load_all_functions(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, "hello");
        assert_eq!(all[0].subdomain, Some("hello".into()));
    }

    #[tokio::test]
    async fn delete_function_works() {
        let pool = test_pool().await;
        let meta = FunctionMeta {
            id: "bye".into(),
            subdomain: None,
            route: "/bye".into(),
            wasm_path: ".rune/functions/bye.wasm".into(),
        };
        upsert_function(&pool, &meta).await.unwrap();
        assert!(delete_function(&pool, "bye").await.unwrap());
        assert!(!delete_function(&pool, "bye").await.unwrap());
        assert!(load_all_functions(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn api_key_create_verify_revoke() {
        let pool = test_pool().await;

        let key = create_api_key(&pool, "laptop").await.unwrap();
        assert!(key.raw.starts_with(KEY_PREFIX));

        // valid key
        let id = verify_api_key(&pool, &key.raw).await.unwrap();
        assert_eq!(id, Some(key.id.clone()));

        // wrong key
        let none = verify_api_key(&pool, "rune_sk_bad").await.unwrap();
        assert!(none.is_none());

        // revoke
        assert!(revoke_api_key(&pool, &key.id).await.unwrap());
        let gone = verify_api_key(&pool, &key.raw).await.unwrap();
        assert!(gone.is_none());
    }
}
