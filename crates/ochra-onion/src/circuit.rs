//! 3-hop Sphinx circuit construction and management.
//!
//! A circuit consists of 3 relay hops, each with independently derived
//! cryptographic keys. The sender performs X25519 key exchange with each relay
//! and derives per-hop keys using BLAKE3 with domain-separated context strings.
//!
//! ## Key Derivation per Hop
//!
//! For each hop, the sender computes a shared secret via X25519 and derives:
//! - **hop key**: `BLAKE3::derive_key("Ochra v1 sphinx-hop-key", shared_secret)`
//! - **hop MAC**: `BLAKE3::derive_key("Ochra v1 sphinx-hop-mac", shared_secret)`
//! - **hop pad**: `BLAKE3::derive_key("Ochra v1 sphinx-hop-pad", shared_secret)`
//! - **hop nonce**: `BLAKE3::derive_key("Ochra v1 sphinx-hop-nonce", shared_secret)[:12]`
//!
//! ## Circuit Rotation
//!
//! Circuits have a maximum lifetime of 10 minutes. After expiry, the circuit
//! must be torn down and a new one constructed with fresh relay selections.

use std::time::Instant;

use ochra_crypto::blake3::contexts;
use ochra_crypto::x25519::{X25519PublicKey, X25519StaticSecret};
use ochra_types::network::RelayDescriptor;

use crate::{OnionError, Result, CIRCUIT_HOPS, CIRCUIT_LIFETIME_SECS};

/// Per-hop cryptographic keys derived from the shared secret.
#[derive(Clone)]
pub struct HopKeys {
    /// Symmetric encryption key for this hop (32 bytes).
    pub hop_key: [u8; 32],
    /// MAC key for this hop (32 bytes).
    pub hop_mac: [u8; 32],
    /// Pad key for XOR stream generation (32 bytes).
    pub hop_pad: [u8; 32],
    /// Nonce for AEAD operations at this hop (12 bytes).
    pub hop_nonce: [u8; 12],
}

/// Information about a single hop in the circuit.
#[derive(Clone)]
pub struct HopInfo {
    /// The relay's node ID.
    pub node_id: [u8; 32],
    /// The relay's X25519 public key.
    pub relay_pk: X25519PublicKey,
    /// The relay's network address.
    pub addr: String,
    /// Derived cryptographic keys for this hop.
    pub keys: HopKeys,
}

/// An active 3-hop Sphinx circuit.
pub struct Circuit {
    /// The three hops in order (entry, middle, exit).
    hops: Vec<HopInfo>,
    /// The ephemeral secret key used for this circuit's key exchanges.
    #[allow(dead_code)]
    ephemeral_secret: X25519StaticSecret,
    /// The ephemeral public key (sent in Sphinx headers).
    ephemeral_pk: X25519PublicKey,
    /// When this circuit was created.
    created_at: Instant,
    /// Circuit identifier (random 16 bytes).
    circuit_id: [u8; 16],
}

impl Circuit {
    /// Return the circuit identifier.
    pub fn circuit_id(&self) -> &[u8; 16] {
        &self.circuit_id
    }

    /// Return the ephemeral public key for this circuit.
    pub fn ephemeral_pk(&self) -> &X25519PublicKey {
        &self.ephemeral_pk
    }

    /// Return the hops in this circuit.
    pub fn hops(&self) -> &[HopInfo] {
        &self.hops
    }

    /// Return the entry (first) hop.
    pub fn entry_hop(&self) -> &HopInfo {
        &self.hops[0]
    }

    /// Return the middle (second) hop.
    pub fn middle_hop(&self) -> &HopInfo {
        &self.hops[1]
    }

    /// Return the exit (third) hop.
    pub fn exit_hop(&self) -> &HopInfo {
        &self.hops[2]
    }

    /// Check whether this circuit has expired (exceeded its 10-minute lifetime).
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() >= CIRCUIT_LIFETIME_SECS
    }

    /// Return the age of this circuit in seconds.
    pub fn age_secs(&self) -> u64 {
        self.created_at.elapsed().as_secs()
    }

    /// Return the remaining lifetime in seconds (0 if expired).
    pub fn remaining_secs(&self) -> u64 {
        let elapsed = self.created_at.elapsed().as_secs();
        CIRCUIT_LIFETIME_SECS.saturating_sub(elapsed)
    }
}

/// Builder for constructing 3-hop Sphinx circuits from relay descriptors.
pub struct CircuitBuilder {
    /// Selected relay descriptors for the circuit hops.
    relays: Vec<RelayDescriptor>,
}

impl CircuitBuilder {
    /// Create a new circuit builder.
    pub fn new() -> Self {
        Self {
            relays: Vec::with_capacity(CIRCUIT_HOPS),
        }
    }

