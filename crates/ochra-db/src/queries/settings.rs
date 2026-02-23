//! Settings query functions.

use rusqlite::Connection;

use crate::{DbError, Result};

/// Get a setting value by key.
pub fn get(conn: &Connection, key: &str) -> Result<String> {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |row| row.get(0),
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            DbError::NotFound(format!("setting '{key}'"))
        }
        other => DbError::Sqlite(other),
    })
}

/// Set a setting value.
pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

/// Get a setting as a boolean, defaulting to `default` if not found.
pub fn get_bool(conn: &Connection, key: &str, default: bool) -> Result<bool> {
    match get(conn, key) {
        Ok(v) => Ok(v == "true" || v == "1"),
        Err(DbError::NotFound(_)) => Ok(default),
        Err(e) => Err(e),
    }
}

/// Get a setting as u64, defaulting to `default` if not found.
pub fn get_u64(conn: &Connection, key: &str, default: u64) -> Result<u64> {
    match get(conn, key) {
        Ok(v) => v
            .parse()
            .map_err(|e: std::num::ParseIntError| DbError::Serialization(e.to_string())),
        Err(DbError::NotFound(_)) => Ok(default),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        crate::open_memory().expect("open test db")
    }

    #[test]
    fn test_get_default_setting() {
        let conn = test_db();
        let theme = get(&conn, "theme_mode").expect("get");
        assert_eq!(theme, "system");
    }

    #[test]
    fn test_set_and_get() {
        let conn = test_db();
        set(&conn, "theme_mode", "dark").expect("set");
        let theme = get(&conn, "theme_mode").expect("get");
        assert_eq!(theme, "dark");
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = test_db();
        let result = get(&conn, "nonexistent");
        assert!(matches!(result, Err(DbError::NotFound(_))));
    }

    #[test]
    fn test_get_bool() {
        let conn = test_db();
        let advanced = get_bool(&conn, "advanced_mode", false).expect("get");
        assert!(!advanced);

        set(&conn, "advanced_mode", "true").expect("set");
        let advanced = get_bool(&conn, "advanced_mode", false).expect("get");
        assert!(advanced);
    }

    #[test]
    fn test_get_u64() {
        let conn = test_db();
        let epoch = get_u64(&conn, "last_epoch", 0).expect("get");
        assert_eq!(epoch, 0);

        set(&conn, "last_epoch", "42").expect("set");
        let epoch = get_u64(&conn, "last_epoch", 0).expect("get");
        assert_eq!(epoch, 42);
    }
}
