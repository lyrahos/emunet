//! Kademlia routing table with XOR-distance metric.
//!
//! Implements a standard Kademlia routing table with 256 k-buckets, each holding
//! up to K=20 node entries. The XOR distance metric determines bucket placement
//! and nearest-neighbor lookups.
//!
//! ## LRU Eviction
//!
//! When a bucket is full and a new node is discovered, the least-recently-seen
//! entry is pinged. If the ping fails, the stale entry is evicted and the new
//! node is inserted. If the ping succeeds, the new node is discarded (Kademlia
//! preference for long-lived nodes).

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::{DhtError, Result, ALPHA, K, NUM_BUCKETS};

/// A 256-bit node identifier derived from `BLAKE3::hash(pik_public_key)`.
pub type NodeId = [u8; 32];

/// Information about a node in the routing table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    /// The node's 256-bit identifier.
    pub node_id: NodeId,
    /// The node's network address.
    #[serde(with = "socket_addr_serde")]
    pub addr: SocketAddr,
    /// The node's PIK public key (Ed25519 verifying key, 32 bytes).
    pub pik_public_key: [u8; 32],
    /// The node's X25519 public key for encrypted communication.
    pub x25519_public_key: [u8; 32],
}

/// Runtime metadata for a node entry within a k-bucket.
#[derive(Clone, Debug)]
struct BucketEntry {
    /// The node information.
    info: NodeInfo,
    /// When this node was last seen (for LRU eviction).
    last_seen: Instant,
    /// Number of consecutive failed pings.
    failed_pings: u32,
}

/// A single k-bucket holding up to K entries, ordered by last-seen time.
///
/// The front of the deque holds the least-recently-seen entry;
/// the back holds the most-recently-seen entry.
#[derive(Clone, Debug)]
struct KBucket {
    /// Entries ordered by last-seen time (front = oldest, back = newest).
    entries: VecDeque<BucketEntry>,
    /// Last time this bucket was refreshed via a lookup.
    last_refresh: Instant,
}

impl KBucket {
    /// Create an empty k-bucket.
    fn new() -> Self {
        Self {
            entries: VecDeque::with_capacity(K),
            last_refresh: Instant::now(),
        }
    }

    /// Return whether this bucket is full.
    fn is_full(&self) -> bool {
        self.entries.len() >= K
    }

    /// Find an entry by node ID, returning its index if present.
    fn find_index(&self, node_id: &NodeId) -> Option<usize> {
        self.entries.iter().position(|e| e.info.node_id == *node_id)
    }

    /// Move an existing entry to the back (most-recently-seen) and update its timestamp.
    fn touch(&mut self, index: usize) {
        if let Some(mut entry) = self.entries.remove(index) {
            entry.last_seen = Instant::now();
            entry.failed_pings = 0;
            self.entries.push_back(entry);
        }
    }

    /// Insert a new entry at the back (most-recently-seen position).
    ///
    /// Caller must ensure the bucket is not full.
    fn insert(&mut self, info: NodeInfo) {
        self.entries.push_back(BucketEntry {
            info,
            last_seen: Instant::now(),
            failed_pings: 0,
        });
    }

    /// Remove an entry by index.
    fn remove(&mut self, index: usize) -> Option<NodeInfo> {
        self.entries.remove(index).map(|e| e.info)
    }

    /// Get a reference to the least-recently-seen entry (front of deque).
    fn least_recently_seen(&self) -> Option<&BucketEntry> {
        self.entries.front()
    }

    /// Mark the least-recently-seen entry as having a failed ping.
    fn mark_lrs_failed(&mut self) {
        if let Some(entry) = self.entries.front_mut() {
            entry.failed_pings += 1;
        }
    }

    /// Get all entries as node info references.
    fn all_nodes(&self) -> Vec<&NodeInfo> {
        self.entries.iter().map(|e| &e.info).collect()
    }
}

/// The Kademlia routing table.
///
/// Maintains 256 k-buckets indexed by the XOR distance prefix length between
/// the local node and remote nodes.
pub struct RoutingTable {
    /// The local node's identifier.
    local_id: NodeId,
    /// The 256 k-buckets.
    buckets: Vec<KBucket>,
}

impl RoutingTable {
    /// Create a new routing table for the given local node ID.
    pub fn new(local_id: NodeId) -> Self {
        let mut buckets = Vec::with_capacity(NUM_BUCKETS);
        for _ in 0..NUM_BUCKETS {
            buckets.push(KBucket::new());
        }
        Self { local_id, buckets }
    }

