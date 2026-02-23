//! # ochra-posrv
//!
//! Proof-of-Service (PoSrv) composite score calculation (Section 9).
//!
//! Every relay in the Ochra network is scored by a composite metric that
//! determines its eligibility for the minting quorum.
//!
//! ## Modules
//!
//! - [`scoring`] — PoSrv scoring formula with sigmoid normalization.
//! - [`sybilguard`] — SybilGuard trust graph for random-walk-based Sybil resistance.

pub mod scoring;
pub mod sybilguard;

/// Error types for PoSrv operations.
#[derive(Debug, thiserror::Error)]
pub enum PoSrvError {
    /// A score component is out of range.
    #[error("score component '{name}' out of range [0,1]: {value}")]
    OutOfRange { name: &'static str, value: f64 },

    /// Insufficient data to compute a score.
    #[error("insufficient data: need at least {required} epochs, have {available}")]
    InsufficientData { required: u32, available: u32 },

    /// Node not found in the trust graph.
    #[error("node not found: {0}")]
    NodeNotFound(String),

    /// Invalid graph operation.
    #[error("graph error: {0}")]
    GraphError(String),
}

/// Convenience result type for PoSrv operations.
pub type Result<T> = std::result::Result<T, PoSrvError>;
