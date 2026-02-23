//! # ochra-frost
//!
//! FROST threshold signature ceremony coordination (Section 12).
//!
//! This crate provides higher-level coordination for FROST DKG ceremonies
//! and signing rounds. It wraps the low-level cryptographic operations from
//! `ochra-crypto::frost` with session management, timeout tracking, and
//! the ROAST wrapper for asynchronous liveness.
//!
//! ## Modules
//!
//! - [`dkg`] — DKG ceremony coordination with multi-round state machine.
//! - [`roast`] — ROAST wrapper for async liveness in signing.
//! - [`quorum`] — Quorum membership management and selection.
//! - [`reshare`] — Proactive secret resharing between quorums.
//!
//! ## ROAST (Robust Asynchronous Schnorr Threshold)
//!
//! ROAST wraps FROST to handle non-responsive signers by maintaining
//! multiple concurrent signing sessions and selecting the first t-of-n
//! signers that respond.

pub mod dkg;
pub mod quorum;
pub mod reshare;
pub mod roast;

/// Default timeout for a signing round in seconds.
pub const ROUND_TIMEOUT_SECS: u64 = 30;

/// Maximum concurrent ROAST sessions.
pub const MAX_ROAST_SESSIONS: usize = 8;

/// Error types for FROST coordination.
#[derive(Debug, thiserror::Error)]
pub enum FrostCoordError {
    /// The underlying FROST crypto operation failed.
    #[error("FROST crypto error: {0}")]
    Crypto(String),

    /// A signer is not registered in this ceremony.
    #[error("unknown signer: {0}")]
    UnknownSigner(String),

    /// The ceremony is in an invalid state for this operation.
    #[error("invalid ceremony state: expected {expected}, in {actual}")]
    InvalidState {
        /// Expected state.
        expected: String,
        /// Actual state.
        actual: String,
    },

    /// Timeout waiting for signer responses.
    #[error("timeout waiting for signers: {missing} of {required} missing")]
    Timeout {
        /// Number of missing signers.
        missing: usize,
        /// Number of required signers.
        required: usize,
    },

    /// Duplicate commitment or share from a signer.
    #[error("duplicate contribution from signer {0}")]
    DuplicateContribution(String),

    /// Insufficient signers responded.
    #[error("insufficient signers: need {required}, have {available}")]
    InsufficientSigners {
        /// Signers required.
        required: usize,
        /// Signers available.
        available: usize,
    },

    /// Quorum configuration error.
    #[error("quorum error: {0}")]
    Quorum(String),

    /// Resharing error.
    #[error("reshare error: {0}")]
    Reshare(String),
}

/// Convenience result type for FROST coordination.
pub type Result<T> = std::result::Result<T, FrostCoordError>;
