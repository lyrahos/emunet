//! # ochra-pow
//!
//! Proof-of-Work and Proof-of-Retrievability for the Ochra network.
//!
//! Ochra uses memory-hard PoW (Argon2id) to rate-limit publishing and handle
//! registration (Section 2.1 of the v5.5 spec).
//!
//! ## Modules
//!
//! - [`argon2id_pow`] — Publishing PoW using Argon2id
//! - [`zk_por`] — zk-PoR circuit interface (Section 31.2)

pub mod argon2id_pow;
pub mod zk_por;

/// Error types for Proof-of-Work operations.
#[derive(Debug, thiserror::Error)]
pub enum PowError {
    /// The underlying Argon2id computation failed.
    #[error("argon2id computation failed: {0}")]
    Argon2(String),

    /// The proof did not meet the required difficulty target.
    #[error("proof does not meet difficulty target (need {required} leading zero bits, got {actual})")]
    InsufficientDifficulty {
        /// Required number of leading zero bits.
        required: u32,
        /// Actual number of leading zero bits found.
        actual: u32,
    },

    /// Invalid nonce length.
    #[error("invalid nonce length: expected {expected}, got {actual}")]
    InvalidNonceLength {
        /// Expected nonce length.
        expected: usize,
        /// Actual nonce length.
        actual: usize,
    },

    /// Proof generation or verification error.
    #[error("proof error: {0}")]
    ProofError(String),
}

/// Convenience result type for PoW operations.
pub type Result<T> = std::result::Result<T, PowError>;
