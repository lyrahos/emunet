//! Content catalog query functions (Section 27.3).

use rusqlite::Connection;

use crate::{DbError, Result};

/// Insert a content item.
pub fn insert(
    conn: &Connection,
    content_hash: &[u8; 32],
    group_id: &[u8; 32],
    title: &str,
    description: Option<&str>,
    pricing_json: &str,
    creator_pik: &[u8; 32],
    key_commitment: &[u8; 32],
    total_size_bytes: u64,
    chunk_count: u32,
    published_at: u64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO content_catalog
         (content_hash, group_id, title, description, pricing, creator_pik,
          key_commitment, total_size_bytes, chunk_count, published_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            content_hash.as_slice(),
            group_id.as_slice(),
            title,
            description,
            pricing_json,
            creator_pik.as_slice(),
            key_commitment.as_slice(),
            total_size_bytes as i64,
            chunk_count as i64,
            published_at as i64,
        ],
    )?;
    Ok(())
}

/// List content for a space.
pub fn list_by_space(conn: &Connection, group_id: &[u8; 32]) -> Result<Vec<ContentRow>> {
    let mut stmt = conn.prepare(
        "SELECT content_hash, title, description, pricing, creator_pik,
                total_size_bytes, chunk_count, published_at, is_tombstoned
         FROM content_catalog
         WHERE group_id = ?1 AND is_tombstoned = 0
         ORDER BY published_at DESC",
    )?;

    let rows = stmt
        .query_map([group_id.as_slice()], |row| {
            Ok(ContentRow {
                content_hash: row.get::<_, Vec<u8>>(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                pricing_json: row.get(3)?,
                creator_pik: row.get::<_, Vec<u8>>(4)?,
                total_size_bytes: row.get::<_, i64>(5)? as u64,
                chunk_count: row.get::<_, i64>(6)? as u32,
                published_at: row.get::<_, i64>(7)? as u64,
                is_tombstoned: row.get(8)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Tombstone a content item.
pub fn tombstone(conn: &Connection, content_hash: &[u8; 32], tombstoned_at: u64) -> Result<()> {
    conn.execute(
        "UPDATE content_catalog SET is_tombstoned = 1, tombstoned_at = ?1 WHERE content_hash = ?2",
        rusqlite::params![tombstoned_at as i64, content_hash.as_slice()],
    )?;
    Ok(())
}

/// A raw content row.
#[derive(Debug)]
pub struct ContentRow {
    pub content_hash: Vec<u8>,
    pub title: String,
    pub description: Option<String>,
    pub pricing_json: String,
    pub creator_pik: Vec<u8>,
    pub total_size_bytes: u64,
    pub chunk_count: u32,
    pub published_at: u64,
    pub is_tombstoned: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::queries::spaces;

    fn test_db() -> Connection {
        let conn = crate::open_memory().expect("open test db");
        // Insert a space first (foreign key)
        spaces::insert(&conn, &[1u8; 32], "Test", "storefront", "host", &[2u8; 32], 1000)
            .expect("insert space");
        conn
    }

    #[test]
    fn test_insert_and_list() {
        let conn = test_db();
        insert(
            &conn,
            &[10u8; 32],
            &[1u8; 32],
            "Test Content",
            Some("A description"),
            r#"[{"tier_type":"permanent","price_seeds":100}]"#,
            &[3u8; 32],
            &[4u8; 32],
            1024,
            4,
            2000,
        )
        .expect("insert");

        let items = list_by_space(&conn, &[1u8; 32]).expect("list");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Test Content");
    }

    #[test]
    fn test_tombstone() {
        let conn = test_db();
        insert(
            &conn,
            &[10u8; 32],
            &[1u8; 32],
            "Item",
            None,
            "[]",
            &[3u8; 32],
            &[4u8; 32],
            512,
            2,
            2000,
        )
        .expect("insert");

        tombstone(&conn, &[10u8; 32], 3000).expect("tombstone");

        let items = list_by_space(&conn, &[1u8; 32]).expect("list");
        assert_eq!(items.len(), 0, "Tombstoned items should not appear");
    }
}
