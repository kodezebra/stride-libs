//! SQLite plumbing.
//!
//! This crate owns *how* a database is opened and upgraded. It owns no tables:
//! `pos` and `school` have nothing in common at the schema level, and a shared
//! `orders` table would be a lie. Each app keeps its own `MIGRATIONS` slice and
//! its own queries, built on the [`Store`] handle below.
//!
//! Why `user_version` rather than a migrations table: it is a single integer in
//! the database header, needs no bootstrap schema, and cannot drift out of sync
//! with the file it describes.

use std::path::Path;

use anyhow::{Context as _, Result};
use rusqlite::Connection;

/// One forward-only schema step.
///
/// Versions must be unique and ascending. A migration is applied once, inside
/// its own transaction; there are no `down` scripts on purpose — ship a new
/// forward migration instead of trying to reverse a data change in the field.
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// An open database connection with the pragmas every stride app needs.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (creating if needed) a database file, ensuring its parent
    /// directory exists.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating data directory {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("opening database {}", path.display()))?;
        Self::prepare(conn)
    }

    /// In-memory database, for tests and for `--dry-run` style tooling.
    pub fn open_in_memory() -> Result<Self> {
        Self::prepare(Connection::open_in_memory()?)
    }

    fn prepare(conn: Connection) -> Result<Self> {
        // NORMAL is the right trade for a WAL till: full durability per commit
        // costs a synchronous write on every sale, and WAL already survives a
        // process crash. Only an OS crash can lose the last commit.
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;
             PRAGMA synchronous = NORMAL;",
        )?;
        // journal_mode returns a row, so it cannot go through execute_batch.
        let mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;
        tracing::debug!(journal_mode = %mode, "sqlite ready");
        Ok(Self { conn })
    }

    /// Raw connection for the app's own queries.
    ///
    /// Deliberately exposed rather than wrapped: a repository layer shared by
    /// two unrelated products would be invented abstraction.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Current schema version, or 0 for a fresh database.
    pub fn user_version(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    /// Stamp the file with the app that owns it, or refuse to open it.
    ///
    /// `application_id` sits in the SQLite header: it reads before a single
    /// table exists and survives the file being copied. A database already
    /// stamped by another app is refused, because opening the wrong file
    /// otherwise shows up much later as missing tables or, worse, as plausible
    /// rows from someone else's shop.
    ///
    /// The value is arbitrary and permanent. It must never be recomputed from
    /// the app's name: a rename would orphan every database in the field.
    pub fn claim_application_id(&self, app_id: i64) -> Result<()> {
        let current: i64 = self
            .conn
            .query_row("PRAGMA application_id", [], |row| row.get(0))?;

        if current == app_id {
            return Ok(());
        }
        if current != 0 {
            anyhow::bail!("database belongs to application {current}, not {app_id}");
        }

        self.conn.pragma_update(None, "application_id", app_id)?;
        tracing::debug!(app_id, "application_id stamped");
        Ok(())
    }

    /// Apply every migration newer than the database's current version.
    pub fn migrate(&mut self, migrations: &[Migration]) -> Result<()> {
        let mut current = self.user_version()?;
        for migration in migrations {
            if migration.version <= current {
                continue;
            }
            let tx = self.conn.transaction()?;
            tx.execute_batch(migration.sql)
                .with_context(|| format!("migration {} ({})", migration.version, migration.name))?;
            tx.pragma_update(None, "user_version", migration.version)?;
            tx.commit()?;
            current = migration.version;
            tracing::info!(
                version = migration.version,
                name = migration.name,
                "migration applied"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: i64 = 1_836_624_513;
    const OTHER: i64 = 2_014_884_902;

    fn path(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("storage-{name}-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn a_fresh_file_is_claimed_once_and_then_matches() {
        let path = path("claim");
        let store = Store::open(&path).expect("open");

        assert_eq!(
            store
                .conn()
                .query_row("PRAGMA application_id", [], |row| row.get::<_, i64>(0))
                .expect("read id"),
            0,
            "a new file has no owner"
        );

        store.claim_application_id(APP).expect("claim");
        store.claim_application_id(APP).expect("idempotent");

        // A second handle sees the stamp: it lives in the file, not in memory.
        let reopened = Store::open(&path).expect("reopen");
        reopened.claim_application_id(APP).expect("same owner");
        assert!(
            reopened.claim_application_id(OTHER).is_err(),
            "another app's database must be refused"
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn migrations_apply_in_order_and_stop_at_the_newest() {
        let path = path("migrate");
        let mut store = Store::open(&path).expect("open");

        let migrations = [
            Migration {
                version: 1,
                name: "first",
                sql: "CREATE TABLE a (id INTEGER PRIMARY KEY);",
            },
            Migration {
                version: 2,
                name: "second",
                sql: "CREATE TABLE b (id INTEGER PRIMARY KEY);",
            },
        ];

        store.migrate(&migrations).expect("first run");
        assert_eq!(store.user_version().expect("version"), 2);

        // Running again is a no-op rather than a "table already exists" error.
        store.migrate(&migrations).expect("second run");
        assert_eq!(store.user_version().expect("version"), 2);

        let _ = std::fs::remove_file(path);
    }
}
