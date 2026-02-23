//! Whisper messaging structures (Section 22.4).

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::Hash;

/// Handle descriptor for Whisper reachability (Section 22.4).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandleDescriptor {
    pub handle: String,
    pub handle_signing_pk: [u8; 32],
    pub intro_points: Vec<IntroPointEntry>,
    pub auth_key: [u8; 32],
    pub pq_auth_key: Vec<u8>,
    pub registered_at: u64,
    pub refresh_at: u64,
    pub pow_proof: Vec<u8>,
    pub status: HandleStatus,
    #[serde_as(as = "serde_with::Bytes")]
    pub sig: [u8; 64],
}

/// Introduction point entry (Section 22.4).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntroPointEntry {
    pub node_id: [u8; 32],
    pub auth_key: [u8; 32],
}

/// Handle status (Section 22.4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HandleStatus {
    Active,
    Deprecated {
        deprecated_at: u64,
        successor_handle: Option<String>,
    },
}

/// Handle registration (Section 22.4).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandleRegistration {
    pub handle: String,
    pub registered_at: u64,
    /// 7 days from registration; auto-refreshed.
    pub expires_at: u64,
}

/// Handle info (Section 22.4).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandleInfo {
    pub handle: String,
    pub registered_at: u64,
    pub last_refreshed: u64,
    pub status: HandleStatus,
}

/// Whisper message (Section 22.4).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperMessage {
    pub sequence: u64,
    pub timestamp: u64,
    pub msg_type: WhisperMsgType,
    pub body: Vec<u8>,
    pub relay_receipts: Vec<RelayReceipt>,
    pub nonce: [u8; 12],
    #[serde_as(as = "serde_with::Bytes")]
    pub tag: [u8; 16],
}

/// Whisper message types (Section 22.4).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WhisperMsgType {
    Text,
    SeedTransfer {
        tx_hash: [u8; 32],
        amount: u64,
    },
    Typing,
    ReadAck,
}

/// Relay receipt for anti-spam accounting (Section 22.4).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayReceipt {
    pub relay_epoch: u32,
    #[serde_as(as = "serde_with::Bytes")]
    pub packet_hash: [u8; 16],
    pub relayer_node_id: [u8; 32],
    pub next_hop_node_id: [u8; 32],
    #[serde_as(as = "serde_with::Bytes")]
    pub sig: [u8; 64],
}

/// Whisper target (Section 22.4).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WhisperTarget {
    Handle(String),
    Contact(Hash),
}

/// Whisper session summary (Section 22.4).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperSessionSummary {
    #[serde_as(as = "serde_with::Bytes")]
    pub session_id: [u8; 16],
    pub counterparty: WhisperCounterparty,
    pub started_at: u64,
    pub last_message_at: u64,
    pub unread_count: u32,
    pub state: SessionState,
}

/// Session state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Active,
    BackgroundGrace,
    Locked,
}

/// Whisper counterparty info (Section 22.4).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperCounterparty {
    pub revealed_handle: Option<String>,
    pub revealed_display_name: Option<String>,
    pub is_contact: bool,
    pub is_verified: bool,
}

/// Throttle status for Whisper anti-spam (Section 22.4).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThrottleStatus {
    pub session_msg_count: u64,
    pub current_tier: String,
    pub receipts_required: u8,
    pub global_hourly_count: u64,
    pub global_surcharge: u8,
    pub total_cost: u8,
    pub receipts_accumulated: u8,
    pub is_contact_exempt: bool,
}

/// Identity reveal for Whisper (Section 22.4).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdentityReveal {
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub proof: IdentityProof,
}

/// Identity proof types (Section 22.4).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IdentityProof {
    HandleProof {
        handle_signing_pk: [u8; 32],
        #[serde_as(as = "serde_with::Bytes")]
        sig: [u8; 64],
    },
    ContactProof {
        pik_hash: [u8; 32],
        #[serde_as(as = "serde_with::Bytes")]
        sig: [u8; 64],
    },
}

/// Deprecation tombstone for handles (Section 22.4).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeprecationTombstone {
    pub handle: String,
    pub deprecated_at: u64,
    pub successor_handle: Option<String>,
    /// Fixed: 30 days.
    pub tombstone_ttl_days: u8,
    #[serde_as(as = "serde_with::Bytes")]
    pub sig: [u8; 64],
}

/// Whisper ping for missed messages (Section 22.4).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhisperPing {
    pub target_addr: [u8; 32],
    pub timestamp: u64,
    #[serde_as(as = "serde_with::Bytes")]
    pub ping_id: [u8; 16],
}
