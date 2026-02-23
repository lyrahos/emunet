//! # ochra-nullifier
//!
//! Double-spend prevention via nullifier tracking (Section 10.4).
//!
//! Nullifiers are deterministic identifiers derived from spent tokens. The
//! network maintains a Bloom filter of all seen nullifiers so that double-spend
//! attempts can be detected without revealing which token was spent.
//!
//! ## Modules
//!
//! - [`bloom`] — Bloom filter nullifier set
//! - [`gossip`] — Nullifier gossip protocol
//! - [`refund`] — Refund commitment tree

pub mod bloom;
pub mod gossip;
pub mod refund;

/// A nullifier value (32-byte hash).
pub type Nullifier = [u8; 32];

/// Error types for nullifier operations.
#[derive(Debug, thiserror::Error)]
pub enum NullifierError {
    /// The nullifier has already been seen (potential double-spend).
    #[error("nullifier already exists (double-spend detected)")]
    DoubleSpend,

    /// The Bloom filter is at capacity.
    #[error("bloom filter is at capacity ({count} entries, max recommended {max})")]
    AtCapacity {
        /// Current number of entries.
        count: usize,
        /// Maximum recommended entries.
        max: usize,
    },

    /// Invalid gossip message.
    #[error("invalid gossip message: {0}")]
    InvalidGossip(String),

    /// Refund tree error.
    #[error("refund tree error: {0}")]
    RefundError(String),
}

/// Convenience result type for nullifier operations.
pub type Result<T> = std::result::Result<T, NullifierError>;

/// Derive a nullifier from a token serial and spend secret.
///
/// `nullifier = BLAKE3::hash(serial || spend_secret)`
pub fn derive_nullifier(serial: &[u8; 32], spend_secret: &[u8; 32]) -> Nullifier {
    let mut input = [0u8; 64];
    input[..32].copy_from_slice(serial);
    input[32..].copy_from_slice(spend_secret);
    ochra_crypto::blake3::hash(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_nullifier_deterministic() {
        let serial = [0xAAu8; 32];
        let secret = [0xBBu8; 32];
        let n1 = derive_nullifier(&serial, &secret);
        let n2 = derive_nullifier(&serial, &secret);
        assert_eq!(n1, n2);
    }

    #[test]
    fn test_derive_nullifier_different_inputs() {
        let n1 = derive_nullifier(&[0x01; 32], &[0x02; 32]);
        let n2 = derive_nullifier(&[0x03; 32], &[0x04; 32]);
        assert_ne!(n1, n2);
    }
}
