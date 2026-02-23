//! Circuit breaker and emergency pause for the oracle system.
//!
//! The circuit breaker detects stale oracle data and can pause the oracle
//! to prevent stale prices from affecting minting and denomination calculations.
//!
//! ## Staleness Detection
//!
//! If the oracle has not received a fresh price update within the
//! [`STALENESS_THRESHOLD`] (1 hour), the oracle is considered stale. Consumers
//! must check staleness before relying on oracle data.

use crate::{OracleError, Result};

/// Staleness threshold in seconds (1 hour).
pub const STALENESS_THRESHOLD: u64 = 3600;

/// Circuit breaker that tracks oracle health and can pause operations.
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Unix timestamp of the last oracle update.
    last_update_time: u64,
    /// Staleness threshold in seconds.
    staleness_threshold: u64,
    /// Whether the oracle is manually paused.
    paused: bool,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the default staleness threshold.
    ///
    /// The `initial_time` is the Unix timestamp of the initial oracle state.
    pub fn new(initial_time: u64) -> Self {
        Self {
            last_update_time: initial_time,
            staleness_threshold: STALENESS_THRESHOLD,
            paused: false,
        }
    }

    /// Create a circuit breaker with a custom staleness threshold.
    pub fn with_threshold(initial_time: u64, staleness_threshold: u64) -> Self {
        Self {
            last_update_time: initial_time,
            staleness_threshold,
            paused: false,
        }
    }

    /// Record a successful oracle update.
    pub fn record_update(&mut self, update_time: u64) {
        self.last_update_time = update_time;
    }

    /// Check whether the oracle data is stale at the given current time.
    ///
    /// Returns `true` if the time since the last update exceeds the
    /// staleness threshold.
    pub fn check_staleness(&self, current_time: u64) -> bool {
        current_time.saturating_sub(self.last_update_time) > self.staleness_threshold
    }

    /// Check whether the oracle is operational (not paused and not stale).
    ///
    /// # Errors
    ///
    /// - [`OracleError::Paused`] if the circuit breaker is manually paused
    /// - [`OracleError::StaleData`] if the oracle data is stale
    pub fn check_operational(&self, current_time: u64) -> Result<()> {
        if self.paused {
            return Err(OracleError::Paused);
        }
        if self.check_staleness(current_time) {
            return Err(OracleError::StaleData {
                last_update: self.last_update_time,
                current: current_time,
                threshold: self.staleness_threshold,
            });
        }
        Ok(())
    }

    /// Trigger an emergency pause of the oracle.
    pub fn trigger_pause(&mut self) {
        tracing::warn!("circuit breaker: oracle paused");
        self.paused = true;
    }

    /// Resume the oracle from an emergency pause.
    pub fn resume(&mut self) {
        tracing::info!("circuit breaker: oracle resumed");
        self.paused = false;
    }

    /// Return whether the oracle is currently paused.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Return the timestamp of the last oracle update.
    pub fn last_update_time(&self) -> u64 {
        self.last_update_time
    }

    /// Return the configured staleness threshold.
    pub fn staleness_threshold(&self) -> u64 {
        self.staleness_threshold
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_circuit_breaker() {
        let cb = CircuitBreaker::new(1000);
        assert!(!cb.is_paused());
        assert_eq!(cb.last_update_time(), 1000);
        assert_eq!(cb.staleness_threshold(), STALENESS_THRESHOLD);
    }

    #[test]
    fn test_not_stale_within_threshold() {
        let cb = CircuitBreaker::new(1000);
        assert!(!cb.check_staleness(1000 + STALENESS_THRESHOLD));
    }

    #[test]
    fn test_stale_after_threshold() {
        let cb = CircuitBreaker::new(1000);
        assert!(cb.check_staleness(1000 + STALENESS_THRESHOLD + 1));
    }

    #[test]
    fn test_record_update_resets_staleness() {
        let mut cb = CircuitBreaker::new(1000);
        assert!(cb.check_staleness(1000 + STALENESS_THRESHOLD + 1));

        cb.record_update(1000 + STALENESS_THRESHOLD + 1);
        assert!(!cb.check_staleness(1000 + STALENESS_THRESHOLD + 1));
    }

    #[test]
    fn test_pause_and_resume() {
        let mut cb = CircuitBreaker::new(1000);
        assert!(!cb.is_paused());

        cb.trigger_pause();
        assert!(cb.is_paused());
        assert!(cb.check_operational(1000).is_err());

        cb.resume();
        assert!(!cb.is_paused());
        assert!(cb.check_operational(1000).is_ok());
    }

    #[test]
    fn test_check_operational_not_stale_not_paused() {
        let cb = CircuitBreaker::new(1000);
        cb.check_operational(1000).expect("should be operational");
    }

    #[test]
    fn test_check_operational_stale() {
        let cb = CircuitBreaker::new(1000);
        let err = cb
            .check_operational(1000 + STALENESS_THRESHOLD + 1)
            .expect_err("should be stale");
        assert!(matches!(err, OracleError::StaleData { .. }));
    }

    #[test]
    fn test_check_operational_paused_takes_priority() {
        let mut cb = CircuitBreaker::new(1000);
        cb.trigger_pause();
        let err = cb
            .check_operational(1000 + STALENESS_THRESHOLD + 1)
            .expect_err("should be paused");
        // Pause check comes before staleness check
        assert!(matches!(err, OracleError::Paused));
    }

    #[test]
    fn test_custom_threshold() {
        let cb = CircuitBreaker::with_threshold(1000, 60);
        assert_eq!(cb.staleness_threshold(), 60);
        assert!(!cb.check_staleness(1060));
        assert!(cb.check_staleness(1061));
    }
}
