//! Quorum membership management.
//!
//! Manages the selection and rotation of quorum members for FROST
//! threshold signing ceremonies. Quorums are selected from eligible
//! nodes based on PoSrv scores, and churn is limited per epoch to
//! maintain key continuity.

use serde::{Deserialize, Serialize};

use crate::{FrostCoordError, Result};

/// Configuration for a quorum.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuorumConfig {
    /// Signing threshold (t in t-of-n).
    pub threshold: u16,
    /// Current quorum member node IDs.
    pub members: Vec<[u8; 32]>,
    /// Maximum number of members that can change per epoch.
    pub max_churn_per_epoch: usize,
}

impl QuorumConfig {
    /// Create a new quorum configuration.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Minimum signers required.
    /// * `members` - Initial quorum member node IDs.
    /// * `max_churn_per_epoch` - Maximum membership changes per epoch.
    pub fn new(threshold: u16, members: Vec<[u8; 32]>, max_churn_per_epoch: usize) -> Result<Self> {
        if threshold == 0 || threshold as usize > members.len() {
            return Err(FrostCoordError::Quorum(format!(
                "invalid threshold {threshold} for {} members",
                members.len()
            )));
        }
        if members.is_empty() {
            return Err(FrostCoordError::Quorum(
                "quorum must have at least one member".to_string(),
            ));
        }

        Ok(Self {
            threshold,
            members,
            max_churn_per_epoch,
        })
    }

    /// Get the number of quorum members.
    pub fn size(&self) -> usize {
        self.members.len()
    }

    /// Check if a node is a quorum member.
    pub fn is_member(&self, node_id: &[u8; 32]) -> bool {
        self.members.iter().any(|m| m == node_id)
    }
}

/// An eligible node with its PoSrv score for quorum selection.
#[derive(Clone, Debug)]
pub struct EligibleNode {
    /// The node's identifier.
    pub node_id: [u8; 32],
    /// The node's PoSrv composite score.
    pub posrv_score: f64,
}

/// Select a quorum from eligible nodes.
///
/// Selects the top `required_size` nodes by PoSrv score. Nodes are
/// sorted by score in descending order, and ties are broken by node ID
/// (lexicographic order, ascending).
///
/// # Arguments
///
/// * `eligible_nodes` - All nodes eligible for quorum participation.
/// * `required_size` - The desired quorum size.
///
/// # Returns
///
/// A vector of the selected node IDs.
pub fn select_quorum(
    eligible_nodes: &[EligibleNode],
    required_size: usize,
) -> Result<Vec<[u8; 32]>> {
    if eligible_nodes.len() < required_size {
        return Err(FrostCoordError::InsufficientSigners {
            required: required_size,
            available: eligible_nodes.len(),
        });
    }

    let mut sorted: Vec<&EligibleNode> = eligible_nodes.iter().collect();
    sorted.sort_by(|a, b| {
        b.posrv_score
            .partial_cmp(&a.posrv_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.node_id.cmp(&b.node_id))
    });

    let selected: Vec<[u8; 32]> = sorted
        .iter()
        .take(required_size)
        .map(|n| n.node_id)
        .collect();

    tracing::debug!(
        selected = selected.len(),
        eligible = eligible_nodes.len(),
        "selected quorum members"
    );

    Ok(selected)
}

/// Check if a proposed quorum rotation is valid.
///
/// A rotation is valid if the number of membership changes (additions +
/// removals) does not exceed `max_churn_per_epoch`.
///
/// # Arguments
///
/// * `current` - The current quorum configuration.
/// * `proposed` - The proposed new member list.
///
/// # Returns
///
/// `true` if the rotation is within the churn limit, `false` otherwise.
pub fn can_rotate(current: &QuorumConfig, proposed: &[[u8; 32]]) -> bool {
    let current_set: std::collections::HashSet<[u8; 32]> =
        current.members.iter().copied().collect();
    let proposed_set: std::collections::HashSet<[u8; 32]> = proposed.iter().copied().collect();

    let added = proposed_set.difference(&current_set).count();
    let removed = current_set.difference(&proposed_set).count();
    let total_churn = added + removed;

    total_churn <= current.max_churn_per_epoch
}

