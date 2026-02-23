//! # ochra-guardian
//!
//! Guardian recovery system (Section 6).
//!
//! Guardians are trusted contacts who can assist with PIK recovery. The system
//! uses a DKG ceremony for key splitting, dead-drop heartbeats for liveness,
//! and a 48-hour veto window for recovery.
//!
//! ## Modules
//!
//! - [`dkg`] — Guardian DKG ceremony
//! - [`heartbeat`] — Dead drop heartbeat system
//! - [`recovery`] — 48-hour Dual-Path Cancellation recovery
//! - [`replacement`] — Guardian replacement

pub mod dkg;
pub mod heartbeat;
pub mod recovery;
pub mod replacement;

/// Error types for guardian operations.
#[derive(Debug, thiserror::Error)]
pub enum GuardianError {
    /// Too few guardians enrolled.
    #[error("too few guardians: have {actual}, need at least {minimum}")]
    TooFewGuardians {
        /// Actual number of guardians.
        actual: usize,
        /// Minimum required.
        minimum: usize,
    },

    /// Too many guardians enrolled.
    #[error("too many guardians: have {actual}, maximum is {maximum}")]
    TooManyGuardians {
        /// Actual number of guardians.
        actual: usize,
        /// Maximum allowed.
        maximum: usize,
    },

    /// Guardian already enrolled.
    #[error("guardian already enrolled: {0}")]
    AlreadyEnrolled(String),

    /// Guardian not found.
    #[error("guardian not found: {0}")]
    NotFound(String),

    /// Heartbeat is stale (guardian may be offline).
    #[error("guardian heartbeat stale: last seen {last_seen}, current {current}")]
    StaleHeartbeat {
        /// Timestamp of the last heartbeat.
        last_seen: u64,
        /// Current timestamp.
        current: u64,
    },

    /// Recovery is already in progress.
    #[error("recovery already in progress")]
    RecoveryInProgress,

    /// Insufficient shards for recovery.
    #[error("insufficient recovery shards: have {available}, need {required}")]
    InsufficientShards {
        /// Number of shards available.
        available: usize,
        /// Number of shards required.
        required: usize,
    },

    /// Recovery was vetoed.
    #[error("recovery was vetoed")]
    Vetoed,

    /// Veto window is still active.
    #[error("veto window still active: {remaining_secs}s remaining")]
    VetoWindowActive {
        /// Seconds remaining in the veto window.
        remaining_secs: u64,
    },

    /// DKG ceremony error.
    #[error("DKG error: {0}")]
    DkgError(String),

    /// No recovery in progress.
    #[error("no recovery in progress")]
    NoRecovery,
}

/// Convenience result type for guardian operations.
pub type Result<T> = std::result::Result<T, GuardianError>;
