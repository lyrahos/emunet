//! # ochra-dht
//!
//! Kademlia-based distributed hash table for the Ochra P2P network.
//!
//! This crate implements:
//! - Kademlia routing table with XOR-distance metric (K=20, alpha=3, 256 buckets)
//! - BEP 44 mutable and immutable record storage with signature validation
//! - Multi-record chunking for payloads exceeding the 1000-byte DHT record limit
//! - Bootstrap logic for joining the network via seed nodes
//!
//! ## Key Parameters
//!
//! | Parameter | Value |
//! |---|---|
//! | K (bucket size) | 20 |
//! | alpha (lookup parallelism) | 3 |
//! | beta (refresh interval) | 1 hour |
//! | Replication factor | 8 |
//! | Max record size | 1000 bytes |
//! | Ping timeout | 5 seconds |
//! | Node ID derivation | `BLAKE3::hash(pik_public_key)[:32]` |

pub mod bep44;
pub mod bootstrap;
pub mod chunking;
pub mod kademlia;

/// Kademlia bucket size: maximum contacts per bucket.
pub const K: usize = 20;

/// Lookup parallelism factor.
pub const ALPHA: usize = 3;

/// Bucket refresh interval in seconds (1 hour).
pub const REFRESH_INTERVAL_SECS: u64 = 3600;

/// Record replication factor: number of nodes to store a record on.
pub const REPLICATION_FACTOR: usize = 8;

/// Maximum size of a single DHT record value in bytes.
pub const MAX_RECORD_SIZE: usize = 1000;

/// Ping timeout in seconds.
pub const PING_TIMEOUT_SECS: u64 = 5;

/// Number of buckets in the routing table (one per bit of the 256-bit key space).
pub const NUM_BUCKETS: usize = 256;

/// Error types for DHT operations.
#[derive(Debug, thiserror::Error)]
pub enum DhtError {
    /// The record exceeds the maximum allowed size.
    #[error("record too large: {size} bytes exceeds maximum of {max} bytes")]
    RecordTooLarge { size: usize, max: usize },

    /// The record's signature is invalid.
    #[error("invalid record signature")]
    InvalidSignature,

    /// The record's sequence number is stale (a newer version exists).
    #[error("stale sequence number: got {got}, have {have}")]
    StaleSequence { got: u64, have: u64 },

    /// The requested record was not found.
    #[error("record not found")]
    NotFound {
        /// The key that was not found.
        key: [u8; 32],
    },

    /// A chunk is missing during reassembly.
    #[error("missing chunk {index} of {total}")]
    MissingChunk { index: u32, total: u32 },

    /// The routing table bucket is full and all entries are still alive.
    #[error("bucket full")]
    BucketFull,

    /// Bootstrap failed to discover any peers.
    #[error("bootstrap failed: {0}")]
    BootstrapFailed(String),

    /// Network or I/O error.
    #[error("network error: {0}")]
    Network(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Cryptographic error from ochra-crypto.
    #[error("crypto error: {0}")]
    Crypto(#[from] ochra_crypto::CryptoError),
}

/// Convenience result type for DHT operations.
pub type Result<T> = std::result::Result<T, DhtError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(K, 20);
        assert_eq!(ALPHA, 3);
        assert_eq!(REFRESH_INTERVAL_SECS, 3600);
        assert_eq!(REPLICATION_FACTOR, 8);
        assert_eq!(MAX_RECORD_SIZE, 1000);
        assert_eq!(PING_TIMEOUT_SECS, 5);
        assert_eq!(NUM_BUCKETS, 256);
    }

    #[test]
    fn test_error_display() {
        let err = DhtError::RecordTooLarge {
            size: 2000,
            max: 1000,
        };
        assert!(err.to_string().contains("2000"));
        assert!(err.to_string().contains("1000"));
    }
}
