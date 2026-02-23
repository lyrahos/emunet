//! DKG ceremony coordination.
//!
//! Manages the state machine for a FROST Distributed Key Generation (DKG)
//! ceremony. The DKG proceeds through three rounds:
//!
//! 1. **Round 1**: Each participant generates and broadcasts a commitment.
//! 2. **Round 2**: Each participant generates and distributes secret shares.
//! 3. **Round 3**: Each participant verifies received shares and computes
//!    their key package.
//!
//! This module coordinates the round transitions and validates that all
//! participants have contributed before advancing.

use std::collections::{HashMap, HashSet};

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{FrostCoordError, Result};

/// The current round of a DKG ceremony.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CeremonyRound {
    /// Round 1: collecting participant commitments.
    Round1,
    /// Round 2: distributing secret shares.
    Round2,
    /// Round 3: verifying shares and computing key packages.
    Round3,
    /// The ceremony is complete.
    Complete,
}

impl std::fmt::Display for CeremonyRound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CeremonyRound::Round1 => write!(f, "round1"),
            CeremonyRound::Round2 => write!(f, "round2"),
            CeremonyRound::Round3 => write!(f, "round3"),
            CeremonyRound::Complete => write!(f, "complete"),
        }
    }
}

/// A participant's commitment for Round 1.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round1Commitment {
    /// The participant's node ID.
    pub participant_id: [u8; 32],
    /// The commitment data (opaque bytes).
    pub commitment: Vec<u8>,
}

/// A participant's secret share package for Round 2.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round2SharePackage {
    /// The sender's node ID.
    pub sender_id: [u8; 32],
    /// The recipient's node ID.
    pub recipient_id: [u8; 32],
    /// The encrypted share data.
    pub encrypted_share: Vec<u8>,
}

/// A participant's verification result for Round 3.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Round3Verification {
    /// The participant's node ID.
    pub participant_id: [u8; 32],
    /// Whether the participant verified all shares successfully.
    pub verified: bool,
    /// The participant's public key share (if verified).
    pub public_key_share: Option<Vec<u8>>,
}

/// DKG ceremony coordinator.
///
/// Tracks the state of a DKG ceremony, including which participants
/// have contributed in each round and when rounds are complete.
pub struct DkgCeremony {
    /// Unique ceremony identifier.
    pub ceremony_id: [u8; 32],
    /// The threshold (minimum signers needed).
    pub threshold: u16,
    /// Current round of the ceremony.
    pub round: CeremonyRound,
    /// Set of participant node IDs.
    participants: HashSet<[u8; 32]>,
    /// Round 1 commitments received.
    round1_commitments: HashMap<[u8; 32], Round1Commitment>,
    /// Round 2 share packages received (keyed by sender).
    round2_shares: HashMap<[u8; 32], Vec<Round2SharePackage>>,
    /// Round 3 verifications received.
    round3_verifications: HashMap<[u8; 32], Round3Verification>,
}

/// Start a new DKG ceremony.
///
/// # Arguments
///
/// * `participants` - The set of participant node IDs.
/// * `threshold` - The signing threshold (t in t-of-n).
///
/// # Returns
///
/// A new [`DkgCeremony`] in Round 1.
pub fn start_ceremony(participants: Vec<[u8; 32]>, threshold: u16) -> Result<DkgCeremony> {
    if participants.is_empty() {
        return Err(FrostCoordError::Quorum(
            "no participants provided".to_string(),
        ));
    }
    if threshold == 0 || threshold as usize > participants.len() {
        return Err(FrostCoordError::Quorum(format!(
            "invalid threshold {threshold} for {} participants",
            participants.len()
        )));
    }

    // Derive ceremony ID from participants and threshold.
    let mut input_parts: Vec<&[u8]> = participants.iter().map(|p| p.as_slice()).collect();
    let threshold_bytes = threshold.to_le_bytes();
    input_parts.push(&threshold_bytes);
    let input = blake3::encode_multi_field(&input_parts);
    let ceremony_id = blake3::hash(&input);

    let participant_set: HashSet<[u8; 32]> = participants.into_iter().collect();

    tracing::info!(
        ceremony_id = hex::encode(ceremony_id),
        participants = participant_set.len(),
        threshold,
        "starting DKG ceremony"
    );

    Ok(DkgCeremony {
        ceremony_id,
        threshold,
        round: CeremonyRound::Round1,
        participants: participant_set,
        round1_commitments: HashMap::new(),
        round2_shares: HashMap::new(),
        round3_verifications: HashMap::new(),
    })
}

