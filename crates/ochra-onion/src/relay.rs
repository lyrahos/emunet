//! Relay selection with PoSrv-weighted random sampling.
//!
//! Selects relays for Sphinx circuit construction, applying constraints to
//! ensure diversity and resist correlation attacks:
//!
//! ## Selection Constraints
//!
//! - No two relays in the same `/24` subnet
//! - No relay sharing an AS number with the source or destination
//! - Geographic diversity (prefer relays in different country codes)
//!
//! ## PoSrv Weighting
//!
//! Relays are selected with probability proportional to their PoSrv score,
//! which reflects their Proof of Service and Routing contribution to the
//! network.

use std::collections::HashSet;
use std::net::Ipv4Addr;

use ochra_types::network::RelayDescriptor;
use tracing::debug;

use crate::{OnionError, Result, CIRCUIT_HOPS};

/// Selects relays for circuit construction with constraint enforcement.
pub struct RelaySelector {
    /// Constraints to apply during selection.
    constraints: SelectionConstraints,
}

/// Constraints applied during relay selection.
#[derive(Clone, Debug, Default)]
pub struct SelectionConstraints {
    /// AS numbers to exclude (e.g., source and destination AS).
    pub excluded_as_numbers: HashSet<u32>,
    /// When true, try to pick relays from different countries.
    pub preferred_diversity: bool,
}

/// Cached relay descriptors for selection.
pub struct RelayCache {
    /// All known relay descriptors.
    relays: Vec<RelayDescriptor>,
}

impl RelayCache {
    /// Create a new empty relay cache.
    pub fn new() -> Self {
        Self { relays: Vec::new() }
    }

    /// Create a relay cache from a list of descriptors.
    pub fn from_descriptors(relays: Vec<RelayDescriptor>) -> Self {
        Self { relays }
    }

    /// Add a relay descriptor to the cache.
    pub fn add(&mut self, relay: RelayDescriptor) {
        // Replace if same node_id already exists.
        if let Some(existing) = self.relays.iter_mut().find(|r| r.node_id == relay.node_id) {
            *existing = relay;
        } else {
            self.relays.push(relay);
        }
    }

    /// Remove a relay by node ID.
    pub fn remove(&mut self, node_id: &[u8; 32]) {
        self.relays.retain(|r| &r.node_id != node_id);
    }

    /// Return all cached relay descriptors.
    pub fn all(&self) -> &[RelayDescriptor] {
        &self.relays
    }

    /// Return the number of cached relays.
    pub fn len(&self) -> usize {
        self.relays.len()
    }

    /// Return whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.relays.is_empty()
    }

    /// Filter relays by minimum PoSrv score.
    pub fn filter_by_min_score(&self, min_score: f32) -> Vec<&RelayDescriptor> {
        self.relays
            .iter()
            .filter(|r| r.posrv_score >= min_score)
            .collect()
    }
}

impl Default for RelayCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RelaySelector {
    /// Create a new relay selector with default constraints.
    pub fn new() -> Self {
        Self {
            constraints: SelectionConstraints::default(),
        }
    }

    /// Create a new relay selector with custom constraints.
    pub fn with_constraints(constraints: SelectionConstraints) -> Self {
        Self { constraints }
    }

