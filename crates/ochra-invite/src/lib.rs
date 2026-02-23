//! # ochra-invite
//!
//! Invite creation, redemption, and contact exchange for the Ochra network.
//!
//! This crate implements:
//!
//! - [`invite`] - Invite link creation and parsing (`ochra://invite` URLs)
//! - [`contact_exchange`] - Contact exchange token system for bidirectional contacts
//! - [`rendezvous`] - Anonymous rendezvous protocol for introduction points
//!
//! ## Invite Flow
//!
//! 1. Inviter creates an `InvitePayload` containing bootstrap relay info.
//! 2. Payload is encrypted with a one-time key and published to a DHT
//!    rendezvous address derived from the invite descriptor.
//! 3. Invitee scans the invite code (containing the descriptor and secret).
//! 4. Invitee derives the rendezvous address, fetches and decrypts the payload.
//! 5. Invitee uses the bootstrap relays to connect to the network.

pub mod contact_exchange;
pub mod invite;
pub mod rendezvous;

use ochra_crypto::blake3::{self, contexts};
use ochra_crypto::chacha20;
use serde::{Deserialize, Serialize};

/// Error types for invite operations.
#[derive(Debug, thiserror::Error)]
pub enum InviteError {
    /// Encryption or decryption failed.
    #[error("crypto error: {0}")]
    Crypto(String),

    /// The invite has expired.
    #[error("invite expired at epoch {expired_at}, current epoch {current_epoch}")]
    Expired {
        /// The epoch at which the invite expired.
        expired_at: u64,
        /// The current epoch.
        current_epoch: u64,
    },

    /// The invite payload is malformed.
    #[error("malformed invite: {0}")]
    Malformed(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Signature verification failed.
    #[error("invalid signature")]
    InvalidSignature,

    /// The invite link URL is invalid.
    #[error("invalid invite URL: {0}")]
    InvalidUrl(String),

    /// The invite has exceeded its maximum number of uses.
    #[error("invite max uses exceeded: {used} of {max}")]
    MaxUsesExceeded {
        /// Number of times the invite has been used.
        used: u32,
        /// Maximum allowed uses.
        max: u32,
    },

    /// The invite TTL has expired.
    #[error("invite TTL expired")]
    TtlExpired,

    /// Token validation failed.
    #[error("invalid token: {0}")]
    InvalidToken(String),

    /// Cryptographic error from ochra-crypto.
    #[error("cryptographic error: {0}")]
    CryptoLib(#[from] ochra_crypto::CryptoError),
}

/// Convenience result type for invite operations.
pub type Result<T> = std::result::Result<T, InviteError>;

/// Bootstrap relay information included in an invite.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapRelay {
    /// Node ID of the relay.
    pub node_id: [u8; 32],
    /// X25519 public key of the relay.
    pub x25519_pk: [u8; 32],
    /// Socket address (e.g., "1.2.3.4:4433").
    pub addr: String,
}

/// The cleartext payload inside an invite.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvitePayload {
    /// Inviter's PIK hash (anonymous, for internal tracking only).
    pub inviter_pik_hash: [u8; 32],
    /// Bootstrap relay list (2-5 relays).
    pub bootstrap_relays: Vec<BootstrapRelay>,
    /// Epoch at which the invite was created.
    pub created_epoch: u64,
    /// Epoch at which the invite expires.
    pub expires_epoch: u64,
    /// Optional welcome message.
    pub welcome_message: Option<String>,
}

/// An invite descriptor: the data encoded in the invite code/QR.
///
/// The invitee uses this to derive the rendezvous DHT address and
/// the decryption key for the invite payload.
#[derive(Clone, Debug)]
pub struct InviteDescriptor {
    /// 32-byte random invite secret.
    pub secret: [u8; 32],
}

/// A sealed (encrypted) invite ready for DHT publication.
#[derive(Clone, Debug)]
pub struct SealedInvite {
    /// The DHT rendezvous address where this invite is stored.
    pub rendezvous_addr: [u8; 32],
    /// The encrypted payload (ChaCha20-Poly1305).
    pub ciphertext: Vec<u8>,
}

impl InviteDescriptor {
    /// Generate a new random invite descriptor.
    pub fn generate() -> Self {
        let mut secret = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut secret);
        Self { secret }
    }

    /// Create from raw secret bytes.
    pub fn from_secret(secret: [u8; 32]) -> Self {
        Self { secret }
    }

    /// Derive the DHT rendezvous address for this invite.
    ///
    /// `addr = BLAKE3::derive_key("Ochra v1 invite-descriptor", secret)`
    pub fn rendezvous_addr(&self) -> [u8; 32] {
        blake3::derive_key(contexts::INVITE_DESCRIPTOR, &self.secret)
    }

    /// Derive the encryption key for the invite payload.
    ///
    /// `key = BLAKE3::derive_key("Ochra v1 invite-payload-key", secret)`
    pub fn payload_key(&self) -> [u8; 32] {
        blake3::derive_key(contexts::INVITE_PAYLOAD_KEY, &self.secret)
    }
}

