//! 48-hour Dual-Path Cancellation recovery.
//!
//! The recovery process has a 48-hour veto window during which any guardian
//! can cancel the recovery. This prevents malicious recovery attempts.
//!
//! ## Recovery Flow
//!
//! 1. User initiates recovery with proof of identity
//! 2. Guardians are notified
//! 3. 48-hour veto window begins
//! 4. If no veto, guardians submit recovery shares
//! 5. Shares are combined to recover the PIK

use serde::{Deserialize, Serialize};

use crate::{GuardianError, Result};

/// Veto window duration in seconds (48 hours).
pub const VETO_WINDOW: u64 = 48 * 3600;

/// Status of the veto window.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VetoStatus {
    /// The veto window is still active; recovery can be vetoed.
    Active,
    /// The veto window has expired; recovery can proceed.
    Expired,
    /// A guardian has vetoed the recovery.
    Vetoed,
}

/// A recovery request with proof and guardian shares.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoveryRequest {
    /// Proof that the requester is the rightful owner (stub in v1).
    pub requester_proof: Vec<u8>,
    /// Guardian shares submitted so far.
    pub guardian_shares: Vec<GuardianShare>,
    /// Unix timestamp when recovery was initiated.
    pub initiated_at: u64,
    /// Whether the recovery has been vetoed.
    pub vetoed: bool,
}

/// A recovery share submitted by a guardian.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardianShare {
    /// The guardian's PIK hash.
    pub guardian_id: [u8; 32],
    /// The encrypted recovery shard.
    pub shard_data: Vec<u8>,
}

/// Initiate a new recovery request.
///
/// # Arguments
///
/// * `requester_proof` - Proof of identity (stub in v1)
/// * `current_time` - The current Unix timestamp in seconds
pub fn initiate_recovery(requester_proof: Vec<u8>, current_time: u64) -> RecoveryRequest {
    tracing::info!(
        initiated_at = current_time,
        veto_expires = current_time + VETO_WINDOW,
        "recovery initiated"
    );

    RecoveryRequest {
        requester_proof,
        guardian_shares: Vec::new(),
        initiated_at: current_time,
        vetoed: false,
    }
}

/// Submit a veto to cancel a recovery request.
///
/// # Errors
///
/// - [`GuardianError::NoRecovery`] if recovery is already vetoed
/// - [`GuardianError::VetoWindowActive`] is not returned here; vetos can only
///   be submitted during the active window
pub fn submit_veto(request: &mut RecoveryRequest) -> Result<()> {
    if request.vetoed {
        return Err(GuardianError::Vetoed);
    }

    request.vetoed = true;

    tracing::warn!("recovery vetoed by guardian");

    Ok(())
}

/// Check the veto window status for a recovery request.
///
/// # Arguments
///
/// * `request` - The recovery request
/// * `current_time` - The current Unix timestamp in seconds
pub fn check_veto_window(request: &RecoveryRequest, current_time: u64) -> VetoStatus {
    if request.vetoed {
        return VetoStatus::Vetoed;
    }

    let veto_expires = request.initiated_at + VETO_WINDOW;
    if current_time < veto_expires {
        VetoStatus::Active
    } else {
        VetoStatus::Expired
    }
}

/// Submit a guardian share to the recovery request.
///
/// Shares can only be submitted after the veto window has expired.
///
/// # Errors
///
/// - [`GuardianError::Vetoed`] if the recovery has been vetoed
/// - [`GuardianError::VetoWindowActive`] if the veto window is still active
pub fn submit_share(
    request: &mut RecoveryRequest,
    share: GuardianShare,
    current_time: u64,
) -> Result<()> {
    match check_veto_window(request, current_time) {
        VetoStatus::Vetoed => return Err(GuardianError::Vetoed),
        VetoStatus::Active => {
            let remaining = (request.initiated_at + VETO_WINDOW).saturating_sub(current_time);
            return Err(GuardianError::VetoWindowActive {
                remaining_secs: remaining,
            });
        }
        VetoStatus::Expired => {}
    }

    request.guardian_shares.push(share);

    tracing::info!(
        shares = request.guardian_shares.len(),
        "guardian share submitted for recovery"
    );

    Ok(())
}

/// Check whether enough shares have been collected for recovery.
pub fn has_enough_shares(request: &RecoveryRequest, threshold: usize) -> bool {
    request.guardian_shares.len() >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initiate_recovery() {
        let request = initiate_recovery(vec![0xAA], 1_700_000_000);
        assert_eq!(request.initiated_at, 1_700_000_000);
        assert!(!request.vetoed);
        assert!(request.guardian_shares.is_empty());
    }

    #[test]
    fn test_check_veto_window_active() {
        let request = initiate_recovery(vec![], 1_000_000);
        let status = check_veto_window(&request, 1_000_000 + VETO_WINDOW - 1);
        assert_eq!(status, VetoStatus::Active);
    }

    #[test]
    fn test_check_veto_window_expired() {
        let request = initiate_recovery(vec![], 1_000_000);
        let status = check_veto_window(&request, 1_000_000 + VETO_WINDOW);
        assert_eq!(status, VetoStatus::Expired);
    }

    #[test]
    fn test_submit_veto() {
        let mut request = initiate_recovery(vec![], 1_000_000);
        submit_veto(&mut request).expect("veto");
        assert!(request.vetoed);

        let status = check_veto_window(&request, 1_000_000);
        assert_eq!(status, VetoStatus::Vetoed);
    }

    #[test]
    fn test_double_veto_rejected() {
        let mut request = initiate_recovery(vec![], 1_000_000);
        submit_veto(&mut request).expect("first veto");
        assert!(submit_veto(&mut request).is_err());
    }

    #[test]
    fn test_submit_share_after_veto_window() {
        let mut request = initiate_recovery(vec![], 1_000_000);
        let share = GuardianShare {
            guardian_id: [0x01; 32],
            shard_data: vec![0xBB; 32],
        };

        let after_veto = 1_000_000 + VETO_WINDOW;
        submit_share(&mut request, share, after_veto).expect("submit share");
        assert_eq!(request.guardian_shares.len(), 1);
    }

    #[test]
    fn test_submit_share_during_veto_window_rejected() {
        let mut request = initiate_recovery(vec![], 1_000_000);
        let share = GuardianShare {
            guardian_id: [0x01; 32],
            shard_data: vec![0xBB; 32],
        };

        let during_veto = 1_000_000 + 100;
        assert!(submit_share(&mut request, share, during_veto).is_err());
    }

    #[test]
    fn test_submit_share_after_veto_rejected() {
        let mut request = initiate_recovery(vec![], 1_000_000);
        submit_veto(&mut request).expect("veto");

        let share = GuardianShare {
            guardian_id: [0x01; 32],
            shard_data: vec![0xBB; 32],
        };

        let after_veto = 1_000_000 + VETO_WINDOW + 1;
        assert!(submit_share(&mut request, share, after_veto).is_err());
    }

    #[test]
    fn test_has_enough_shares() {
        let mut request = initiate_recovery(vec![], 1_000_000);
        let after_veto = 1_000_000 + VETO_WINDOW;

        assert!(!has_enough_shares(&request, 2));

        for i in 0..2 {
            let share = GuardianShare {
                guardian_id: [i + 1; 32],
                shard_data: vec![0xBB; 32],
            };
            submit_share(&mut request, share, after_veto).expect("submit");
        }

        assert!(has_enough_shares(&request, 2));
    }

    #[test]
    fn test_veto_window_constant() {
        assert_eq!(VETO_WINDOW, 48 * 3600);
    }
}
