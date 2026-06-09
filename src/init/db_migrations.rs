//! In-process database migrations.
//!
//! All migrations under `migrations/` are embedded into the binary at compile
//! time via [`embed_migrations!`] and applied at startup before any query runs.
//! This keeps the `FROM scratch` image self-contained (no `migrations/` dir or
//! `diesel` CLI required at runtime) and guarantees the schema matches the binary.
//!
//! `diesel_migrations` only operates on a synchronous connection, so a one-shot
//! libpq [`PgConnection`] is established on a blocking thread purely to run the
//! pending migrations, then dropped; all request-path queries continue to use the
//! async `bb8` pool.

use diesel::Connection;
use diesel::pg::PgConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tracing::info;

/// Migrations embedded from the crate-root `migrations/` directory.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// Apply all pending embedded migrations against `db_url`.
///
/// Runs on a blocking thread (libpq is synchronous). Returns an error if the
/// connection cannot be established or any migration fails, so the caller can
/// treat a migration failure as fatal and refuse to start with a stale schema.
pub async fn run_pending_migrations(db_url: String) -> anyhow::Result<()> {
    let applied = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<String>> {
        let mut conn = PgConnection::establish(&db_url).map_err(|e| {
            anyhow::anyhow!("Failed to establish sync connection for migrations: {e}")
        })?;

        let versions = conn
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow::anyhow!("Failed to run pending migrations: {e}"))?;

        Ok(versions.iter().map(|version| version.to_string()).collect())
    })
    .await
    .map_err(|e| anyhow::anyhow!("Migration task panicked or was cancelled: {e}"))??;

    if applied.is_empty() {
        info!("Database schema up to date; no pending migrations.");
    } else {
        info!(
            count = applied.len(),
            versions = ?applied,
            "Applied pending database migrations."
        );
    }

    Ok(())
}
