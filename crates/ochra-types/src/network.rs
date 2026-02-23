//! Network & Protocol structures (Section 22.10).

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::Hash;

/// Service receipt for chunk serving (Section 22.10).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceReceipt {
    pub server_node_id: [u8; 32],
    pub chunk_id: [u8; 32],
    #[serde_as(as = "serde_with::Bytes")]
    pub requester_circuit_id: [u8; 16],
    pub bytes_served: u32,
    pub timestamp: u64,
    pub relay_epoch: u32,
    #[serde_as(as = "serde_with::Bytes")]
    pub nonce: [u8; 16],
    #[serde_as(as = "serde_with::Bytes")]
    pub requester_ack: [u8; 64],
    #[serde_as(as = "serde_with::Bytes")]
    pub server_sig: [u8; 64],
}

/// Relay descriptor (Section 22.10).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayDescriptor {
    pub node_id: [u8; 32],
    pub pik_hash: [u8; 32],
    pub x25519_pk: [u8; 32],
    pub mlkem768_ek: Vec<u8>, // 1184 bytes
    pub relay_epoch: u32,
    pub posrv_score: f32,
    pub ip_addr: String, // Serialized SocketAddr
    pub as_number: u32,
    pub country_code: [u8; 2],
    pub bandwidth_cap_mbps: u16,
    pub uptime_epochs: u32,
    #[serde_as(as = "serde_with::Bytes")]
    pub sig: [u8; 64],
}

/// Epoch state (Section 22.10).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochState {
    pub epoch: u32,
    pub reward_per_token: u128,
    pub total_vys_staked: u64,
    pub fee_pool_balance: u64,
    /// Poseidon Merkle root.
    pub holder_balances_root: Hash,
    /// BLAKE3 of Bloom filter snapshot.
    pub nullifier_bloom_hash: Hash,
    /// Top 100 (or quorum size).
    pub posrv_rankings: Vec<PoSrvEntry>,
    /// FROST group signature.
    #[serde_as(as = "serde_with::Bytes")]
    pub quorum_sig: [u8; 64],
}

/// PoSrv ranking entry (Section 22.10).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoSrvEntry {
    pub pik_hash: [u8; 32],
    pub posrv_score: f32,
}

/// Nullifier gossip message (Section 22.10).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NullifierGossipMsg {
    pub msg_type: u8,
    pub epoch: u32,
    pub nullifiers: Vec<[u8; 32]>,
    #[serde_as(as = "Option<serde_with::Bytes>")]
    pub source_quorum_sig: Option<[u8; 64]>,
    pub hop_count: u8,
    #[serde_as(as = "serde_with::Bytes")]
    pub msg_id: [u8; 16],
}
