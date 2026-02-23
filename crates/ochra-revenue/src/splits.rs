//! Revenue splits and 30-day timelock.
//!
//! Revenue from content sales is distributed among three parties:
//!
//! - **Host** (Space owner): Default 10%
//! - **Creator**: Default 70%
//! - **Network** (ABR): Default 20%
//!
//! The split percentages must always sum to 100. Changes require a 30-day
//! timelock to protect creators from sudden changes.
//!
//! ## Timelock
//!
//! [`TIMELOCK_SECONDS`] = 30 * 24 * 3600 = 2,592,000 seconds (30 days)

use serde::{Deserialize, Serialize};

use crate::{Result, RevenueError};

/// Timelock duration for split changes (30 days in seconds).
pub const TIMELOCK_SECONDS: u64 = 30 * 24 * 3600;

/// Default host (Space owner) revenue share percentage.
pub const DEFAULT_HOST_PCT: u8 = 10;

/// Default content creator revenue share percentage.
pub const DEFAULT_CREATOR_PCT: u8 = 70;

/// Default network (ABR) revenue share percentage.
pub const DEFAULT_NETWORK_PCT: u8 = 20;

/// Revenue split configuration for a Space.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevenueSplitConfig {
    /// Host (Space owner) share percentage.
    pub host_pct: u8,
    /// Creator share percentage.
    pub creator_pct: u8,
    /// Network (ABR) share percentage.
    pub network_pct: u8,
}

/// Default revenue split: host=10, creator=70, network=20.
pub const DEFAULT_SPLIT: RevenueSplitConfig = RevenueSplitConfig {
    host_pct: DEFAULT_HOST_PCT,
    creator_pct: DEFAULT_CREATOR_PCT,
    network_pct: DEFAULT_NETWORK_PCT,
};

/// A proposal to change the revenue split, subject to a 30-day timelock.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SplitChangeProposal {
    /// The proposed new split configuration.
    pub new_split: RevenueSplitConfig,
    /// Unix timestamp when the proposal was made.
    pub proposed_at: u64,
    /// Unix timestamp when the new split becomes effective.
    pub effective_at: u64,
}

/// Validate a revenue split configuration.
///
/// # Errors
///
/// - [`RevenueError::InvalidSplitTotal`] if percentages do not sum to 100
pub fn validate_split(config: &RevenueSplitConfig) -> Result<()> {
    let total = config.host_pct as u16 + config.creator_pct as u16 + config.network_pct as u16;
    if total != 100 {
        return Err(RevenueError::InvalidSplitTotal { total });
    }
    Ok(())
}

/// Propose a split change with a 30-day timelock.
///
/// # Arguments
///
/// * `current` - The current split (for reference)
/// * `new` - The proposed new split
/// * `current_time` - The current Unix timestamp in seconds
///
/// # Errors
///
/// - [`RevenueError::InvalidSplitTotal`] if the new split percentages do not sum to 100
/// - [`RevenueError::InvalidSplit`] if the new split is identical to the current one
pub fn propose_split_change(
    current: &RevenueSplitConfig,
    new: RevenueSplitConfig,
    current_time: u64,
) -> Result<SplitChangeProposal> {
    validate_split(&new)?;

    if current == &new {
        return Err(RevenueError::InvalidSplit(
            "proposed split is identical to current split".to_string(),
        ));
    }

    let effective_at = current_time + TIMELOCK_SECONDS;

    tracing::info!(
        host = new.host_pct,
        creator = new.creator_pct,
        network = new.network_pct,
        effective_at,
        "revenue split change proposed"
    );

    Ok(SplitChangeProposal {
        new_split: new,
        proposed_at: current_time,
        effective_at,
    })
}

/// Check whether a split change proposal is effective at the given time.
///
/// Returns `true` if the current time is at or past the effective time.
pub fn is_effective(proposal: &SplitChangeProposal, current_time: u64) -> bool {
    current_time >= proposal.effective_at
}

