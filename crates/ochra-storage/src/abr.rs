//! ABR (Always-Be-Relaying) lifecycle and LFU-DA eviction.
//!
//! The ABR store manages encrypted chunk storage for the Ochra P2P network.
//! Chunks are opaque encrypted blobs; the ABR node does not know their contents.
//!
//! ## Eviction Policy: LFU-DA (Least Frequently Used with Dynamic Aging)
//!
//! When storage reaches capacity, the chunk with the lowest eviction score
//! is removed. The score is computed as:
//!
//! ```text
//! score = access_count / (current_time - stored_at + 1)
//! ```
//!
//! This favors recently stored and frequently accessed chunks while naturally
//! aging out stale entries.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{Result, StorageError};

/// Metadata for a stored ABR chunk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkMeta {
    /// BLAKE3 hash identifying the chunk.
    pub chunk_id: [u8; 32],
    /// Shard index within the erasure-coded set (0..7).
    pub shard_index: u8,
    /// Unix timestamp when the chunk was stored.
    pub stored_at: u64,
    /// Number of times this chunk has been accessed/served.
    pub access_count: u64,
    /// Dynamic age parameter for LFU-DA scoring.
    pub dynamic_age: u64,
    /// Size of the stored data in bytes.
    pub data_size: u64,
}

/// Entry in the ABR store containing metadata and data.
#[derive(Clone, Debug)]
struct StoreEntry {
    /// Chunk metadata.
    meta: ChunkMeta,
    /// The opaque encrypted chunk data.
    data: Vec<u8>,
}

/// ABR store managing chunk storage with LFU-DA eviction.
///
/// Chunks are stored in-memory in this implementation. A production
/// version would persist to disk via `ochra-db`.
pub struct AbrStore {
    /// Stored chunks keyed by chunk_id.
    entries: HashMap<[u8; 32], StoreEntry>,
    /// Maximum storage capacity in bytes.
    capacity_bytes: u64,
    /// Current total bytes stored.
    used_bytes: u64,
}

impl AbrStore {
    /// Create a new ABR store with the given capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity_bytes` - Maximum number of bytes the store can hold.
    pub fn new(capacity_bytes: u64) -> Self {
        Self {
            entries: HashMap::new(),
            capacity_bytes,
            used_bytes: 0,
        }
    }

    /// Store an encrypted chunk.
    ///
    /// If storing the chunk would exceed capacity, evicts the least valuable
    /// chunks using the LFU-DA policy until there is enough space.
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - The 32-byte chunk identifier.
    /// * `shard_index` - The shard index (0..7).
    /// * `data` - The opaque encrypted chunk data.
    /// * `current_time` - The current Unix timestamp.
    pub fn store_chunk(
        &mut self,
        chunk_id: [u8; 32],
        shard_index: u8,
        data: Vec<u8>,
        current_time: u64,
    ) -> Result<()> {
        let data_size = data.len() as u64;

        // If updating an existing chunk, remove the old entry first.
        if let Some(old) = self.entries.remove(&chunk_id) {
            self.used_bytes = self.used_bytes.saturating_sub(old.meta.data_size);
        }

        // Evict chunks until we have enough space.
        while self.used_bytes + data_size > self.capacity_bytes {
            if self.entries.is_empty() {
                return Err(StorageError::AllocationExceeded {
                    used: self.used_bytes,
                    limit: self.capacity_bytes,
                });
            }
            self.evict_lfu(current_time)?;
        }

        let meta = ChunkMeta {
            chunk_id,
            shard_index,
            stored_at: current_time,
            access_count: 0,
            dynamic_age: current_time,
            data_size,
        };

        self.entries.insert(chunk_id, StoreEntry { meta, data });
        self.used_bytes += data_size;

        tracing::debug!(
            chunk_id = hex::encode(chunk_id),
            shard_index,
            data_size,
            used = self.used_bytes,
            capacity = self.capacity_bytes,
            "stored ABR chunk"
        );

        Ok(())
    }