/// Compute the churn between current and proposed quorums.
///
/// # Returns
///
/// A tuple of `(added_count, removed_count)`.
pub fn compute_churn(current: &QuorumConfig, proposed: &[[u8; 32]]) -> (usize, usize) {
    let current_set: std::collections::HashSet<[u8; 32]> =
        current.members.iter().copied().collect();
    let proposed_set: std::collections::HashSet<[u8; 32]> = proposed.iter().copied().collect();

    let added = proposed_set.difference(&current_set).count();
    let removed = current_set.difference(&proposed_set).count();

    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u8) -> [u8; 32] {
        [id; 32]
    }

    #[test]
    fn test_quorum_config_creation() {
        let members = vec![node(1), node(2), node(3)];
        let config = QuorumConfig::new(2, members, 1).expect("config");
        assert_eq!(config.size(), 3);
        assert_eq!(config.threshold, 2);
        assert!(config.is_member(&node(1)));
        assert!(!config.is_member(&node(4)));
    }

    #[test]
    fn test_quorum_config_invalid_threshold() {
        let result = QuorumConfig::new(5, vec![node(1), node(2)], 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_quorum_config_empty() {
        let result = QuorumConfig::new(1, vec![], 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_select_quorum_by_score() {
        let nodes = vec![
            EligibleNode {
                node_id: node(1),
                posrv_score: 0.5,
            },
            EligibleNode {
                node_id: node(2),
                posrv_score: 0.9,
            },
            EligibleNode {
                node_id: node(3),
                posrv_score: 0.7,
            },
            EligibleNode {
                node_id: node(4),
                posrv_score: 0.8,
            },
            EligibleNode {
                node_id: node(5),
                posrv_score: 0.6,
            },
        ];

        let selected = select_quorum(&nodes, 3).expect("select");
        assert_eq!(selected.len(), 3);
        // Should be nodes 2 (0.9), 4 (0.8), 3 (0.7).
        assert_eq!(selected[0], node(2));
        assert_eq!(selected[1], node(4));
        assert_eq!(selected[2], node(3));
    }

    #[test]
    fn test_select_quorum_insufficient_nodes() {
        let nodes = vec![EligibleNode {
            node_id: node(1),
            posrv_score: 0.5,
        }];
        let result = select_quorum(&nodes, 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_select_quorum_tiebreaker() {
        let nodes = vec![
            EligibleNode {
                node_id: node(3),
                posrv_score: 0.8,
            },
            EligibleNode {
                node_id: node(1),
                posrv_score: 0.8,
            },
            EligibleNode {
                node_id: node(2),
                posrv_score: 0.8,
            },
        ];

        let selected = select_quorum(&nodes, 3).expect("select");
        // All same score, tiebreak by node_id ascending.
        assert_eq!(selected[0], node(1));
        assert_eq!(selected[1], node(2));
        assert_eq!(selected[2], node(3));
    }

    #[test]
    fn test_can_rotate_within_churn() {
        let current = QuorumConfig::new(2, vec![node(1), node(2), node(3)], 2).expect("config");
        // Replace node(3) with node(4): 1 add + 1 remove = 2 churn.
        let proposed = [node(1), node(2), node(4)];
        assert!(can_rotate(&current, &proposed));
    }

    #[test]
    fn test_can_rotate_exceeds_churn() {
        let current = QuorumConfig::new(2, vec![node(1), node(2), node(3)], 1).expect("config");
        // Replace node(2) and node(3): 2 add + 2 remove = 4 churn (limit is 1).
        let proposed = [node(1), node(4), node(5)];
        assert!(!can_rotate(&current, &proposed));
    }

    #[test]
    fn test_can_rotate_no_change() {
        let current = QuorumConfig::new(2, vec![node(1), node(2), node(3)], 0).expect("config");
        let proposed = [node(1), node(2), node(3)];
        assert!(can_rotate(&current, &proposed));
    }

    #[test]
    fn test_compute_churn() {
        let current = QuorumConfig::new(2, vec![node(1), node(2), node(3)], 5).expect("config");
        let proposed = [node(2), node(3), node(4), node(5)];
        let (added, removed) = compute_churn(&current, &proposed);
        assert_eq!(added, 2); // node(4), node(5)
        assert_eq!(removed, 1); // node(1)
    }
}
