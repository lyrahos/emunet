//! # ochra-oracle
//!
//! MPC TLS Oracle and price feed system (Section 13).
//!
//! The oracle system provides price data for the Ochra network. A Time-Weighted
//! Average Price (TWAP) is computed over a configurable window and used to adjust
//! collateral ratios for Seed minting.
//!
//! ## Modules
//!
//! - [`twap`] — TWAP (Time-Weighted Average Price) calculation
//! - [`denomination`] — Denomination formula (Section 11.9)
//! - [`circuit_breaker`] — Circuit breaker and emergency pause
//! - [`stub`] — Hardcoded rate oracle for v1

pub mod circuit_breaker;
pub mod denomination;
pub mod stub;
pub mod twap;

/// Error types for oracle operations.
#[derive(Debug, thiserror::Error)]
pub enum OracleError {
    /// Insufficient observations for TWAP computation.
    #[error("insufficient observations: need {required}, have {available}")]
    InsufficientObservations {
        /// Number of observations required.
        required: usize,
        /// Number of observations available.
        available: usize,
    },

    /// Price is zero or negative.
    #[error("invalid price: {0}")]
    InvalidPrice(u64),

    /// Observation timestamp is not monotonically increasing.
    #[error("non-monotonic timestamp: {new} <= {last}")]
    NonMonotonicTimestamp {
        /// The new timestamp that violated monotonicity.
        new: u64,
        /// The last accepted timestamp.
        last: u64,
    },

    /// The TWAP window is empty (no observations in range).
    #[error("no observations in TWAP window")]
    EmptyWindow,

    /// Oracle data is stale beyond the staleness threshold.
    #[error(
        "oracle data is stale: last update {last_update}, current {current}, threshold {threshold}"
    )]
    StaleData {
        /// Timestamp of the last update.
        last_update: u64,
        /// Current timestamp.
        current: u64,
        /// Staleness threshold in seconds.
        threshold: u64,
    },

    /// The oracle is paused via the circuit breaker.
    #[error("oracle is paused")]
    Paused,

    /// Invalid denomination parameters.
    #[error("invalid denomination: {0}")]
    InvalidDenomination(String),
}

/// Convenience result type for oracle operations.
pub type Result<T> = std::result::Result<T, OracleError>;
