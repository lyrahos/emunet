//! Decay, slash, and CR formula for VYS rewards.
//!
//! Unclaimed rewards decay over time to incentivize regular claiming.
//! Nodes that misbehave can be slashed, reducing their accumulated rewards.

use crate::accounting::VysAccumulator;

/// Decay rate per epoch (0.1% per epoch).
pub const DECAY_RATE_PER_EPOCH: f64 = 0.001;

/// Apply decay to an accumulator's rewards.
///
/// Reduces accumulated rewards by the given decay rate:
/// `new_rewards = rewards * (1 - decay_rate)`
///
/// # Arguments
///
/// * `accumulator` - The accumulator to decay
/// * `decay_rate` - The fraction to decay (e.g., 0.001 for 0.1%)
pub fn apply_decay(accumulator: &mut VysAccumulator, decay_rate: f64) {
    let current = accumulator.accumulated_rewards;
    let decay_amount = (current as f64 * decay_rate) as u64;
    accumulator.accumulated_rewards = current.saturating_sub(decay_amount);

    tracing::trace!(
        decay_amount,
        remaining = accumulator.accumulated_rewards,
        "VYS: applied decay"
    );
}

/// Apply a slash to an accumulator's rewards.
///
/// Reduces accumulated rewards by the given fraction:
/// `new_rewards = rewards * (1 - slash_fraction)`
///
/// # Arguments
///
/// * `accumulator` - The accumulator to slash
/// * `slash_fraction` - The fraction to slash (e.g., 0.5 for 50%)
pub fn apply_slash(accumulator: &mut VysAccumulator, slash_fraction: f64) {
    let current = accumulator.accumulated_rewards;
    let slash_amount = (current as f64 * slash_fraction.clamp(0.0, 1.0)) as u64;
    accumulator.accumulated_rewards = current.saturating_sub(slash_amount);

    tracing::warn!(
        slash_amount,
        slash_fraction,
        remaining = accumulator.accumulated_rewards,
        "VYS: applied slash"
    );
}

/// Apply the default per-epoch decay to an accumulator.
pub fn apply_epoch_decay(accumulator: &mut VysAccumulator) {
    apply_decay(accumulator, DECAY_RATE_PER_EPOCH);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_decay() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulated_rewards = 1_000_000;

        apply_decay(&mut acc, 0.001);
        // 0.1% of 1_000_000 = 1000 decayed
        assert_eq!(acc.claimable_amount(), 999_000);
    }

    #[test]
    fn test_apply_decay_multiple() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulated_rewards = 1_000_000;

        for _ in 0..10 {
            apply_epoch_decay(&mut acc);
        }

        // After 10 epochs of 0.1% decay, ~990,045 remains
        assert!(acc.claimable_amount() < 1_000_000);
        assert!(acc.claimable_amount() > 900_000);
    }

    #[test]
    fn test_apply_decay_zero_rewards() {
        let mut acc = VysAccumulator::new(1.0);
        apply_decay(&mut acc, 0.5);
        assert_eq!(acc.claimable_amount(), 0);
    }

    #[test]
    fn test_apply_slash() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulated_rewards = 1_000_000;

        apply_slash(&mut acc, 0.5);
        assert_eq!(acc.claimable_amount(), 500_000);
    }

    #[test]
    fn test_apply_slash_full() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulated_rewards = 1_000_000;

        apply_slash(&mut acc, 1.0);
        assert_eq!(acc.claimable_amount(), 0);
    }

    #[test]
    fn test_apply_slash_zero() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulated_rewards = 1_000_000;

        apply_slash(&mut acc, 0.0);
        assert_eq!(acc.claimable_amount(), 1_000_000);
    }

    #[test]
    fn test_apply_slash_clamped() {
        let mut acc = VysAccumulator::new(1.0);
        acc.accumulated_rewards = 1_000_000;

        // Slash fraction > 1.0 should be clamped to 1.0
        apply_slash(&mut acc, 2.0);
        assert_eq!(acc.claimable_amount(), 0);
    }

    #[test]
    fn test_decay_rate_constant() {
        assert!((DECAY_RATE_PER_EPOCH - 0.001).abs() < f64::EPSILON);
    }
}
