//! Pull-based claims for VYS rewards.
//!
//! Nodes claim their accumulated rewards by submitting a claim request.
//! The claim is verified and the rewards are disbursed.

use serde::{Deserialize, Serialize};

use crate::accounting::VysAccumulator;
use crate::{Result, VysError};

/// A request to claim accumulated VYS rewards.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClaimRequest {
    /// The node's identifier (PIK hash).
    pub node_id: [u8; 32],
    /// The amount of micro-seeds being claimed.
    pub amount: u64,
    /// The epoch at which the claim is being made.
    pub epoch: u64,
    /// Proof of entitlement (signature or ZK proof, stub in v1).
    pub proof: Vec<u8>,
}

/// Result of a successful claim.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClaimResult {
    /// The amount of micro-seeds disbursed.
    pub disbursed: u64,
    /// The epoch at which the claim was processed.
    pub epoch: u64,
}

/// Process a claim request against an accumulator.
///
/// Verifies the claim and, if valid, resets the accumulator's rewards
/// and returns the disbursed amount.
///
/// # Errors
///
/// - [`VysError::NoRewards`] if the accumulator has no claimable rewards
/// - [`VysError::InvalidProof`] if the claim fails verification
pub fn process_claim(request: &ClaimRequest, accumulator: &mut VysAccumulator) -> Result<u64> {
    if !verify_claim(request) {
        return Err(VysError::InvalidProof(
            "claim verification failed".to_string(),
        ));
    }

    let claimable = accumulator.claimable_amount();
    if claimable == 0 {
        return Err(VysError::NoRewards);
    }

    // The claimed amount cannot exceed the available balance
    let disbursed = if request.amount > claimable {
        claimable
    } else {
        request.amount
    };

    // For partial claims, just subtract; for full claims, reset
    if disbursed >= claimable {
        accumulator.reset_rewards(request.epoch);
    } else {
        // Partial claim: reduce accumulated rewards
        accumulator.accumulated_rewards = accumulator.accumulated_rewards.saturating_sub(disbursed);
        accumulator.last_claim_epoch = request.epoch;
    }

    tracing::info!(disbursed, epoch = request.epoch, "VYS claim processed");

    Ok(disbursed)
}

/// Verify a claim request (stub in v1).
///
/// In v1, verification checks basic well-formedness. In production, this
/// would verify a cryptographic proof of entitlement.
pub fn verify_claim(request: &ClaimRequest) -> bool {
    // Basic validation
    if request.amount == 0 {
        return false;
    }
    if request.node_id == [0u8; 32] {
        return false;
    }
    // In v1, proof is not verified cryptographically
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_claim_full() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulate(1_000_000, 1.0, 1.0).expect("accumulate");

        let request = ClaimRequest {
            node_id: [0x01; 32],
            amount: 1_000_000,
            epoch: 5,
            proof: vec![0xAA],
        };

        let disbursed = process_claim(&request, &mut acc).expect("claim");
        assert_eq!(disbursed, 1_000_000);
        assert_eq!(acc.claimable_amount(), 0);
        assert_eq!(acc.last_claim_epoch, 5);
    }

    #[test]
    fn test_process_claim_partial() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulate(1_000_000, 1.0, 1.0).expect("accumulate");

        let request = ClaimRequest {
            node_id: [0x01; 32],
            amount: 500_000,
            epoch: 5,
            proof: vec![0xAA],
        };

        let disbursed = process_claim(&request, &mut acc).expect("claim");
        assert_eq!(disbursed, 500_000);
        assert_eq!(acc.claimable_amount(), 500_000);
    }

    #[test]
    fn test_process_claim_no_rewards() {
        let mut acc = VysAccumulator::new(1.0);

        let request = ClaimRequest {
            node_id: [0x01; 32],
            amount: 1000,
            epoch: 5,
            proof: vec![0xAA],
        };

        let result = process_claim(&request, &mut acc);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_claim_exceeds_balance() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulate(500, 1.0, 1.0).expect("accumulate");

        let request = ClaimRequest {
            node_id: [0x01; 32],
            amount: 1000, // more than available
            epoch: 5,
            proof: vec![0xAA],
        };

        let disbursed = process_claim(&request, &mut acc).expect("claim");
        assert_eq!(disbursed, 500);
    }

    #[test]
    fn test_verify_claim_zero_amount() {
        let request = ClaimRequest {
            node_id: [0x01; 32],
            amount: 0,
            epoch: 5,
            proof: vec![],
        };
        assert!(!verify_claim(&request));
    }

    #[test]
    fn test_verify_claim_zero_node_id() {
        let request = ClaimRequest {
            node_id: [0x00; 32],
            amount: 1000,
            epoch: 5,
            proof: vec![],
        };
        assert!(!verify_claim(&request));
    }

    #[test]
    fn test_verify_claim_valid() {
        let request = ClaimRequest {
            node_id: [0x01; 32],
            amount: 1000,
            epoch: 5,
            proof: vec![0xAA],
        };
        assert!(verify_claim(&request));
    }
}
