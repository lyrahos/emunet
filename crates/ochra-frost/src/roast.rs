//! ROAST wrapper for asynchronous liveness.
//!
//! ROAST (Robust Asynchronous Schnorr Threshold) wraps FROST to handle
//! non-responsive signers. It maintains multiple concurrent signing
//! sessions so that even if some signers are unresponsive, the ceremony
//! can complete with the first t-of-n group that responds.
//!
//! ## Design
//!
//! The coordinator maintains a set of "responsive" signers. When a
//! signing request comes in:
//! 1. The coordinator starts a signing session with the current responsive set.
//! 2. If a signer fails to respond, they are removed from the responsive set.
//! 3. A new session is started with a different subset.
//! 4. The first session to collect t valid shares produces the signature.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::{FrostCoordError, Result, MAX_ROAST_SESSIONS};

/// A signature share from a participant.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignatureShare {
    /// The participant's node ID.
    pub participant_id: [u8; 32],
    /// The signature share bytes.
    pub share: Vec<u8>,
}

/// State of a single ROAST signing attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// Collecting nonce commitments.
    CollectingCommitments,
    /// Collecting signature shares.
    CollectingShares,
    /// Successfully produced a signature.
    Complete,
    /// Session failed (too many non-responsive signers).
    Failed,
}

/// A single ROAST signing session attempt.
#[derive(Clone, Debug)]
struct SigningAttempt {
    /// The session's unique index.
    _index: usize,
    /// Participants included in this attempt.
    participants: HashSet<[u8; 32]>,
    /// Shares collected so far.
    shares: HashMap<[u8; 32], SignatureShare>,
    /// Current state.
    state: SessionState,
}

/// ROAST session for coordinating asynchronous threshold signing.
///
/// Manages multiple concurrent signing attempts and collects the
/// first t-of-n valid signature.
pub struct RoastSession {
    /// The message being signed.
    message: Vec<u8>,
    /// Signing threshold (minimum signers needed).
    threshold: usize,
    /// All eligible signers.
    eligible_signers: HashSet<[u8; 32]>,
    /// Currently responsive signers.
    responsive_signers: HashSet<[u8; 32]>,
    /// Active signing attempts.
    attempts: Vec<SigningAttempt>,
    /// The final aggregated signature (if any attempt completed).
    final_signature: Option<Vec<u8>>,
}

impl RoastSession {
    /// Start a new ROAST signing session.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to be signed.
    /// * `eligible_signers` - The full set of eligible signer node IDs.
    /// * `threshold` - The minimum number of signers needed.
    pub fn start_signing(
        message: Vec<u8>,
        eligible_signers: Vec<[u8; 32]>,
        threshold: usize,
    ) -> Result<Self> {
        if eligible_signers.len() < threshold {
            return Err(FrostCoordError::InsufficientSigners {
                required: threshold,
                available: eligible_signers.len(),
            });
        }

        let signer_set: HashSet<[u8; 32]> = eligible_signers.into_iter().collect();

        tracing::info!(
            eligible = signer_set.len(),
            threshold,
            "starting ROAST session"
        );

        Ok(Self {
            message,
            threshold,
            eligible_signers: signer_set.clone(),
            responsive_signers: signer_set,
            attempts: Vec::new(),
            final_signature: None,
        })
    }

    /// Create a new signing attempt with the current responsive signers.
    ///
    /// Returns the index of the new attempt, or an error if the maximum
    /// number of attempts has been reached.
    pub fn new_attempt(&mut self) -> Result<usize> {
        if self.attempts.len() >= MAX_ROAST_SESSIONS {
            return Err(FrostCoordError::InvalidState {
                expected: "below max sessions".to_string(),
                actual: "at max sessions".to_string(),
            });
        }

        if self.responsive_signers.len() < self.threshold {
            return Err(FrostCoordError::InsufficientSigners {
                required: self.threshold,
                available: self.responsive_signers.len(),
            });
        }

        let index = self.attempts.len();
        self.attempts.push(SigningAttempt {
            _index: index,
            participants: self.responsive_signers.clone(),
            shares: HashMap::new(),
            state: SessionState::CollectingCommitments,
        });

        tracing::debug!(
            attempt = index,
            participants = self.responsive_signers.len(),
            "created new ROAST signing attempt"
        );

        Ok(index)
    }

