//! # ochra-mint
//!
//! Blind token minting via VOPRF (Section 10).
//!
//! Seeds are minted through a Verifiable Oblivious Pseudorandom Function
//! (VOPRF, RFC 9497) so that the minting quorum never learns which tokens
//! belong to which user.
//!
//! ## Modules
//!
//! - [`voprf_mint`] — VOPRF blind token issuance protocol
//! - [`groth16_mint`] — Minting circuit proof (Section 31.1)
//! - [`cr_throttle`] — Collateral Ratio throttling

pub mod cr_throttle;
pub mod groth16_mint;
pub mod voprf_mint;

/// Denomination of a minted token in micro-seeds.
pub type Denomination = u64;

/// Error types for minting operations.
#[derive(Debug, thiserror::Error)]
pub enum MintError {
    /// The underlying VOPRF operation failed.
    #[error("VOPRF error: {0}")]
    Voprf(String),

    /// Invalid denomination.
    #[error("invalid denomination: {0}")]
    InvalidDenomination(u64),

    /// Token verification failed.
    #[error("token verification failed")]
    VerificationFailed,

    /// Session is in an invalid state for this operation.
    #[error("invalid session state: expected {expected}, got {actual}")]
    InvalidState {
        /// The expected state.
        expected: &'static str,
        /// The actual state.
        actual: &'static str,
    },

    /// Proof generation or verification error.
    #[error("proof error: {0}")]
    ProofError(String),

    /// Collateral ratio out of allowed range.
    #[error("collateral ratio {ratio} out of range [{min}, {max}]")]
    CollateralRatioOutOfRange {
        /// The computed ratio.
        ratio: f64,
        /// Minimum allowed ratio.
        min: f64,
        /// Maximum allowed ratio.
        max: f64,
    },

    /// Minting throttled due to insufficient collateral.
    #[error("minting throttled: requested {requested}, max allowed {max_allowed}")]
    Throttled {
        /// The requested minting amount.
        requested: u64,
        /// The maximum allowed amount.
        max_allowed: u64,
    },
}

/// Convenience result type for mint operations.
pub type Result<T> = std::result::Result<T, MintError>;
