//! Diagnostics structures (Section 22.5).

use serde::{Deserialize, Serialize};

use crate::Hash;

/// Circuit metrics (Section 22.5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitMetrics {
    pub active_circuits: u32,
    pub circuits_rotated_24h: u32,
    pub avg_latency_ms: u32,
    pub relay_count_known: u32,
    pub nat_traversal_status: NatStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatStatus {
    Direct,
    HolePunched,
    Relayed,
}

/// Cover traffic metrics (Section 22.5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverTrafficMetrics {
    pub current_mode: CoverTrafficMode,
    pub lambda_p: f64,
    pub lambda_l: f64,
    pub lambda_d: f64,
    pub bandwidth_kbps: f64,
    pub mode_dwell_remaining_s: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverTrafficMode {
    Sleep,
    Idle,
    Active,
    Burst,
}

/// Update status (Section 22.5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub current_version: String,
    pub available_version: Option<String>,
    pub manifest_hash: Option<Hash>,
    pub activation_epoch: Option<u64>,
    pub is_mandatory: bool,
}

/// Log entry (Section 22.5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub module: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// PoR submission status (Section 22.5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PorSubmissionStatus {
    pub status: PorStatus,
    pub epoch: u64,
    pub proof_size_bytes: u32,
    pub proving_time_ms: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PorStatus {
    Submitted,
    Verified,
    Failed,
    Late,
}
