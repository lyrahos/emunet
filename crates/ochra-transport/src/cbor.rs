//! CBOR serialization helpers for the Ochra wire protocol.
//!
//! This module wraps [`ciborium`] to provide convenient serialization and
//! deserialization of protocol payloads to/from CBOR (RFC 8949). All message
//! payloads in the Ochra protocol are CBOR-encoded before being placed into
//! the [`ProtocolMessage`](crate::wire::ProtocolMessage) envelope.

use serde::{de::DeserializeOwned, Serialize};

use crate::TransportError;

/// Serialize a value to CBOR bytes.
///
/// # Errors
///
/// Returns [`TransportError::Serialization`] if the value cannot be serialized.
pub fn to_vec<T: Serialize>(value: &T) -> Result<Vec<u8>, TransportError> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).map_err(|e| {
        TransportError::Serialization(format!("CBOR serialization failed: {e}"))
    })?;
    Ok(buf)
}

/// Deserialize a value from CBOR bytes.
///
/// # Errors
///
/// Returns [`TransportError::Deserialization`] if the bytes cannot be deserialized
/// into the target type.
pub fn from_slice<T: DeserializeOwned>(data: &[u8]) -> Result<T, TransportError> {
    ciborium::from_reader(data).map_err(|e| {
        TransportError::Deserialization(format!("CBOR deserialization failed: {e}"))
    })
}

/// Serialize a value to CBOR bytes, returning an error with context.
///
/// This is a convenience wrapper that includes the type name in the error message
/// for easier debugging.
pub fn to_vec_named<T: Serialize>(value: &T, type_name: &str) -> Result<Vec<u8>, TransportError> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf).map_err(|e| {
        TransportError::Serialization(format!(
            "CBOR serialization of {type_name} failed: {e}"
        ))
    })?;
    Ok(buf)
}

/// Deserialize a value from CBOR bytes, returning an error with context.
///
/// This is a convenience wrapper that includes the type name in the error message
/// for easier debugging.
pub fn from_slice_named<T: DeserializeOwned>(
    data: &[u8],
    type_name: &str,
) -> Result<T, TransportError> {
    ciborium::from_reader(data).map_err(|e| {
        TransportError::Deserialization(format!(
            "CBOR deserialization of {type_name} failed: {e}"
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::Ping;

    #[test]
    fn test_roundtrip_ping() {
        let ping = Ping {
            nonce: [1, 2, 3, 4, 5, 6, 7, 8],
        };
        let bytes = to_vec(&ping).expect("serialize");
        let restored: Ping = from_slice(&bytes).expect("deserialize");
        assert_eq!(ping.nonce, restored.nonce);
    }

    #[test]
    fn test_roundtrip_named() {
        let ping = Ping {
            nonce: [10, 20, 30, 40, 50, 60, 70, 80],
        };
        let bytes = to_vec_named(&ping, "Ping").expect("serialize");
        let restored: Ping = from_slice_named(&bytes, "Ping").expect("deserialize");
        assert_eq!(ping.nonce, restored.nonce);
    }

    #[test]
    fn test_invalid_data_returns_error() {
        let bad_data = &[0xFF, 0xFF, 0xFF];
        let result: Result<Ping, _> = from_slice(bad_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_data_returns_error() {
        let result: Result<Ping, _> = from_slice(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cbor_is_compact() {
        let ping = Ping {
            nonce: [0; 8],
        };
        let cbor = to_vec(&ping).expect("serialize");
        let json = serde_json::to_vec(&ping).expect("serialize json");
        // CBOR should generally be more compact than JSON
        assert!(cbor.len() <= json.len());
    }
}