    /// Select `CIRCUIT_HOPS` (3) relays from the cache for circuit construction.
    ///
    /// Uses PoSrv-weighted random sampling with constraint enforcement.
    /// Returns an ordered list: `[entry, middle, exit]`.
    pub fn select_relays(&self, cache: &RelayCache) -> Result<Vec<RelayDescriptor>> {
        let available = cache.all();

        if available.len() < CIRCUIT_HOPS {
            return Err(OnionError::InsufficientRelays {
                need: CIRCUIT_HOPS,
                have: available.len(),
            });
        }

        // Filter out relays in excluded AS numbers.
        let mut candidates: Vec<&RelayDescriptor> = available
            .iter()
            .filter(|r| !self.constraints.excluded_as_numbers.contains(&r.as_number))
            .collect();

        if candidates.len() < CIRCUIT_HOPS {
            return Err(OnionError::InsufficientRelays {
                need: CIRCUIT_HOPS,
                have: candidates.len(),
            });
        }

        let mut selected: Vec<RelayDescriptor> = Vec::with_capacity(CIRCUIT_HOPS);
        let mut used_subnets: HashSet<[u8; 3]> = HashSet::new();
        let mut used_as: HashSet<u32> = HashSet::new();
        let mut used_countries: HashSet<[u8; 2]> = HashSet::new();

        for hop_idx in 0..CIRCUIT_HOPS {
            // Filter candidates for this hop.
            let eligible: Vec<&&RelayDescriptor> = candidates
                .iter()
                .filter(|r| {
                    // Subnet constraint: no two relays in same /24.
                    let subnet = extract_subnet_24(&r.ip_addr);
                    if let Some(s) = subnet {
                        if used_subnets.contains(&s) {
                            return false;
                        }
                    }

                    // AS constraint: no shared AS with already-selected relays.
                    if used_as.contains(&r.as_number) {
                        return false;
                    }

                    // Geographic diversity: prefer different countries (soft).
                    if self.constraints.preferred_diversity
                        && used_countries.contains(&r.country_code)
                        && candidates.len() > CIRCUIT_HOPS
                    {
                        return false;
                    }

                    true
                })
                .collect();

            if eligible.is_empty() {
                // Fall back: drop geographic diversity constraint.
                let fallback: Vec<&&RelayDescriptor> = candidates
                    .iter()
                    .filter(|r| {
                        let subnet = extract_subnet_24(&r.ip_addr);
                        if let Some(s) = subnet {
                            if used_subnets.contains(&s) {
                                return false;
                            }
                        }
                        if used_as.contains(&r.as_number) {
                            return false;
                        }
                        true
                    })
                    .collect();

                if fallback.is_empty() {
                    return Err(OnionError::ConstraintViolation(format!(
                        "cannot select relay for hop {} with subnet/AS constraints",
                        hop_idx
                    )));
                }

                let chosen = weighted_select(&fallback)?;
                record_selection(chosen, &mut used_subnets, &mut used_as, &mut used_countries);
                selected.push(chosen.clone());
                let chosen_id = chosen.node_id;
                candidates.retain(|r| r.node_id != chosen_id);
            } else {
                let chosen = weighted_select(&eligible)?;
                record_selection(chosen, &mut used_subnets, &mut used_as, &mut used_countries);
                selected.push(chosen.clone());
                let chosen_id = chosen.node_id;
                candidates.retain(|r| r.node_id != chosen_id);
            }
        }

        debug!("Selected {} relays for circuit", selected.len());

        Ok(selected)
    }
}

impl Default for RelaySelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Record a selected relay's properties for constraint tracking.
fn record_selection(
    relay: &RelayDescriptor,
    used_subnets: &mut HashSet<[u8; 3]>,
    used_as: &mut HashSet<u32>,
    used_countries: &mut HashSet<[u8; 2]>,
) {
    if let Some(subnet) = extract_subnet_24(&relay.ip_addr) {
        used_subnets.insert(subnet);
    }
    used_as.insert(relay.as_number);
    used_countries.insert(relay.country_code);
}

/// Extract the /24 subnet prefix from an IP address string.
///
/// Parses addresses like "1.2.3.4:4433" and returns the first 3 octets.
fn extract_subnet_24(addr_str: &str) -> Option<[u8; 3]> {
    let ip_part = addr_str.split(':').next()?;
    let ip: Ipv4Addr = ip_part.parse().ok()?;
    let octets = ip.octets();
    Some([octets[0], octets[1], octets[2]])
}