    /// Retrieve a chunk's data and update its access count.
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - The 32-byte chunk identifier.
    /// * `current_time` - The current Unix timestamp (used to update dynamic age).
    pub fn get_chunk(&mut self, chunk_id: &[u8; 32], current_time: u64) -> Result<&[u8]> {
        let entry = self.entries.get_mut(chunk_id).ok_or_else(|| {
            StorageError::ChunkNotFound(hex::encode(chunk_id))
        })?;

        entry.meta.access_count += 1;
        entry.meta.dynamic_age = current_time;

        Ok(&entry.data)
    }

    /// Get the metadata for a stored chunk without updating access counts.
    pub fn get_meta(&self, chunk_id: &[u8; 32]) -> Result<&ChunkMeta> {
        self.entries
            .get(chunk_id)
            .map(|e| &e.meta)
            .ok_or_else(|| StorageError::ChunkNotFound(hex::encode(chunk_id)))
    }

    /// Evict the chunk with the lowest LFU-DA score.
    ///
    /// The LFU-DA score is:
    /// ```text
    /// score = access_count / (current_time - stored_at + 1)
    /// ```
    ///
    /// # Arguments
    ///
    /// * `current_time` - The current Unix timestamp.
    pub fn evict_lfu(&mut self, current_time: u64) -> Result<[u8; 32]> {
        let victim_id = self
            .entries
            .iter()
            .min_by(|a, b| {
                let score_a = lfu_da_score(&a.1.meta, current_time);
                let score_b = lfu_da_score(&b.1.meta, current_time);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)
            .ok_or_else(|| {
                StorageError::ChunkNotFound("store is empty, nothing to evict".to_string())
            })?;

        if let Some(entry) = self.entries.remove(&victim_id) {
            self.used_bytes = self.used_bytes.saturating_sub(entry.meta.data_size);

            tracing::debug!(
                chunk_id = hex::encode(victim_id),
                access_count = entry.meta.access_count,
                data_size = entry.meta.data_size,
                "evicted ABR chunk via LFU-DA"
            );
        }

        Ok(victim_id)
    }

    /// Check if a chunk is stored.
    pub fn contains(&self, chunk_id: &[u8; 32]) -> bool {
        self.entries.contains_key(chunk_id)
    }

    /// Get the number of stored chunks.
    pub fn chunk_count(&self) -> usize {
        self.entries.len()
    }

    /// Get the total bytes currently used.
    pub fn used_bytes(&self) -> u64 {
        self.used_bytes
    }

    /// Get the store capacity in bytes.
    pub fn capacity_bytes(&self) -> u64 {
        self.capacity_bytes
    }

    /// List all stored chunk IDs.
    pub fn chunk_ids(&self) -> Vec<[u8; 32]> {
        self.entries.keys().copied().collect()
    }
}

/// Compute the LFU-DA eviction score for a chunk.
///
/// Higher scores indicate more valuable chunks (accessed more frequently
/// relative to how long they have been stored).
///
/// Formula: `access_count / (current_time - stored_at + 1)`
fn lfu_da_score(meta: &ChunkMeta, current_time: u64) -> f64 {
    let age = current_time.saturating_sub(meta.stored_at) + 1;
    meta.access_count as f64 / age as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let mut store = AbrStore::new(1024 * 1024);
        let chunk_id = [0xAAu8; 32];
        let data = vec![0xBBu8; 4096];

        store
            .store_chunk(chunk_id, 0, data.clone(), 1000)
            .expect("store");
        assert!(store.contains(&chunk_id));
        assert_eq!(store.chunk_count(), 1);

        let retrieved = store.get_chunk(&chunk_id, 1001).expect("get");
        assert_eq!(retrieved, data.as_slice());
    }

    #[test]
    fn test_get_updates_access_count() {
        let mut store = AbrStore::new(1024 * 1024);
        let chunk_id = [0x01u8; 32];
        store
            .store_chunk(chunk_id, 0, vec![0u8; 100], 1000)
            .expect("store");

        let _ = store.get_chunk(&chunk_id, 1001).expect("get");
        let _ = store.get_chunk(&chunk_id, 1002).expect("get");
        let _ = store.get_chunk(&chunk_id, 1003).expect("get");

        let meta = store.get_meta(&chunk_id).expect("meta");
        assert_eq!(meta.access_count, 3);
    }

    #[test]
    fn test_chunk_not_found() {
        let mut store = AbrStore::new(1024);
        let result = store.get_chunk(&[0xFFu8; 32], 1000);
        assert!(result.is_err());
    }

