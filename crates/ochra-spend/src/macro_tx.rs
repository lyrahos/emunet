//! Macro transactions (>= 5 Seeds) with escrow.
//!
//! Macro transactions use a two-phase commit with escrow to ensure atomicity
//! for larger purchases. The escrow has a 60-second timeout after which the
//! funds can be refunded.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{Result, SpendError};

/// Escrow timeout in seconds.
pub const ESCROW_TIMEOUT: u64 = 60;

/// Micro threshold in micro-seeds (5 Seeds). Transactions at or above this
/// amount must use the macro transaction flow.
pub const MACRO_MINIMUM: u64 = 500_000_000;

/// A macro transaction request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroTransaction {
    /// Amount in micro-seeds (must be >= [`MACRO_MINIMUM`]).
    pub amount: u64,
    /// Escrow identifier (deterministic from nullifier + amount).
    pub escrow_id: [u8; 32],
    /// Nullifier for double-spend prevention.
    pub nullifier: [u8; 32],
}

/// Handle to an active escrow.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EscrowHandle {
    /// The escrow identifier.
    pub escrow_id: [u8; 32],
    /// Amount held in escrow (micro-seeds).
    pub amount: u64,
    /// Unix timestamp when the escrow was created.
    pub created_at: u64,
    /// Unix timestamp when the escrow expires.
    pub expires_at: u64,
    /// Nullifier bound to this escrow.
    pub nullifier: [u8; 32],
    /// Whether the escrow has been finalized.
    pub finalized: bool,
}

/// Receipt for a finalized macro transaction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MacroReceipt {
    /// Transaction hash.
    pub tx_hash: [u8; 32],
    /// The amount transferred (micro-seeds).
    pub amount: u64,
    /// The escrow identifier.
    pub escrow_id: [u8; 32],
    /// Unix timestamp of finalization.
    pub timestamp: u64,
}

/// Refund receipt for a timed-out escrow.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Refund {
    /// The escrow identifier.
    pub escrow_id: [u8; 32],
    /// The refunded amount (micro-seeds).
    pub amount: u64,
    /// Unix timestamp of the refund.
    pub timestamp: u64,
}

/// Initiate a macro transaction by creating an escrow.
///
/// # Errors
///
/// - [`SpendError::BelowMinimum`] if amount < [`MACRO_MINIMUM`]
/// - [`SpendError::InvalidProof`] if nullifier is all zeros
pub fn initiate_macro(tx: &MacroTransaction) -> Result<EscrowHandle> {
    if tx.amount < MACRO_MINIMUM {
        return Err(SpendError::BelowMinimum {
            amount: tx.amount,
            minimum: MACRO_MINIMUM,
        });
    }
    if tx.nullifier == [0u8; 32] {
        return Err(SpendError::InvalidProof(
            "nullifier must be non-zero".to_string(),
        ));
    }

    let now = current_timestamp();

    tracing::info!(
        amount = tx.amount,
        escrow_timeout = ESCROW_TIMEOUT,
        "macro transaction: escrow initiated"
    );

    Ok(EscrowHandle {
        escrow_id: tx.escrow_id,
        amount: tx.amount,
        created_at: now,
        expires_at: now + ESCROW_TIMEOUT,
        nullifier: tx.nullifier,
        finalized: false,
    })
}

/// Finalize a macro transaction, releasing the escrow to the recipient.
///
/// # Errors
///
/// - [`SpendError::EscrowError`] if the escrow is already finalized
/// - [`SpendError::EscrowTimeout`] if the escrow has expired
pub fn finalize_macro(escrow: &mut EscrowHandle) -> Result<MacroReceipt> {
    if escrow.finalized {
        return Err(SpendError::EscrowError(
            "escrow already finalized".to_string(),
        ));
    }

    let now = current_timestamp();
    if now > escrow.expires_at {
        return Err(SpendError::EscrowTimeout {
            expired_at: escrow.expires_at,
        });
    }

    escrow.finalized = true;

    // Compute transaction hash
    let amount_bytes = escrow.amount.to_le_bytes();
    let fields =
        blake3::encode_multi_field(&[&escrow.escrow_id[..], &escrow.nullifier[..], &amount_bytes]);
    let tx_hash = blake3::hash(&fields);

    tracing::info!(amount = escrow.amount, "macro transaction: finalized");

    Ok(MacroReceipt {
        tx_hash,
        amount: escrow.amount,
        escrow_id: escrow.escrow_id,
        timestamp: now,
    })
}

