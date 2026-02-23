//! # ochra-vys
//!
//! Validator Yield Shares (VYS) fee distribution (Section 11).
//!
//! VYS is the reward mechanism for the Ochra network. Relay operators earn
//! rewards proportional to their PoSrv contribution. The accumulator pattern
//! allows O(1) reward claims regardless of the number of stakers.
//!
//! ## Modules
//!
//! - [`accounting`] — VYS reward accumulator
//! - [`claims`] — Pull-based claims
//! - [`decay`] — Decay, slash, and CR formula

pub mod accounting;
pub mod claims;
pub mod decay;

/// Error types for VYS operations.
#[derive(Debug, thiserror::Error)]
pub enum VysError {
    /// No rewards available to claim.
    #[error("no rewards available")]
    NoRewards,

    /// Arithmetic overflow in accumulator calculation.
    #[error("arithmetic overflow")]
    Overflow,

    /// Invalid claim proof.
    #[error("invalid claim proof: {0}")]
    InvalidProof(String),

    /// Epoch mismatch.
    #[error("epoch mismatch: expected {expected}, got {actual}")]
    EpochMismatch {
        /// Expected epoch.
        expected: u64,
        /// Actual epoch.
        actual: u64,
    },

    /// Invalid PoSrv contribution value.
    #[error("invalid PoSrv contribution: {0}")]
    InvalidContribution(String),
}

/// Convenience result type for VYS operations.
pub type Result<T> = std::result::Result<T, VysError>;
