//! Event emission system (Section 23).
//!
//! Events are pushed from the daemon to UI subscribers via JSON-RPC
//! notifications. Each subscriber has an independent buffer with
//! backpressure at 1000 events.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// An event emitted by the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event type name (e.g. "ContentPurchased", "DaemonStarted").
    pub event_type: String,
    /// Unix timestamp.
    pub timestamp: u64,
    /// Type-specific payload.
    pub payload: serde_json::Value,
}

/// Filter for event subscriptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Category filter: "space", "economy", "system", "whisper".
    pub categories: Option<Vec<String>>,
    /// Filter to specific Space group_ids.
    pub group_ids: Option<Vec<String>>,
    /// Minimum severity: "info" | "warning" | "critical".
    pub min_severity: Option<String>,
}

/// A subscription handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionId(pub String);

/// Event bus for broadcasting events to subscribers.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
    sequence: Arc<AtomicU64>,
}

impl EventBus {
    /// Create a new event bus with the given buffer capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Emit an event to all subscribers.
    pub fn emit(&self, event: Event) {
        self.sequence.fetch_add(1, Ordering::SeqCst);
        // Ignore send errors (no subscribers)
        let _ = self.sender.send(event);
    }

    /// Subscribe to events. Returns a receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Get the current sequence number.
    pub fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }
}

impl EventFilter {
    /// Check if an event matches this filter.
    pub fn matches(&self, event: &Event) -> bool {
        // Category filter
        if let Some(ref categories) = self.categories {
            let event_category = categorize_event(&event.event_type);
            if !categories.contains(&event_category) {
                return false;
            }
        }

        // Group ID filter (check payload for group_id field)
        if let Some(ref group_ids) = self.group_ids {
            if let Some(gid) = event.payload.get("group_id").and_then(|v| v.as_str()) {
                if !group_ids.iter().any(|id| id == gid) {
                    return false;
                }
            }
        }

        true
    }
}

/// Categorize an event type into a category.
fn categorize_event(event_type: &str) -> String {
    match event_type {
        s if s.starts_with("Member")
            || s.starts_with("Content")
            || s.starts_with("Creator")
            || s.starts_with("Moderator")
            || s.starts_with("Settings")
            || s.starts_with("Ownership") =>
        {
            "space".to_string()
        }
        s if s.starts_with("Epoch")
            || s.starts_with("Refund")
            || s.starts_with("Escrow")
            || s.starts_with("Vys")
            || s.starts_with("Funds")
            || s.starts_with("Minting")
            || s.starts_with("Collateral") =>
        {
            "economy".to_string()
        }
        s if s.starts_with("Whisper") || s.starts_with("Handle") => "whisper".to_string(),
        _ => "system".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_bus_emit_subscribe() {
        let bus = EventBus::new(16);
        let mut rx = bus.subscribe();

        bus.emit(Event {
            event_type: "DaemonStarted".to_string(),
            timestamp: 1000,
            payload: serde_json::json!({"version": "0.1.0"}),
        });

        let event = rx.try_recv().expect("receive event");
        assert_eq!(event.event_type, "DaemonStarted");
        assert_eq!(bus.sequence(), 1);
    }

    #[test]
    fn test_event_filter_categories() {
        let filter = EventFilter {
            categories: Some(vec!["space".to_string()]),
            group_ids: None,
            min_severity: None,
        };

        let space_event = Event {
            event_type: "MemberJoined".to_string(),
            timestamp: 1000,
            payload: serde_json::json!({}),
        };
        assert!(filter.matches(&space_event));

        let economy_event = Event {
            event_type: "FundsReceived".to_string(),
            timestamp: 1000,
            payload: serde_json::json!({}),
        };
        assert!(!filter.matches(&economy_event));
    }

    #[test]
    fn test_categorize_event() {
        assert_eq!(categorize_event("MemberJoined"), "space");
        assert_eq!(categorize_event("ContentPublished"), "space");
        assert_eq!(categorize_event("EpochEarningsSummary"), "economy");
        assert_eq!(categorize_event("FundsReceived"), "economy");
        assert_eq!(categorize_event("WhisperReceived"), "whisper");
        assert_eq!(categorize_event("HandleDeprecated"), "whisper");
        assert_eq!(categorize_event("DaemonStarted"), "system");
    }
}
