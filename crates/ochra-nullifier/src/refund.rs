//! Refund commitment tree.
//!
//! The refund tree tracks refund commitments for tokens that need to be
//! returned (e.g., escrow timeouts, disputed transactions). Each commitment
//! is a 32-byte hash, and the tree provides a Merkle root for epoch snapshots.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

// Refund tree does not currently use crate-level error types directly.

/// A refund commitment entry with its associated epoch.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RefundEntry {
    /// The refund commitment hash.
    pub commitment: [u8; 32],
    /// The epoch in which this refund was created.
    pub epoch: u64,
}

/// A tree of refund commitments providing a Merkle root for epoch snapshots.
pub struct RefundTree {
    /// The list of refund commitment entries.
    commitments: Vec<RefundEntry>,
}

impl RefundTree {
    /// Create a new empty refund tree.
    pub fn new() -> Self {
        Self {
            commitments: Vec::new(),
        }
    }

    /// Add a refund commitment to the tree.
    ///
    /// # Arguments
    ///
    /// * `commitment` - The 32-byte refund commitment hash
    /// * `epoch` - The epoch for this refund
    pub fn add_commitment(&mut self, commitment: [u8; 32], epoch: u64) {
        self.commitments.push(RefundEntry { commitment, epoch });
    }

    /// Get the Merkle root of all current commitments.
    ///
    /// If the tree is empty, returns an all-zero hash. Otherwise, computes
    /// a binary Merkle tree using domain-separated BLAKE3 hashing.
    pub fn get_merkle_root(&self) -> [u8; 32] {
        if self.commitments.is_empty() {
            return [0u8; 32];
        }

        // Compute leaf hashes
        let mut layer: Vec<[u8; 32]> = self
            .commitments
            .iter()
            .map(|entry| blake3::merkle_leaf(&entry.commitment))
            .collect();

        // Build the Merkle tree bottom-up
        while layer.len() > 1 {
            let mut next_layer = Vec::with_capacity(layer.len().div_ceil(2));
            let mut i = 0;
            while i < layer.len() {
                if i + 1 < layer.len() {
                    next_layer.push(blake3::merkle_inner(&layer[i], &layer[i + 1]));
                } else {
                    // Odd node: hash with itself
                    next_layer.push(blake3::merkle_inner(&layer[i], &layer[i]));
                }
                i += 2;
            }
            layer = next_layer;
        }

        layer[0]
    }

    /// Prune all entries from the given epoch or earlier.
    ///
    /// # Arguments
    ///
    /// * `epoch` - Remove all entries with epoch <= this value
    pub fn prune_epoch(&mut self, epoch: u64) {
        let before = self.commitments.len();
        self.commitments.retain(|entry| entry.epoch > epoch);
        let pruned = before - self.commitments.len();
        if pruned > 0 {
            tracing::debug!(pruned, epoch, "refund tree: pruned expired entries");
        }
    }

    /// Get the number of commitments in the tree.
    pub fn len(&self) -> usize {
        self.commitments.len()
    }

    /// Check whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.commitments.is_empty()
    }

    /// Get all commitments for a specific epoch.
    pub fn commitments_for_epoch(&self, epoch: u64) -> Vec<&RefundEntry> {
        self.commitments
            .iter()
            .filter(|e| e.epoch == epoch)
            .collect()
    }

    /// Verify that a commitment exists in the tree.
    pub fn contains(&self, commitment: &[u8; 32]) -> bool {
        self.commitments.iter().any(|e| &e.commitment == commitment)
    }
}

impl Default for RefundTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Derive a refund commitment from token data.
///
/// `commitment = BLAKE3::derive_key("Ochra v1 refund-commitment", serial || amount_le)`
pub fn derive_refund_commitment(serial: &[u8; 32], amount: u64) -> [u8; 32] {
    let amount_bytes = amount.to_le_bytes();
    let input = blake3::encode_multi_field(&[serial.as_slice(), &amount_bytes]);
    blake3::derive_key(blake3::contexts::REFUND_COMMITMENT, &input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let tree = RefundTree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
        assert_eq!(tree.get_merkle_root(), [0u8; 32]);
    }

    #[test]
    fn test_add_and_root() {
        let mut tree = RefundTree::new();
        tree.add_commitment([0xAA; 32], 1);
        assert_eq!(tree.len(), 1);
        assert!(!tree.is_empty());

        let root = tree.get_merkle_root();
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_merkle_root_changes() {
        let mut tree = RefundTree::new();
        tree.add_commitment([0xAA; 32], 1);
        let root1 = tree.get_merkle_root();

        tree.add_commitment([0xBB; 32], 1);
        let root2 = tree.get_merkle_root();

        assert_ne!(root1, root2);
    }

    #[test]
    fn test_prune_epoch() {
        let mut tree = RefundTree::new();
        tree.add_commitment([0x01; 32], 1);
        tree.add_commitment([0x02; 32], 2);
        tree.add_commitment([0x03; 32], 3);
        assert_eq!(tree.len(), 3);

        // Prune epoch <= 2
        tree.prune_epoch(2);
        assert_eq!(tree.len(), 1);
        assert!(tree.contains(&[0x03; 32]));
        assert!(!tree.contains(&[0x01; 32]));
    }

    #[test]
    fn test_commitments_for_epoch() {
        let mut tree = RefundTree::new();
        tree.add_commitment([0x01; 32], 1);
        tree.add_commitment([0x02; 32], 2);
        tree.add_commitment([0x03; 32], 2);

        let epoch2 = tree.commitments_for_epoch(2);
        assert_eq!(epoch2.len(), 2);
    }

    #[test]
    fn test_derive_refund_commitment_deterministic() {
        let c1 = derive_refund_commitment(&[0xAA; 32], 1000);
        let c2 = derive_refund_commitment(&[0xAA; 32], 1000);
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_derive_refund_commitment_varies() {
        let c1 = derive_refund_commitment(&[0xAA; 32], 1000);
        let c2 = derive_refund_commitment(&[0xBB; 32], 1000);
        assert_ne!(c1, c2);

        let c3 = derive_refund_commitment(&[0xAA; 32], 2000);
        assert_ne!(c1, c3);
    }
}