impl DkgCeremony {
    /// Get the current ceremony round.
    pub fn current_round(&self) -> &CeremonyRound {
        &self.round
    }

    /// Get the number of participants.
    pub fn participant_count(&self) -> usize {
        self.participants.len()
    }

    /// Check if a node is a participant.
    pub fn is_participant(&self, node_id: &[u8; 32]) -> bool {
        self.participants.contains(node_id)
    }

    /// Process a Round 1 commitment from a participant.
    ///
    /// When all participants have submitted commitments, the ceremony
    /// automatically advances to Round 2.
    pub fn process_round1(&mut self, commitment: Round1Commitment) -> Result<()> {
        if self.round != CeremonyRound::Round1 {
            return Err(FrostCoordError::InvalidState {
                expected: "round1".to_string(),
                actual: self.round.to_string(),
            });
        }

        if !self.participants.contains(&commitment.participant_id) {
            return Err(FrostCoordError::UnknownSigner(hex::encode(
                commitment.participant_id,
            )));
        }

        if self
            .round1_commitments
            .contains_key(&commitment.participant_id)
        {
            return Err(FrostCoordError::DuplicateContribution(hex::encode(
                commitment.participant_id,
            )));
        }

        let pid = commitment.participant_id;
        self.round1_commitments.insert(pid, commitment);

        tracing::debug!(
            ceremony_id = hex::encode(self.ceremony_id),
            participant = hex::encode(pid),
            progress = format!(
                "{}/{}",
                self.round1_commitments.len(),
                self.participants.len()
            ),
            "received Round 1 commitment"
        );

        // Advance to Round 2 when all commitments are collected.
        if self.round1_commitments.len() == self.participants.len() {
            self.round = CeremonyRound::Round2;
            tracing::info!(
                ceremony_id = hex::encode(self.ceremony_id),
                "advancing to Round 2"
            );
        }

        Ok(())
    }

    /// Process a Round 2 share package from a participant.
    ///
    /// When all participants have submitted their share packages, the
    /// ceremony advances to Round 3.
    pub fn process_round2(&mut self, share_package: Round2SharePackage) -> Result<()> {
        if self.round != CeremonyRound::Round2 {
            return Err(FrostCoordError::InvalidState {
                expected: "round2".to_string(),
                actual: self.round.to_string(),
            });
        }

        if !self.participants.contains(&share_package.sender_id) {
            return Err(FrostCoordError::UnknownSigner(hex::encode(
                share_package.sender_id,
            )));
        }

        if !self.participants.contains(&share_package.recipient_id) {
            return Err(FrostCoordError::UnknownSigner(hex::encode(
                share_package.recipient_id,
            )));
        }

        let sender = share_package.sender_id;
        self.round2_shares
            .entry(sender)
            .or_default()
            .push(share_package);

        tracing::debug!(
            ceremony_id = hex::encode(self.ceremony_id),
            sender = hex::encode(sender),
            progress = format!("{}/{}", self.round2_shares.len(), self.participants.len()),
            "received Round 2 share package"
        );

        // Advance to Round 3 when all participants have sent shares to all others.
        // Each participant must send (n-1) share packages.
        let expected_shares_per_sender = self.participants.len() - 1;
        let all_senders_complete = self.round2_shares.len() == self.participants.len()
            && self
                .round2_shares
                .values()
                .all(|shares| shares.len() >= expected_shares_per_sender);
        if all_senders_complete {
            self.round = CeremonyRound::Round3;
            tracing::info!(
                ceremony_id = hex::encode(self.ceremony_id),
                "advancing to Round 3"
            );
        }

        Ok(())
    }

