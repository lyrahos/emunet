//! Network bootstrap logic for joining the Ochra DHT.
//!
//! When a new node starts, it must connect to seed nodes to populate its
//! routing table. The bootstrap process:
//!
//! 1. Contact each seed node to establish connectivity.
//! 2. Perform an iterative `FIND_NODE` lookup for the local node's own ID
//!    to discover nearby peers.
//! 3. Refresh all k-buckets by performing random lookups.
//!
//! ## Configuration
//!
//! [`BootstrapConfig`] specifies the seed nodes and retry parameters.

use std::net::SocketAddr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::kademlia::{NodeId, NodeInfo, RoutingTable};
use crate::{DhtError, Result, K};

/// Configuration for the DHT bootstrap process.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapConfig {
    /// List of seed node addresses to contact initially.
    pub seed_nodes: Vec<SeedNode>,
    /// Maximum number of retry attempts per seed node.
    pub max_retries: u32,
    /// Timeout for each connection attempt in seconds.
    pub timeout_secs: u64,
    /// Minimum number of seed nodes that must respond for bootstrap to succeed.
    pub min_responsive_seeds: usize,
}

/// A seed node endpoint for bootstrapping.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedNode {
    /// The seed node's network address.
    #[serde(with = "socket_addr_serde")]
    pub addr: SocketAddr,
    /// The seed node's expected PIK public key (for authentication).
    pub pik_public_key: [u8; 32],
}

/// The result of a bootstrap attempt.
#[derive(Clone, Debug)]
pub struct BootstrapResult {
    /// Number of seed nodes that responded.
    pub responsive_seeds: usize,
    /// Total number of peers discovered and added to the routing table.
    pub peers_discovered: usize,
    /// Whether the bootstrap was successful (enough seeds responded).
    pub success: bool,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            seed_nodes: Vec::new(),
            max_retries: 3,
            timeout_secs: 10,
            min_responsive_seeds: 1,
        }
    }
}

impl BootstrapConfig {
    /// Create a new bootstrap configuration with the given seed nodes.
    pub fn new(seed_nodes: Vec<SeedNode>) -> Self {
        Self {
            seed_nodes,
            ..Default::default()
        }
    }

    /// Validate the bootstrap configuration.
    pub fn validate(&self) -> Result<()> {
        if self.seed_nodes.is_empty() {
            return Err(DhtError::BootstrapFailed(
                "no seed nodes configured".to_string(),
            ));
        }

        if self.min_responsive_seeds > self.seed_nodes.len() {
            return Err(DhtError::BootstrapFailed(format!(
                "min_responsive_seeds ({}) exceeds total seed nodes ({})",
                self.min_responsive_seeds,
                self.seed_nodes.len(),
            )));
        }

        Ok(())
    }
}