/// Select a relay using PoSrv-weighted random sampling.
///
/// Relays with higher PoSrv scores are more likely to be selected.
fn weighted_select<'a>(relays: &[&'a &RelayDescriptor]) -> Result<&'a RelayDescriptor> {
    if relays.is_empty() {
        return Err(OnionError::InsufficientRelays { need: 1, have: 0 });
    }

    // Compute total weight (PoSrv scores, clamped to positive).
    let total_weight: f64 = relays
        .iter()
        .map(|r| f64::from(r.posrv_score).max(0.001))
        .sum();

    if total_weight <= 0.0 {
        let idx = rand::Rng::gen_range(&mut rand::thread_rng(), 0..relays.len());
        return Ok(relays[idx]);
    }

    let mut rng = rand::thread_rng();
    let threshold: f64 = rand::Rng::gen_range(&mut rng, 0.0..total_weight);

    let mut cumulative = 0.0;
    for relay in relays {
        cumulative += f64::from(relay.posrv_score).max(0.001);
        if cumulative >= threshold {
            return Ok(relay);
        }
    }

    // Fallback to last relay (floating point edge case).
    Ok(relays[relays.len() - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_relay(id: u8, ip: &str, as_num: u32, country: [u8; 2], score: f32) -> RelayDescriptor {
        RelayDescriptor {
            node_id: [id; 32],
            pik_hash: [id; 32],
            x25519_pk: [id; 32],
            mlkem768_ek: vec![0u8; 1184],
            relay_epoch: 1,
            posrv_score: score,
            ip_addr: ip.to_string(),
            as_number: as_num,
            country_code: country,
            bandwidth_cap_mbps: 100,
            uptime_epochs: 100,
            sig: [0u8; 64],
        }
    }

    #[test]
    fn test_extract_subnet_24() {
        assert_eq!(extract_subnet_24("192.168.1.100:4433"), Some([192, 168, 1]));
        assert_eq!(extract_subnet_24("10.0.0.1:4433"), Some([10, 0, 0]));
        assert_eq!(extract_subnet_24("invalid"), None);
    }

    #[test]
    fn test_relay_cache_operations() {
        let mut cache = RelayCache::new();
        assert!(cache.is_empty());

        let r1 = make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0);
        cache.add(r1);
        assert_eq!(cache.len(), 1);

        let r2 = make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 2.0);
        cache.add(r2);
        assert_eq!(cache.len(), 2);

        cache.remove(&[1u8; 32]);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_relay_cache_update_existing() {
        let mut cache = RelayCache::new();
        let r1 = make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0);
        cache.add(r1);

        let r1_updated = make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 5.0);
        cache.add(r1_updated);
        assert_eq!(cache.len(), 1);
        assert!((cache.all()[0].posrv_score - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_select_relays_success() {
        let cache = RelayCache::from_descriptors(vec![
            make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0),
            make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 2.0),
            make_relay(3, "10.0.3.1:4433", 300, [b'J', b'P'], 3.0),
        ]);

        let selector = RelaySelector::new();
        let selected = selector.select_relays(&cache).expect("select relays");
        assert_eq!(selected.len(), CIRCUIT_HOPS);

        let ids: HashSet<[u8; 32]> = selected.iter().map(|r| r.node_id).collect();
        assert_eq!(ids.len(), CIRCUIT_HOPS);
    }

    #[test]
    fn test_select_relays_insufficient() {
        let cache = RelayCache::from_descriptors(vec![
            make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0),
            make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 2.0),
        ]);

        let selector = RelaySelector::new();
        let result = selector.select_relays(&cache);
        assert!(result.is_err());
    }

    #[test]
    fn test_select_relays_subnet_constraint() {
        let cache = RelayCache::from_descriptors(vec![
            make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0),
            make_relay(2, "10.0.1.2:4433", 200, [b'D', b'E'], 2.0),
            make_relay(3, "10.0.2.1:4433", 300, [b'J', b'P'], 3.0),
            make_relay(4, "10.0.3.1:4433", 400, [b'G', b'B'], 4.0),
        ]);

        let selector = RelaySelector::new();
        let selected = selector.select_relays(&cache).expect("select relays");

        let mut subnets = HashSet::new();
        for relay in &selected {
            let subnet = extract_subnet_24(&relay.ip_addr);
            if let Some(s) = subnet {
                assert!(subnets.insert(s), "Duplicate /24 subnet in selected relays");
            }
        }
    }

    #[test]
    fn test_select_relays_as_exclusion() {
        let mut constraints = SelectionConstraints::default();
        constraints.excluded_as_numbers.insert(100);

        let cache = RelayCache::from_descriptors(vec![
            make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0),
            make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 2.0),
            make_relay(3, "10.0.3.1:4433", 300, [b'J', b'P'], 3.0),
            make_relay(4, "10.0.4.1:4433", 400, [b'G', b'B'], 4.0),
        ]);

        let selector = RelaySelector::with_constraints(constraints);
        let selected = selector.select_relays(&cache).expect("select relays");

        for relay in &selected {
            assert_ne!(relay.as_number, 100);
        }
    }

    #[test]
    fn test_filter_by_min_score() {
        let cache = RelayCache::from_descriptors(vec![
            make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 0.5),
            make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 1.5),
            make_relay(3, "10.0.3.1:4433", 300, [b'J', b'P'], 2.5),
        ]);

        let filtered = cache.filter_by_min_score(1.0);
        assert_eq!(filtered.len(), 2);
    }
}