/// Timeout a macro transaction, refunding the escrowed amount.
///
/// # Errors
///
/// - [`SpendError::EscrowError`] if the escrow is already finalized
/// - [`SpendError::EscrowError`] if the escrow has not yet expired
pub fn timeout_macro(escrow: &EscrowHandle) -> Result<Refund> {
    if escrow.finalized {
        return Err(SpendError::EscrowError(
            "escrow already finalized, cannot refund".to_string(),
        ));
    }

    let now = current_timestamp();
    if now <= escrow.expires_at {
        return Err(SpendError::EscrowError(format!(
            "escrow has not yet expired (expires at {})",
            escrow.expires_at
        )));
    }

    tracing::info!(
        amount = escrow.amount,
        "macro transaction: timed out, refunding"
    );

    Ok(Refund {
        escrow_id: escrow.escrow_id,
        amount: escrow.amount,
        timestamp: now,
    })
}

/// Derive an escrow ID from a nullifier and amount.
pub fn derive_escrow_id(nullifier: &[u8; 32], amount: u64) -> [u8; 32] {
    let amount_bytes = amount.to_le_bytes();
    let fields = blake3::encode_multi_field(&[nullifier.as_slice(), &amount_bytes]);
    blake3::hash(&fields)
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

    fn make_macro_tx(amount: u64) -> MacroTransaction {
        let nullifier = [0x42; 32];
        let escrow_id = derive_escrow_id(&nullifier, amount);
        MacroTransaction {
            amount,
            escrow_id,
            nullifier,
        }
    }

    #[test]
    fn test_initiate_macro() {
        let tx = make_macro_tx(MACRO_MINIMUM);
        let escrow = initiate_macro(&tx).expect("initiate");
        assert_eq!(escrow.amount, MACRO_MINIMUM);
        assert!(!escrow.finalized);
        assert!(escrow.expires_at > escrow.created_at);
    }

    #[test]
    fn test_initiate_macro_below_minimum() {
        let tx = MacroTransaction {
            amount: MACRO_MINIMUM - 1,
            escrow_id: [0xAA; 32],
            nullifier: [0x42; 32],
        };
        assert!(initiate_macro(&tx).is_err());
    }

    #[test]
    fn test_initiate_macro_zero_nullifier() {
        let tx = MacroTransaction {
            amount: MACRO_MINIMUM,
            escrow_id: [0xAA; 32],
            nullifier: [0u8; 32],
        };
        assert!(initiate_macro(&tx).is_err());
    }

    #[test]
    fn test_finalize_macro() {
        let tx = make_macro_tx(MACRO_MINIMUM);
        let mut escrow = initiate_macro(&tx).expect("initiate");
        let receipt = finalize_macro(&mut escrow).expect("finalize");
        assert_eq!(receipt.amount, MACRO_MINIMUM);
        assert!(escrow.finalized);
    }

    #[test]
    fn test_double_finalize_rejected() {
        let tx = make_macro_tx(MACRO_MINIMUM);
        let mut escrow = initiate_macro(&tx).expect("initiate");
        finalize_macro(&mut escrow).expect("first finalize");
        assert!(finalize_macro(&mut escrow).is_err());
    }

    #[test]
    fn test_derive_escrow_id_deterministic() {
        let id1 = derive_escrow_id(&[0x42; 32], 1000);
        let id2 = derive_escrow_id(&[0x42; 32], 1000);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_derive_escrow_id_varies() {
        let id1 = derive_escrow_id(&[0x42; 32], 1000);
        let id2 = derive_escrow_id(&[0x42; 32], 2000);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_escrow_timeout_constant() {
        assert_eq!(ESCROW_TIMEOUT, 60);
    }
}