    /// Process a Round 3 verification from a participant.
    ///
    /// When all participants have verified their shares, the ceremony
    /// is marked as complete.
    pub fn process_round3(&mut self, verification: Round3Verification) -> Result<()> {
        if self.round != CeremonyRound::Round3 {
            return Err(FrostCoordError::InvalidState {
                expected: "round3".to_string(),
                actual: self.round.to_string(),
            });
        }

        if !self.participants.contains(&verification.participant_id) {
            return Err(FrostCoordError::UnknownSigner(hex::encode(
                verification.participant_id,
            )));
        }

        if self
            .round3_verifications
            .contains_key(&verification.participant_id)
        {
            return Err(FrostCoordError::DuplicateContribution(hex::encode(
                verification.participant_id,
            )));
        }

        if !verification.verified {
            tracing::warn!(
                ceremony_id = hex::encode(self.ceremony_id),
                participant = hex::encode(verification.participant_id),
                "participant failed share verification"
            );
        }

        let pid = verification.participant_id;
        self.round3_verifications.insert(pid, verification);

        tracing::debug!(
            ceremony_id = hex::encode(self.ceremony_id),
            participant = hex::encode(pid),
            progress = format!(
                "{}/{}",
                self.round3_verifications.len(),
                self.participants.len()
            ),
            "received Round 3 verification"
        );

        // Complete when all verifications are collected.
        if self.round3_verifications.len() == self.participants.len() {
            self.round = CeremonyRound::Complete;
            tracing::info!(
                ceremony_id = hex::encode(self.ceremony_id),
                "DKG ceremony complete"
            );
        }

        Ok(())
    }

    /// Check if all participants verified successfully.
    ///
    /// Only valid after the ceremony is complete.
    pub fn all_verified(&self) -> bool {
        self.round == CeremonyRound::Complete
            && self.round3_verifications.values().all(|v| v.verified)
    }

    /// Get the Round 1 commitments (available after Round 1 completes).
    pub fn commitments(&self) -> &HashMap<[u8; 32], Round1Commitment> {
        &self.round1_commitments
    }

    /// Get the Round 3 verifications (available after ceremony completes).
    pub fn verifications(&self) -> &HashMap<[u8; 32], Round3Verification> {
        &self.round3_verifications
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u8) -> [u8; 32] {
        [id; 32]
    }

    fn make_participants(n: u8) -> Vec<[u8; 32]> {
        (1..=n).map(node).collect()
    }

    #[test]
    fn test_start_ceremony() {
        let participants = make_participants(5);
        let ceremony = start_ceremony(participants, 3).expect("start");
        assert_eq!(*ceremony.current_round(), CeremonyRound::Round1);
        assert_eq!(ceremony.participant_count(), 5);
        assert_eq!(ceremony.threshold, 3);
    }