    /// Return the local node's ID.
    pub fn local_id(&self) -> &NodeId {
        &self.local_id
    }

    /// Compute the XOR distance between two node IDs.
    pub fn xor_distance(a: &NodeId, b: &NodeId) -> NodeId {
        let mut result = [0u8; 32];
        for i in 0..32 {
            result[i] = a[i] ^ b[i];
        }
        result
    }

    /// Determine the bucket index for a given node ID based on XOR distance
    /// from the local node.
    ///
    /// The bucket index is the number of leading zero bits in the XOR distance.
    /// Nodes closer to the local node go into higher-numbered buckets.
    /// Returns `None` if the node ID is identical to the local ID.
    pub fn bucket_index(&self, node_id: &NodeId) -> Option<usize> {
        let distance = Self::xor_distance(&self.local_id, node_id);
        leading_zeros(&distance)
    }

    /// Add a node to the routing table.
    ///
    /// Behavior follows Kademlia rules:
    /// - If the node is already in the table, move it to the most-recently-seen position.
    /// - If the appropriate bucket has room, insert the node.
    /// - If the bucket is full, return [`AddNodeResult::BucketFull`] with the
    ///   least-recently-seen entry so the caller can ping it and decide whether
    ///   to evict.
    pub fn add_node(&mut self, info: NodeInfo) -> AddNodeResult {
        if info.node_id == self.local_id {
            return AddNodeResult::Ignored;
        }

        let bucket_idx = match self.bucket_index(&info.node_id) {
            Some(idx) => idx,
            None => return AddNodeResult::Ignored,
        };

        let bucket = &mut self.buckets[bucket_idx];

        // If already present, move to back (most-recently-seen).
        if let Some(idx) = bucket.find_index(&info.node_id) {
            bucket.touch(idx);
            return AddNodeResult::Updated;
        }

        // If bucket has room, insert.
        if !bucket.is_full() {
            bucket.insert(info);
            return AddNodeResult::Inserted;
        }

        // Bucket is full: return the LRS entry for the caller to ping.
        match bucket.least_recently_seen() {
            Some(lrs) => AddNodeResult::BucketFull {
                least_recently_seen: lrs.info.clone(),
            },
            None => AddNodeResult::Ignored,
        }
    }

    /// Evict the least-recently-seen node from the bucket containing `stale_id`
    /// and insert `new_node` in its place.
    ///
    /// Call this after a failed ping to the LRS node returned by [`add_node`].
    pub fn evict_and_insert(&mut self, stale_id: &NodeId, new_node: NodeInfo) -> Result<()> {
        let bucket_idx = self.bucket_index(stale_id).ok_or(DhtError::BucketFull)?;

        let bucket = &mut self.buckets[bucket_idx];

        if let Some(idx) = bucket.find_index(stale_id) {
            bucket.remove(idx);
            bucket.insert(new_node);
            Ok(())
        } else {
            Err(DhtError::BucketFull)
        }
    }

    /// Mark the least-recently-seen entry in the bucket for `node_id` as having
    /// a failed ping.
    pub fn mark_failed_ping(&mut self, node_id: &NodeId) {
        if let Some(idx) = self.bucket_index(node_id) {
            self.buckets[idx].mark_lrs_failed();
        }
    }

    /// Remove a node from the routing table.
    pub fn remove_node(&mut self, node_id: &NodeId) -> Option<NodeInfo> {
        let bucket_idx = self.bucket_index(node_id)?;
        let bucket = &mut self.buckets[bucket_idx];
        let entry_idx = bucket.find_index(node_id)?;
        bucket.remove(entry_idx)
    }

    /// Find the `count` closest nodes to the given target ID.
    ///
    /// Searches all buckets and returns nodes sorted by ascending XOR distance
    /// to the target.
    pub fn find_closest(&self, target: &NodeId, count: usize) -> Vec<NodeInfo> {
        let mut all_nodes: Vec<(&NodeInfo, NodeId)> = Vec::new();

        for bucket in &self.buckets {
            for node_info in bucket.all_nodes() {
                let distance = Self::xor_distance(&node_info.node_id, target);
                all_nodes.push((node_info, distance));
            }
        }

        // Sort by XOR distance (lexicographic byte comparison is correct for XOR distances).
        all_nodes.sort_by(|a, b| a.1.cmp(&b.1));

        all_nodes
            .into_iter()
            .take(count)
            .map(|(info, _)| info.clone())
            .collect()
    }

