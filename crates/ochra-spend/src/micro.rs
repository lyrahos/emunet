//! Micro transactions (< 5 Seeds = < 500,000,000 micro-seeds).
//!
//! Micro transactions are optimized for small purchases and tips. They
//! use a simplified one-step flow without escrow.
//!
//! ## Fee Rate
//!
//! A 0.1% fee is applied to all micro transactions.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{SpendError, Result};

/// Fee rate for micro transactions (0.1%).
pub const FEE_RATE: f64 = 0.001;

/// Micro transaction threshold in micro-seeds (5 Seeds).
pub const MICRO_THRESHOLD: u64 = 500_000_000;

/// A micro transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MicroTransaction {
    /// Amount in micro-seeds.
    pub amount: u64,
    /// Nullifier for double-spend prevention.
    pub nullifier: [u8; 32],
    /// Blind token proof.
    pub blind_token: Vec<u8>,
}

/// Receipt for a completed micro transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    /// Transaction hash.
    pub tx_hash: [u8; 32],
    /// Net amount after fees (micro-seeds).
    pub net_amount: u64,
    /// Fee amount (micro-seeds).
    pub fee_amount: u64,
    /// Unix timestamp.
    pub timestamp: u64,
}

/// Execute a micro transaction.
///
/// Validates the transaction, computes the fee, and generates a receipt.
///
/// # Errors
///
/// - [`SpendError::InsufficientBalance`] if amount is zero
/// - [`SpendError::InvalidProof`] if the amount exceeds [`MICRO_THRESHOLD`]
/// - [`SpendError::InvalidProof`] if the nullifier is all zeros
pub fn execute_micro(tx: &MicroTransaction) -> Result<Receipt> {
    if tx.amount == 0 {
        return Err(SpendError::InsufficientBalance {
            available: 0,
            required: 1,
        });
    }
    if tx.amount >= MICRO_THRESHOLD {
        return Err(SpendError::InvalidProof(format!(
            "amount {} exceeds micro threshold {}",
            tx.amount, MICRO_THRESHOLD
        )));
    }
    if tx.nullifier == [0u8; 32] {
        return Err(SpendError::InvalidProof(
            "nullifier must be non-zero".to_string(),
        ));
    }

    let fee_amount = compute_fee(tx.amount);
    let net_amount = tx.amount.saturating_sub(fee_amount);

    // Compute transaction hash
    let amount_bytes = tx.amount.to_le_bytes();
    let fields = blake3::encode_multi_field(&[
        &tx.nullifier[..],
        &amount_bytes,
        &tx.blind_token,
    ]);
    let tx_hash = blake3::hash(&fields);

    let timestamp = current_timestamp();

    tracing::debug!(
        amount = tx.amount,
        fee = fee_amount,
        net = net_amount,
        "micro transaction executed"
    );

    Ok(Receipt {
        tx_hash,
        net_amount,
        fee_amount,
        timestamp,
    })
}

/// Compute the fee for a given amount.
///
/// Fee = amount * FEE_RATE, minimum 1 micro-seed (unless amount is 0).
pub fn compute_fee(amount: u64) -> u64 {
    if amount == 0 {
        return 0;
    }
    let fee = (amount as f64 * FEE_RATE) as u64;
    // Minimum fee is 1 micro-seed
    if fee == 0 { 1 } else { fee }
}

/// Get the current Unix timestamp in seconds.
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_micro() {
        let tx = MicroTransaction {
            amount: 1_000_000,
            nullifier: [0x42; 32],
            blind_token: vec![0xAA; 32],
        };

        let receipt = execute_micro(&tx).expect("execute micro");
        assert_eq!(receipt.fee_amount, 1_000); // 0.1% of 1M
        assert_eq!(receipt.net_amount, 999_000);
        assert_ne!(receipt.tx_hash, [0u8; 32]);
    }

    #[test]
    fn test_execute_micro_exceeds_threshold() {
        let tx = MicroTransaction {
            amount: MICRO_THRESHOLD,
            nullifier: [0x42; 32],
            blind_token: vec![0xAA; 32],
        };

        assert!(execute_micro(&tx).is_err());
    }

    #[test]
    fn test_execute_micro_zero_amount() {
        let tx = MicroTransaction {
            amount: 0,
            nullifier: [0x42; 32],
            blind_token: vec![0xAA; 32],
        };

        assert!(execute_micro(&tx).is_err());
    }

    #[test]
    fn test_execute_micro_zero_nullifier() {
        let tx = MicroTransaction {
            amount: 1000,
            nullifier: [0u8; 32],
            blind_token: vec![0xAA; 32],
        };

        assert!(execute_micro(&tx).is_err());
    }

    #[test]
    fn test_compute_fee() {
        assert_eq!(compute_fee(1_000_000), 1_000);
        assert_eq!(compute_fee(100_000_000), 100_000);
        assert_eq!(compute_fee(100), 1); // minimum fee
        assert_eq!(compute_fee(0), 0);
    }

    #[test]
    fn test_fee_rate_constant() {
        assert!((FEE_RATE - 0.001).abs() < f64::EPSILON);
    }

    #[test]
    fn test_micro_threshold_constant() {
        assert_eq!(MICRO_THRESHOLD, 500_000_000);
    }
}
