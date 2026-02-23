//! Nullifier gossip protocol.
//!
//! Nullifiers are propagated across the network using a gossip protocol.
//! Each gossip message contains a batch of nullifiers along with the epoch
//! and sender identification.

use serde::{Deserialize, Serialize};

use crate::bloom::NullifierSet;

/// A gossip message carrying nullifiers to propagate.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GossipMessage {
    /// Batch of nullifiers to propagate.
    pub nullifiers: Vec<[u8; 32]>,
    /// Epoch number this batch belongs to.
    pub epoch: u64,
    /// Sender node identifier.
    pub sender_id: [u8; 32],
}

/// Process a gossip message against a local nullifier set.
///
/// Inserts any nullifiers from the message that are not already in the local set.
/// Returns the list of nullifiers that were genuinely new (not previously seen).
///
/// # Arguments
///
/// * `msg` - The incoming gossip message
/// * `local_set` - The local nullifier Bloom filter to update
pub fn process_gossip(msg: &GossipMessage, local_set: &mut NullifierSet) -> Vec<[u8; 32]> {
    let mut new_nullifiers = Vec::new();

    for nullifier in &msg.nullifiers {
        if !local_set.contains(nullifier) {
            local_set.insert(nullifier);
            new_nullifiers.push(*nullifier);
        }
    }

    if !new_nullifiers.is_empty() {
        tracing::debug!(
            count = new_nullifiers.len(),
            epoch = msg.epoch,
            "processed gossip: new nullifiers inserted"
        );
    }

    new_nullifiers
}

/// Create a gossip message from a batch of new nullifiers.
///
/// # Arguments
///
/// * `nullifiers` - The nullifiers to gossip
/// * `epoch` - The current epoch
/// * `sender_id` - The sender's node ID
pub fn create_gossip_message(
    nullifiers: Vec<[u8; 32]>,
    epoch: u64,
    sender_id: [u8; 32],
) -> GossipMessage {
    GossipMessage {
        nullifiers,
        epoch,
        sender_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_gossip_new_nullifiers() {
        let mut local_set = NullifierSet::new();
        let msg = GossipMessage {
            nullifiers: vec![[0x01; 32], [0x02; 32], [0x03; 32]],
            epoch: 1,
            sender_id: [0xAA; 32],
        };

        let new = process_gossip(&msg, &mut local_set);
        assert_eq!(new.len(), 3);
        assert!(local_set.contains(&[0x01; 32]));
        assert!(local_set.contains(&[0x02; 32]));
        assert!(local_set.contains(&[0x03; 32]));
    }

    #[test]
    fn test_process_gossip_duplicate_nullifiers() {
        let mut local_set = NullifierSet::new();
        local_set.insert(&[0x01; 32]);

        let msg = GossipMessage {
            nullifiers: vec![[0x01; 32], [0x02; 32]],
            epoch: 1,
            sender_id: [0xAA; 32],
        };

        let new = process_gossip(&msg, &mut local_set);
        // Only [0x02] is new
        assert_eq!(new.len(), 1);
        assert_eq!(new[0], [0x02; 32]);
    }

    #[test]
    fn test_process_gossip_all_duplicates() {
        let mut local_set = NullifierSet::new();
        local_set.insert(&[0x01; 32]);
        local_set.insert(&[0x02; 32]);

        let msg = GossipMessage {
            nullifiers: vec![[0x01; 32], [0x02; 32]],
            epoch: 1,
            sender_id: [0xAA; 32],
        };

        let new = process_gossip(&msg, &mut local_set);
        assert!(new.is_empty());
    }

    #[test]
    fn test_create_gossip_message() {
        let msg = create_gossip_message(
            vec![[0x01; 32], [0x02; 32]],
            42,
            [0xAA; 32],
        );
        assert_eq!(msg.nullifiers.len(), 2);
        assert_eq!(msg.epoch, 42);
        assert_eq!(msg.sender_id, [0xAA; 32]);
    }
}
