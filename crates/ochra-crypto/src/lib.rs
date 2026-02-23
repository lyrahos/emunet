//! # ochra-crypto
//!
//! Cryptographic primitives for the Ochra protocol.
//!
//! This crate implements all cryptographic operations required by the Ochra v5.5
//! Unified Technical Specification. No algorithm negotiation is permitted — the
//! cryptographic suite is fixed.
//!
//! ## Modules
//!
//! - [`blake3`] — Domain-separated BLAKE3 hashing (all 36 context strings)
//! - [`ed25519`] — Ed25519 signing and verification (RFC 8032)
//! - [`x25519`] — X25519 key agreement (RFC 7748)
//! - [`chacha20`] — ChaCha20-Poly1305 AEAD encryption (RFC 8439)
//! - [`argon2id`] — Password hashing and Proof-of-Work
//! - [`ecies`] — ECIES encrypt/decrypt (Section 2.5)
//! - [`poseidon`] — Poseidon hash on BLS12-381 scalar field
//! - [`groth16`] — Groth16/BLS12-381 proving and verification
//! - [`pedersen`] — Pedersen commitments on BLS12-381
//! - [`voprf`] — Ristretto255 VOPRF (RFC 9497)
//! - [`frost`] — FROST Ed25519 DKG + ROAST wrapper

pub mod argon2id;
pub mod blake3;
pub mod chacha20;
pub mod ecies;
pub mod ed25519;
pub mod frost;
pub mod groth16;
pub mod pedersen;
pub mod poseidon;
pub mod voprf;
pub mod x25519;

/// Error types for cryptographic operations.
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    /// Ed25519 signature verification failed.
    #[error("signature verification failed")]
    SignatureVerification,

    /// AEAD decryption failed (authentication tag mismatch).
    #[error("AEAD decryption failed")]
    AeadDecryption,

    /// Key derivation failed.
    #[error("key derivation failed: {0}")]
    KeyDerivation(String),

    /// Invalid key length.
    #[error("invalid key length: expected {expected}, got {actual}")]
    InvalidKeyLength { expected: usize, actual: usize },

    /// Argon2id hashing failed.
    #[error("argon2id error: {0}")]
    Argon2(String),

    /// Groth16 proof generation or verification failed.
    #[error("proof error: {0}")]
    Proof(String),

    /// VOPRF error.
    #[error("VOPRF error: {0}")]
    Voprf(String),

    /// FROST threshold signature error.
    #[error("FROST error: {0}")]
    Frost(String),

    /// ECIES encryption/decryption failed.
    #[error("ECIES error: {0}")]
    Ecies(String),

    /// Invalid input data.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, CryptoError>;