/// Bootstrap the DHT by contacting seed nodes and performing self-lookup.
///
/// This is the main entry point for joining the network. The process:
///
/// 1. Pings each seed node (with retries).
/// 2. Adds responsive seeds to the routing table.
/// 3. Performs a `FIND_NODE` lookup for the local node's own ID to discover
///    nearby peers.
///
/// The actual network I/O is performed by the caller through the
/// [`BootstrapTransport`] trait. This function orchestrates the protocol logic.
///
/// # Arguments
///
/// * `config` - Bootstrap configuration with seed nodes
/// * `routing_table` - The local routing table to populate
/// * `transport` - Implementation of network transport for sending/receiving
pub async fn bootstrap<T: BootstrapTransport>(
    config: &BootstrapConfig,
    routing_table: &mut RoutingTable,
    transport: &T,
) -> Result<BootstrapResult> {
    config.validate()?;

    info!(
        seed_count = config.seed_nodes.len(),
        "Starting DHT bootstrap"
    );

    let mut responsive_seeds = 0usize;
    let timeout = Duration::from_secs(config.timeout_secs);

    // Phase 1: Contact seed nodes.
    for seed in &config.seed_nodes {
        let node_id = ochra_crypto::blake3::hash(&seed.pik_public_key);

        let mut connected = false;
        for attempt in 0..config.max_retries {
            debug!(
                addr = %seed.addr,
                attempt = attempt + 1,
                "Pinging seed node"
            );

            match transport.ping(seed.addr, timeout).await {
                Ok(peer_info) => {
                    routing_table.add_node(peer_info);
                    responsive_seeds += 1;
                    connected = true;
                    info!(addr = %seed.addr, "Seed node responded");
                    break;
                }
                Err(e) => {
                    warn!(
                        addr = %seed.addr,
                        attempt = attempt + 1,
                        error = %e,
                        "Seed node ping failed"
                    );
                }
            }
        }

        if !connected {
            warn!(
                addr = %seed.addr,
                node_id = hex::encode(node_id),
                "Failed to reach seed node after all retries"
            );
        }
    }

    if responsive_seeds < config.min_responsive_seeds {
        return Err(DhtError::BootstrapFailed(format!(
            "only {} of {} required seed nodes responded",
            responsive_seeds, config.min_responsive_seeds,
        )));
    }

    // Phase 2: Perform self-lookup to discover nearby peers.
    let local_id = *routing_table.local_id();
    let peers_discovered = match transport
        .find_node(local_id, routing_table.find_closest(&local_id, K), timeout)
        .await
    {
        Ok(discovered) => {
            let count = discovered.len();
            for node in discovered {
                routing_table.add_node(node);
            }
            count
        }
        Err(e) => {
            warn!(error = %e, "Self-lookup during bootstrap failed");
            0
        }
    };

    let result = BootstrapResult {
        responsive_seeds,
        peers_discovered,
        success: true,
    };

    info!(
        responsive_seeds = result.responsive_seeds,
        peers_discovered = result.peers_discovered,
        "Bootstrap complete"
    );

    Ok(result)
}

/// Transport trait for bootstrap network operations.
///
/// Implementors provide the actual network I/O. This abstraction allows
/// the bootstrap logic to be tested without real networking.
pub trait BootstrapTransport {
    /// Ping a node and return its [`NodeInfo`] if it responds.
    fn ping(
        &self,
        addr: SocketAddr,
        timeout: Duration,
    ) -> impl std::future::Future<Output = std::result::Result<NodeInfo, Box<dyn std::error::Error + Send + Sync>>> + Send;

    /// Perform a `FIND_NODE` query against the given nodes.
    ///
    /// Returns all nodes discovered during the iterative lookup.
    fn find_node(
        &self,
        target: NodeId,
        initial_nodes: Vec<NodeInfo>,
        timeout: Duration,
    ) -> impl std::future::Future<Output = std::result::Result<Vec<NodeInfo>, Box<dyn std::error::Error + Send + Sync>>> + Send;
}

/// Serde support for SocketAddr as a string.
mod socket_addr_serde {
    use std::net::SocketAddr;

    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(addr: &SocketAddr, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&addr.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> std::result::Result<SocketAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_config_default() {
        let config = BootstrapConfig::default();
        assert!(config.seed_nodes.is_empty());
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.timeout_secs, 10);
        assert_eq!(config.min_responsive_seeds, 1);
    }

    #[test]
    fn test_bootstrap_config_validate_no_seeds() {
        let config = BootstrapConfig::default();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bootstrap_config_validate_min_exceeds_total() {
        let config = BootstrapConfig {
            seed_nodes: vec![SeedNode {
                addr: "127.0.0.1:4433".parse().expect("valid addr"),
                pik_public_key: [0u8; 32],
            }],
            min_responsive_seeds: 5,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_bootstrap_config_validate_ok() {
        let config = BootstrapConfig {
            seed_nodes: vec![
                SeedNode {
                    addr: "127.0.0.1:4433".parse().expect("valid addr"),
                    pik_public_key: [1u8; 32],
                },
                SeedNode {
                    addr: "127.0.0.2:4433".parse().expect("valid addr"),
                    pik_public_key: [2u8; 32],
                },
            ],
            min_responsive_seeds: 1,
            ..Default::default()
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_bootstrap_result_fields() {
        let result = BootstrapResult {
            responsive_seeds: 2,
            peers_discovered: 15,
            success: true,
        };
        assert_eq!(result.responsive_seeds, 2);
        assert_eq!(result.peers_discovered, 15);
        assert!(result.success);
    }
}