    /// Return the total number of nodes in the routing table.
    pub fn len(&self) -> usize {
        self.buckets.iter().map(|b| b.entries.len()).sum()
    }

    /// Return whether the routing table is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return bucket indices that need refreshing (haven't been touched
    /// within `refresh_interval`).
    pub fn stale_buckets(&self, refresh_interval: std::time::Duration) -> Vec<usize> {
        let now = Instant::now();
        self.buckets
            .iter()
            .enumerate()
            .filter(|(_, b)| {
                !b.entries.is_empty() && now.duration_since(b.last_refresh) > refresh_interval
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Mark a bucket as refreshed.
    pub fn mark_bucket_refreshed(&mut self, bucket_idx: usize) {
        if bucket_idx < NUM_BUCKETS {
            self.buckets[bucket_idx].last_refresh = Instant::now();
        }
    }
}

/// Result of attempting to add a node to the routing table.
#[derive(Clone, Debug)]
pub enum AddNodeResult {
    /// The node was newly inserted into a bucket.
    Inserted,
    /// The node was already present and its position was updated.
    Updated,
    /// The node was ignored (e.g., same as local ID).
    Ignored,
    /// The target bucket is full. Contains the least-recently-seen entry
    /// that should be pinged to check liveness.
    BucketFull {
        /// The least-recently-seen node in the full bucket.
        least_recently_seen: NodeInfo,
    },
}

/// Iterative `FIND_NODE` lookup state machine.
///
/// Performs an iterative Kademlia lookup by querying `ALPHA` nodes in parallel
/// at each step, converging on the `K` closest nodes to the target.
pub struct FindNodeLookup {
    /// The target node ID we are looking for.
    target: NodeId,
    /// Nodes we have already queried (to avoid re-querying).
    queried: Vec<NodeId>,
    /// Candidate nodes sorted by distance, with query status.
    candidates: Vec<LookupCandidate>,
    /// Maximum number of results to return.
    result_count: usize,
}

/// A candidate node in the iterative lookup process.
#[derive(Clone, Debug)]
struct LookupCandidate {
    /// The node information.
    info: NodeInfo,
    /// XOR distance to the lookup target.
    distance: NodeId,
    /// Whether this candidate has been queried.
    queried: bool,
}

impl FindNodeLookup {
    /// Create a new `FIND_NODE` lookup for the given target.
    ///
    /// `seed_nodes` are the initial closest nodes from the local routing table.
    pub fn new(target: NodeId, seed_nodes: Vec<NodeInfo>) -> Self {
        let mut candidates: Vec<LookupCandidate> = seed_nodes
            .into_iter()
            .map(|info| {
                let distance = RoutingTable::xor_distance(&info.node_id, &target);
                LookupCandidate {
                    info,
                    distance,
                    queried: false,
                }
            })
            .collect();

        candidates.sort_by(|a, b| a.distance.cmp(&b.distance));

        Self {
            target,
            queried: Vec::new(),
            candidates,
            result_count: K,
        }
    }

    /// Return the next batch of up to `ALPHA` un-queried nodes to send
    /// `FIND_NODE` requests to.
    ///
    /// Returns an empty vec when the lookup is complete.
    pub fn next_queries(&mut self) -> Vec<NodeInfo> {
        let mut batch = Vec::with_capacity(ALPHA);

        for candidate in &mut self.candidates {
            if batch.len() >= ALPHA {
                break;
            }
            if !candidate.queried {
                candidate.queried = true;
                self.queried.push(candidate.info.node_id);
                batch.push(candidate.info.clone());
            }
        }

        batch
    }

    /// Incorporate responses from a queried node.
    ///
    /// `new_nodes` are the nodes returned by the remote peer in response to
    /// a `FIND_NODE` query.
    pub fn add_responses(&mut self, new_nodes: Vec<NodeInfo>) {
        for info in new_nodes {
            // Skip nodes we've already seen as candidates or queried.
            if self.queried.contains(&info.node_id) {
                continue;
            }
            if self
                .candidates
                .iter()
                .any(|c| c.info.node_id == info.node_id)
            {
                continue;
            }

            let distance = RoutingTable::xor_distance(&info.node_id, &self.target);
            self.candidates.push(LookupCandidate {
                info,
                distance,
                queried: false,
            });
        }

        // Re-sort by distance.
        self.candidates.sort_by(|a, b| a.distance.cmp(&b.distance));

        // Trim to reasonable size to avoid unbounded growth.
        self.candidates.truncate(self.result_count * 3);
    }

    /// Check whether the lookup has converged.
    ///
    /// The lookup is complete when all of the `K` closest candidates have been
    /// queried.
    pub fn is_complete(&self) -> bool {
        self.candidates
            .iter()
            .take(self.result_count)
            .all(|c| c.queried)
    }

    /// Return the final results: the `K` closest nodes found.
    pub fn results(&self) -> Vec<NodeInfo> {
        self.candidates
            .iter()
            .take(self.result_count)
            .map(|c| c.info.clone())
            .collect()
    }
}

/// Compute the number of leading zero bits in a 256-bit value.
///
/// Returns `None` if the value is all zeros (meaning the two node IDs are equal).
fn leading_zeros(value: &[u8; 32]) -> Option<usize> {
    for (i, byte) in value.iter().enumerate() {
        if *byte != 0 {
            return Some(i * 8 + byte.leading_zeros() as usize);
        }
    }
    None
}

/// Serde support for `SocketAddr` as a string.
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
    use std::net::SocketAddr;

    fn make_node(id_byte: u8) -> NodeInfo {
        NodeInfo {
            node_id: [id_byte; 32],
            addr: SocketAddr::from(([127, 0, 0, 1], 4433 + u16::from(id_byte))),
            pik_public_key: [id_byte; 32],
            x25519_public_key: [id_byte; 32],
        }
    }

    fn make_node_with_id(id: NodeId) -> NodeInfo {
        NodeInfo {
            node_id: id,
            addr: SocketAddr::from(([127, 0, 0, 1], 4433)),
            pik_public_key: [0u8; 32],
            x25519_public_key: [0u8; 32],
        }
    }

    #[test]
    fn test_xor_distance() {
        let a = [0x00u8; 32];
        let b = [0xFFu8; 32];
        let d = RoutingTable::xor_distance(&a, &b);
        assert_eq!(d, [0xFFu8; 32]);

        let d_self = RoutingTable::xor_distance(&a, &a);
        assert_eq!(d_self, [0x00u8; 32]);
    }

    #[test]
    fn test_leading_zeros() {
        let mut val = [0u8; 32];
        assert_eq!(leading_zeros(&val), None);

        val[0] = 0x80; // 0 leading zeros
        assert_eq!(leading_zeros(&val), Some(0));

        val[0] = 0x01; // 7 leading zeros
        assert_eq!(leading_zeros(&val), Some(7));

        let mut val2 = [0u8; 32];
        val2[1] = 0x01; // 8 + 7 = 15 leading zeros
        assert_eq!(leading_zeros(&val2), Some(15));
    }

    #[test]
    fn test_bucket_index() {
        let local_id = [0x00u8; 32];
        let table = RoutingTable::new(local_id);

        // Same ID -> None
        assert_eq!(table.bucket_index(&local_id), None);

        // ID with first bit different
        let mut far = [0x00u8; 32];
        far[0] = 0x80;
        assert_eq!(table.bucket_index(&far), Some(0));

        // ID with only last bit different
        let mut close = [0x00u8; 32];
        close[31] = 0x01;
        assert_eq!(table.bucket_index(&close), Some(255));
    }

    #[test]
    fn test_add_and_find_node() {
        let local_id = [0x00u8; 32];
        let mut table = RoutingTable::new(local_id);

        let node = make_node(0x01);
        let result = table.add_node(node.clone());
        assert!(matches!(result, AddNodeResult::Inserted));
        assert_eq!(table.len(), 1);

        // Adding same node again should update
        let result = table.add_node(node.clone());
        assert!(matches!(result, AddNodeResult::Updated));
        assert_eq!(table.len(), 1);

        // Find closest should return the node
        let closest = table.find_closest(&[0x01u8; 32], 5);
        assert_eq!(closest.len(), 1);
        assert_eq!(closest[0].node_id, node.node_id);
    }

    #[test]
    fn test_add_self_ignored() {
        let local_id = [0x42u8; 32];
        let mut table = RoutingTable::new(local_id);

        let self_node = make_node(0x42);
        let result = table.add_node(self_node);
        assert!(matches!(result, AddNodeResult::Ignored));
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_remove_node() {
        let local_id = [0x00u8; 32];
        let mut table = RoutingTable::new(local_id);

        let node = make_node(0x01);
        table.add_node(node.clone());
        assert_eq!(table.len(), 1);

        let removed = table.remove_node(&node.node_id);
        assert!(removed.is_some());
        assert_eq!(table.len(), 0);

        // Removing non-existent node returns None
        let removed = table.remove_node(&[0xFFu8; 32]);
        assert!(removed.is_none());
    }

    #[test]
    fn test_bucket_full() {
        let local_id = [0x00u8; 32];
        let mut table = RoutingTable::new(local_id);

        // Fill bucket 0 (nodes with first bit set) with K entries
        for i in 0..K {
            let mut id = [0x80u8; 32];
            id[31] = i as u8;
            let node = make_node_with_id(id);
            let result = table.add_node(node);
            assert!(matches!(result, AddNodeResult::Inserted));
        }

        assert_eq!(table.len(), K);

        // The next insert into the same bucket should report BucketFull
        let mut overflow_id = [0x80u8; 32];
        overflow_id[31] = K as u8;
        let overflow_node = make_node_with_id(overflow_id);
        let result = table.add_node(overflow_node);
        assert!(matches!(result, AddNodeResult::BucketFull { .. }));
    }

    #[test]
    fn test_evict_and_insert() {
        let local_id = [0x00u8; 32];
        let mut table = RoutingTable::new(local_id);

        // Fill bucket 0
        let mut first_id = [0x80u8; 32];
        first_id[31] = 0;
        let first_node = make_node_with_id(first_id);
        table.add_node(first_node);

        for i in 1..K {
            let mut id = [0x80u8; 32];
            id[31] = i as u8;
            table.add_node(make_node_with_id(id));
        }

        // Evict the first node and insert a new one
        let mut new_id = [0x80u8; 32];
        new_id[31] = K as u8;
        let new_node = make_node_with_id(new_id);

        let result = table.evict_and_insert(&first_id, new_node);
        assert!(result.is_ok());
        assert_eq!(table.len(), K);
    }

    #[test]
    fn test_find_closest_sorted() {
        let local_id = [0x00u8; 32];
        let mut table = RoutingTable::new(local_id);

        // Add several nodes at different distances
        for i in 1..=10u8 {
            let mut id = [0x00u8; 32];
            id[0] = i;
            table.add_node(make_node_with_id(id));
        }

        let target = [0x05u8; 32];
        let closest = table.find_closest(&target, 5);
        assert_eq!(closest.len(), 5);

        // Verify they are sorted by XOR distance to target
        for i in 0..closest.len() - 1 {
            let d1 = RoutingTable::xor_distance(&closest[i].node_id, &target);
            let d2 = RoutingTable::xor_distance(&closest[i + 1].node_id, &target);
            assert!(d1 <= d2, "Results not sorted by distance");
        }
    }

    #[test]
    fn test_find_node_lookup() {
        let target = [0xFFu8; 32];
        let seed_nodes: Vec<NodeInfo> = (1..=5u8)
            .map(|i| {
                let mut id = [0x00u8; 32];
                id[0] = i;
                make_node_with_id(id)
            })
            .collect();

        let mut lookup = FindNodeLookup::new(target, seed_nodes);
        assert!(!lookup.is_complete());

        // Get first batch of queries
        let batch = lookup.next_queries();
        assert_eq!(batch.len(), ALPHA);

        // Simulate responses with new nodes
        let response_nodes: Vec<NodeInfo> = (10..=12u8)
            .map(|i| {
                let mut id = [0xF0u8; 32];
                id[31] = i;
                make_node_with_id(id)
            })
            .collect();
        lookup.add_responses(response_nodes);

        // Continue querying
        let batch2 = lookup.next_queries();
        assert!(!batch2.is_empty());
    }

    #[test]
    fn test_lookup_convergence() {
        let target = [0x42u8; 32];

        let seed_nodes: Vec<NodeInfo> = (1..=3u8)
            .map(|i| {
                let mut id = [0x00u8; 32];
                id[0] = i;
                make_node_with_id(id)
            })
            .collect();

        let mut lookup = FindNodeLookup::new(target, seed_nodes);

        // Query all candidates without adding new ones -> should converge
        loop {
            let batch = lookup.next_queries();
            if batch.is_empty() {
                break;
            }
            // No new responses -> converges
        }

        assert!(lookup.is_complete());
        let results = lookup.results();
        assert!(results.len() <= K);
    }

    #[test]
    fn test_is_empty() {
        let table = RoutingTable::new([0u8; 32]);
        assert!(table.is_empty());
    }
}
