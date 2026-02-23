//! Collateral Ratio (CR) throttling for minting operations.
//!
//! The collateral ratio represents the relationship between total infrastructure
//! value contributed and total Seeds minted. When the CR is low (undercollateralized),
//! minting is throttled. When the CR is high, minting proceeds at full rate.
//!
//! ## CR Range
//!
//! The CR is clamped to `[MIN_CR, MAX_CR]` = `[0.5, 2.0]`:
//!
//! - CR < 0.5: Emergency undercollateralization, minting halted
//! - CR 0.5 - 1.0: Minting throttled proportionally
//! - CR 1.0 - 2.0: Full minting rate
//! - CR > 2.0: Overcollateralized, full minting

use crate::{MintError, Result};

/// Minimum collateral ratio (50%).
pub const MIN_CR: f64 = 0.5;

/// Maximum collateral ratio (200%).
pub const MAX_CR: f64 = 2.0;

/// Collateral ratio state.
#[derive(Debug, Clone)]
pub struct CollateralRatio {
    /// The current collateral ratio (clamped to [MIN_CR, MAX_CR]).
    ratio: f64,
}

impl CollateralRatio {
    /// Create a new collateral ratio from a raw value.
    ///
    /// The ratio is clamped to `[MIN_CR, MAX_CR]`.
    pub fn new(ratio: f64) -> Self {
        Self {
            ratio: ratio.clamp(MIN_CR, MAX_CR),
        }
    }

    /// Get the current ratio value.
    pub fn ratio(&self) -> f64 {
        self.ratio
    }
}

/// Compute the collateral ratio from total minted and total infrastructure value.
///
/// `CR = total_infra_value / total_minted`
///
/// If `total_minted` is zero, returns `MAX_CR` (no tokens outstanding means
/// the system is fully collateralized by definition).
///
/// # Arguments
///
/// * `total_minted` - Total micro-seeds minted across all epochs
/// * `total_infra_value` - Total infrastructure value in equivalent micro-seeds
pub fn compute_cr(total_minted: u64, total_infra_value: u64) -> f64 {
    if total_minted == 0 {
        return MAX_CR;
    }
    let ratio = total_infra_value as f64 / total_minted as f64;
    ratio.clamp(MIN_CR, MAX_CR)
}

/// Compute the maximum amount that can be minted at the given collateral ratio.
///
/// At CR >= 1.0, the full `base_amount` can be minted.
/// At CR between MIN_CR and 1.0, minting is throttled proportionally.
/// At CR <= MIN_CR, minting is halted (returns 0).
///
/// The formula is:
/// ```text
/// max_mintable = base_amount * (cr - MIN_CR) / (1.0 - MIN_CR)   [for MIN_CR < cr < 1.0]
/// max_mintable = base_amount                                      [for cr >= 1.0]
/// max_mintable = 0                                                [for cr <= MIN_CR]
/// ```
///
/// # Arguments
///
/// * `cr` - The current collateral ratio
/// * `base_amount` - The base amount requested for minting in micro-seeds
pub fn max_mintable(cr: f64, base_amount: u64) -> u64 {
    if cr <= MIN_CR {
        return 0;
    }
    if cr >= 1.0 {
        return base_amount;
    }

    // Linear interpolation between MIN_CR and 1.0
    let fraction = (cr - MIN_CR) / (1.0 - MIN_CR);
    (base_amount as f64 * fraction) as u64
}

/// Check whether a minting request of `amount` is allowed at the given CR.
///
/// # Errors
///
/// - [`MintError::Throttled`] if the requested amount exceeds the maximum mintable
pub fn check_mintable(cr: f64, requested: u64) -> Result<()> {
    let max = max_mintable(cr, requested);
    if max < requested {
        return Err(MintError::Throttled {
            requested,
            max_allowed: max,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_cr_equal() {
        let cr = compute_cr(1_000_000, 1_000_000);
        assert!((cr - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_cr_overcollateralized() {
        let cr = compute_cr(1_000_000, 2_000_000);
        assert!((cr - MAX_CR).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_cr_undercollateralized() {
        let cr = compute_cr(1_000_000, 500_000);
        assert!((cr - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_cr_zero_minted() {
        let cr = compute_cr(0, 1_000_000);
        assert!((cr - MAX_CR).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_cr_clamped_low() {
        let cr = compute_cr(1_000_000, 100_000);
        assert!((cr - MIN_CR).abs() < f64::EPSILON);
    }

    #[test]
    fn test_max_mintable_full_cr() {
        assert_eq!(max_mintable(1.0, 1_000_000), 1_000_000);
        assert_eq!(max_mintable(1.5, 1_000_000), 1_000_000);
        assert_eq!(max_mintable(2.0, 1_000_000), 1_000_000);
    }

    #[test]
    fn test_max_mintable_halted() {
        assert_eq!(max_mintable(0.5, 1_000_000), 0);
        assert_eq!(max_mintable(0.3, 1_000_000), 0);
    }

    #[test]
    fn test_max_mintable_throttled() {
        // At CR = 0.75, fraction = (0.75 - 0.5) / (1.0 - 0.5) = 0.5
        let m = max_mintable(0.75, 1_000_000);
        assert_eq!(m, 500_000);
    }

    #[test]
    fn test_collateral_ratio_struct() {
        let cr = CollateralRatio::new(1.5);
        assert!((cr.ratio() - 1.5).abs() < f64::EPSILON);

        let cr_low = CollateralRatio::new(0.1);
        assert!((cr_low.ratio() - MIN_CR).abs() < f64::EPSILON);

        let cr_high = CollateralRatio::new(5.0);
        assert!((cr_high.ratio() - MAX_CR).abs() < f64::EPSILON);
    }

    #[test]
    fn test_check_mintable_ok() {
        check_mintable(1.0, 1_000_000).expect("should allow full minting");
    }

    #[test]
    fn test_check_mintable_throttled() {
        let err = check_mintable(0.6, 1_000_000).expect_err("should be throttled");
        assert!(matches!(err, MintError::Throttled { .. }));
    }
}
