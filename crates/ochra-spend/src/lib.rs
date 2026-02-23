//! # ochra-spend
//!
//! Token spending and redemption (Section 10).
//!
//! Supports micro transactions (< 5 Seeds), macro transactions (>= 5 Seeds),
//! blind receipt tokens, and P2P transfer notes.
//!
//! ## Modules
//!
//! - [`micro`] — Micro transactions (< 5 Seeds)
//! - [`macro_tx`] — Macro transactions (>= 5 Seeds) with escrow
//! - [`blind_receipt`] — Blind receipt token system
//! - [`transfer`] — P2P transfer notes

pub mod blind_receipt;
pub mod macro_tx;
pub mod micro;
pub mod transfer;

/// Error types for spend operations.
#[derive(Debug, thiserror::Error)]
pub enum SpendError {
    /// Insufficient balance for the spend.
    #[error("insufficient balance: have {available}, need {required}")]
    InsufficientBalance {
        /// Available balance in micro-seeds.
        available: u64,
        /// Required amount in micro-seeds.
        required: u64,
    },

    /// The token has already been spent (nullifier exists).
    #[error("token already spent (nullifier exists)")]
    AlreadySpent,

    /// The spend proof is invalid.
    #[error("invalid spend proof: {0}")]
    InvalidProof(String),

    /// Escrow operation error.
    #[error("escrow error: {0}")]
    EscrowError(String),

    /// Escrow timeout expired.
    #[error("escrow timed out at {expired_at}")]
    EscrowTimeout {
        /// The timestamp when the escrow expired.
        expired_at: u64,
    },

    /// Invalid receipt.
    #[error("invalid receipt: {0}")]
    InvalidReceipt(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Encryption/decryption error.
    #[error("crypto error: {0}")]
    CryptoError(String),

    /// Amount below minimum threshold.
    #[error("amount {amount} is below minimum {minimum}")]
    BelowMinimum {
        /// The amount provided.
        amount: u64,
        /// The minimum required amount.
        minimum: u64,
    },
}

/// Convenience result type for spend operations.
pub type Result<T> = std::result::Result<T, SpendError>;
