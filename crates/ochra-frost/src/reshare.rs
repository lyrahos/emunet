//! Proactive secret resharing between quorums.
//!
//! Allows the FROST group public key to be preserved while transferring
//! signing authority from an old quorum to a new quorum. This is essential
//! for quorum rotation without changing the group's public key.
//!
//! ## Reshare Protocol
//!
//! 1. **Phase 1 (Commitments)**: Old quorum members generate commitments
//!    for a new Shamir sharing of their key shares.
//! 2. **Phase 2 (Distribution)**: Old quorum members distribute new shares
//!    to new quorum members.
//! 3. **Phase 3 (Verification)**: New quorum members verify their shares
//!    and confirm the group public key is unchanged.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{FrostCoordError, Result};

/// State of a resharing ceremony.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReshareState {
    /// Ceremony initialized but not started.
    Idle,
    /// Collecting commitments from old quorum members.
    Phase1Commitments,
    /// Distributing new shares to new quorum members.
    Phase2Distribution,
    /// New quorum members verifying their shares.
    Phase3Verification,
    /// Reshare completed successfully.
    Complete,
    /// Reshare failed.
    Failed,
}

impl std::fmt::Display for ReshareState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReshareState::Idle => write!(f, "idle"),
            ReshareState::Phase1Commitments => write!(f, "phase1_commitments"),
            ReshareState::Phase2Distribution => write!(f, "phase2_distribution"),
            ReshareState::Phase3Verification => write!(f, "phase3_verification"),
            ReshareState::Complete => write!(f, "complete"),
            ReshareState::Failed => write!(f, "failed"),
        }
    }
}

/// A commitment from an old quorum member during Phase 1.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReshareCommitment {
    /// The old quorum member's node ID.
    pub participant_id: [u8; 32],
    /// The commitment data (opaque bytes).
    pub commitment: Vec<u8>,
}

/// A share distribution from an old quorum member to a new quorum member.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReshareSharePackage {
    /// The old quorum member's node ID (sender).
    pub sender_id: [u8; 32],
    /// The new quorum member's node ID (recipient).
    pub recipient_id: [u8; 32],
    /// The encrypted new share.
    pub encrypted_share: Vec<u8>,
}

/// A verification result from a new quorum member.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReshareVerification {
    /// The new quorum member's node ID.
    pub participant_id: [u8; 32],
    /// Whether the verification succeeded.
    pub verified: bool,
    /// The new public key share (if verified).
    pub public_key_share: Option<Vec<u8>>,
}

/// Resharing ceremony between old and new quorum.
///
/// Coordinates the transfer of signing authority from one quorum to
/// another while preserving the group public key.
pub struct ReshareCeremony {
    /// Old quorum member IDs.
    old_quorum: HashSet<[u8; 32]>,
    /// New quorum member IDs.
    new_quorum: HashSet<[u8; 32]>,
    /// New signing threshold.
    new_threshold: u16,
    /// Current state.
    state: ReshareState,
    /// Phase 1 commitments from old quorum members.
    commitments: HashMap<[u8; 32], ReshareCommitment>,
    /// Phase 2 share distributions.
    distributions: HashMap<[u8; 32], Vec<ReshareSharePackage>>,
    /// Phase 3 verifications from new quorum members.
    verifications: HashMap<[u8; 32], ReshareVerification>,
}

/// Initiate a resharing ceremony.
///
/// # Arguments
///
/// * `old_quorum` - Node IDs of the current quorum members.
/// * `new_quorum` - Node IDs of the new quorum members.
/// * `new_threshold` - The signing threshold for the new quorum.
pub fn initiate_reshare(
    old_quorum: Vec<[u8; 32]>,
    new_quorum: Vec<[u8; 32]>,
    new_threshold: u16,
) -> Result<ReshareCeremony> {
    if old_quorum.is_empty() {
        return Err(FrostCoordError::Reshare(
            "old quorum is empty".to_string(),
        ));
    }
    if new_quorum.is_empty() {
        return Err(FrostCoordError::Reshare(
            "new quorum is empty".to_string(),
        ));
    }
    if new_threshold == 0 || new_threshold as usize > new_quorum.len() {
        return Err(FrostCoordError::Reshare(format!(
            "invalid threshold {new_threshold} for {} new members",
            new_quorum.len()
        )));
    }

    let old_set: HashSet<[u8; 32]> = old_quorum.into_iter().collect();
    let new_set: HashSet<[u8; 32]> = new_quorum.into_iter().collect();

    tracing::info!(
        old_size = old_set.len(),
        new_size = new_set.len(),
        new_threshold,
        "initiating reshare ceremony"
    );

    Ok(ReshareCeremony {
        old_quorum: old_set,
        new_quorum: new_set,
        new_threshold,
        state: ReshareState::Idle,
        commitments: HashMap::new(),
        distributions: HashMap::new(),
        verifications: HashMap::new(),
    })
}

impl ReshareCeremony {
    /// Get the current state of the ceremony.
    pub fn state(&self) -> ReshareState {
        self.state
    }

