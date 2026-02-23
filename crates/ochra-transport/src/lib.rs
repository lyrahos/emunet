//! # ochra-transport
//!
//! Encrypted network transport for the Ochra P2P network.
//!
//! This crate provides the complete transport layer for the Ochra protocol v5,
//! including:
//!
//! - **QUIC/TLS 1.3** connection management via [`quic`]
//! - **Sphinx packets** for sender-anonymous 3-hop onion routing via [`sphinx`]
//! - **Wire protocol** message envelope (CBOR-serialized) via [`wire`]
//! - **CBOR serialization** helpers via [`cbor`]
//! - **Message types** for all protocol message payloads via [`messages`]
//!
//! ## Architecture
//!
//! ```text
//! Application
//!     |
//!     v
//! ProtocolMessage (wire.rs)  -- CBOR envelope with version, type, payload
//!     |
//!     v
//! SphinxPacket (sphinx.rs)   -- 8192-byte fixed-size onion-routed packet
//!     |
//!     v
//! QuicNode (quic.rs)         -- QUIC/TLS 1.3 bidirectional streams
//!     |
//!     v
//! UDP socket
//! ```

pub mod cbor;
pub mod messages;
pub mod quic;
pub mod sphinx;
pub mod wire;

/// Error types for transport operations.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// CBOR serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// CBOR deserialization error.
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// Protocol violation (version mismatch, oversized payload, etc.).
    #[error("protocol violation: {0}")]
    ProtocolViolation(String),

    /// Invalid or malformed Sphinx packet.
    #[error("invalid packet: {0}")]
    InvalidPacket(String),

    /// MAC verification failed on a Sphinx header.
    #[error("MAC verification failed")]
    MacVerification,

    /// Cryptographic operation failed.
    #[error("crypto error: {0}")]
    Crypto(String),

    /// TLS/certificate error.
    #[error("TLS error: {0}")]
    Tls(String),

    /// QUIC connection error.
    #[error("connection error: {0}")]
    Connection(String),

    /// I/O error (socket, stream read/write).
    #[error("I/O error: {0}")]
    Io(String),

    /// Internal error (should not occur in normal operation).
    #[error("internal error: {0}")]
    Internal(String),
}

/// Result type alias for transport operations.
pub type Result<T> = std::result::Result<T, TransportError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TransportError::Serialization("test".to_string());
        assert_eq!(err.to_string(), "serialization error: test");
    }

    #[test]
    fn test_error_variants() {
        let _e1 = TransportError::Serialization("s".into());
        let _e2 = TransportError::Deserialization("d".into());
        let _e3 = TransportError::ProtocolViolation("p".into());
        let _e4 = TransportError::InvalidPacket("i".into());
        let _e5 = TransportError::MacVerification;
        let _e6 = TransportError::Crypto("c".into());
        let _e7 = TransportError::Tls("t".into());
        let _e8 = TransportError::Connection("conn".into());
        let _e9 = TransportError::Io("io".into());
        let _e10 = TransportError::Internal("int".into());
    }
}