    /// Add a relay to the circuit path.
    ///
    /// Relays must be added in order: entry, middle, exit.
    pub fn add_relay(mut self, relay: RelayDescriptor) -> Result<Self> {
        if self.relays.len() >= CIRCUIT_HOPS {
            return Err(OnionError::CircuitConstruction(format!(
                "circuit already has {} hops (maximum {})",
                self.relays.len(),
                CIRCUIT_HOPS,
            )));
        }
        self.relays.push(relay);
        Ok(self)
    }

    /// Build the circuit by performing key exchange with each relay.
    ///
    /// Generates an ephemeral X25519 keypair and derives per-hop keys
    /// using the Ochra key derivation scheme.
    pub fn build(self) -> Result<Circuit> {
        if self.relays.len() != CIRCUIT_HOPS {
            return Err(OnionError::InsufficientRelays {
                need: CIRCUIT_HOPS,
                have: self.relays.len(),
            });
        }

        let ephemeral_secret = X25519StaticSecret::random();
        let ephemeral_pk = ephemeral_secret.public_key();

        let mut hops = Vec::with_capacity(CIRCUIT_HOPS);

        for relay in &self.relays {
            let relay_pk = X25519PublicKey::from_bytes(relay.x25519_pk);
            let shared_secret = ephemeral_secret.diffie_hellman(&relay_pk);
            let keys = derive_hop_keys(shared_secret.as_bytes());

            hops.push(HopInfo {
                node_id: relay.node_id,
                relay_pk,
                addr: relay.ip_addr.clone(),
                keys,
            });
        }

        let mut circuit_id = [0u8; 16];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut circuit_id);

        Ok(Circuit {
            hops,
            ephemeral_secret,
            ephemeral_pk,
            created_at: Instant::now(),
            circuit_id,
        })
    }
}

impl Default for CircuitBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Derive all per-hop cryptographic keys from a shared secret.
///
/// Uses BLAKE3 `derive_key` with the following context strings:
/// - `"Ochra v1 sphinx-hop-key"` for the symmetric encryption key
/// - `"Ochra v1 sphinx-hop-mac"` for the MAC key
/// - `"Ochra v1 sphinx-hop-pad"` for the XOR pad generation key
/// - `"Ochra v1 sphinx-hop-nonce"` for the AEAD nonce (first 12 bytes)
pub fn derive_hop_keys(shared_secret: &[u8; 32]) -> HopKeys {
    let hop_key = ochra_crypto::blake3::derive_key(contexts::SPHINX_HOP_KEY, shared_secret);
    let hop_mac = ochra_crypto::blake3::derive_key(contexts::SPHINX_HOP_MAC, shared_secret);
    let hop_pad = ochra_crypto::blake3::derive_key(contexts::SPHINX_HOP_PAD, shared_secret);
    let nonce_full = ochra_crypto::blake3::derive_key(contexts::SPHINX_HOP_NONCE, shared_secret);

    let mut hop_nonce = [0u8; 12];
    hop_nonce.copy_from_slice(&nonce_full[..12]);

    HopKeys {
        hop_key,
        hop_mac,
        hop_pad,
        hop_nonce,
    }
}