    /// Get the new threshold.
    pub fn new_threshold(&self) -> u16 {
        self.new_threshold
    }

    /// Get the old quorum size.
    pub fn old_quorum_size(&self) -> usize {
        self.old_quorum.len()
    }

    /// Get the new quorum size.
    pub fn new_quorum_size(&self) -> usize {
        self.new_quorum.len()
    }

    /// Start the ceremony (transition from Idle to Phase 1).
    pub fn start(&mut self) -> Result<()> {
        if self.state != ReshareState::Idle {
            return Err(FrostCoordError::InvalidState {
                expected: "idle".to_string(),
                actual: self.state.to_string(),
            });
        }
        self.state = ReshareState::Phase1Commitments;

        tracing::info!("reshare ceremony started: Phase 1 (commitments)");
        Ok(())
    }

    /// Submit a Phase 1 commitment from an old quorum member.
    ///
    /// When all old quorum members have committed, the ceremony
    /// advances to Phase 2.
    pub fn submit_commitment(&mut self, commitment: ReshareCommitment) -> Result<()> {
        if self.state != ReshareState::Phase1Commitments {
            return Err(FrostCoordError::InvalidState {
                expected: "phase1_commitments".to_string(),
                actual: self.state.to_string(),
            });
        }

        if !self.old_quorum.contains(&commitment.participant_id) {
            return Err(FrostCoordError::UnknownSigner(
                hex::encode(commitment.participant_id),
            ));
        }

        if self.commitments.contains_key(&commitment.participant_id) {
            return Err(FrostCoordError::DuplicateContribution(
                hex::encode(commitment.participant_id),
            ));
        }

        let pid = commitment.participant_id;
        self.commitments.insert(pid, commitment);

        if self.commitments.len() == self.old_quorum.len() {
            self.state = ReshareState::Phase2Distribution;
            tracing::info!("reshare advancing to Phase 2 (distribution)");
        }

        Ok(())
    }

    /// Submit a Phase 2 share distribution from an old quorum member.
    ///
    /// When all old quorum members have distributed shares, the
    /// ceremony advances to Phase 3.
    pub fn submit_distribution(&mut self, package: ReshareSharePackage) -> Result<()> {
        if self.state != ReshareState::Phase2Distribution {
            return Err(FrostCoordError::InvalidState {
                expected: "phase2_distribution".to_string(),
                actual: self.state.to_string(),
            });
        }

        if !self.old_quorum.contains(&package.sender_id) {
            return Err(FrostCoordError::UnknownSigner(
                hex::encode(package.sender_id),
            ));
        }

        if !self.new_quorum.contains(&package.recipient_id) {
            return Err(FrostCoordError::UnknownSigner(
                hex::encode(package.recipient_id),
            ));
        }

        let sender = package.sender_id;
        self.distributions
            .entry(sender)
            .or_default()
            .push(package);

        // Advance to Phase 3 when all old quorum members have distributed
        // shares to all new quorum members.
        let expected_per_sender = self.new_quorum.len();
        let all_senders_complete = self.distributions.len() == self.old_quorum.len()
            && self
                .distributions
                .values()
                .all(|shares| shares.len() >= expected_per_sender);
        if all_senders_complete {
            self.state = ReshareState::Phase3Verification;
            tracing::info!("reshare advancing to Phase 3 (verification)");
        }

        Ok(())
    }

    /// Submit a Phase 3 verification from a new quorum member.
    ///
    /// When all new quorum members have verified, the ceremony is
    /// marked as complete (or failed if any verification failed).
    pub fn submit_verification(&mut self, verification: ReshareVerification) -> Result<()> {
        if self.state != ReshareState::Phase3Verification {
            return Err(FrostCoordError::InvalidState {
                expected: "phase3_verification".to_string(),
                actual: self.state.to_string(),
            });
        }

        if !self.new_quorum.contains(&verification.participant_id) {
            return Err(FrostCoordError::UnknownSigner(
                hex::encode(verification.participant_id),
            ));
        }

        if self.verifications.contains_key(&verification.participant_id) {
            return Err(FrostCoordError::DuplicateContribution(
                hex::encode(verification.participant_id),
            ));
        }

        let pid = verification.participant_id;
        self.verifications.insert(pid, verification);

        if self.verifications.len() == self.new_quorum.len() {
            if self.all_verified() {
                self.state = ReshareState::Complete;
                tracing::info!("reshare ceremony complete");
            } else {
                self.state = ReshareState::Failed;
                tracing::warn!("reshare ceremony failed: not all verifications passed");
            }
        }

        Ok(())
    }

    /// Check if all new quorum members verified successfully.
    pub fn all_verified(&self) -> bool {
        self.verifications.values().all(|v| v.verified)
    }

