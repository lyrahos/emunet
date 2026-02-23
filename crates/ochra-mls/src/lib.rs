//! # ochra-mls
//!
//! Messaging Layer Security (MLS) group key agreement (Section 8, RFC 9420).
//!
//! Ochra uses MLS for group key management in Spaces. Each Space subgroup
//! maintains a separate MLS group with forward secrecy and post-compromise
//! security.
//!
//! ## Modules
//!
//! - [`group`] — MLS group lifecycle: create, add/remove members, encrypt/decrypt.
//! - [`ratchet`] — Double Ratchet for group key derivation using BLAKE3 KDF.
//! - [`subgroup`] — Subgroup/Channel management within a parent group.
//!
//! ## Key Concepts
//!
//! - **Group**: A set of members sharing a symmetric key schedule.
//! - **Epoch**: A version of the group state; incremented on membership changes.
//! - **KeyPackage**: A member's public key material used for group joins.
//! - **Welcome**: An encrypted message allowing a new member to join the group.

pub mod group;
pub mod ratchet;
pub mod subgroup;

/// Maximum group size per MLS group (Section 8).
pub const MAX_GROUP_SIZE: usize = 1000;

/// Error types for MLS operations.
#[derive(Debug, thiserror::Error)]
pub enum MlsError {
    /// Member already exists in the group.
    #[error("member already in group: {0}")]
    MemberExists(String),

    /// Member not found in the group.
    #[error("member not found: {0}")]
    MemberNotFound(String),

    /// Group is at maximum capacity.
    #[error("group is at maximum capacity ({max} members)")]
    GroupFull { max: usize },

    /// Group is empty.
    #[error("group is empty")]
    GroupEmpty,

    /// Invalid epoch transition.
    #[error("invalid epoch: expected {expected}, got {actual}")]
    InvalidEpoch { expected: u64, actual: u64 },

    /// Key derivation error.
    #[error("key derivation error: {0}")]
    KeyDerivation(String),

    /// Encryption or decryption error.
    #[error("encryption error: {0}")]
    Encryption(String),

    /// Subgroup error.
    #[error("subgroup error: {0}")]
    Subgroup(String),
}

/// Convenience result type for MLS operations.
pub type Result<T> = std::result::Result<T, MlsError>;
