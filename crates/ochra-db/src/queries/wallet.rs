//! Wallet & Economy query functions (Section 27.4).

use rusqlite::Connection;

use crate::{DbError, Result};

/// Get the total unspent balance in micro-seeds.
pub fn balance(conn: &Connection) -> Result<u64> {
    let balance: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0) FROM wallet_tokens WHERE spent = 0",
            [],
            |row| row.get(0),
        )
        .map_err(DbError::Sqlite)?;
    Ok(balance as u64)
}

/// Insert a minted token.
pub fn insert_token(
    conn: &Connection,
    token_id: &[u8],
    amount: u64,
    nullifier: &[u8; 32],
    minted_at: u64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO wallet_tokens (token_id, amount, nullifier, minted_at)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            token_id,
            amount as i64,
            nullifier.as_slice(),
            minted_at as i64,
        ],
    )?;
    Ok(())
}

/// Mark a token as spent.
pub fn spend_token(conn: &Connection, token_id: &[u8], spent_at: u64) -> Result<()> {
    let updated = conn.execute(
        "UPDATE wallet_tokens SET spent = 1, spent_at = ?1 WHERE token_id = ?2 AND spent = 0",
        rusqlite::params![spent_at as i64, token_id],
    )?;
    if updated == 0 {
        return Err(DbError::NotFound("token not found or already spent".into()));
    }
    Ok(())
}

/// Record a transaction in history.
pub fn record_transaction(
    conn: &Connection,
    tx_hash: &[u8; 32],
    tx_type: &str,
    amount: u64,
    epoch: u64,
    timestamp: u64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO transaction_history (tx_hash, tx_type, amount, epoch, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            tx_hash.as_slice(),
            tx_type,
            amount as i64,
            epoch as i64,
            timestamp as i64,
        ],
    )?;
    Ok(())
}

/// List recent transactions.
pub fn recent_transactions(conn: &Connection, limit: u32) -> Result<Vec<TxRow>> {
    let mut stmt = conn.prepare(
        "SELECT tx_hash, tx_type, amount, epoch, timestamp
         FROM transaction_history ORDER BY timestamp DESC LIMIT ?1",
    )?;

    let rows = stmt
        .query_map([limit], |row| {
            Ok(TxRow {
                tx_hash: row.get::<_, Vec<u8>>(0)?,
                tx_type: row.get(1)?,
                amount: row.get::<_, i64>(2)? as u64,
                epoch: row.get::<_, i64>(3)? as u64,
                timestamp: row.get::<_, i64>(4)? as u64,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(rows)
}

/// A raw transaction row.
#[derive(Debug)]
pub struct TxRow {
    pub tx_hash: Vec<u8>,
    pub tx_type: String,
    pub amount: u64,
    pub epoch: u64,
    pub timestamp: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Connection {
        crate::open_memory().expect("open test db")
    }

    #[test]
    fn test_empty_balance() {
        let conn = test_db();
        assert_eq!(balance(&conn).expect("balance"), 0);
    }

    #[test]
    fn test_insert_and_balance() {
        let conn = test_db();
        insert_token(&conn, &[1u8; 16], 1000, &[10u8; 32], 100).expect("insert");
        insert_token(&conn, &[2u8; 16], 2000, &[20u8; 32], 100).expect("insert");
        assert_eq!(balance(&conn).expect("balance"), 3000);
    }

    #[test]
    fn test_spend_token() {
        let conn = test_db();
        insert_token(&conn, &[1u8; 16], 1000, &[10u8; 32], 100).expect("insert");
        spend_token(&conn, &[1u8; 16], 200).expect("spend");
        assert_eq!(balance(&conn).expect("balance"), 0);
    }

    #[test]
    fn test_double_spend_fails() {
        let conn = test_db();
        insert_token(&conn, &[1u8; 16], 1000, &[10u8; 32], 100).expect("insert");
        spend_token(&conn, &[1u8; 16], 200).expect("first spend");
        let result = spend_token(&conn, &[1u8; 16], 300);
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_history() {
        let conn = test_db();
        record_transaction(&conn, &[1u8; 32], "purchase", 500, 1, 1000).expect("record");
        record_transaction(&conn, &[2u8; 32], "mint", 1000, 1, 1001).expect("record");

        let txs = recent_transactions(&conn, 10).expect("list");
        assert_eq!(txs.len(), 2);
        assert_eq!(txs[0].tx_type, "mint"); // Most recent first
    }
}
