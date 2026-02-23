//! # ochra-storage
//!
//! Encrypted local and remote storage for the Ochra P2P network.
//!
//! This crate implements content chunking with Merkle trees, Reed-Solomon
//! erasure coding, ABR (Always-Be-Relaying) chunk lifecycle management
//! with LFU-DA eviction, and configurable storage earning levels.
//!
//! ## Modules
//!
//! - [`chunker`] — 4 MB chunk splitting with Merkle tree verification.
//! - [`reed_solomon`] — Reed-Solomon k=4, n=8 erasure coding.
//! - [`abr`] — ABR store with LFU-DA eviction policy.
//! - [`earning`] — Storage earning level configuration.

pub mod abr;
pub mod chunker;
pub mod earning;
pub mod reed_solomon;

/// Error types for storage operations.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Content data is empty.
    #[error("content data is empty")]
    EmptyContent,

    /// Chunk not found in store.
    #[error("chunk not found: {0}")]
    ChunkNotFound(String),

    /// Merkle proof verification failed.
    #[error("merkle proof verification failed")]
    MerkleVerification,

    /// Reed-Solomon encoding error.
    #[error("reed-solomon encoding error: {0}")]
    ReedSolomonEncode(String),

    /// Reed-Solomon decoding error (insufficient shards).
    #[error("reed-solomon decoding error: {0}")]
    ReedSolomonDecode(String),

    /// Storage allocation exceeded.
    #[error("storage allocation exceeded: used {used} of {limit} bytes")]
    AllocationExceeded { used: u64, limit: u64 },

    /// I/O error during storage operations.
    #[error("I/O error: {0}")]
    Io(String),

    /// Shard index out of range.
    #[error("shard index out of range: {index}, max {max}")]
    ShardIndexOutOfRange { index: usize, max: usize },
}

/// Convenience result type for storage operations.
pub type Result<T> = std::result::Result<T, StorageError>;
