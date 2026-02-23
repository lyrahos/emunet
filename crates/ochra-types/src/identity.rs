//! Identity & Contact structures (Section 22.1).

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

/// PIK metadata (Section 22.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PikMeta {
    pub pik_hash: [u8; 32],
    pub created_at: u64,
    pub encrypted_key_path: String,
    pub argon2id_salt: [u8; 32],
}

/// A contact entry (Section 22.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Contact {
    pub pik_hash: [u8; 32],
    pub display_name: String,
    pub profile_key: [u8; 32],
    pub added_at: u64,
    pub last_seen_epoch: u64,
}

/// Peer profile within a Space (Section 22.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerProfile {
    pub pik_hash: [u8; 32],
    pub display_name: String,
    pub role: MemberRole,
    pub joined_at: u64,
}

/// Member role within a Space.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    Host,
    Creator,
    Moderator,
    Member,
}

/// Guardian status for recovery contacts (Section 22.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardianStatus {
    pub contact_pik: [u8; 32],
    pub display_name: String,
    pub last_heartbeat_epoch: u64,
    /// True if heartbeat within 30 days.
    pub is_healthy: bool,
    pub days_since_heartbeat: u16,
}

/// Profile key exchange payload (Section 22.1).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileKeyExchange {
    /// 256-bit profile key for encrypted profile lookup.
    pub profile_key: [u8; 32],
    /// Encrypted with recipient's ephemeral session key.
    pub display_name_ciphertext: Vec<u8>,
    /// Ed25519 from sender's PIK, over profile_key.
    #[serde_as(as = "serde_with::Bytes")]
    pub sig: [u8; 64],
}

/// Timelock status for delayed operations (Section 22.1).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelockStatus {
    pub action: TimelockAction,
    pub initiated_at: u64,
    pub completes_at: u64,
    pub can_veto: bool,
    pub is_complete: bool,
}

/// Types of timelocked actions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelockAction {
    Recovery,
    OwnershipTransfer,
    RevenueSplit,
}

/// Contact exchange token for establishing bidirectional contacts (Section 22.6).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactExchangeToken {
    pub ephemeral_x25519_pk: [u8; 32],
    pub ephemeral_mlkem768_ek: Vec<u8>, // 1184 bytes
    pub intro_points: Vec<super::whisper::IntroPointEntry>,
    pub ttl_hours: u16,
    pub created_at: u64,
    #[serde_as(as = "serde_with::Bytes")]
    pub pik_sig: [u8; 64],
}
