//! Database migration system (Section 27.8).
//!
//! Schema version stored in `PRAGMA user_version`. Migrations are forward-only;
//! rollback requires database rebuild from network state.

use rusqlite::Connection;

use crate::{schema, DbError, Result, SCHEMA_VERSION};

/// Run all pending migrations.
pub fn run(conn: &Connection) -> Result<()> {
    let current_version: u32 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(DbError::Sqlite)?;

    if current_version == 0 {
        // Fresh database — apply initial schema
        tracing::info!("Initializing database schema v{SCHEMA_VERSION}");
        conn.execute_batch(schema::SCHEMA_V1)
            .map_err(DbError::Sqlite)?;

        // Insert default settings
        insert_default_settings(conn)?;

        // Set version
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(DbError::Sqlite)?;
    } else if current_version < SCHEMA_VERSION {
        // Run incremental migrations
        for version in (current_version + 1)..=SCHEMA_VERSION {
            tracing::info!("Running migration to v{version}");
            run_migration(conn, version)?;
            conn.pragma_update(None, "user_version", version)
                .map_err(DbError::Sqlite)?;
        }
    } else if current_version > SCHEMA_VERSION {
        return Err(DbError::Migration(format!(
            "Database version {current_version} is newer than supported {SCHEMA_VERSION}"
        )));
    }

    Ok(())
}

/// Insert default settings.
fn insert_default_settings(conn: &Connection) -> Result<()> {
    let defaults = [
        ("earning_level", "standard"),
        ("smart_night_mode", "true"),
        ("theme_mode", "system"),
        ("accent_color", "#FF6B35"),
        ("advanced_mode", "false"),
        ("notification_global", "true"),
        ("last_epoch", "0"),
        ("bootstrap_complete", "false"),
    ];

    let mut stmt = conn
        .prepare("INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)")
        .map_err(DbError::Sqlite)?;

    for (key, value) in &defaults {
        stmt.execute(rusqlite::params![key, value])
            .map_err(DbError::Sqlite)?;
    }

    Ok(())
}

/// Run a specific migration.
fn run_migration(conn: &Connection, version: u32) -> Result<()> {
    match version {
        // Future migrations go here:
        // 2 => migration_v2(conn),
        _ => Err(DbError::Migration(format!(
            "Unknown migration version: {version}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_migration() {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch("PRAGMA foreign_keys = ON;").expect("pragma");
        run(&conn).expect("migrate");

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("version");
        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn test_idempotent_migration() {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch("PRAGMA foreign_keys = ON;").expect("pragma");
        run(&conn).expect("first run");
        run(&conn).expect("second run should be no-op");
    }

    #[test]
    fn test_default_settings() {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch("PRAGMA foreign_keys = ON;").expect("pragma");
        run(&conn).expect("migrate");

        let theme: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'theme_mode'",
                [],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(theme, "system");
    }

    #[test]
    fn test_tables_created() {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch("PRAGMA foreign_keys = ON;").expect("pragma");
        run(&conn).expect("migrate");

        let expected_tables = [
            "pik",
            "contacts",
            "recovery_contacts",
            "spaces",
            "space_members",
            "invites",
            "content_catalog",
            "wallet_tokens",
            "purchase_receipts",
            "transaction_history",
            "vys_state",
            "abr_chunks",
            "abr_service_receipts",
            "my_handle",
            "blocked_handles",
            "settings",
            "kademlia_routing",
            "pending_timelocks",
        ];

        for table in &expected_tables {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap_or_else(|_| panic!("table {table} check"));
            assert_eq!(count, 1, "Table '{table}' should exist");
        }
    }
}
