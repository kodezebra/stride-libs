//! School schema.
//!
//! Note what is *not* here: nothing shared with `pos`. Two products with the
//! same table names would still mean different things by them, so each app
//! keeps its own migrations and its own database file.

use storage::Migration;

/// Owner stamp in the SQLite header. Arbitrary, permanent, and never derived
/// from the app's name — a rename must not orphan a school's database.
pub const APPLICATION_ID: i64 = 2_014_884_902;

pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "create_students",
    sql: r#"
        CREATE TABLE IF NOT EXISTS students (
            id         INTEGER PRIMARY KEY,
            full_name  TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
    "#,
}];