    /// Mark an attempt as ready to collect shares (commitments phase done).
    pub fn advance_to_shares(&mut self, attempt_index: usize) -> Result<()> {
        let attempt =
            self.attempts
                .get_mut(attempt_index)
                .ok_or_else(|| FrostCoordError::InvalidState {
                    expected: format!("attempt {attempt_index} exists"),
                    actual: "attempt not found".to_string(),
                })?;

        if attempt.state != SessionState::CollectingCommitments {
            return Err(FrostCoordError::InvalidState {
                expected: "collecting_commitments".to_string(),
                actual: format!("{:?}", attempt.state),
            });
        }

        attempt.state = SessionState::CollectingShares;
        Ok(())
    }

    /// Receive a signature share from a participant.
    ///
    /// If the share causes an attempt to reach threshold, the session
    /// produces a final signature (returned as `Some`).
    ///
    /// # Arguments
    ///
    /// * `participant` - The participant's node ID.
    /// * `share` - The signature share.
    ///
    /// # Returns
    ///
    /// `Some(signature_bytes)` if this share completed a signing session,
    /// `None` if more shares are needed.
    pub fn receive_share(
        &mut self,
        participant: [u8; 32],
        share: SignatureShare,
    ) -> Result<Option<Vec<u8>>> {
        if !self.eligible_signers.contains(&participant) {
            return Err(FrostCoordError::UnknownSigner(hex::encode(participant)));
        }

        if self.final_signature.is_some() {
            // Already complete; ignore additional shares.
            return Ok(self.final_signature.clone());
        }

        // Add share to all active attempts that include this participant.
        let mut completed_attempt = None;
        for attempt in &mut self.attempts {
            if attempt.state != SessionState::CollectingShares {
                continue;
            }
            if !attempt.participants.contains(&participant) {
                continue;
            }
            if attempt.shares.contains_key(&participant) {
                continue; // Already have a share from this participant.
            }

            attempt.shares.insert(participant, share.clone());

            if attempt.shares.len() >= self.threshold {
                attempt.state = SessionState::Complete;
                completed_attempt = Some(attempt._index);
                break;
            }
        }

        if let Some(attempt_idx) = completed_attempt {
            // Aggregate shares from the completed attempt into a "signature".
            // In a real implementation, this would call FROST aggregation.
            let sig = self.aggregate_shares(attempt_idx)?;
            self.final_signature = Some(sig.clone());

            tracing::info!(attempt = attempt_idx, "ROAST session complete");

            return Ok(Some(sig));
        }

        Ok(None)
    }

    /// Mark a signer as non-responsive.
    ///
    /// Removes them from the responsive set for future attempts.
    pub fn mark_non_responsive(&mut self, signer: &[u8; 32]) {
        self.responsive_signers.remove(signer);

        tracing::debug!(
            signer = hex::encode(signer),
            remaining = self.responsive_signers.len(),
            "marked signer as non-responsive"
        );
    }

    /// Check if the session has completed (produced a signature).
    pub fn is_completed(&self) -> bool {
        self.final_signature.is_some()
    }

    /// Get the final signature if the session is complete.
    pub fn signature(&self) -> Option<&[u8]> {
        self.final_signature.as_deref()
    }

    /// Get the number of active signing attempts.
    pub fn attempt_count(&self) -> usize {
        self.attempts.len()
    }

    /// Get the message being signed.
    pub fn message(&self) -> &[u8] {
        &self.message
    }

    /// Get the signing threshold.
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Get the number of responsive signers.
    pub fn responsive_count(&self) -> usize {
        self.responsive_signers.len()
    }