    #[test]
    fn test_eviction_removes_least_accessed() {
        let mut store = AbrStore::new(300);

        // Store three 100-byte chunks.
        let id_a = [0x01u8; 32];
        let id_b = [0x02u8; 32];
        let id_c = [0x03u8; 32];

        store
            .store_chunk(id_a, 0, vec![0u8; 100], 1000)
            .expect("store a");
        store
            .store_chunk(id_b, 1, vec![0u8; 100], 1000)
            .expect("store b");
        store
            .store_chunk(id_c, 2, vec![0u8; 100], 1000)
            .expect("store c");

        // Access A and C but not B.
        let _ = store.get_chunk(&id_a, 1001).expect("get a");
        let _ = store.get_chunk(&id_a, 1002).expect("get a");
        let _ = store.get_chunk(&id_c, 1001).expect("get c");

        // Evict should remove B (0 accesses = lowest score).
        let evicted = store.evict_lfu(1003).expect("evict");
        assert_eq!(evicted, id_b);
        assert!(!store.contains(&id_b));
        assert!(store.contains(&id_a));
        assert!(store.contains(&id_c));
    }

    #[test]
    fn test_auto_eviction_on_store() {
        let mut store = AbrStore::new(200);

        let id_a = [0x01u8; 32];
        let id_b = [0x02u8; 32];
        let id_c = [0x03u8; 32];

        store
            .store_chunk(id_a, 0, vec![0u8; 100], 1000)
            .expect("store a");
        store
            .store_chunk(id_b, 1, vec![0u8; 100], 1000)
            .expect("store b");

        // Store a third chunk should trigger eviction.
        store
            .store_chunk(id_c, 2, vec![0u8; 100], 1001)
            .expect("store c should evict");

        assert_eq!(store.chunk_count(), 2);
        assert!(store.contains(&id_c));
    }

    #[test]
    fn test_lfu_da_score_formula() {
        let meta = ChunkMeta {
            chunk_id: [0u8; 32],
            shard_index: 0,
            stored_at: 100,
            access_count: 10,
            dynamic_age: 100,
            data_size: 100,
        };

        // score = 10 / (200 - 100 + 1) = 10 / 101
        let score = lfu_da_score(&meta, 200);
        let expected = 10.0 / 101.0;
        assert!((score - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_lfu_da_newer_chunks_favored() {
        let old = ChunkMeta {
            chunk_id: [0u8; 32],
            shard_index: 0,
            stored_at: 100,
            access_count: 5,
            dynamic_age: 100,
            data_size: 100,
        };
        let new = ChunkMeta {
            chunk_id: [1u8; 32],
            shard_index: 0,
            stored_at: 900,
            access_count: 5,
            dynamic_age: 900,
            data_size: 100,
        };

        let now = 1000;
        let score_old = lfu_da_score(&old, now);
        let score_new = lfu_da_score(&new, now);

        // New chunk should have higher score (5/101 > 5/901).
        assert!(score_new > score_old);
    }

    #[test]
    fn test_used_bytes_tracking() {
        let mut store = AbrStore::new(1024 * 1024);
        assert_eq!(store.used_bytes(), 0);

        store
            .store_chunk([0x01u8; 32], 0, vec![0u8; 500], 1000)
            .expect("store");
        assert_eq!(store.used_bytes(), 500);

        store
            .store_chunk([0x02u8; 32], 1, vec![0u8; 300], 1000)
            .expect("store");
        assert_eq!(store.used_bytes(), 800);

        store.evict_lfu(1001).expect("evict");
        assert!(store.used_bytes() < 800);
    }

    #[test]
    fn test_store_replaces_existing() {
        let mut store = AbrStore::new(1024 * 1024);
        let chunk_id = [0xAAu8; 32];

        store
            .store_chunk(chunk_id, 0, vec![0u8; 100], 1000)
            .expect("store");
        assert_eq!(store.used_bytes(), 100);

        // Store again with different data.
        store
            .store_chunk(chunk_id, 0, vec![1u8; 200], 1001)
            .expect("store");
        assert_eq!(store.used_bytes(), 200);
        assert_eq!(store.chunk_count(), 1);
    }
}
