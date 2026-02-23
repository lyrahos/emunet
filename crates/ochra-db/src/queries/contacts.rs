//! Contact query functions (Section 27.1).

use rusqlite::Connection;

use crate::{DbError, Result};

/// Insert a new contact.
pub fn insert(
    conn: &Connection,
    pik_hash: &[u8; 32],
    display_name: &str,
    profile_key: &[u8; 32],
    added_at: u64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO contacts (pik_hash, display_name, profile_key, added_at, last_seen_epoch)
         VALUES (?1, ?2, ?3, ?4, 0)",
        rusqlite::params![
            pik_hash.as_slice(),
            display_name,
            profile_key.as_slice(),
            added_at as i64,
        ],
    )?;
    Ok(())
}

/// Get a contact by PIK hash.
pub fn get(conn: &Connection, pik_hash: &[u8; 32]) -> Result<ContactRow> {
    conn.query_row(
        "SELECT pik_hash, display_name, profile_key, added_at, last_seen_epoch, is_blocked
         FROM contacts WHERE pik_hash = ?1",
        [pik_hash.as_slice()],
        |row| {
            Ok(ContactRow {
                pik_hash: row.get::<_, Vec<u8>>(0)?,
                display_name: row.get(1)?,
                profile_key: row.get::<_, Vec<u8>>(2)?,
                added_at: row.get::<_, i64>(3)? as u64,
                last_seen_epoch: row.get::<_, i64>(4)? as u64,
                is_blocked: row.get::<_, bool>(5)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            DbError::NotFound("contact".into())
        }
        other => DbError::Sqlite(other),
    })
}

/// List all contacts.
pub fn list(conn: &Connection) -> Result<Vec<ContactRow>> {
    let mut stmt = conn.prepare(
        "SELECT pik_hash, display_name, profile_key, added_at, last_seen_epoch, is_blocked
         FROM contacts ORDER BY display_name",
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ContactRow {
                pik_hash: row.get::<_, Vec<u8>>(0)?,
                display_name: row.get(1)?,
                profile_key: row.get::<_, Vec<u8>>(2)?,
                added_at: row.get::<_, i64>(3)? as u64,
                last_seen_epoch: row.get::<_, i64>(4)? as u64,
                is_blocked: row.get::<_, bool>(5)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Block a contact.
pub fn block(conn: &Connection, pik_hash: &[u8; 32]) -> Result<()> {
    conn.execute(
        "UPDATE contacts SET is_blocked = 1 WHERE pik_hash = ?1",
        [pik_hash.as_slice()],
    )?;
    Ok(())
}

/// Remove a contact.
pub fn remove(conn: &Connection, pik_hash: &[u8; 32]) -> Result<()> {
    conn.execute(
        "DELETE FROM contacts WHERE pik_hash = ?1",
        [pik_hash.as_slice()],
    )?;
    Ok(())
}

/// A raw contact row from the database.
#[derive(Debug)]
pub struct ContactRow {
    pub pik_hash: Vec<u8>,
    pub display_name: String,
    pub profile_key: Vec<u8>,
    pub added_at: u64,
    pub last_seen_epoch: u64,
    pub is_blocked: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        crate::open_memory().expect("open test db")
    }

    #[test]
    fn test_insert_and_get() {
        let conn = test_db();
        let pik = [1u8; 32];
        let profile_key = [2u8; 32];

        insert(&conn, &pik, "Alice", &profile_key, 1000).expect("insert");
        let contact = get(&conn, &pik).expect("get");

        assert_eq!(contact.display_name, "Alice");
        assert_eq!(contact.added_at, 1000);
        assert!(!contact.is_blocked);
    }

    #[test]
    fn test_list_contacts() {
        let conn = test_db();
        insert(&conn, &[1u8; 32], "Bob", &[10u8; 32], 100).expect("insert");
        insert(&conn, &[2u8; 32], "Alice", &[20u8; 32], 200).expect("insert");

        let contacts = list(&conn).expect("list");
        assert_eq!(contacts.len(), 2);
        // Should be sorted by display_name
        assert_eq!(contacts[0].display_name, "Alice");
        assert_eq!(contacts[1].display_name, "Bob");
    }

    #[test]
    fn test_block_contact() {
        let conn = test_db();
        let pik = [1u8; 32];
        insert(&conn, &pik, "Eve", &[10u8; 32], 100).expect("insert");

        block(&conn, &pik).expect("block");
        let contact = get(&conn, &pik).expect("get");
        assert!(contact.is_blocked);
    }

    #[test]
    fn test_remove_contact() {
        let conn = test_db();
        let pik = [1u8; 32];
        insert(&conn, &pik, "Alice", &[10u8; 32], 100).expect("insert");
        remove(&conn, &pik).expect("remove");

        let result = get(&conn, &pik);
        assert!(matches!(result, Err(DbError::NotFound(_))));
    }
}
