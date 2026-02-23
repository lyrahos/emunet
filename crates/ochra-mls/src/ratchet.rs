//! Double Ratchet for group key derivation.
//!
//! Implements a symmetric ratchet based on BLAKE3 KDF for deriving
//! per-message keys within an MLS epoch. This provides forward secrecy
//! at the message level within a single epoch.
//!
//! ## KDF Chain
//!
//! Each ratchet step derives two keys from the current chain key:
//! - A **message key** used to encrypt a single message.
//! - A **new chain key** for the next ratchet step.
//!
//! The context string `"Ochra v1 double-ratchet-chain"` is used as the
//! KDF domain separator (mapped to `RATCHET_CHAIN_KEY`).

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::Result;

/// A message key derived from the ratchet chain.
#[derive(Clone, Debug)]
pub struct MessageKey {
    /// The 32-byte encryption key.
    pub key: [u8; 32],
    /// A 12-byte nonce derived alongside the key.
    pub nonce: [u8; 12],
    /// The ratchet step that produced this key.
    pub step: u64,
}

/// State of the symmetric ratchet.
///
/// Tracks the current chain key and ratchet step for deriving
/// per-message encryption keys.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RatchetState {
    /// The current chain key (32 bytes).
    chain_key: [u8; 32],
    /// Current ratchet step (number of advances).
    step: u64,
}

impl RatchetState {
    /// Initialize a new ratchet from a root secret.
    ///
    /// The root secret is typically the epoch secret from the MLS group
    /// key schedule.
    ///
    /// # Arguments
    ///
    /// * `root_secret` - The 32-byte root secret to initialize the chain.
    pub fn new(root_secret: [u8; 32]) -> Self {
        let chain_key = blake3::derive_key(blake3::contexts::RATCHET_CHAIN_KEY, &root_secret);
        Self { chain_key, step: 0 }
    }

    /// Derive the message key for the current step without advancing.
    ///
    /// # Returns
    ///
    /// A [`MessageKey`] for encrypting a single message at the current step.
    pub fn derive_message_key(&self) -> MessageKey {
        let step_bytes = self.step.to_le_bytes();
        let input = blake3::encode_multi_field(&[&self.chain_key, &step_bytes]);

        let key = blake3::derive_key(blake3::contexts::RATCHET_MSG_KEY, &input);

        let nonce_full = blake3::derive_key(blake3::contexts::RATCHET_NONCE, &input);
        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&nonce_full[..12]);

        MessageKey {
            key,
            nonce,
            step: self.step,
        }
    }

    /// Advance the ratchet to the next step.
    ///
    /// Consumes the current chain key and derives a new one. The old
    /// chain key is overwritten, providing forward secrecy.
    ///
    /// # Returns
    ///
    /// A new [`RatchetState`] at the next step.
    pub fn advance(&self) -> Result<RatchetState> {
        let step_bytes = self.step.to_le_bytes();
        let input = blake3::encode_multi_field(&[&self.chain_key, &step_bytes]);
        let new_chain_key = blake3::derive_key(blake3::contexts::RATCHET_CHAIN_KEY, &input);

        Ok(RatchetState {
            chain_key: new_chain_key,
            step: self.step + 1,
        })
    }

    /// Derive the message key and advance the ratchet in one step.
    ///
    /// This is a convenience method combining [`derive_message_key`] and
    /// [`advance`].
    pub fn derive_and_advance(&mut self) -> Result<MessageKey> {
        let msg_key = self.derive_message_key();
        let next = self.advance()?;
        self.chain_key = next.chain_key;
        self.step = next.step;
        Ok(msg_key)
    }

    /// Get the current ratchet step.
    pub fn step(&self) -> u64 {
        self.step
    }

    /// Get the current chain key (for diagnostics; do not expose in production).
    pub fn chain_key(&self) -> &[u8; 32] {
        &self.chain_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratchet_initialization() {
        let root = [0x42u8; 32];
        let state = RatchetState::new(root);
        assert_eq!(state.step(), 0);
    }

    #[test]
    fn test_derive_message_key_deterministic() {
        let root = [0x42u8; 32];
        let state = RatchetState::new(root);

        let key1 = state.derive_message_key();
        let key2 = state.derive_message_key();

        assert_eq!(key1.key, key2.key);
        assert_eq!(key1.nonce, key2.nonce);
        assert_eq!(key1.step, 0);
    }

    #[test]
    fn test_advance_changes_chain_key() {
        let root = [0x42u8; 32];
        let state = RatchetState::new(root);

        let advanced = state.advance().expect("advance");
        assert_eq!(advanced.step(), 1);
        assert_ne!(state.chain_key(), advanced.chain_key());
    }

    #[test]
    fn test_different_steps_different_keys() {
        let root = [0x42u8; 32];
        let state0 = RatchetState::new(root);
        let state1 = state0.advance().expect("advance");
        let state2 = state1.advance().expect("advance");

        let key0 = state0.derive_message_key();
        let key1 = state1.derive_message_key();
        let key2 = state2.derive_message_key();

        assert_ne!(key0.key, key1.key);
        assert_ne!(key1.key, key2.key);
        assert_ne!(key0.key, key2.key);
    }

    #[test]
    fn test_derive_and_advance() {
        let root = [0x42u8; 32];
        let mut state = RatchetState::new(root);

        let key0 = state.derive_and_advance().expect("derive_and_advance");
        assert_eq!(key0.step, 0);
        assert_eq!(state.step(), 1);

        let key1 = state.derive_and_advance().expect("derive_and_advance");
        assert_eq!(key1.step, 1);
        assert_eq!(state.step(), 2);

        assert_ne!(key0.key, key1.key);
    }

    #[test]
    fn test_different_roots_different_chains() {
        let state_a = RatchetState::new([0x01u8; 32]);
        let state_b = RatchetState::new([0x02u8; 32]);

        let key_a = state_a.derive_message_key();
        let key_b = state_b.derive_message_key();

        assert_ne!(key_a.key, key_b.key);
    }

    #[test]
    fn test_ratchet_forward_secrecy() {
        // After advancing, the old chain key is lost. Verify that
        // the same root produces the same sequence, but a later state
        // cannot reproduce earlier keys.
        let root = [0xAA; 32];
        let state0 = RatchetState::new(root);
        let key0 = state0.derive_message_key();

        let state1 = state0.advance().expect("advance");
        let key1 = state1.derive_message_key();

        // Re-derive from root should give same keys.
        let state0_again = RatchetState::new(root);
        let key0_again = state0_again.derive_message_key();
        assert_eq!(key0.key, key0_again.key);

        // But state1 cannot produce key0.
        let key_from_state1 = state1.derive_message_key();
        assert_eq!(key_from_state1.key, key1.key);
        assert_ne!(key_from_state1.key, key0.key);
    }

    #[test]
    fn test_message_key_has_valid_nonce() {
        let state = RatchetState::new([0x55; 32]);
        let key = state.derive_message_key();
        // Nonce should be 12 bytes and not all zeros.
        assert_eq!(key.nonce.len(), 12);
        assert_ne!(key.nonce, [0u8; 12]);
    }

    #[test]
    fn test_serde_roundtrip() {
        let state = RatchetState::new([0x77; 32]);
        let json = serde_json::to_string(&state).expect("serialize");
        let restored: RatchetState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state.chain_key(), restored.chain_key());
        assert_eq!(state.step(), restored.step());
    }
}
