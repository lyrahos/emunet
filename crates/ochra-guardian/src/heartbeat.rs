//! Dead drop heartbeat system.
//!
//! Guardians publish periodic heartbeats to deterministic DHT addresses
//! (dead drops). The heartbeat proves the guardian is still online and
//! available for recovery.
//!
//! ## Dead Drop Address Derivation
//!
//! ```text
//! dead_drop_addr = BLAKE3::derive_key(
//!     "Ochra v1 guardian-dead-drop",
//!     guardian_pik_hash || epoch_number_le
//! )
//! ```
//!
//! ## Health Status
//!
//! - **Healthy**: Last heartbeat within [`MAX_HEARTBEAT_AGE`] (7 days)
//! - **Warning**: Last heartbeat between 5 and 7 days ago
//! - **Unresponsive**: Last heartbeat older than 7 days

use ochra_crypto::blake3::{self, contexts};
use serde::{Deserialize, Serialize};

/// Maximum heartbeat age in seconds (7 days).
pub const MAX_HEARTBEAT_AGE: u64 = 7 * 24 * 3600;

/// Warning threshold in seconds (5 days).
pub const WARNING_AGE: u64 = 5 * 24 * 3600;

/// Health status of a guardian.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Guardian is healthy (heartbeat within 5 days).
    Healthy,
    /// Guardian heartbeat is aging (between 5 and 7 days old).
    Warning,
    /// Guardian has not sent a heartbeat within 7 days.
    Unresponsive,
}

/// A heartbeat message from a guardian.
#[derive(Clone, Debug)]
pub struct Heartbeat {
    /// The guardian's PIK hash.
    pub guardian_id: [u8; 32],
    /// Unix timestamp of the heartbeat.
    pub timestamp: u64,
    /// Ed25519 signature from the guardian's PIK over (guardian_id || timestamp).
    pub signature: [u8; 64],
}

/// Publish a heartbeat for a guardian.
///
/// In v1, the signature is a stub (all zeros). In production, this
/// would be signed by the guardian's PIK.
///
/// # Arguments
///
/// * `guardian_id` - The guardian's PIK hash
/// * `timestamp` - The current Unix timestamp
pub fn publish_heartbeat(guardian_id: [u8; 32], timestamp: u64) -> Heartbeat {
    // Stub signature in v1
    let signature = [0u8; 64];

    tracing::debug!(
        timestamp,
        "guardian heartbeat published"
    );

    Heartbeat {
        guardian_id,
        timestamp,
        signature,
    }
}

/// Check the health status of a guardian based on their last heartbeat.
///
/// # Arguments
///
/// * `guardian_id` - The guardian's PIK hash (for logging)
/// * `last_heartbeat` - The Unix timestamp of the last heartbeat
/// * `max_age` - The maximum acceptable age in seconds
pub fn check_heartbeat(
    _guardian_id: &[u8; 32],
    last_heartbeat: u64,
    current_time: u64,
) -> HealthStatus {
    let age = current_time.saturating_sub(last_heartbeat);

    if age <= WARNING_AGE {
        HealthStatus::Healthy
    } else if age <= MAX_HEARTBEAT_AGE {
        HealthStatus::Warning
    } else {
        HealthStatus::Unresponsive
    }
}

/// Derive the dead-drop DHT address for a guardian at a given epoch.
///
/// `addr = BLAKE3::derive_key("Ochra v1 guardian-dead-drop", guardian_pik || epoch_le)`
pub fn derive_dead_drop_addr(guardian_pik_hash: &[u8; 32], epoch: u64) -> [u8; 32] {
    let epoch_bytes = epoch.to_le_bytes();
    let input = blake3::encode_multi_field(&[guardian_pik_hash.as_slice(), &epoch_bytes]);
    blake3::derive_key(contexts::GUARDIAN_DEAD_DROP, &input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_heartbeat() {
        let hb = publish_heartbeat([0x01; 32], 1_700_000_000);
        assert_eq!(hb.guardian_id, [0x01; 32]);
        assert_eq!(hb.timestamp, 1_700_000_000);
    }

    #[test]
    fn test_check_heartbeat_healthy() {
        let status = check_heartbeat(
            &[0x01; 32],
            1_000_000,
            1_000_000 + WARNING_AGE - 1,
        );
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[test]
    fn test_check_heartbeat_warning() {
        let status = check_heartbeat(
            &[0x01; 32],
            1_000_000,
            1_000_000 + WARNING_AGE + 1,
        );
        assert_eq!(status, HealthStatus::Warning);
    }

    #[test]
    fn test_check_heartbeat_unresponsive() {
        let status = check_heartbeat(
            &[0x01; 32],
            1_000_000,
            1_000_000 + MAX_HEARTBEAT_AGE + 1,
        );
        assert_eq!(status, HealthStatus::Unresponsive);
    }

    #[test]
    fn test_check_heartbeat_exact_boundary() {
        // At exactly WARNING_AGE, should be Healthy
        let status = check_heartbeat(&[0x01; 32], 0, WARNING_AGE);
        assert_eq!(status, HealthStatus::Healthy);

        // At exactly MAX_HEARTBEAT_AGE, should be Warning
        let status = check_heartbeat(&[0x01; 32], 0, MAX_HEARTBEAT_AGE);
        assert_eq!(status, HealthStatus::Warning);
    }

    #[test]
    fn test_dead_drop_addr_deterministic() {
        let addr1 = derive_dead_drop_addr(&[0x01; 32], 100);
        let addr2 = derive_dead_drop_addr(&[0x01; 32], 100);
        assert_eq!(addr1, addr2);
    }

    #[test]
    fn test_dead_drop_addr_varies_by_epoch() {
        let addr1 = derive_dead_drop_addr(&[0x01; 32], 100);
        let addr2 = derive_dead_drop_addr(&[0x01; 32], 101);
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_dead_drop_addr_varies_by_guardian() {
        let addr1 = derive_dead_drop_addr(&[0x01; 32], 100);
        let addr2 = derive_dead_drop_addr(&[0x02; 32], 100);
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn test_max_heartbeat_age_constant() {
        assert_eq!(MAX_HEARTBEAT_AGE, 7 * 24 * 3600);
    }
}
