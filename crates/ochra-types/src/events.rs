//! Event types for daemon-to-UI notification (Section 23).
//!
//! All events are emitted via the JSON-RPC event subscription channel.

use serde::{Deserialize, Serialize};

use crate::{ContentHash, GroupId, Hash, SubscriptionId};

/// Envelope for all daemon events.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub event_type: EventType,
    pub timestamp: u64,
    pub payload: serde_json::Value,
}

/// All event types (Section 23).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Space & Content events
    NewContent,
    ContentTombstoned,
    MemberJoined,
    MemberLeft,
    RoleChanged,
    PurchaseComplete,
    EarningsUpdate,
    NewReport,
    InviteRedeemed,

    // Economy events
    BalanceUpdate,
    ReceiptFlushed,
    MintingComplete,
    RefundProcessed,
    TransferReceived,

    // Whisper events
    WhisperMessage,
    WhisperSessionStart,
    WhisperSessionEnd,
    WhisperTyping,
    WhisperReadAck,
    IdentityRevealed,

    // Network events
    CircuitRotated,
    EpochTransition,
    PeerConnected,
    PeerDisconnected,
    PorChallengeReceived,

    // System events
    UpdateAvailable,
    GuardianHeartbeat,
    DaemonStatus,
    ErrorOccurred,
}
