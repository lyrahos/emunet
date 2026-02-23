//! Ed25519 signing and verification (RFC 8032).
//!
//! Ed25519 is the root asymmetric signature algorithm for Ochra. It is used for:
//! - PIK (Platform Identity Key) generation and signing
//! - Receipt signing and manifest authorization
//! - Handle signing
//!
//! This module wraps `ed25519-dalek` with Ochra-specific types.

use ed25519_dalek::{Signer, Verifier};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use crate::{CryptoError, Result};

/// An Ed25519 signing key (private key).
pub struct SigningKey {
    inner: ed25519_dalek::SigningKey,
}

impl Clone for SigningKey {
    fn clone(&self) -> Self {
        Self {
            inner: ed25519_dalek::SigningKey::from_bytes(&self.inner.to_bytes()),
        }
    }
}

impl Drop for SigningKey {
    fn drop(&mut self) {
        let mut bytes = self.inner.to_bytes();
        bytes.zeroize();
    }
}

/// An Ed25519 verification key (public key).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyingKey {
    inner: ed25519_dalek::VerifyingKey,
}

/// An Ed25519 signature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature {
    inner: ed25519_dalek::Signature,
}

/// An Ed25519 keypair for PIK or other signing operations.
pub struct KeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

impl SigningKey {
    /// Generate a new random signing key.
    pub fn generate() -> Self {
        let mut csprng = rand::rngs::OsRng;
        Self {
            inner: ed25519_dalek::SigningKey::generate(&mut csprng),
        }
    }

    /// Create a signing key from raw bytes.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self {
            inner: ed25519_dalek::SigningKey::from_bytes(bytes),
        }
    }

    /// Get the raw bytes of this signing key.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// Get the corresponding verifying key.
    pub fn verifying_key(&self) -> VerifyingKey {
        VerifyingKey {
            inner: self.inner.verifying_key(),
        }
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Signature {
        Signature {
            inner: self.inner.sign(message),
        }
    }
}

impl VerifyingKey {
    /// Create a verifying key from raw bytes.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self> {
        let inner = ed25519_dalek::VerifyingKey::from_bytes(bytes)
            .map_err(|e| CryptoError::InvalidInput(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Get the raw bytes of this verifying key.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// Get the raw bytes as a slice.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.inner.as_bytes()
    }

    /// Verify a signature on a message.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<()> {
        self.inner
            .verify(message, &signature.inner)
            .map_err(|_| CryptoError::SignatureVerification)
    }
}

impl Signature {
    /// Create a signature from raw bytes.
    pub fn from_bytes(bytes: &[u8; 64]) -> Self {
        Self {
            inner: ed25519_dalek::Signature::from_bytes(bytes),
        }
    }

    /// Get the raw bytes of this signature.
    pub fn to_bytes(&self) -> [u8; 64] {
        self.inner.to_bytes()
    }
}

impl KeyPair {
    /// Generate a new random Ed25519 keypair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate();
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }

    /// Create a keypair from a signing key's raw bytes.
    pub fn from_bytes(secret: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(secret);
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }
}

/// Derive a node ID from a PIK public key.
///
/// `node_id = BLAKE3::hash(pik_public_key)[:32]`
pub fn derive_node_id(pik_public_key: &VerifyingKey) -> [u8; 32] {
    crate::blake3::hash(pik_public_key.as_bytes())
}

impl std::fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningKey")
            .field("public", &self.verifying_key())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = KeyPair::generate();
        let msg = b"test message";
        let sig = kp.signing_key.sign(msg);
        assert!(kp.verifying_key.verify(msg, &sig).is_ok());
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let kp = KeyPair::generate();
        let msg = b"Ochra protocol test";
        let sig = kp.signing_key.sign(msg);
        assert!(kp.verifying_key.verify(msg, &sig).is_ok());
    }

    #[test]
    fn test_wrong_message_fails() {
        let kp = KeyPair::generate();
        let sig = kp.signing_key.sign(b"correct message");
        assert!(kp.verifying_key.verify(b"wrong message", &sig).is_err());
    }

    #[test]
    fn test_wrong_key_fails() {
        let kp1 = KeyPair::generate();
        let kp2 = KeyPair::generate();
        let sig = kp1.signing_key.sign(b"test");
        assert!(kp2.verifying_key.verify(b"test", &sig).is_err());
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let kp = KeyPair::generate();
        let secret_bytes = kp.signing_key.to_bytes();
        let restored = KeyPair::from_bytes(&secret_bytes);
        assert_eq!(
            kp.verifying_key.to_bytes(),
            restored.verifying_key.to_bytes()
        );
    }

    #[test]
    fn test_signature_serialization() {
        let kp = KeyPair::generate();
        let sig = kp.signing_key.sign(b"test");
        let bytes = sig.to_bytes();
        let restored = Signature::from_bytes(&bytes);
        assert_eq!(sig, restored);
    }

    #[test]
    fn test_verifying_key_serialization() {
        let kp = KeyPair::generate();
        let bytes = kp.verifying_key.to_bytes();
        let restored = VerifyingKey::from_bytes(&bytes).expect("valid key");
        assert_eq!(kp.verifying_key, restored);
    }

    #[test]
    fn test_node_id_derivation() {
        let kp = KeyPair::generate();
        let node_id = derive_node_id(&kp.verifying_key);
        // Should be deterministic
        let node_id2 = derive_node_id(&kp.verifying_key);
        assert_eq!(node_id, node_id2);
        // Should be a BLAKE3 hash of the public key
        assert_eq!(node_id, crate::blake3::hash(kp.verifying_key.as_bytes()));
    }

    #[test]
    fn test_deterministic_key_derivation() {
        // Verify that the same seed always produces the same keypair
        let seed = [42u8; 32];
        let kp1 = KeyPair::from_bytes(&seed);
        let kp2 = KeyPair::from_bytes(&seed);
        assert_eq!(kp1.verifying_key.to_bytes(), kp2.verifying_key.to_bytes());

        // And that different seeds produce different keys
        let kp3 = KeyPair::from_bytes(&[43u8; 32]);
        assert_ne!(kp1.verifying_key.to_bytes(), kp3.verifying_key.to_bytes());
    }

    #[test]
    fn test_sign_verify_with_known_seed() {
        // Use a known seed, sign, and verify roundtrip
        let seed = hex::decode(
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        )
        .expect("valid hex");
        let mut seed_bytes = [0u8; 32];
        seed_bytes.copy_from_slice(&seed);
        let kp = KeyPair::from_bytes(&seed_bytes);

        // Public key should be 32 bytes and non-zero
        assert_eq!(kp.verifying_key.to_bytes().len(), 32);
        assert_ne!(kp.verifying_key.to_bytes(), [0u8; 32]);

        // Sign and verify empty message
        let sig = kp.signing_key.sign(b"");
        assert!(kp.verifying_key.verify(b"", &sig).is_ok());

        // Sign and verify non-empty message
        let sig2 = kp.signing_key.sign(b"test message");
        assert!(kp.verifying_key.verify(b"test message", &sig2).is_ok());
    }
}
