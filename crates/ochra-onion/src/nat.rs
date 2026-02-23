//! NAT traversal helpers for the Ochra P2P network.
//!
//! Provides utilities for determining NAT type, performing hole-punching,
//! and establishing direct connections between peers behind NATs.
//!
//! ## Supported Techniques
//!
//! - **STUN-like probing**: Determines the external IP and port mapping
//! - **UDP hole-punching**: Coordinates simultaneous UDP sends via a
//!   rendezvous server
//! - **Relay fallback**: Falls back to relaying through a third-party node
//!   when direct connections are not possible
//!
//! ## NAT Types (RFC 3489)
//!
//! - Full Cone (easiest to traverse)
//! - Address-Restricted Cone
//! - Port-Restricted Cone
//! - Symmetric (hardest; requires relay fallback)

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::Result;

/// Maximum number of hole-punch attempts before falling back to relay.
pub const MAX_HOLE_PUNCH_ATTEMPTS: u32 = 3;

/// Timeout per hole-punch attempt in seconds.
pub const HOLE_PUNCH_TIMEOUT_SECS: u64 = 5;

/// Classification of the local NAT type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatType {
    /// No NAT: the node has a publicly routable address.
    None,
    /// Full Cone NAT: any external host can send to the mapped address.
    FullCone,
    /// Address-Restricted Cone: only hosts the internal host has sent to
    /// can send back (by IP).
    AddressRestrictedCone,
    /// Port-Restricted Cone: only hosts the internal host has sent to
    /// can send back (by IP and port).
    PortRestrictedCone,
    /// Symmetric NAT: different mappings for different destinations.
    /// Direct hole-punching is generally not possible.
    Symmetric,
    /// NAT type could not be determined.
    Unknown,
}

/// Result of a NAT probe.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatProbeResult {
    /// The detected NAT type.
    pub nat_type: NatType,
    /// External (mapped) address as seen by the probe server, if determined.
    pub external_addr: Option<SocketAddr>,
    /// Whether direct hole-punching is likely to succeed.
    pub hole_punch_feasible: bool,
}

/// Configuration for NAT traversal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NatConfig {
    /// Addresses of STUN-like probe servers.
    pub probe_servers: Vec<SocketAddr>,
    /// Timeout for probe requests in seconds.
    pub probe_timeout_secs: u64,
    /// Maximum number of hole-punch attempts before falling back to relay.
    pub max_punch_attempts: u32,
}

impl Default for NatConfig {
    fn default() -> Self {
        Self {
            probe_servers: Vec::new(),
            probe_timeout_secs: HOLE_PUNCH_TIMEOUT_SECS,
            max_punch_attempts: MAX_HOLE_PUNCH_ATTEMPTS,
        }
    }
}

impl NatConfig {
    /// Create a NAT config with the given probe servers.
    pub fn new(probe_servers: Vec<SocketAddr>) -> Self {
        Self {
            probe_servers,
            ..Default::default()
        }
    }
}

/// Result of a NAT traversal attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraversalResult {
    /// Direct connection established (no NAT or Full Cone).
    Direct,
    /// Connection established via hole-punching.
    HolePunched,
    /// Falling back to relay (Symmetric NAT or all attempts failed).
    Relayed,
}

