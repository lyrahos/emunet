//! Space query functions (Section 27.2).

use rusqlite::Connection;

use crate::Result;

/// Insert a new space.
pub fn insert(
    conn: &Connection,
    group_id: &[u8; 32],
    name: &str,
    template: &str,
    my_role: &str,
    owner_pik: &[u8; 32],
    joined_at: u64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO spaces (group_id, name, template, my_role, owner_pik, joined_at, last_activity_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        rusqlite::params![
            group_id.as_slice(),
            name,
            template,
            my_role,
            owner_pik.as_slice(),
            joined_at as i64,
        ],
    )?;
    Ok(())
}

/// List all spaces.
pub fn list(conn: &Connection) -> Result<Vec<SpaceRow>> {
    let mut stmt = conn.prepare(
        "SELECT group_id, name, template, my_role, member_count, last_activity_at, pinned
         FROM spaces ORDER BY pinned DESC, last_activity_at DESC",
    )?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SpaceRow {
                group_id: row.get::<_, Vec<u8>>(0)?,
                name: row.get(1)?,
                template: row.get(2)?,
                my_role: row.get(3)?,
                member_count: row.get::<_, i64>(4)? as u32,
                last_activity_at: row.get::<_, i64>(5)? as u64,
                pinned: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// Pin or unpin a space.
pub fn set_pinned(conn: &Connection, group_id: &[u8; 32], pinned: bool) -> Result<()> {
    conn.execute(
        "UPDATE spaces SET pinned = ?1 WHERE group_id = ?2",
        rusqlite::params![pinned, group_id.as_slice()],
    )?;
    Ok(())
}

/// A raw space row from the database.
#[derive(Debug)]
pub struct SpaceRow {
    pub group_id: Vec<u8>,
    pub name: String,
    pub template: String,
    pub my_role: String,
    pub member_count: u32,
    pub last_activity_at: u64,
    pub pinned: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        crate::open_memory().expect("open test db")
    }

    #[test]
    fn test_insert_and_list() {
        let conn = test_db();
        insert(
            &conn,
            &[1u8; 32],
            "Test Space",
            "storefront",
            "host",
            &[2u8; 32],
            1000,
        )
        .expect("insert");

        let spaces = list(&conn).expect("list");
        assert_eq!(spaces.len(), 1);
        assert_eq!(spaces[0].name, "Test Space");
        assert_eq!(spaces[0].template, "storefront");
        assert_eq!(spaces[0].my_role, "host");
    }

    #[test]
    fn test_pin_space() {
        let conn = test_db();
        insert(
            &conn, &[1u8; 32], "Space A", "forum", "member", &[2u8; 32], 1000,
        )
        .expect("insert");

        set_pinned(&conn, &[1u8; 32], true).expect("pin");
        let spaces = list(&conn).expect("list");
        assert!(spaces[0].pinned);
    }
}
