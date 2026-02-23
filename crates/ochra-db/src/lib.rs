//! # ochra-db
//!
//! Database access layer for the Ochra daemon.
//! Manages the single SQLite database at `$OCHRA_DATA_DIR/ochra.db`.
//!
//! ## Schema
//!
//! The schema follows Section 27 of the v5.5 spec exactly:
//! - WAL mode mandatory
//! - Foreign keys enforced
//! - All timestamps are Unix epoch seconds (u64)
//! - Schema version stored in `PRAGMA user_version`

pub mod migrations;
pub mod queries;
pub mod schema;

use rusqlite::Connection;
use std::path::Path;

/// Current schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// Database error types.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("migration failed: {0}")]
    Migration(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("constraint violation: {0}")]
    Constraint(String),

    #[error("serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, DbError>;

/// Open or create the Ochra database at the given path.
///
/// Configures WAL mode, foreign keys, and runs any pending migrations.
pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    configure(&conn)?;
    migrations::run(&conn)?;
    Ok(conn)
}

/// Open an in-memory database (for testing).
pub fn open_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::run(&conn)?;
    Ok(conn)
}

/// Configure SQLite pragmas.
fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -8000;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_memory() {
        let conn = open_memory().expect("open in-memory db");
        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("get user_version");
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_wal_mode() {
        let conn = open_memory().expect("open");
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .expect("get journal_mode");
        // In-memory databases use "memory" mode, not WAL
        assert!(mode == "wal" || mode == "memory");
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let conn = open_memory().expect("open");
        let fk: i32 = conn
            .pragma_query_value(None, "foreign_keys", |row| row.get(0))
            .expect("get foreign_keys");
        assert_eq!(fk, 1);
    }
}