    /// Mark the ceremony as failed.
    pub fn fail(&mut self) {
        self.state = ReshareState::Failed;
        tracing::warn!("reshare ceremony explicitly failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u8) -> [u8; 32] {
        [id; 32]
    }

    #[test]
    fn test_initiate_reshare() {
        let old = vec![node(1), node(2), node(3)];
        let new = vec![node(4), node(5), node(6)];
        let ceremony = initiate_reshare(old, new, 2).expect("initiate");
        assert_eq!(ceremony.state(), ReshareState::Idle);
        assert_eq!(ceremony.old_quorum_size(), 3);
        assert_eq!(ceremony.new_quorum_size(), 3);
        assert_eq!(ceremony.new_threshold(), 2);
    }

    #[test]
    fn test_initiate_empty_old_quorum() {
        let result = initiate_reshare(vec![], vec![node(1)], 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_initiate_empty_new_quorum() {
        let result = initiate_reshare(vec![node(1)], vec![], 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_initiate_invalid_threshold() {
        let result = initiate_reshare(vec![node(1)], vec![node(2), node(3)], 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_reshare_ceremony() {
        let old = vec![node(1), node(2), node(3)];
        let new = vec![node(4), node(5), node(6)];
        let mut ceremony = initiate_reshare(old, new, 2).expect("initiate");

        // Start.
        ceremony.start().expect("start");
        assert_eq!(ceremony.state(), ReshareState::Phase1Commitments);

        // Phase 1: commitments from old quorum.
        for i in 1..=3u8 {
            ceremony
                .submit_commitment(ReshareCommitment {
                    participant_id: node(i),
                    commitment: vec![i; 32],
                })
                .expect("commitment");
        }
        assert_eq!(ceremony.state(), ReshareState::Phase2Distribution);

        // Phase 2: distributions from old to new.
        for sender in 1..=3u8 {
            for recipient in 4..=6u8 {
                ceremony
                    .submit_distribution(ReshareSharePackage {
                        sender_id: node(sender),
                        recipient_id: node(recipient),
                        encrypted_share: vec![sender ^ recipient; 64],
                    })
                    .expect("distribution");
            }
        }
        assert_eq!(ceremony.state(), ReshareState::Phase3Verification);

        // Phase 3: verifications from new quorum.
        for i in 4..=6u8 {
            ceremony
                .submit_verification(ReshareVerification {
                    participant_id: node(i),
                    verified: true,
                    public_key_share: Some(vec![i; 32]),
                })
                .expect("verification");
        }
        assert_eq!(ceremony.state(), ReshareState::Complete);
        assert!(ceremony.all_verified());
    }

    #[test]
    fn test_reshare_verification_failure() {
        let old = vec![node(1), node(2)];
        let new = vec![node(3), node(4)];
        let mut ceremony = initiate_reshare(old, new, 2).expect("initiate");

        ceremony.start().expect("start");

        // Phase 1.
        for i in 1..=2u8 {
            ceremony
                .submit_commitment(ReshareCommitment {
                    participant_id: node(i),
                    commitment: vec![i; 32],
                })
                .expect("commitment");
        }

        // Phase 2.
        for sender in 1..=2u8 {
            for recipient in 3..=4u8 {
                ceremony
                    .submit_distribution(ReshareSharePackage {
                        sender_id: node(sender),
                        recipient_id: node(recipient),
                        encrypted_share: vec![0; 64],
                    })
                    .expect("distribution");
            }
        }

        // Phase 3: one verification fails.
        ceremony
            .submit_verification(ReshareVerification {
                participant_id: node(3),
                verified: true,
                public_key_share: Some(vec![3; 32]),
            })
            .expect("verification");
        ceremony
            .submit_verification(ReshareVerification {
                participant_id: node(4),
                verified: false,
                public_key_share: None,
            })
            .expect("verification");

        assert_eq!(ceremony.state(), ReshareState::Failed);
        assert!(!ceremony.all_verified());
    }

    #[test]
    fn test_wrong_state_rejected() {
        let old = vec![node(1)];
        let new = vec![node(2)];
        let mut ceremony = initiate_reshare(old, new, 1).expect("initiate");

        // Cannot submit commitment before starting.
        let result = ceremony.submit_commitment(ReshareCommitment {
            participant_id: node(1),
            commitment: vec![1; 32],
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_commitment_rejected() {
        let old = vec![node(1), node(2)];
        let new = vec![node(3)];
        let mut ceremony = initiate_reshare(old, new, 1).expect("initiate");
        ceremony.start().expect("start");

        ceremony
            .submit_commitment(ReshareCommitment {
                participant_id: node(1),
                commitment: vec![1; 32],
            })
            .expect("commitment");

        let result = ceremony.submit_commitment(ReshareCommitment {
            participant_id: node(1),
            commitment: vec![1; 32],
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_explicit_fail() {
        let old = vec![node(1)];
        let new = vec![node(2)];
        let mut ceremony = initiate_reshare(old, new, 1).expect("initiate");
        ceremony.fail();
        assert_eq!(ceremony.state(), ReshareState::Failed);
    }

    #[test]
    fn test_overlapping_quorums() {
        // Some nodes are in both old and new quorums.
        let old = vec![node(1), node(2), node(3)];
        let new = vec![node(2), node(3), node(4)];
        let ceremony = initiate_reshare(old, new, 2).expect("initiate");
        assert_eq!(ceremony.old_quorum_size(), 3);
        assert_eq!(ceremony.new_quorum_size(), 3);
    }
}
