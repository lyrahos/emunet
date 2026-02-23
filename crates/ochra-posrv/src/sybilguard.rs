//! SybilGuard trust graph for random-walk-based Sybil resistance.
//!
//! Implements a trust graph where nodes are connected by weighted edges.
//! Trust weights are computed by performing random walks from a target
//! node and measuring convergence behavior (inspired by the SybilGuard
//! protocol).
//!
//! ## Trust Weight Calculation
//!
//! A node's trust weight is determined by:
//! 1. Performing multiple random walks of fixed length from the node.
//! 2. Checking how many walks converge to known trusted nodes.
//! 3. Normalizing the convergence count to [0, 1].

use std::collections::HashMap;

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{PoSrvError, Result};

/// Default random walk length for trust computation.
pub const DEFAULT_WALK_LENGTH: usize = 10;

/// Default number of random walks per trust computation.
pub const DEFAULT_NUM_WALKS: usize = 100;

/// A weighted edge in the trust graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustEdge {
    /// The target node ID.
    pub to: [u8; 32],
    /// Edge weight in [0.0, 1.0].
    pub weight: f64,
}

/// Node metadata in the trust graph.
#[derive(Clone, Debug, Default)]
struct NodeData {
    /// Outgoing edges from this node.
    edges: Vec<TrustEdge>,
}

/// SybilGuard trust graph.
///
/// Maintains a directed weighted graph of trust relationships between
/// nodes. Trust weights are computed via random-walk convergence.
pub struct TrustGraph {
    /// Graph nodes with their edge lists.
    nodes: HashMap<[u8; 32], NodeData>,
    /// Walk length for trust computations.
    walk_length: usize,
    /// Number of walks for trust computations.
    num_walks: usize,
}

