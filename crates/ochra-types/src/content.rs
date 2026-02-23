//! Content & Economy structures (Section 22.3).

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{Bytes, ContentHash, GroupId, Hash};

/// Content manifest (Section 22.3).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentManifest {
    /// Merkle root.
    pub content_hash: ContentHash,
    pub title: String,
    pub description: Option<String>,
    /// Max 5 tags.
    pub tags: Vec<String>,
    /// Max 4, min 1.
    pub pricing: Vec<PricingTier>,
    pub creator_pik: [u8; 32],
    pub group_id: GroupId,
    pub successor_hash: Option<ContentHash>,
    /// BLAKE3::hash(decryption_key).
    pub key_commitment: Hash,
    pub total_size_bytes: u64,
    pub chunk_count: u32,
    pub force_macro: bool,
    pub published_at: u64,
    pub pow_proof: Bytes,
    /// Creator's PIK signature.
    #[serde_as(as = "serde_with::Bytes")]
    pub sig: [u8; 64],
}

/// Pricing tier (Section 22.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PricingTier {
    pub tier_type: TierType,
    /// Price in micro-seeds.
    pub price_seeds: u64,
    /// Days for rental tiers.
    pub rental_days: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierType {
    Permanent,
    Rental,
}

/// Purchase record (Section 22.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseRecord {
    pub content_hash: ContentHash,
    pub title: String,
    pub tier_type: TierType,
    /// Price in micro-seeds.
    pub price_paid: u64,
    pub purchased_at: u64,
    /// None for permanent.
    pub expires_at: Option<u64>,
    /// Local only, never transmitted.
    pub receipt_secret: [u8; 32],
}

/// Receipt info for blind receipt management (Section 22.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptInfo {
    pub content_hash: ContentHash,
    pub receipt_id: [u8; 32],
    pub tier_type: TierType,
    pub last_republished_epoch: u64,
    pub expires_at: Option<u64>,
}

/// Access status for content (Section 22.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccessStatus {
    pub has_access: bool,
    pub tier_type: Option<TierType>,
    pub expires_at: Option<u64>,
    pub can_redownload: bool,
}

/// Earnings report for a Space (Section 22.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EarningsReport {
    pub group_id: GroupId,
    /// All-time earnings in micro-seeds.
    pub total_all_time: u64,
    /// This epoch earnings in micro-seeds.
    pub this_epoch: u64,
    pub owner_share: u64,
    pub creator_share: u64,
    pub abr_share: u64,
    pub per_content: Vec<ContentEarning>,
}

/// Per-content earning detail (Section 22.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentEarning {
    pub content_hash: ContentHash,
    pub title: String,
    pub earnings_all_time: u64,
    pub earnings_this_epoch: u64,
    pub purchase_count: u32,
}

/// Refund status (Section 22.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefundStatus {
    pub status: RefundState,
    pub refund_amount: Option<u64>,
    pub epoch: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefundState {
    Submitted,
    Approved,
    Rejected,
}

/// Receipt flush statistics (Section 22.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlushStats {
    pub receipts_flushed: u32,
    pub seeds_minted: u64,
    pub epoch: u64,
}

/// MPC session info (Section 22.3).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MpcSession {
    #[serde_as(as = "serde_with::Bytes")]
    pub session_id: [u8; 16],
    pub target_api: String,
    pub status: MpcStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MpcStatus {
    Initiating,
    Active,
    Complete,
    Failed,
}

/// Revenue split change proposal (Section 22.3).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevenueSplitChangeProposal {
    pub group_id: GroupId,
    pub sequence: u32,
    pub proposed_owner_pct: u8,
    pub proposed_pub_pct: u8,
    pub proposed_abr_pct: u8,
    pub effective_at: u64,
    pub broadcast_at: u64,
    #[serde_as(as = "serde_with::Bytes")]
    pub owner_sig: [u8; 64],
}
