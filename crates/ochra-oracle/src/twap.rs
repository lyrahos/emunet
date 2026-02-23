//! TWAP (Time-Weighted Average Price) calculation.
//!
//! Computes a time-weighted average price from a series of (timestamp, price) pairs.
//! The formula is:
//!
//! ```text
//! TWAP = sum(price_i * duration_i) / sum(duration_i)
//! ```
//!
//! Where `duration_i` is the time between consecutive observations, and `price_i`
//! is the price that prevailed during that interval.

use crate::{OracleError, Result};

/// Minimum number of observations for a valid TWAP.
pub const MIN_OBSERVATIONS: usize = 3;

/// Maximum number of observations to retain.
pub const MAX_OBSERVATIONS: usize = 1440;

/// Compute the TWAP from a slice of (timestamp, price) pairs.
///
/// Each pair is `(timestamp_seconds, price_in_microseed_units)`. The pairs
/// **must** be sorted by timestamp in ascending order.
///
/// # Errors
///
/// - [`OracleError::InsufficientObservations`] if fewer than [`MIN_OBSERVATIONS`] pairs
/// - [`OracleError::EmptyWindow`] if total duration is zero
/// - [`OracleError::NonMonotonicTimestamp`] if timestamps are not strictly increasing
///
/// # Examples
///
/// ```
/// use ochra_oracle::twap::compute_twap;
///
/// // Constant price of 100 over three intervals
/// let prices = vec![(1000u64, 100u64), (2000, 100), (3000, 100)];
/// let twap = compute_twap(&prices).unwrap();
/// assert_eq!(twap, 100);
/// ```
pub fn compute_twap(prices: &[(u64, u64)]) -> Result<u64> {
    if prices.len() < MIN_OBSERVATIONS {
        return Err(OracleError::InsufficientObservations {
            required: MIN_OBSERVATIONS,
            available: prices.len(),
        });
    }

    // Validate monotonicity
    for window in prices.windows(2) {
        let (t_prev, _) = window[0];
        let (t_next, _) = window[1];
        if t_next <= t_prev {
            return Err(OracleError::NonMonotonicTimestamp {
                new: t_next,
                last: t_prev,
            });
        }
    }

    // TWAP = sum(price_i * duration_i) / sum(duration_i)
    // price_i is the price observed at the *start* of each interval.
    let mut weighted_sum: u128 = 0;
    let mut total_duration: u128 = 0;

    for window in prices.windows(2) {
        let (t_prev, p_prev) = window[0];
        let (t_next, _) = window[1];
        let duration = (t_next - t_prev) as u128;
        weighted_sum = weighted_sum.saturating_add(p_prev as u128 * duration);
        total_duration = total_duration.saturating_add(duration);
    }

    if total_duration == 0 {
        return Err(OracleError::EmptyWindow);
    }

    // Integer division; truncates toward zero
    let twap = weighted_sum / total_duration;

    // Safe cast: prices are u64, so the weighted average cannot exceed u64::MAX
    Ok(twap as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_price() {
        let prices = vec![(1000, 100), (2000, 100), (3000, 100)];
        let twap = compute_twap(&prices).expect("constant price TWAP");
        assert_eq!(twap, 100);
    }

    #[test]
    fn test_weighted_average() {
        // Price 100 for 1000s, then 200 for 1000s => TWAP = 150
        let prices = vec![(1000, 100), (2000, 200), (3000, 200)];
        let twap = compute_twap(&prices).expect("weighted TWAP");
        assert_eq!(twap, 150);
    }

    #[test]
    fn test_unequal_durations() {
        // Price 100 for 3000s, then 200 for 1000s => TWAP = (100*3000 + 200*1000)/4000 = 125
        let prices = vec![(0, 100), (3000, 200), (4000, 200)];
        let twap = compute_twap(&prices).expect("unequal duration TWAP");
        assert_eq!(twap, 125);
    }

    #[test]
    fn test_insufficient_observations() {
        let prices = vec![(1000, 100), (2000, 200)];
        let err = compute_twap(&prices).unwrap_err();
        assert!(matches!(
            err,
            OracleError::InsufficientObservations { required: 3, available: 2 }
        ));
    }

    #[test]
    fn test_non_monotonic_rejected() {
        let prices = vec![(3000, 100), (2000, 200), (4000, 300)];
        let err = compute_twap(&prices).unwrap_err();
        assert!(matches!(
            err,
            OracleError::NonMonotonicTimestamp { new: 2000, last: 3000 }
        ));
    }

    #[test]
    fn test_empty_slice() {
        let err = compute_twap(&[]).unwrap_err();
        assert!(matches!(
            err,
            OracleError::InsufficientObservations { required: 3, available: 0 }
        ));
    }

    #[test]
    fn test_large_values() {
        // Ensure no overflow for large u64 prices
        let prices = vec![
            (0, u64::MAX / 2),
            (1, u64::MAX / 2),
            (2, u64::MAX / 2),
        ];
        let twap = compute_twap(&prices).expect("large values TWAP");
        assert_eq!(twap, u64::MAX / 2);
    }
}