    #[test]
    fn test_start_ceremony_no_participants() {
        let result = start_ceremony(vec![], 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_start_ceremony_invalid_threshold() {
        let result = start_ceremony(make_participants(3), 5);
        assert!(result.is_err());
    }

    #[test]
    fn test_round1_progression() {
        let participants = make_participants(3);
        let mut ceremony = start_ceremony(participants, 2).expect("start");

        for i in 1..=3u8 {
            ceremony
                .process_round1(Round1Commitment {
                    participant_id: node(i),
                    commitment: vec![i; 32],
                })
                .expect("round1");
        }

        assert_eq!(*ceremony.current_round(), CeremonyRound::Round2);
    }

    #[test]
    fn test_round1_duplicate_rejected() {
        let participants = make_participants(3);
        let mut ceremony = start_ceremony(participants, 2).expect("start");

        ceremony
            .process_round1(Round1Commitment {
                participant_id: node(1),
                commitment: vec![1; 32],
            })
            .expect("round1");

        let result = ceremony.process_round1(Round1Commitment {
            participant_id: node(1),
            commitment: vec![1; 32],
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_round1_unknown_signer_rejected() {
        let participants = make_participants(3);
        let mut ceremony = start_ceremony(participants, 2).expect("start");

        let result = ceremony.process_round1(Round1Commitment {
            participant_id: node(99),
            commitment: vec![99; 32],
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_full_ceremony() {
        let participants = make_participants(3);
        let mut ceremony = start_ceremony(participants, 2).expect("start");

        // Round 1.
        for i in 1..=3u8 {
            ceremony
                .process_round1(Round1Commitment {
                    participant_id: node(i),
                    commitment: vec![i; 32],
                })
                .expect("round1");
        }
        assert_eq!(*ceremony.current_round(), CeremonyRound::Round2);

        // Round 2.
        for sender in 1..=3u8 {
            for recipient in 1..=3u8 {
                if sender != recipient {
                    ceremony
                        .process_round2(Round2SharePackage {
                            sender_id: node(sender),
                            recipient_id: node(recipient),
                            encrypted_share: vec![sender ^ recipient; 64],
                        })
                        .expect("round2");
                }
            }
        }
        assert_eq!(*ceremony.current_round(), CeremonyRound::Round3);

        // Round 3.
        for i in 1..=3u8 {
            ceremony
                .process_round3(Round3Verification {
                    participant_id: node(i),
                    verified: true,
                    public_key_share: Some(vec![i; 32]),
                })
                .expect("round3");
        }
        assert_eq!(*ceremony.current_round(), CeremonyRound::Complete);
        assert!(ceremony.all_verified());
    }

    #[test]
    fn test_round2_wrong_state_rejected() {
        let participants = make_participants(3);
        let mut ceremony = start_ceremony(participants, 2).expect("start");

        let result = ceremony.process_round2(Round2SharePackage {
            sender_id: node(1),
            recipient_id: node(2),
            encrypted_share: vec![0; 64],
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_round3_failed_verification() {
        let participants = make_participants(2);
        let mut ceremony = start_ceremony(participants, 2).expect("start");

        // Fast-forward through rounds 1 and 2.
        for i in 1..=2u8 {
            ceremony
                .process_round1(Round1Commitment {
                    participant_id: node(i),
                    commitment: vec![i; 32],
                })
                .expect("round1");
        }
        for sender in 1..=2u8 {
            for recipient in 1..=2u8 {
                if sender != recipient {
                    ceremony
                        .process_round2(Round2SharePackage {
                            sender_id: node(sender),
                            recipient_id: node(recipient),
                            encrypted_share: vec![0; 64],
                        })
                        .expect("round2");
                }
            }
        }

        // Round 3 with one failure.
        ceremony
            .process_round3(Round3Verification {
                participant_id: node(1),
                verified: true,
                public_key_share: Some(vec![1; 32]),
            })
            .expect("round3");
        ceremony
            .process_round3(Round3Verification {
                participant_id: node(2),
                verified: false,
                public_key_share: None,
            })
            .expect("round3");

        assert_eq!(*ceremony.current_round(), CeremonyRound::Complete);
        assert!(!ceremony.all_verified());
    }

    #[test]
    fn test_ceremony_id_deterministic() {
        let p1 = make_participants(3);
        let p2 = make_participants(3);
        let c1 = start_ceremony(p1, 2).expect("start");
        let c2 = start_ceremony(p2, 2).expect("start");
        assert_eq!(c1.ceremony_id, c2.ceremony_id);
    }
}
