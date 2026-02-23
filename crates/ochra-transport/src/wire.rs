//! Wire-protocol message envelope for the Ochra P2P network.
//!
//! Every message exchanged between Ochra nodes is wrapped in a
//! [`ProtocolMessage`] envelope. The envelope is CBOR-serialized for
//! transmission over QUIC streams.
//!
//! ## Wire format
//!
//! ```text
//! ProtocolMessage {
//!     version:   u8,       // Protocol version (5)
//!     msg_type:  u16,      // Message type from registry
//!     msg_id:    [u8; 16], // Random unique message ID
//!     timestamp: u64,      // Unix timestamp (seconds)
//!     payload:   Vec<u8>,  // CBOR-encoded payload
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::cbor;
use crate::messages::TypedMessage;
use crate::TransportError;

/// Current Ochra protocol version.
pub const PROTOCOL_VERSION: u8 = 5;

/// Maximum payload size (to prevent allocation attacks).
/// Slightly less than the Sphinx packet body to leave room for overhead.
pub const MAX_PAYLOAD_SIZE: usize = 65536;

/// Protocol message envelope.
///
/// All messages exchanged between Ochra peers are wrapped in this envelope.
/// The `payload` field contains the CBOR-serialized message body, and `msg_type`
/// identifies which message struct to deserialize it as.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Protocol version. Must be [`PROTOCOL_VERSION`] (5).
    pub version: u8,
    /// Message type code from the message type registry.
    pub msg_type: u16,
    /// Random 128-bit unique message identifier.
    pub msg_id: [u8; 16],
    /// Unix timestamp in seconds when the message was created.
    pub timestamp: u64,
    /// CBOR-encoded payload bytes.
    pub payload: Vec<u8>,
}

impl ProtocolMessage {
    /// Create a new `ProtocolMessage` from a typed message payload.
    ///
    /// Generates a random `msg_id` and captures the current Unix timestamp.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Serialization`] if the payload cannot be
    /// CBOR-serialized.
    pub fn from_typed(msg: &TypedMessage) -> Result<Self, TransportError> {
        let payload = cbor::to_vec(msg)?;
        let mut msg_id = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut msg_id);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| TransportError::Internal(format!("system time error: {e}")))?
            .as_secs();

        Ok(Self {
            version: PROTOCOL_VERSION,
            msg_type: msg.msg_type(),
            msg_id,
            timestamp,
            payload,
        })
    }

    /// Create a `ProtocolMessage` with an explicit `msg_type` and pre-serialized
    /// CBOR payload.
    ///
    /// This is useful when you have already serialized the payload and know the
    /// message type code.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Internal`] if the system clock is unavailable.
    pub fn from_raw_payload(msg_type: u16, payload: Vec<u8>) -> Result<Self, TransportError> {
        let mut msg_id = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut msg_id);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| TransportError::Internal(format!("system time error: {e}")))?
            .as_secs();

        Ok(Self {
            version: PROTOCOL_VERSION,
            msg_type,
            msg_id,
            timestamp,
            payload,
        })
    }

    /// Decode the payload as a [`TypedMessage`].
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Deserialization`] if the payload is not valid CBOR
    /// or does not match the expected message schema.
    pub fn decode_payload(&self) -> Result<TypedMessage, TransportError> {
        cbor::from_slice(&self.payload)
    }

    /// Serialize this protocol message to CBOR bytes for transmission.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Serialization`] if serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, TransportError> {
        cbor::to_vec(self)
    }

    /// Deserialize a protocol message from CBOR bytes received from the wire.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::Deserialization`] if the bytes are not valid CBOR
    /// or do not match the `ProtocolMessage` schema.
    ///
    /// Returns [`TransportError::ProtocolViolation`] if the protocol version is
    /// unsupported or the payload exceeds the maximum size.
    pub fn from_bytes(data: &[u8]) -> Result<Self, TransportError> {
        let msg: Self = cbor::from_slice(data)?;
        msg.validate()?;
        Ok(msg)
    }

    /// Validate the protocol message envelope.
    ///
    /// # Errors
    ///
    /// Returns [`TransportError::ProtocolViolation`] if the version is unsupported
    /// or the payload is too large.
    pub fn validate(&self) -> Result<(), TransportError> {
        if self.version != PROTOCOL_VERSION {
            return Err(TransportError::ProtocolViolation(format!(
                "unsupported protocol version {}, expected {PROTOCOL_VERSION}",
                self.version
            )));
        }
        if self.payload.len() > MAX_PAYLOAD_SIZE {
            return Err(TransportError::ProtocolViolation(format!(
                "payload too large: {} bytes, max {MAX_PAYLOAD_SIZE}",
                self.payload.len()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{Ping, Pong, MSG_PING, MSG_PONG};

    #[test]
    fn test_from_typed_roundtrip() {
        let ping = TypedMessage::Ping(Ping {
            nonce: [1, 2, 3, 4, 5, 6, 7, 8],
        });
        let msg = ProtocolMessage::from_typed(&ping).expect("create msg");
        assert_eq!(msg.version, PROTOCOL_VERSION);
        assert_eq!(msg.msg_type, MSG_PING);
        assert!(!msg.payload.is_empty());

        let bytes = msg.to_bytes().expect("serialize");
        let restored = ProtocolMessage::from_bytes(&bytes).expect("deserialize");
        assert_eq!(restored.version, PROTOCOL_VERSION);
        assert_eq!(restored.msg_type, MSG_PING);
        assert_eq!(restored.msg_id, msg.msg_id);
    }

    #[test]
    fn test_from_raw_payload() {
        let pong = Pong {
            nonce: [10, 20, 30, 40, 50, 60, 70, 80],
        };
        let payload = cbor::to_vec(&TypedMessage::Pong(pong)).expect("serialize");
        let msg = ProtocolMessage::from_raw_payload(MSG_PONG, payload).expect("create msg");
        assert_eq!(msg.msg_type, MSG_PONG);
        assert_eq!(msg.version, PROTOCOL_VERSION);
    }

    #[test]
    fn test_invalid_version_rejected() {
        let ping = TypedMessage::Ping(Ping { nonce: [0; 8] });
        let mut msg = ProtocolMessage::from_typed(&ping).expect("create msg");
        msg.version = 99;

        let bytes = cbor::to_vec(&msg).expect("serialize");
        let result = ProtocolMessage::from_bytes(&bytes);
        assert!(result.is_err());
    }

    #[test]
    fn test_oversized_payload_rejected() {
        let msg = ProtocolMessage {
            version: PROTOCOL_VERSION,
            msg_type: 0xFFFF,
            msg_id: [0; 16],
            timestamp: 0,
            payload: vec![0u8; MAX_PAYLOAD_SIZE + 1],
        };
        assert!(msg.validate().is_err());
    }

    #[test]
    fn test_msg_id_is_random() {
        let ping = TypedMessage::Ping(Ping { nonce: [0; 8] });
        let msg1 = ProtocolMessage::from_typed(&ping).expect("create msg");
        let msg2 = ProtocolMessage::from_typed(&ping).expect("create msg");
        // It is theoretically possible but astronomically unlikely for two
        // random 128-bit IDs to collide.
        assert_ne!(msg1.msg_id, msg2.msg_id);
    }
}
