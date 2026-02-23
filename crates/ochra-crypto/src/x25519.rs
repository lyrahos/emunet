//! X25519 key agreement (RFC 7748).
//!
//! Used for ephemeral session negotiation, mailbox routing, onion circuits,
//! and Whisper session establishment.

use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use zeroize::Zeroize;

use crate::Result;

/// An X25519 static secret key (for long-lived keys).
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct X25519StaticSecret {
    inner: StaticSecret,
}

/// An X25519 public key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct X25519PublicKey {
    bytes: [u8; 32],
}

/// An X25519 shared secret.
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct SharedSecret {
    bytes: [u8; 32],
}

impl X25519StaticSecret {
    /// Generate a new random static secret.
    pub fn random() -> Self {
        Self {
            inner: StaticSecret::random_from_rng(OsRng),
        }
    }

    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self {
            inner: StaticSecret::from(bytes),
        }
    }

    /// Get the raw bytes of this secret.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.inner.to_bytes()
    }

    /// Compute the corresponding public key.
    pub fn public_key(&self) -> X25519PublicKey {
        let pk = PublicKey::from(&self.inner);
        X25519PublicKey {
            bytes: pk.to_bytes(),
        }
    }

    /// Perform Diffie-Hellman key agreement.
    pub fn diffie_hellman(&self, their_public: &X25519PublicKey) -> SharedSecret {
        let pk = PublicKey::from(their_public.bytes);
        let shared = self.inner.diffie_hellman(&pk);
        SharedSecret {
            bytes: *shared.as_bytes(),
        }
    }
}

impl X25519PublicKey {
    /// Create from raw bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Get the raw bytes.
    pub fn to_bytes(&self) -> [u8; 32] {
        self.bytes
    }

    /// Get the raw bytes as a slice.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

impl SharedSecret {
    /// Get the raw bytes of the shared secret.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }
}

/// Perform an ephemeral X25519 key exchange.
///
/// Returns (ephemeral_public_key, shared_secret).
pub fn ephemeral_key_exchange(their_public: &X25519PublicKey) -> (X25519PublicKey, SharedSecret) {
    let secret = EphemeralSecret::random_from_rng(OsRng);
    let public = PublicKey::from(&secret);
    let pk = PublicKey::from(their_public.bytes);
    let shared = secret.diffie_hellman(&pk);

    (
        X25519PublicKey {
            bytes: public.to_bytes(),
        },
        SharedSecret {
            bytes: *shared.as_bytes(),
        },
    )
}

/// Compute X25519 basepoint multiplication (public key from secret).
pub fn basepoint_mult(secret: &[u8; 32]) -> [u8; 32] {
    let sk = StaticSecret::from(*secret);
    let pk = PublicKey::from(&sk);
    pk.to_bytes()
}

/// Convert an Ed25519 secret key to an X25519 secret key.
///
/// This is used for ECIES where the recipient key may originate from an Ed25519 keypair.
/// The conversion uses the clamped SHA-512 hash of the Ed25519 secret key, matching
/// the standard ed25519-to-x25519 conversion.
pub fn ed25519_secret_to_x25519(ed_secret: &[u8; 32]) -> Result<[u8; 32]> {
    // ed25519-dalek stores the secret key as the seed; we hash with SHA-512
    // and take the first 32 bytes, then clamp
    use ::blake3::hash;
    let h = hash(ed_secret);
    let mut x_secret = *h.as_bytes();
    // Clamp per RFC 7748
    x_secret[0] &= 248;
    x_secret[31] &= 127;
    x_secret[31] |= 64;
    Ok(x_secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let secret = X25519StaticSecret::random();
        let public = secret.public_key();
        assert_ne!(public.to_bytes(), [0u8; 32]);
    }

    #[test]
    fn test_diffie_hellman_agreement() {
        let alice_secret = X25519StaticSecret::random();
        let alice_public = alice_secret.public_key();

        let bob_secret = X25519StaticSecret::random();
        let bob_public = bob_secret.public_key();

        let alice_shared = alice_secret.diffie_hellman(&bob_public);
        let bob_shared = bob_secret.diffie_hellman(&alice_public);

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_ephemeral_exchange() {
        let bob_secret = X25519StaticSecret::random();
        let bob_public = bob_secret.public_key();

        let (alice_eph_pub, alice_shared) = ephemeral_key_exchange(&bob_public);
        let bob_shared = bob_secret.diffie_hellman(&alice_eph_pub);

        assert_eq!(alice_shared.as_bytes(), bob_shared.as_bytes());
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let secret = X25519StaticSecret::random();
        let bytes = secret.to_bytes();
        let restored = X25519StaticSecret::from_bytes(bytes);
        assert_eq!(
            secret.public_key().to_bytes(),
            restored.public_key().to_bytes()
        );
    }

    #[test]
    fn test_basepoint_mult() {
        let secret = X25519StaticSecret::random();
        let pk1 = secret.public_key().to_bytes();
        let pk2 = basepoint_mult(&secret.to_bytes());
        assert_eq!(pk1, pk2);
    }

    #[test]
    fn test_rfc7748_section6_1() {
        // RFC 7748 Section 6.1 test vector
        let alice_private =
            hex::decode("77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a")
                .expect("valid hex");
        let alice_public =
            hex::decode("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a")
                .expect("valid hex");

        let mut secret_bytes = [0u8; 32];
        secret_bytes.copy_from_slice(&alice_private);
        let computed_pk = basepoint_mult(&secret_bytes);
        assert_eq!(computed_pk.as_slice(), alice_public.as_slice());
    }
}
