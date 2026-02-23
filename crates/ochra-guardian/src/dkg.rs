//! Guardian DKG (Distributed Key Generation) ceremony.
//!
//! The DKG ceremony creates threshold key shares among guardians so that
//! recovery requires a quorum (default: 2-of-3).

use serde::{Deserialize, Serialize};

use crate::{GuardianError, Result};

/// Default number of guardians.
pub const DEFAULT_GUARDIAN_COUNT: u32 = 3;

/// Default threshold (2-of-3).
pub const DEFAULT_THRESHOLD: u32 = 2;

/// Information about a guardian participating in DKG.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuardianInfo {
    /// Guardian's PIK hash.
    pub pik_hash: [u8; 32],
    /// Display name.
    pub display_name: String,
    /// Guardian's public key for key exchange.
    pub public_key: [u8; 32],
}

/// Status of a DKG ceremony.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DkgStatus {
    /// DKG has been initiated but shares not yet processed.
    Initiated,
    /// Share distribution is in progress.
    SharesDistributed,
    /// DKG completed successfully.
    Complete,
    /// DKG failed.
    Failed,
}

/// Guardian DKG ceremony state.
pub struct GuardianDkg {
    /// The list of participating guardians.
    pub guardians: Vec<GuardianInfo>,
    /// The quorum threshold.
    pub threshold: u32,
    /// Current DKG status.
    pub status: DkgStatus,
    /// Generated shares (one per guardian, populated after process_shares).
    shares: Vec<Vec<u8>>,
}

/// Initiate a DKG ceremony with the given guardians and threshold.
///
/// # Arguments
///
/// * `guardians` - The guardians participating in the ceremony
/// * `threshold` - The minimum number of guardians needed for recovery
///
/// # Errors
///
/// - [`GuardianError::TooFewGuardians`] if fewer than `threshold` guardians
/// - [`GuardianError::DkgError`] if threshold is zero
/// - [`GuardianError::DkgError`] if threshold exceeds the number of guardians
pub fn initiate_dkg(
    guardians: Vec<GuardianInfo>,
    threshold: u32,
) -> Result<GuardianDkg> {
    if threshold == 0 {
        return Err(GuardianError::DkgError(
            "threshold must be at least 1".to_string(),
        ));
    }
    if guardians.len() < threshold as usize {
        return Err(GuardianError::TooFewGuardians {
            actual: guardians.len(),
            minimum: threshold as usize,
        });
    }
    if threshold > guardians.len() as u32 {
        return Err(GuardianError::DkgError(
            "threshold cannot exceed number of guardians".to_string(),
        ));
    }

    tracing::info!(
        guardian_count = guardians.len(),
        threshold,
        "DKG ceremony initiated"
    );

    Ok(GuardianDkg {
        guardians,
        threshold,
        status: DkgStatus::Initiated,
        shares: Vec::new(),
    })
}

impl GuardianDkg {
    /// Process key shares for all guardians.
    ///
    /// In v1, this generates deterministic stub shares using BLAKE3.
    /// In production, this would use a full Shamir secret sharing or
    /// FROST DKG protocol.
    ///
    /// # Errors
    ///
    /// - [`GuardianError::DkgError`] if the ceremony is not in the Initiated state
    pub fn process_shares(&mut self) -> Result<()> {
        if self.status != DkgStatus::Initiated {
            return Err(GuardianError::DkgError(format!(
                "cannot process shares in {:?} state",
                self.status
            )));
        }

        // Generate a stub secret
        let mut secret = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut secret);

        // Generate stub shares: one per guardian
        self.shares = Vec::with_capacity(self.guardians.len());
        for (i, guardian) in self.guardians.iter().enumerate() {
            // Stub share = BLAKE3::hash(secret || guardian_pik || index)
            let idx_bytes = (i as u32).to_le_bytes();
            let fields = ochra_crypto::blake3::encode_multi_field(&[
                &secret[..],
                &guardian.pik_hash[..],
                &idx_bytes,
            ]);
            let share = ochra_crypto::blake3::hash(&fields);
            self.shares.push(share.to_vec());
        }

        self.status = DkgStatus::SharesDistributed;

        tracing::info!(
            shares = self.shares.len(),
            "DKG shares distributed"
        );

        // In a real implementation, guardians would verify and acknowledge.
        // For v1, we mark as complete immediately.
        self.status = DkgStatus::Complete;

        Ok(())
    }

    /// Get the share for a specific guardian by index.
    pub fn get_share(&self, guardian_index: usize) -> Option<&[u8]> {
        self.shares.get(guardian_index).map(|s| s.as_slice())
    }

    /// Get the number of guardians.
    pub fn guardian_count(&self) -> usize {
        self.guardians.len()
    }

    /// Check if the DKG ceremony is complete.
    pub fn is_complete(&self) -> bool {
        self.status == DkgStatus::Complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_guardians(n: usize) -> Vec<GuardianInfo> {
        (0..n)
            .map(|i| GuardianInfo {
                pik_hash: [i as u8 + 1; 32],
                display_name: format!("Guardian {i}"),
                public_key: [i as u8 + 100; 32],
            })
            .collect()
    }

    #[test]
    fn test_initiate_dkg_default() {
        let guardians = make_guardians(3);
        let dkg = initiate_dkg(guardians, DEFAULT_THRESHOLD).expect("initiate");
        assert_eq!(dkg.guardian_count(), 3);
        assert_eq!(dkg.threshold, 2);
        assert_eq!(dkg.status, DkgStatus::Initiated);
    }

    #[test]
    fn test_initiate_dkg_too_few() {
        let guardians = make_guardians(1);
        let result = initiate_dkg(guardians, 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_initiate_dkg_zero_threshold() {
        let guardians = make_guardians(3);
        let result = initiate_dkg(guardians, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_shares() {
        let guardians = make_guardians(3);
        let mut dkg = initiate_dkg(guardians, 2).expect("initiate");
        dkg.process_shares().expect("process");
        assert!(dkg.is_complete());
        assert!(dkg.get_share(0).is_some());
        assert!(dkg.get_share(1).is_some());
        assert!(dkg.get_share(2).is_some());
        assert!(dkg.get_share(3).is_none());
    }

    #[test]
    fn test_process_shares_twice_rejected() {
        let guardians = make_guardians(3);
        let mut dkg = initiate_dkg(guardians, 2).expect("initiate");
        dkg.process_shares().expect("first process");
        assert!(dkg.process_shares().is_err());
    }

    #[test]
    fn test_shares_are_distinct() {
        let guardians = make_guardians(3);
        let mut dkg = initiate_dkg(guardians, 2).expect("initiate");
        dkg.process_shares().expect("process");

        let s0 = dkg.get_share(0).expect("share 0");
        let s1 = dkg.get_share(1).expect("share 1");
        let s2 = dkg.get_share(2).expect("share 2");
        assert_ne!(s0, s1);
        assert_ne!(s1, s2);
        assert_ne!(s0, s2);
    }
}
