//! Space structures (Section 22.2, 22.11).

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::{ContentHash, GroupId, Hash};

/// Summary of a Space for listing views (Section 22.2).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct GroupSummary {
    #[ts(type = "string")]
    pub group_id: GroupId,
    pub name: String,
    #[ts(type = "string | null")]
    pub icon: Option<Vec<u8>>,
    pub template: SpaceTemplate,
    pub is_host: bool,
    pub role: super::identity::MemberRole,
    pub member_count: u32,
    pub last_activity_at: u64,
    pub unread: bool,
    pub pinned: bool,
}

/// Space template types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SpaceTemplate {
    Storefront,
    Forum,
    Newsfeed,
    Gallery,
    Library,
}

/// Space settings (Section 22.2).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct GroupSettings {
    pub invite_permission: InvitePermission,
    pub publish_policy: PublishPolicy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum InvitePermission {
    Anyone,
    HostOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum PublishPolicy {
    CreatorsOnly,
    Everyone,
}

/// Invite information (Section 22.2).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct InviteInfo {
    #[ts(type = "string")]
    pub invite_hash: Hash,
    pub creator_flag: bool,
    pub uses_limit: Option<u32>,
    pub uses_consumed: u32,
    pub ttl_days: u8,
    pub created_at: u64,
    pub expires_at: u64,
    pub is_expired: bool,
}

/// Space statistics (Section 22.2).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SpaceStats {
    pub total_members: u32,
    pub total_creators: u32,
    pub total_moderators: u32,
    pub total_content_items: u32,
    pub total_earnings_all_time: u64,
    pub earnings_this_epoch: u64,
    pub earnings_trend: EarningsTrend,
    pub pending_reports: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum EarningsTrend {
    Up,
    Down,
    Flat,
}

/// Activity event (Section 22.2).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct ActivityEvent {
    pub event_type: String,
    pub timestamp: u64,
    pub actor_display_name: Option<String>,
    pub content_title: Option<String>,
    pub amount_seeds: Option<u64>,
}

/// Content report (Section 22.2).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct ContentReport {
    #[ts(type = "string")]
    pub content_hash: Hash,
    pub content_title: String,
    pub creator_display_name: String,
    pub reports: Vec<SingleReport>,
}

/// Individual report entry (Section 22.2).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SingleReport {
    /// Salted pseudonym (Section 16.7). NOT reporter PIK.
    #[serde_as(as = "serde_with::Bytes")]
    #[ts(type = "string")]
    pub reporter_hash: [u8; 16],
    pub reason: ReportReason,
    pub detail: Option<String>,
    pub timestamp: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum ReportReason {
    Spam,
    Offensive,
    Broken,
    Other,
}

/// Space manifest (Section 22.11).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct SpaceManifest {
    #[ts(type = "string")]
    pub group_id: GroupId,
    pub name: String,
    #[ts(type = "string | null")]
    pub icon_hash: Option<Hash>,
    pub template: SpaceTemplate,
    pub accent_color: String,
    #[ts(type = "string")]
    pub host_pik: [u8; 32],
    pub publish_policy: PublishPolicy,
    pub invite_permission: InvitePermission,
    pub owner_pct: u8,
    pub pub_pct: u8,
    pub abr_pct: u8,
    #[ts(type = "string[]")]
    pub creator_piks: Vec<[u8; 32]>,
    #[ts(type = "string[]")]
    pub moderator_piks: Vec<[u8; 32]>,
    pub member_count: u32,
    pub created_at: u64,
    pub updated_at: u64,
    #[ts(type = "string | null")]
    pub layout_manifest_hash: Option<Hash>,
    pub pending_transfer: Option<OwnershipTransferRecord>,
    pub pending_split_change: Option<super::content::RevenueSplitChangeProposal>,
    pub version: u32,
    #[serde_as(as = "serde_with::Bytes")]
    #[ts(type = "string")]
    pub host_sig: [u8; 64],
}

/// Ownership transfer record (Section 22.11).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct OwnershipTransferRecord {
    #[ts(type = "string")]
    pub new_owner_pik: [u8; 32],
    pub initiated_at: u64,
    /// initiated_at + 7 days.
    pub completes_at: u64,
}

/// Catalog diff request (Section 22.11).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct CatalogDiffRequest {
    pub msg_type: u8,
    #[ts(type = "string")]
    pub group_id: GroupId,
    pub last_known_epoch: u32,
    #[ts(type = "string")]
    pub last_known_catalog_hash: Hash,
}

/// Catalog diff response (Section 22.11).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct CatalogDiffResponse {
    pub msg_type: u8,
    pub added: Vec<super::content::ContentManifest>,
    #[ts(type = "string[]")]
    pub tombstoned: Vec<ContentHash>,
    #[ts(type = "string")]
    pub current_catalog_hash: Hash,
}