/// Check whether a circuit needs rotation (has exceeded its lifetime).
pub fn needs_rotation(circuit: &Circuit) -> bool {
    circuit.is_expired()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_relay_descriptor(id_byte: u8) -> RelayDescriptor {
        let secret = X25519StaticSecret::random();
        let pk = secret.public_key();
        RelayDescriptor {
            node_id: [id_byte; 32],
            pik_hash: [id_byte; 32],
            x25519_pk: pk.to_bytes(),
            mlkem768_ek: vec![0u8; 1184],
            relay_epoch: 1,
            posrv_score: 1.0,
            ip_addr: format!("10.0.0.{}:4433", id_byte),
            as_number: u32::from(id_byte),
            country_code: [b'U', b'S'],
            bandwidth_cap_mbps: 100,
            uptime_epochs: 100,
            sig: [0u8; 64],
        }
    }

    #[test]
    fn test_derive_hop_keys_deterministic() {
        let shared = [0x42u8; 32];
        let k1 = derive_hop_keys(&shared);
        let k2 = derive_hop_keys(&shared);
        assert_eq!(k1.hop_key, k2.hop_key);
        assert_eq!(k1.hop_mac, k2.hop_mac);
        assert_eq!(k1.hop_pad, k2.hop_pad);
        assert_eq!(k1.hop_nonce, k2.hop_nonce);
    }

    #[test]
    fn test_derive_hop_keys_different_secrets() {
        let k1 = derive_hop_keys(&[0x01u8; 32]);
        let k2 = derive_hop_keys(&[0x02u8; 32]);
        assert_ne!(k1.hop_key, k2.hop_key);
        assert_ne!(k1.hop_mac, k2.hop_mac);
    }

    #[test]
    fn test_derive_hop_keys_all_different() {
        let keys = derive_hop_keys(&[0x42u8; 32]);
        // Each derived key should be different from the others.
        assert_ne!(keys.hop_key, keys.hop_mac);
        assert_ne!(keys.hop_key, keys.hop_pad);
        assert_ne!(keys.hop_mac, keys.hop_pad);
    }

    #[test]
    fn test_circuit_builder_success() {
        let r1 = make_relay_descriptor(1);
        let r2 = make_relay_descriptor(2);
        let r3 = make_relay_descriptor(3);

        let circuit = CircuitBuilder::new()
            .add_relay(r1)
            .expect("add r1")
            .add_relay(r2)
            .expect("add r2")
            .add_relay(r3)
            .expect("add r3")
            .build()
            .expect("build circuit");

        assert_eq!(circuit.hops().len(), 3);
        assert_eq!(circuit.entry_hop().node_id, [1u8; 32]);
        assert_eq!(circuit.middle_hop().node_id, [2u8; 32]);
        assert_eq!(circuit.exit_hop().node_id, [3u8; 32]);
    }

    #[test]
    fn test_circuit_builder_too_many_relays() {
        let r1 = make_relay_descriptor(1);
        let r2 = make_relay_descriptor(2);
        let r3 = make_relay_descriptor(3);
        let r4 = make_relay_descriptor(4);

        let result = CircuitBuilder::new()
            .add_relay(r1)
            .expect("add r1")
            .add_relay(r2)
            .expect("add r2")
            .add_relay(r3)
            .expect("add r3")
            .add_relay(r4);

        assert!(result.is_err());
    }

    #[test]
    fn test_circuit_builder_insufficient_relays() {
        let r1 = make_relay_descriptor(1);

        let result = CircuitBuilder::new().add_relay(r1).expect("add r1").build();

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(OnionError::InsufficientRelays { need: 3, have: 1 })
        ));
    }

    #[test]
    fn test_circuit_not_expired() {
        let r1 = make_relay_descriptor(1);
        let r2 = make_relay_descriptor(2);
        let r3 = make_relay_descriptor(3);

        let circuit = CircuitBuilder::new()
            .add_relay(r1)
            .expect("add")
            .add_relay(r2)
            .expect("add")
            .add_relay(r3)
            .expect("add")
            .build()
            .expect("build");

        assert!(!circuit.is_expired());
        assert!(circuit.remaining_secs() > 0);
    }

    #[test]
    fn test_circuit_id_unique() {
        let r1a = make_relay_descriptor(1);
        let r2a = make_relay_descriptor(2);
        let r3a = make_relay_descriptor(3);
        let r1b = make_relay_descriptor(4);
        let r2b = make_relay_descriptor(5);
        let r3b = make_relay_descriptor(6);

        let c1 = CircuitBuilder::new()
            .add_relay(r1a)
            .expect("add")
            .add_relay(r2a)
            .expect("add")
            .add_relay(r3a)
            .expect("add")
            .build()
            .expect("build");

        let c2 = CircuitBuilder::new()
            .add_relay(r1b)
            .expect("add")
            .add_relay(r2b)
            .expect("add")
            .add_relay(r3b)
            .expect("add")
            .build()
            .expect("build");

        assert_ne!(c1.circuit_id(), c2.circuit_id());
    }

    #[test]
    fn test_hop_keys_per_hop() {
        let r1 = make_relay_descriptor(1);
        let r2 = make_relay_descriptor(2);
        let r3 = make_relay_descriptor(3);

        let circuit = CircuitBuilder::new()
            .add_relay(r1)
            .expect("add")
            .add_relay(r2)
            .expect("add")
            .add_relay(r3)
            .expect("add")
            .build()
            .expect("build");

        // Each hop should have different keys.
        let h1 = &circuit.entry_hop().keys;
        let h2 = &circuit.middle_hop().keys;
        let h3 = &circuit.exit_hop().keys;

        assert_ne!(h1.hop_key, h2.hop_key);
        assert_ne!(h2.hop_key, h3.hop_key);
        assert_ne!(h1.hop_key, h3.hop_key);
    }

    #[test]
    fn test_needs_rotation_fresh() {
        let r1 = make_relay_descriptor(1);
        let r2 = make_relay_descriptor(2);
        let r3 = make_relay_descriptor(3);

        let circuit = CircuitBuilder::new()
            .add_relay(r1)
            .expect("add")
            .add_relay(r2)
            .expect("add")
            .add_relay(r3)
            .expect("add")
            .build()
            .expect("build");

        assert!(!needs_rotation(&circuit));
    }
}
