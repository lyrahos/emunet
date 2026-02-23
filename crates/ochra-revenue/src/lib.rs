//! # ochra-revenue
//!
//! Revenue distribution and accounting (Section 15).
//!
//! Content sales revenue is split between three parties with a 30-day
//! timelock on split changes.
//!
//! ## Modules
//!
//! - [`splits`] — Revenue splits and 30-day timelock

pub mod splits;

/// Error types for revenue operations.
#[derive(Debug, thiserror::Error)]
pub enum RevenueError {
    /// Revenue split percentages do not sum to 100.
    #[error("split percentages must sum to 100, got {total}")]
    InvalidSplitTotal {
        /// The actual total.
        total: u16,
    },

    /// Amount is zero.
    #[error("revenue amount is zero")]
    ZeroAmount,

    /// Arithmetic overflow.
    #[error("arithmetic overflow in revenue calculation")]
    Overflow,

    /// Timelock has not yet expired.
    #[error("timelock not expired: effective at {effective_at}, current time {current_time}")]
    TimelockNotExpired {
        /// When the proposal becomes effective.
        effective_at: u64,
        /// The current time.
        current_time: u64,
    },

    /// Invalid split configuration.
    #[error("invalid split: {0}")]
    InvalidSplit(String),
}

/// Convenience result type for revenue operations.
pub type Result<T> = std::result::Result<T, RevenueError>;
