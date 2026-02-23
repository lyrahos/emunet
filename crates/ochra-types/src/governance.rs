//! Governance & Upgrade structures (Section 22.8).

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::Hash;

/// Revenue split (Section 22.8).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevenueSplit {
    pub owner_pct: u8,
    pub pub_pct: u8,
    /// Must sum to 100.
    pub abr_pct: u8,
}

/// Upgrade manifest (Section 22.8).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpgradeManifest {
    /// Semver, e.g. "5.2.0".
    pub version: String,
    /// Must be >= now + 14 days in epochs.
    pub activation_epoch: u64,
    pub platform_hashes: Vec<PlatformHash>,
    pub changelog_url: Option<String>,
    pub is_mandatory: bool,
    /// 3-of-5 minimum.
    pub multisig_sigs: Vec<MultisigEntry>,
    pub published_at: u64,
}

/// Platform-specific binary hash (Section 22.8).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlatformHash {
    pub platform: Platform,
    pub blake3_hash: Hash,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    MacosArm64,
    MacosX86_64,
    WindowsX86_64,
    LinuxX86_64,
    AndroidArm64,
    IosArm64,
}

/// Multisig entry (Section 22.8).
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultisigEntry {
    /// Keyholder index (0-4).
    pub keyholder_index: u8,
    #[serde_as(as = "serde_with::Bytes")]
    pub sig: [u8; 64],
}

/// Rollback manifest (Section 22.8).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollbackManifest {
    /// Version to rollback to.
    pub target_version: String,
    pub reason: String,
    /// 0-day timelock allowed.
    pub activation_epoch: u64,
    /// 3-of-5 minimum.
    pub multisig_sigs: Vec<MultisigEntry>,
}

/// Genesis manifest (Section 22.8).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisManifest {
    pub genesis_epoch: u64,
    /// 1,000,000 Seeds in micro-seeds.
    pub total_supply: u64,
    pub allocations: Vec<GenesisAllocation>,
    /// Published for transparency.
    pub nullifiers: Vec<[u8; 32]>,
    /// 5-of-5.
    pub multisig_sigs: Vec<MultisigEntry>,
}

/// Genesis allocation (Section 22.8).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisAllocation {
    pub name: String,
    /// In micro-seeds.
    pub amount: u64,
    /// None for immediate.
    pub vest_months: Option<u16>,
    /// e.g. "3-of-5".
    pub multisig_threshold: String,
}