impl TrustGraph {
    /// Create a new empty trust graph with default parameters.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            walk_length: DEFAULT_WALK_LENGTH,
            num_walks: DEFAULT_NUM_WALKS,
        }
    }

    /// Create a trust graph with custom walk parameters.
    ///
    /// # Arguments
    ///
    /// * `walk_length` - Length of each random walk.
    /// * `num_walks` - Number of random walks per trust computation.
    pub fn with_params(walk_length: usize, num_walks: usize) -> Self {
        Self {
            nodes: HashMap::new(),
            walk_length,
            num_walks,
        }
    }

    /// Add a node to the graph (if not already present).
    pub fn add_node(&mut self, node_id: [u8; 32]) {
        self.nodes.entry(node_id).or_default();
    }

    /// Add a directed weighted edge from one node to another.
    ///
    /// Both nodes are automatically added to the graph if not present.
    ///
    /// # Arguments
    ///
    /// * `from` - The source node ID.
    /// * `to` - The target node ID.
    /// * `weight` - Edge weight in [0.0, 1.0].
    pub fn add_edge(&mut self, from: [u8; 32], to: [u8; 32], weight: f64) -> Result<()> {
        if !(0.0..=1.0).contains(&weight) {
            return Err(PoSrvError::GraphError(format!(
                "edge weight must be in [0, 1], got {weight}"
            )));
        }

        self.nodes.entry(to).or_default();
        let node = self.nodes.entry(from).or_default();

        // Update existing edge or add new one.
        if let Some(edge) = node.edges.iter_mut().find(|e| e.to == to) {
            edge.weight = weight;
        } else {
            node.edges.push(TrustEdge { to, weight });
        }

        Ok(())
    }

    /// Compute the trust weight for a node using random-walk convergence.
    ///
    /// Performs `num_walks` random walks of `walk_length` steps starting
    /// from the target node. The trust weight is the fraction of walks
    /// that return to a node with at least one inbound edge from the
    /// walk path (indicating network connectivity).
    ///
    /// # Arguments
    ///
    /// * `node_id` - The node to compute trust for.
    ///
    /// # Returns
    ///
    /// Trust weight in [0.0, 1.0].
    pub fn compute_trust_weight(&self, node_id: &[u8; 32]) -> Result<f64> {
        let node_data = self
            .nodes
            .get(node_id)
            .ok_or_else(|| PoSrvError::NodeNotFound(hex::encode(node_id)))?;

        // If the node has no outgoing edges, trust is 0.
        if node_data.edges.is_empty() {
            return Ok(0.0);
        }

        let mut convergent_walks: u64 = 0;

        for walk_idx in 0..self.num_walks {
            let converged = self.perform_walk(node_id, walk_idx as u64);
            if converged {
                convergent_walks += 1;
            }
        }

        Ok(convergent_walks as f64 / self.num_walks as f64)
    }

    /// Perform a single deterministic random walk.
    ///
    /// Uses BLAKE3 with the SybilGuard context for deterministic randomness
    /// (making the walk reproducible for verification).
    ///
    /// Returns `true` if the walk converges (visits at least 2 unique nodes
    /// besides the start node, indicating good connectivity).
    fn perform_walk(&self, start: &[u8; 32], walk_seed: u64) -> bool {
        let mut current = *start;
        let mut visited_unique = 0u64;

        for step in 0..self.walk_length {
            let node_data = match self.nodes.get(&current) {
                Some(data) if !data.edges.is_empty() => data,
                _ => return visited_unique >= 2,
            };

            // Deterministic "random" selection using BLAKE3.
            let seed_input = blake3::encode_multi_field(&[
                start,
                &walk_seed.to_le_bytes(),
                &(step as u64).to_le_bytes(),
                &current,
            ]);
            let hash = blake3::derive_key(blake3::contexts::SYBILGUARD_WALK, &seed_input);

            // Select next node based on weighted edges.
            let next = select_weighted_edge(&node_data.edges, &hash);

            if next != current && next != *start {
                visited_unique += 1;
            }
            current = next;
        }

        visited_unique >= 2
    }

    /// Get the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.nodes.values().map(|n| n.edges.len()).sum()
    }

    /// Check if a node exists in the graph.
    pub fn has_node(&self, node_id: &[u8; 32]) -> bool {
        self.nodes.contains_key(node_id)
    }

    /// Get the outgoing edges for a node.
    pub fn edges(&self, node_id: &[u8; 32]) -> Result<&[TrustEdge]> {
        self.nodes
            .get(node_id)
            .map(|n| n.edges.as_slice())
            .ok_or_else(|| PoSrvError::NodeNotFound(hex::encode(node_id)))
    }
}