/// Create a sealed invite from a payload and descriptor.
///
/// The payload is serialized to JSON, encrypted with ChaCha20-Poly1305 using
/// a key derived from the invite secret, and returned alongside the DHT
/// rendezvous address.
pub fn create_invite(
    payload: &InvitePayload,
    descriptor: &InviteDescriptor,
) -> Result<SealedInvite> {
    let plaintext =
        serde_json::to_vec(payload).map_err(|e| InviteError::Serialization(e.to_string()))?;

    let key = descriptor.payload_key();
    // Derive nonce from the secret (deterministic for idempotent publish).
    let nonce_full = blake3::derive_key(contexts::INVITE_DESCRIPTOR, &key);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_full[..12]);

    let ciphertext = chacha20::encrypt(&key, &nonce, &plaintext, &[])
        .map_err(|e| InviteError::Crypto(e.to_string()))?;

    Ok(SealedInvite {
        rendezvous_addr: descriptor.rendezvous_addr(),
        ciphertext,
    })
}

/// Redeem an invite: decrypt the sealed payload using the invite descriptor.
///
/// Returns the cleartext `InvitePayload` containing bootstrap relay info.
pub fn redeem_invite(
    sealed: &SealedInvite,
    descriptor: &InviteDescriptor,
    current_epoch: u64,
) -> Result<InvitePayload> {
    let key = descriptor.payload_key();
    let nonce_full = blake3::derive_key(contexts::INVITE_DESCRIPTOR, &key);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_full[..12]);

    let plaintext = chacha20::decrypt(&key, &nonce, &sealed.ciphertext, &[])
        .map_err(|e| InviteError::Crypto(e.to_string()))?;

    let payload: InvitePayload =
        serde_json::from_slice(&plaintext).map_err(|e| InviteError::Malformed(e.to_string()))?;

    // Check expiration.
    if current_epoch > payload.expires_epoch {
        return Err(InviteError::Expired {
            expired_at: payload.expires_epoch,
            current_epoch,
        });
    }

    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_payload() -> InvitePayload {
        InvitePayload {
            inviter_pik_hash: [0x01u8; 32],
            bootstrap_relays: vec![
                BootstrapRelay {
                    node_id: [0x02u8; 32],
                    x25519_pk: [0x03u8; 32],
                    addr: "192.168.1.1:4433".to_string(),
                },
                BootstrapRelay {
                    node_id: [0x04u8; 32],
                    x25519_pk: [0x05u8; 32],
                    addr: "192.168.1.2:4433".to_string(),
                },
            ],
            created_epoch: 100,
            expires_epoch: 200,
            welcome_message: Some("Welcome to Ochra!".to_string()),
        }
    }

    #[test]
    fn test_create_and_redeem_roundtrip() {
        let payload = make_test_payload();
        let descriptor = InviteDescriptor::generate();

        let sealed = create_invite(&payload, &descriptor).expect("create invite");
        let redeemed = redeem_invite(&sealed, &descriptor, 150).expect("redeem invite");

        assert_eq!(redeemed.inviter_pik_hash, payload.inviter_pik_hash);
        assert_eq!(redeemed.bootstrap_relays.len(), 2);
        assert_eq!(redeemed.welcome_message, payload.welcome_message);
    }

    #[test]
    fn test_expired_invite_rejected() {
        let payload = make_test_payload();
        let descriptor = InviteDescriptor::generate();

        let sealed = create_invite(&payload, &descriptor).expect("create invite");
        let result = redeem_invite(&sealed, &descriptor, 300);
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret_fails() {
        let payload = make_test_payload();
        let descriptor1 = InviteDescriptor::generate();
        let descriptor2 = InviteDescriptor::generate();

        let sealed = create_invite(&payload, &descriptor1).expect("create invite");
        let result = redeem_invite(&sealed, &descriptor2, 150);
        assert!(result.is_err());
    }

    #[test]
    fn test_rendezvous_addr_deterministic() {
        let descriptor = InviteDescriptor::from_secret([0x42u8; 32]);
        let addr1 = descriptor.rendezvous_addr();
        let addr2 = descriptor.rendezvous_addr();
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_different_secrets_different_addresses() {
        let d1 = InviteDescriptor::from_secret([0x01u8; 32]);
        let d2 = InviteDescriptor::from_secret([0x02u8; 32]);
        assert_ne!(d1.rendezvous_addr(), d2.rendezvous_addr());
    }
}