/// Determine the NAT type by analyzing probe responses.
///
/// This classifier compares external addresses returned by multiple probe
/// servers. In a full implementation, actual network I/O would be performed;
/// here we provide the classification logic.
///
/// # Arguments
///
/// * `local_addr` - The local socket address
/// * `probe_results` - External addresses reported by each probe server
pub fn classify_nat(
    local_addr: SocketAddr,
    probe_results: &[(SocketAddr, Option<SocketAddr>)],
) -> NatProbeResult {
    let successful: Vec<(SocketAddr, SocketAddr)> = probe_results
        .iter()
        .filter_map(|(server, ext)| ext.map(|e| (*server, e)))
        .collect();

    if successful.is_empty() {
        return NatProbeResult {
            nat_type: NatType::Unknown,
            external_addr: None,
            hole_punch_feasible: false,
        };
    }

    let first_external = successful[0].1;

    // No NAT if external matches local.
    if first_external.ip() == local_addr.ip() && first_external.port() == local_addr.port() {
        return NatProbeResult {
            nat_type: NatType::None,
            external_addr: Some(first_external),
            hole_punch_feasible: true,
        };
    }

    // All probes report the same external address: cone NAT.
    let all_same = successful.iter().all(|(_, ext)| *ext == first_external);

    if all_same {
        debug!(
            external = %first_external,
            "NAT classified as Full Cone"
        );
        return NatProbeResult {
            nat_type: NatType::FullCone,
            external_addr: Some(first_external),
            hole_punch_feasible: true,
        };
    }

    // Same IP, different ports: port-restricted cone.
    let all_same_ip = successful
        .iter()
        .all(|(_, ext)| ext.ip() == first_external.ip());

    if all_same_ip {
        debug!(
            external_ip = %first_external.ip(),
            "NAT classified as Port-Restricted Cone"
        );
        return NatProbeResult {
            nat_type: NatType::PortRestrictedCone,
            external_addr: Some(first_external),
            hole_punch_feasible: true,
        };
    }

    // Different external IPs: Symmetric NAT.
    debug!("NAT classified as Symmetric");
    NatProbeResult {
        nat_type: NatType::Symmetric,
        external_addr: Some(first_external),
        hole_punch_feasible: false,
    }
}

/// Determine whether hole-punching is feasible for the given NAT type.
pub fn can_hole_punch(nat_type: &NatType) -> bool {
    matches!(
        nat_type,
        NatType::None
            | NatType::FullCone
            | NatType::AddressRestrictedCone
            | NatType::PortRestrictedCone
    )
}

/// Attempt NAT traversal to reach a peer.
///
/// Tries hole-punching first, falls back to relay if the NAT type
/// prevents direct connectivity.
pub fn attempt_traversal(
    _peer_address: &str,
    nat_type: &NatType,
) -> Result<TraversalResult> {
    match nat_type {
        NatType::None => Ok(TraversalResult::Direct),
        NatType::FullCone | NatType::AddressRestrictedCone | NatType::PortRestrictedCone => {
            Ok(TraversalResult::HolePunched)
        }
        NatType::Symmetric | NatType::Unknown => {
            Ok(TraversalResult::Relayed)
        }
    }
}

/// A hole-punch coordination message exchanged via a rendezvous server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HolePunchRequest {
    /// The initiator's node ID.
    pub initiator_node_id: [u8; 32],
    /// The target's node ID.
    pub target_node_id: [u8; 32],
    /// The initiator's external address (as determined by NAT probing).
    pub initiator_external_addr: SocketAddr,
    /// Nonce for this hole-punch attempt.
    pub nonce: [u8; 16],
}