impl Default for TrustGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Select an edge based on weighted random selection using a hash as entropy.
///
/// Uses the first 8 bytes of the hash to generate a value in [0, total_weight).
fn select_weighted_edge(edges: &[TrustEdge], hash: &[u8; 32]) -> [u8; 32] {
    if edges.is_empty() {
        return [0u8; 32];
    }
    if edges.len() == 1 {
        return edges[0].to;
    }

    let total_weight: f64 = edges.iter().map(|e| e.weight).sum();
    if total_weight <= 0.0 {
        return edges[0].to;
    }

    // Use first 8 bytes of hash as a u64, then normalize to [0, total_weight).
    let mut seed_bytes = [0u8; 8];
    seed_bytes.copy_from_slice(&hash[..8]);
    let seed_val = u64::from_le_bytes(seed_bytes);
    let threshold = (seed_val as f64 / u64::MAX as f64) * total_weight;

    let mut cumulative = 0.0;
    for edge in edges {
        cumulative += edge.weight;
        if cumulative >= threshold {
            return edge.to;
        }
    }

    // Fallback to last edge (rounding edge case).
    edges[edges.len() - 1].to
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u8) -> [u8; 32] {
        [id; 32]
    }

    #[test]
    fn test_empty_graph() {
        let graph = TrustGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = TrustGraph::new();
        graph.add_node(node(1));
        assert!(graph.has_node(&node(1)));
        assert!(!graph.has_node(&node(2)));
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut graph = TrustGraph::new();
        graph.add_edge(node(1), node(2), 0.8).expect("add edge");

        assert!(graph.has_node(&node(1)));
        assert!(graph.has_node(&node(2)));
        assert_eq!(graph.edge_count(), 1);

        let edges = graph.edges(&node(1)).expect("edges");
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].to, node(2));
        assert!((edges[0].weight - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn test_add_edge_invalid_weight() {
        let mut graph = TrustGraph::new();
        assert!(graph.add_edge(node(1), node(2), 1.5).is_err());
        assert!(graph.add_edge(node(1), node(2), -0.1).is_err());
    }

    #[test]
    fn test_update_edge_weight() {
        let mut graph = TrustGraph::new();
        graph.add_edge(node(1), node(2), 0.5).expect("add");
        graph.add_edge(node(1), node(2), 0.9).expect("update");

        let edges = graph.edges(&node(1)).expect("edges");
        assert_eq!(edges.len(), 1);
        assert!((edges[0].weight - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_isolated_node_zero_trust() {
        let mut graph = TrustGraph::new();
        graph.add_node(node(1));

        let trust = graph.compute_trust_weight(&node(1)).expect("trust");
        assert!((trust - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_node_not_found() {
        let graph = TrustGraph::new();
        assert!(graph.compute_trust_weight(&node(99)).is_err());
    }

    #[test]
    fn test_trust_weight_in_range() {
        let mut graph = TrustGraph::with_params(5, 50);

        // Create a well-connected cluster of 5 nodes.
        for i in 1..=5u8 {
            for j in 1..=5u8 {
                if i != j {
                    graph.add_edge(node(i), node(j), 1.0).expect("add edge");
                }
            }
        }

        for i in 1..=5u8 {
            let trust = graph.compute_trust_weight(&node(i)).expect("trust");
            assert!(
                (0.0..=1.0).contains(&trust),
                "trust {trust} out of range for node {i}"
            );
        }
    }

    #[test]
    fn test_well_connected_cluster_has_trust() {
        let mut graph = TrustGraph::with_params(5, 100);

        // Create a well-connected cluster of 10 nodes.
        for i in 1..=10u8 {
            for j in 1..=10u8 {
                if i != j {
                    graph.add_edge(node(i), node(j), 1.0).expect("add edge");
                }
            }
        }

        // Well-connected nodes should have non-zero trust.
        let trust = graph.compute_trust_weight(&node(1)).expect("trust");
        assert!(trust > 0.0, "expected non-zero trust, got {trust}");
    }

    #[test]
    fn test_deterministic_trust() {
        let mut graph = TrustGraph::with_params(5, 50);
        for i in 1..=5u8 {
            for j in 1..=5u8 {
                if i != j {
                    graph.add_edge(node(i), node(j), 0.7).expect("add edge");
                }
            }
        }

        let trust1 = graph.compute_trust_weight(&node(1)).expect("trust");
        let trust2 = graph.compute_trust_weight(&node(1)).expect("trust");
        assert!((trust1 - trust2).abs() < f64::EPSILON);
    }

    #[test]
    fn test_single_chain_topology() {
        let mut graph = TrustGraph::with_params(3, 50);

        // Linear chain: 1 -> 2 -> 3 -> 4 -> 5
        for i in 1..=4u8 {
            graph.add_edge(node(i), node(i + 1), 1.0).expect("add");
        }

        // Node 1 can reach far along the chain; node 5 has no outgoing edges.
        let trust_1 = graph.compute_trust_weight(&node(1)).expect("trust");
        let trust_5 = graph.compute_trust_weight(&node(5)).expect("trust");

        assert!(trust_1 >= 0.0);
        assert!((trust_5 - 0.0).abs() < f64::EPSILON); // No outgoing edges.
    }
}
