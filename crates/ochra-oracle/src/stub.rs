//! Hardcoded rate oracle for v1.
//!
//! In the initial version of Ochra, a stub oracle provides a fixed exchange
//! rate. This avoids the complexity of MPC-TLS oracle infrastructure while
//! still allowing the rest of the economy to function.
//!
//! The default rate is `100_000_000` micro-seeds per unit, corresponding to
//! 1 Seed = 1 USD at the baseline.

use crate::Result;

/// Default rate: 1 Seed = 1 USD = 100,000,000 micro-seeds.
pub const DEFAULT_RATE: u64 = 100_000_000;

/// A stub oracle that returns a hardcoded exchange rate.
///
/// Used in v1 where a real MPC-TLS oracle is not yet deployed. The rate
/// can be adjusted for development and testing purposes via [`dev_set_rate`](StubOracle::dev_set_rate).
#[derive(Debug, Clone)]
pub struct StubOracle {
    /// The current exchange rate in micro-seeds per unit.
    rate: u64,
}

impl StubOracle {
    /// Create a new stub oracle with the default rate.
    pub fn new() -> Self {
        Self { rate: DEFAULT_RATE }
    }

    /// Create a stub oracle with a custom rate.
    ///
    /// # Arguments
    ///
    /// * `rate` - The exchange rate in micro-seeds per unit
    pub fn with_rate(rate: u64) -> Self {
        Self { rate }
    }

    /// Get the current exchange rate in micro-seeds per unit.
    pub fn get_rate(&self) -> u64 {
        self.rate
    }

    /// Set the exchange rate (development/testing only).
    ///
    /// In production, the rate should be determined by the MPC-TLS oracle.
    /// This method is provided for development and integration testing.
    ///
    /// # Arguments
    ///
    /// * `rate` - The new exchange rate in micro-seeds per unit
    pub fn dev_set_rate(&mut self, rate: u64) {
        tracing::warn!(new_rate = rate, "stub oracle: rate changed (dev only)");
        self.rate = rate;
    }

    /// Convert a fiat amount to micro-seeds using the current rate.
    ///
    /// # Arguments
    ///
    /// * `fiat_amount` - The amount in fiat units
    pub fn to_micro_seeds(&self, fiat_amount: u64) -> Result<u64> {
        let result = (fiat_amount as u128)
            .checked_mul(self.rate as u128)
            .and_then(|v| u64::try_from(v).ok())
            .ok_or(crate::OracleError::InvalidPrice(fiat_amount))?;
        Ok(result)
    }

    /// Convert micro-seeds to a fiat amount using the current rate.
    ///
    /// # Arguments
    ///
    /// * `micro_seeds` - The amount in micro-seeds
    pub fn from_micro_seeds(&self, micro_seeds: u64) -> u64 {
        if self.rate == 0 {
            return 0;
        }
        micro_seeds / self.rate
    }
}

impl Default for StubOracle {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_rate() {
        let oracle = StubOracle::new();
        assert_eq!(oracle.get_rate(), DEFAULT_RATE);
    }

    #[test]
    fn test_custom_rate() {
        let oracle = StubOracle::with_rate(200_000_000);
        assert_eq!(oracle.get_rate(), 200_000_000);
    }

    #[test]
    fn test_dev_set_rate() {
        let mut oracle = StubOracle::new();
        assert_eq!(oracle.get_rate(), DEFAULT_RATE);

        oracle.dev_set_rate(50_000_000);
        assert_eq!(oracle.get_rate(), 50_000_000);
    }

    #[test]
    fn test_to_micro_seeds() {
        let oracle = StubOracle::new();
        // 1 fiat unit at default rate = 100_000_000 micro-seeds
        let result = oracle.to_micro_seeds(1).expect("conversion");
        assert_eq!(result, DEFAULT_RATE);
    }

    #[test]
    fn test_from_micro_seeds() {
        let oracle = StubOracle::new();
        // 100_000_000 micro-seeds = 1 fiat unit at default rate
        assert_eq!(oracle.from_micro_seeds(DEFAULT_RATE), 1);
        assert_eq!(oracle.from_micro_seeds(DEFAULT_RATE * 5), 5);
    }

    #[test]
    fn test_roundtrip_conversion() {
        let oracle = StubOracle::new();
        let fiat = 42u64;
        let micro = oracle.to_micro_seeds(fiat).expect("to micro");
        let back = oracle.from_micro_seeds(micro);
        assert_eq!(back, fiat);
    }

    #[test]
    fn test_default_impl() {
        let oracle = StubOracle::default();
        assert_eq!(oracle.get_rate(), DEFAULT_RATE);
    }
}