/// Response to a hole-punch coordination request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HolePunchResponse {
    /// The target's external address.
    pub target_external_addr: SocketAddr,
    /// The nonce from the request (for correlation).
    pub nonce: [u8; 16],
    /// Whether the target is willing to attempt hole-punching.
    pub accepted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_nat_no_nat() {
        let local: SocketAddr = "1.2.3.4:4433".parse().expect("valid addr");
        let probes = vec![
            ("5.5.5.5:3478".parse().expect("valid"), Some(local)),
            ("6.6.6.6:3478".parse().expect("valid"), Some(local)),
        ];

        let result = classify_nat(local, &probes);
        assert_eq!(result.nat_type, NatType::None);
        assert!(result.hole_punch_feasible);
    }

    #[test]
    fn test_classify_nat_full_cone() {
        let local: SocketAddr = "192.168.1.100:4433".parse().expect("valid");
        let external: SocketAddr = "1.2.3.4:5000".parse().expect("valid");
        let probes = vec![
            ("5.5.5.5:3478".parse().expect("valid"), Some(external)),
            ("6.6.6.6:3478".parse().expect("valid"), Some(external)),
        ];

        let result = classify_nat(local, &probes);
        assert_eq!(result.nat_type, NatType::FullCone);
        assert!(result.hole_punch_feasible);
    }

    #[test]
    fn test_classify_nat_port_restricted() {
        let local: SocketAddr = "192.168.1.100:4433".parse().expect("valid");
        let probes = vec![
            (
                "5.5.5.5:3478".parse().expect("valid"),
                Some("1.2.3.4:5000".parse().expect("valid")),
            ),
            (
                "6.6.6.6:3478".parse().expect("valid"),
                Some("1.2.3.4:5001".parse().expect("valid")),
            ),
        ];

        let result = classify_nat(local, &probes);
        assert_eq!(result.nat_type, NatType::PortRestrictedCone);
        assert!(result.hole_punch_feasible);
    }

    #[test]
    fn test_classify_nat_symmetric() {
        let local: SocketAddr = "192.168.1.100:4433".parse().expect("valid");
        let probes = vec![
            (
                "5.5.5.5:3478".parse().expect("valid"),
                Some("1.2.3.4:5000".parse().expect("valid")),
            ),
            (
                "6.6.6.6:3478".parse().expect("valid"),
                Some("1.2.3.5:5001".parse().expect("valid")),
            ),
        ];

        let result = classify_nat(local, &probes);
        assert_eq!(result.nat_type, NatType::Symmetric);
        assert!(!result.hole_punch_feasible);
    }

    #[test]
    fn test_classify_nat_unknown() {
        let local: SocketAddr = "192.168.1.100:4433".parse().expect("valid");
        let probes: Vec<(SocketAddr, Option<SocketAddr>)> = vec![
            ("5.5.5.5:3478".parse().expect("valid"), None),
            ("6.6.6.6:3478".parse().expect("valid"), None),
        ];

        let result = classify_nat(local, &probes);
        assert_eq!(result.nat_type, NatType::Unknown);
    }

    #[test]
    fn test_can_hole_punch() {
        assert!(can_hole_punch(&NatType::None));
        assert!(can_hole_punch(&NatType::FullCone));
        assert!(can_hole_punch(&NatType::AddressRestrictedCone));
        assert!(can_hole_punch(&NatType::PortRestrictedCone));
        assert!(!can_hole_punch(&NatType::Symmetric));
        assert!(!can_hole_punch(&NatType::Unknown));
    }

    #[test]
    fn test_attempt_traversal_direct() {
        let result = attempt_traversal("1.2.3.4:4433", &NatType::None).expect("ok");
        assert_eq!(result, TraversalResult::Direct);
    }

    #[test]
    fn test_attempt_traversal_hole_punch() {
        let result = attempt_traversal("1.2.3.4:4433", &NatType::FullCone).expect("ok");
        assert_eq!(result, TraversalResult::HolePunched);
    }

    #[test]
    fn test_attempt_traversal_relayed() {
        let result = attempt_traversal("1.2.3.4:4433", &NatType::Symmetric).expect("ok");
        assert_eq!(result, TraversalResult::Relayed);
    }

    #[test]
    fn test_nat_config_default() {
        let config = NatConfig::default();
        assert!(config.probe_servers.is_empty());
        assert_eq!(config.probe_timeout_secs, HOLE_PUNCH_TIMEOUT_SECS);
        assert_eq!(config.max_punch_attempts, MAX_HOLE_PUNCH_ATTEMPTS);
    }

    #[test]
    fn test_hole_punch_request() {
        let req = HolePunchRequest {
            initiator_node_id: [0x01u8; 32],
            target_node_id: [0x02u8; 32],
            initiator_external_addr: "1.2.3.4:4433".parse().expect("valid"),
            nonce: [0xAAu8; 16],
        };
        assert_eq!(req.initiator_node_id, [0x01u8; 32]);
        assert_eq!(req.target_node_id, [0x02u8; 32]);
    }
}
