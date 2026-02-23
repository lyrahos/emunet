//! VYS reward accumulator.
//!
//! Tracks reward accumulation for nodes based on their PoSrv contribution.
//! Each node receives a share of the epoch reward pool proportional to its
//! PoSrv score relative to the total network PoSrv.
//!
//! ## Formula
//!
//! ```text
//! node_reward = epoch_pool * (node_posrv / total_posrv)
//! ```

use serde::{Deserialize, Serialize};

use crate::{Result, VysError};

/// VYS reward accumulator for a single node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VysAccumulator {
    /// Total accumulated rewards in micro-seeds.
    pub accumulated_rewards: u64,
    /// The last epoch at which rewards were claimed.
    pub last_claim_epoch: u64,
    /// The node's PoSrv contribution score (0.0 - 1.0).
    pub posrv_contribution: f64,
}

impl VysAccumulator {
    /// Create a new accumulator with zero rewards.
    ///
    /// # Arguments
    ///
    /// * `posrv_contribution` - The node's initial PoSrv contribution score
    pub fn new(posrv_contribution: f64) -> Self {
        Self {
            accumulated_rewards: 0,
            last_claim_epoch: 0,
            posrv_contribution,
        }
    }

    /// Accumulate rewards for a given epoch.
    ///
    /// Computes the node's share of the epoch reward pool based on its
    /// PoSrv contribution relative to the total.
    ///
    /// # Arguments
    ///
    /// * `epoch_reward_pool` - Total micro-seeds available for distribution this epoch
    /// * `node_posrv` - This node's PoSrv contribution score
    /// * `total_posrv` - Sum of all nodes' PoSrv contribution scores
    ///
    /// # Errors
    ///
    /// - [`VysError::InvalidContribution`] if `total_posrv` is zero or negative
    /// - [`VysError::InvalidContribution`] if `node_posrv` is negative
    /// - [`VysError::Overflow`] on arithmetic overflow
    pub fn accumulate(
        &mut self,
        epoch_reward_pool: u64,
        node_posrv: f64,
        total_posrv: f64,
    ) -> Result<()> {
        if total_posrv <= 0.0 {
            return Err(VysError::InvalidContribution(
                "total PoSrv must be positive".to_string(),
            ));
        }
        if node_posrv < 0.0 {
            return Err(VysError::InvalidContribution(
                "node PoSrv must be non-negative".to_string(),
            ));
        }
        if node_posrv > total_posrv {
            return Err(VysError::InvalidContribution(
                "node PoSrv cannot exceed total PoSrv".to_string(),
            ));
        }

        self.posrv_contribution = node_posrv;

        let share = node_posrv / total_posrv;
        let reward = (epoch_reward_pool as f64 * share) as u64;

        self.accumulated_rewards = self
            .accumulated_rewards
            .checked_add(reward)
            .ok_or(VysError::Overflow)?;

        tracing::trace!(
            reward,
            total = self.accumulated_rewards,
            share,
            "VYS: accumulated epoch reward"
        );

        Ok(())
    }

    /// Return the total claimable amount.
    pub fn claimable_amount(&self) -> u64 {
        self.accumulated_rewards
    }

    /// Reset accumulated rewards to zero (after a successful claim).
    pub fn reset_rewards(&mut self, claim_epoch: u64) {
        self.accumulated_rewards = 0;
        self.last_claim_epoch = claim_epoch;
    }

    /// Update the PoSrv contribution score.
    pub fn update_posrv(&mut self, new_posrv: f64) {
        self.posrv_contribution = new_posrv;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_accumulate_equal_share() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulate(1_000_000, 1.0, 1.0).expect("accumulate");
        assert_eq!(acc.claimable_amount(), 1_000_000);
    }

    #[test]
    fn test_accumulate_half_share() {
        let mut acc = VysAccumulator::new(0.5);
        acc.accumulate(1_000_000, 0.5, 1.0).expect("accumulate");
        assert_eq!(acc.claimable_amount(), 500_000);
    }

    #[test]
    fn test_accumulate_multiple_epochs() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulate(1_000, 1.0, 2.0).expect("epoch 1");
        acc.accumulate(1_000, 1.0, 2.0).expect("epoch 2");
        acc.accumulate(1_000, 1.0, 2.0).expect("epoch 3");
        assert_eq!(acc.claimable_amount(), 1_500);
    }

    #[test]
    fn test_accumulate_zero_pool() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulate(0, 1.0, 1.0).expect("zero pool");
        assert_eq!(acc.claimable_amount(), 0);
    }

    #[test]
    fn test_accumulate_zero_total_posrv_rejected() {
        let mut acc = VysAccumulator::new(0.0);
        let result = acc.accumulate(1000, 0.0, 0.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_accumulate_negative_posrv_rejected() {
        let mut acc = VysAccumulator::new(0.0);
        let result = acc.accumulate(1000, -1.0, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_reset_rewards() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulate(1_000_000, 1.0, 1.0).expect("accumulate");
        assert_eq!(acc.claimable_amount(), 1_000_000);

        acc.reset_rewards(5);
        assert_eq!(acc.claimable_amount(), 0);
        assert_eq!(acc.last_claim_epoch, 5);
    }

    #[test]
    fn test_node_posrv_exceeds_total_rejected() {
        let mut acc = VysAccumulator::new(2.0);
        let result = acc.accumulate(1000, 2.0, 1.0);
        assert!(result.is_err());
    }
}
