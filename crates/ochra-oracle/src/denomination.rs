//! Denomination formula (Section 11.9).
//!
//! The denomination module converts TWAP oracle prices and infrastructure
//! metrics into a denomination value expressed in micro-seeds.
//!
//! Formula:
//! ```text
//! denomination = (twap * MICRO_SEEDS_PER_SEED) / infra_metric
//! ```

use crate::{OracleError, Result};

/// Micro-seeds per Seed (1 Seed = 100,000,000 micro-seeds).
pub const MICRO_SEEDS_PER_SEED: u64 = 100_000_000;

/// Compute the denomination value from a TWAP price and infrastructure metric.
///
/// The denomination represents the micro-seed value adjusted by the
/// infrastructure metric. A higher infrastructure metric lowers the
/// denomination (infrastructure contribution is rewarded).
///
/// # Arguments
///
/// * `twap` - Time-weighted average price in oracle units
/// * `infra_metric` - Infrastructure contribution metric (must be non-zero)
///
/// # Errors
///
/// - [`OracleError::InvalidDenomination`] if `twap` is zero
/// - [`OracleError::InvalidDenomination`] if `infra_metric` is zero
///
/// # Examples
///
/// ```
/// use ochra_oracle::denomination::compute_denomination;
///
/// let denom = compute_denomination(100_000_000, 1).unwrap();
/// assert_eq!(denom, 100_000_000 * 100_000_000); // at baseline
/// ```
pub fn compute_denomination(twap: u64, infra_metric: u64) -> Result<u64> {
    if twap == 0 {
        return Err(OracleError::InvalidDenomination(
            "TWAP must be non-zero".to_string(),
        ));
    }
    if infra_metric == 0 {
        return Err(OracleError::InvalidDenomination(
            "infrastructure metric must be non-zero".to_string(),
        ));
    }

    // Use u128 to avoid overflow: (twap * MICRO_SEEDS_PER_SEED) / infra_metric
    let numerator = twap as u128 * MICRO_SEEDS_PER_SEED as u128;
    let result = numerator / infra_metric as u128;

    // Clamp to u64::MAX if the result overflows
    if result > u64::MAX as u128 {
        Ok(u64::MAX)
    } else {
        Ok(result as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baseline_denomination() {
        // At baseline: twap = 1 Seed in micro-seeds, infra_metric = 1
        let denom = compute_denomination(MICRO_SEEDS_PER_SEED, 1).expect("baseline denom");
        assert_eq!(
            denom,
            MICRO_SEEDS_PER_SEED as u64 * MICRO_SEEDS_PER_SEED as u64
        );
    }

    #[test]
    fn test_higher_infra_lowers_denomination() {
        let denom_low = compute_denomination(100, 1).expect("low infra");
        let denom_high = compute_denomination(100, 10).expect("high infra");
        assert!(denom_high < denom_low);
    }

    #[test]
    fn test_zero_twap_rejected() {
        let err = compute_denomination(0, 1).unwrap_err();
        assert!(matches!(err, OracleError::InvalidDenomination(_)));
    }

    #[test]
    fn test_zero_infra_rejected() {
        let err = compute_denomination(100, 0).unwrap_err();
        assert!(matches!(err, OracleError::InvalidDenomination(_)));
    }

    #[test]
    fn test_equal_twap_and_infra() {
        let denom = compute_denomination(50, 50).expect("equal");
        assert_eq!(denom, MICRO_SEEDS_PER_SEED);
    }

    #[test]
    fn test_micro_seeds_constant() {
        assert_eq!(MICRO_SEEDS_PER_SEED, 100_000_000);
    }
}