/// Distribute a revenue amount according to the split configuration.
///
/// Returns `(host_amount, creator_amount, network_amount)` in micro-seeds.
/// Rounding remainder is awarded to the creator.
///
/// # Errors
///
/// - [`RevenueError::ZeroAmount`] if the amount is zero
/// - [`RevenueError::InvalidSplitTotal`] if the split is invalid
pub fn distribute(amount: u64, split: &RevenueSplitConfig) -> Result<(u64, u64, u64)> {
    if amount == 0 {
        return Err(RevenueError::ZeroAmount);
    }
    validate_split(split)?;

    let host_amount = amount
        .checked_mul(split.host_pct as u64)
        .ok_or(RevenueError::Overflow)?
        / 100;

    let network_amount = amount
        .checked_mul(split.network_pct as u64)
        .ok_or(RevenueError::Overflow)?
        / 100;

    // Creator gets the remainder to avoid rounding loss
    let creator_amount = amount - host_amount - network_amount;

    Ok((host_amount, creator_amount, network_amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_split_valid() {
        validate_split(&DEFAULT_SPLIT).expect("default split should be valid");
        assert_eq!(DEFAULT_SPLIT.host_pct, 10);
        assert_eq!(DEFAULT_SPLIT.creator_pct, 70);
        assert_eq!(DEFAULT_SPLIT.network_pct, 20);
    }

    #[test]
    fn test_validate_split_invalid_total() {
        let split = RevenueSplitConfig {
            host_pct: 10,
            creator_pct: 70,
            network_pct: 30,
        };
        assert!(validate_split(&split).is_err());
    }

    #[test]
    fn test_distribute_default_100_seeds() {
        let amount = 10_000_000_000u64; // 100 Seeds
        let (host, creator, network) = distribute(amount, &DEFAULT_SPLIT).expect("distribute");
        assert_eq!(host, 1_000_000_000); // 10%
        assert_eq!(creator, 7_000_000_000); // 70%
        assert_eq!(network, 2_000_000_000); // 20%
        assert_eq!(host + creator + network, amount);
    }

    #[test]
    fn test_distribute_rounding() {
        // An amount that doesn't divide evenly by 100
        let amount = 33u64;
        let (host, creator, network) = distribute(amount, &DEFAULT_SPLIT).expect("distribute");
        assert_eq!(host + creator + network, amount, "must sum to total");
    }

    #[test]
    fn test_distribute_zero_amount() {
        assert!(distribute(0, &DEFAULT_SPLIT).is_err());
    }

    #[test]
    fn test_propose_split_change() {
        let current = DEFAULT_SPLIT;
        let new = RevenueSplitConfig {
            host_pct: 5,
            creator_pct: 80,
            network_pct: 15,
        };

        let proposal = propose_split_change(&current, new.clone(), 1_000_000).expect("propose");
        assert_eq!(proposal.new_split, new);
        assert_eq!(proposal.proposed_at, 1_000_000);
        assert_eq!(proposal.effective_at, 1_000_000 + TIMELOCK_SECONDS);
    }

    #[test]
    fn test_propose_identical_split_rejected() {
        let current = DEFAULT_SPLIT;
        let new = DEFAULT_SPLIT;
        assert!(propose_split_change(&current, new, 1_000_000).is_err());
    }

    #[test]
    fn test_propose_invalid_split_rejected() {
        let current = DEFAULT_SPLIT;
        let bad = RevenueSplitConfig {
            host_pct: 50,
            creator_pct: 50,
            network_pct: 50,
        };
        assert!(propose_split_change(&current, bad, 1_000_000).is_err());
    }

    #[test]
    fn test_is_effective() {
        let proposal = SplitChangeProposal {
            new_split: DEFAULT_SPLIT,
            proposed_at: 1_000_000,
            effective_at: 1_000_000 + TIMELOCK_SECONDS,
        };

        assert!(!is_effective(&proposal, 1_000_000));
        assert!(!is_effective(&proposal, 1_000_000 + TIMELOCK_SECONDS - 1));
        assert!(is_effective(&proposal, 1_000_000 + TIMELOCK_SECONDS));
        assert!(is_effective(&proposal, 1_000_000 + TIMELOCK_SECONDS + 1));
    }

    #[test]
    fn test_timelock_constant() {
        assert_eq!(TIMELOCK_SECONDS, 30 * 24 * 3600);
    }

    #[test]
    fn test_custom_split() {
        let split = RevenueSplitConfig {
            host_pct: 0,
            creator_pct: 100,
            network_pct: 0,
        };
        validate_split(&split).expect("valid 100% creator");
        let (host, creator, network) = distribute(1000, &split).expect("distribute");
        assert_eq!(host, 0);
        assert_eq!(creator, 1000);
        assert_eq!(network, 0);
    }
}