    /// Aggregate shares from a completed attempt into a signature.
    ///
    /// This v1 implementation hashes all shares together as a placeholder.
    /// A real implementation would call `ochra_crypto::frost::aggregate`.
    fn aggregate_shares(&self, attempt_index: usize) -> Result<Vec<u8>> {
        let attempt =
            self.attempts
                .get(attempt_index)
                .ok_or_else(|| FrostCoordError::InvalidState {
                    expected: format!("attempt {attempt_index} exists"),
                    actual: "attempt not found".to_string(),
                })?;

        // Placeholder aggregation: hash all shares together.
        let mut all_share_data = Vec::new();
        for share in attempt.shares.values() {
            all_share_data.extend_from_slice(&share.share);
        }
        all_share_data.extend_from_slice(&self.message);

        Ok(ochra_crypto::blake3::hash(&all_share_data).to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u8) -> [u8; 32] {
        [id; 32]
    }

    fn make_signers(n: u8) -> Vec<[u8; 32]> {
        (1..=n).map(node).collect()
    }

    #[test]
    fn test_start_session() {
        let session =
            RoastSession::start_signing(b"test".to_vec(), make_signers(5), 3).expect("start");
        assert!(!session.is_completed());
        assert_eq!(session.threshold(), 3);
        assert_eq!(session.responsive_count(), 5);
    }

    #[test]
    fn test_insufficient_signers() {
        let result = RoastSession::start_signing(b"test".to_vec(), make_signers(2), 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_new_attempt() {
        let mut session =
            RoastSession::start_signing(b"test".to_vec(), make_signers(5), 3).expect("start");
        let idx = session.new_attempt().expect("attempt");
        assert_eq!(idx, 0);
        assert_eq!(session.attempt_count(), 1);
    }

    #[test]
    fn test_complete_signing_session() {
        let mut session = RoastSession::start_signing(b"test message".to_vec(), make_signers(5), 3)
            .expect("start");

        let idx = session.new_attempt().expect("attempt");
        session.advance_to_shares(idx).expect("advance");

        // Submit 3 shares (threshold).
        let result1 = session
            .receive_share(
                node(1),
                SignatureShare {
                    participant_id: node(1),
                    share: vec![0x01; 32],
                },
            )
            .expect("share1");
        assert!(result1.is_none());

        let result2 = session
            .receive_share(
                node(2),
                SignatureShare {
                    participant_id: node(2),
                    share: vec![0x02; 32],
                },
            )
            .expect("share2");
        assert!(result2.is_none());

        let result3 = session
            .receive_share(
                node(3),
                SignatureShare {
                    participant_id: node(3),
                    share: vec![0x03; 32],
                },
            )
            .expect("share3");
        assert!(result3.is_some());
        assert!(session.is_completed());
    }

    #[test]
    fn test_mark_non_responsive() {
        let mut session =
            RoastSession::start_signing(b"test".to_vec(), make_signers(5), 3).expect("start");
        session.mark_non_responsive(&node(5));
        assert_eq!(session.responsive_count(), 4);

        session.mark_non_responsive(&node(4));
        assert_eq!(session.responsive_count(), 3);
    }

    #[test]
    fn test_too_many_non_responsive() {
        let mut session =
            RoastSession::start_signing(b"test".to_vec(), make_signers(5), 3).expect("start");
        session.mark_non_responsive(&node(5));
        session.mark_non_responsive(&node(4));
        session.mark_non_responsive(&node(3));

        // Only 2 responsive signers, threshold is 3.
        let result = session.new_attempt();
        assert!(result.is_err());
    }

    #[test]
    fn test_max_attempts() {
        let mut session =
            RoastSession::start_signing(b"test".to_vec(), make_signers(5), 3).expect("start");

        for _ in 0..MAX_ROAST_SESSIONS {
            session.new_attempt().expect("attempt");
        }

        let result = session.new_attempt();
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_signer_share_rejected() {
        let mut session =
            RoastSession::start_signing(b"test".to_vec(), make_signers(3), 2).expect("start");
        let idx = session.new_attempt().expect("attempt");
        session.advance_to_shares(idx).expect("advance");

        let result = session.receive_share(
            node(99),
            SignatureShare {
                participant_id: node(99),
                share: vec![0; 32],
            },
        );
        assert!(result.is_err());
    }
}
