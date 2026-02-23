//! # ochra-onion
//!
//! Onion-routed message relay for the Ochra P2P network.
//!
//! This crate implements Sphinx-based onion routing with 3-hop circuits:
//!
//! - [`circuit`] - Circuit construction, hop key derivation, and rotation
//! - [`relay`] - Relay selection with PoSrv-weighted random sampling
//! - [`cover`] - Cover traffic generation using Poisson timing
//! - [`nat`] - NAT traversal helpers
//!
//! ## Key Parameters
//!
//! | Parameter | Value |
//! |---|---|
//! | Sphinx packet size | 8192 bytes |
//! | Circuit hops | 3 |
//! | Circuit lifetime | 10 minutes |
//! | Key derivation contexts | `Ochra v1 sphinx-hop-{key,mac,pad,nonce}` |

pub mod circuit;
pub mod cover;
pub mod nat;
pub mod relay;

/// Sphinx packet size in bytes (matches `ochra_types::SPHINX_PACKET_SIZE`).
pub const SPHINX_PACKET_SIZE: usize = 8192;

/// Number of hops in a Sphinx circuit.
pub const CIRCUIT_HOPS: usize = 3;

/// Circuit lifetime in seconds (10 minutes).
pub const CIRCUIT_LIFETIME_SECS: u64 = 600;

/// Error types for onion routing operations.
#[derive(Debug, thiserror::Error)]
pub enum OnionError {
    /// Not enough relays available to construct a circuit.
    #[error("insufficient relays: need {need}, have {have}")]
    InsufficientRelays { need: usize, have: usize },

    /// A relay was rejected due to subnet/AS constraints.
    #[error("relay constraint violation: {0}")]
    ConstraintViolation(String),

    /// Circuit construction failed.
    #[error("circuit construction failed: {0}")]
    CircuitConstruction(String),

    /// Circuit has expired and must be rotated.
    #[error("circuit expired")]
    CircuitExpired,

    /// Key derivation failed.
    #[error("key derivation error: {0}")]
    KeyDerivation(String),

    /// Sphinx packet processing error.
    #[error("sphinx error: {0}")]
    Sphinx(String),

    /// NAT traversal failed.
    #[error("NAT traversal failed: {0}")]
    NatTraversal(String),

    /// Cryptographic error from ochra-crypto.
    #[error("crypto error: {0}")]
    Crypto(#[from] ochra_crypto::CryptoError),

    /// Network error.
    #[error("network error: {0}")]
    Network(String),
}

/// Convenience result type for onion routing operations.
pub type Result<T> = std::result::Result<T, OnionError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(SPHINX_PACKET_SIZE, 8192);
        assert_eq!(CIRCUIT_HOPS, 3);
        assert_eq!(CIRCUIT_LIFETIME_SECS, 600);
    }

    #[test]
    fn test_error_display() {
        let err = OnionError::InsufficientRelays { need: 3, have: 1 };
        assert!(err.to_string().contains("need 3"));
        assert!(err.to_string().contains("have 1"));
    }
}
