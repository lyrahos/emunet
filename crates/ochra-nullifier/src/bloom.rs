//! Bloom filter nullifier set for double-spend detection.
//!
//! ## Parameters (Section 10.4)
//!
//! - 20 hash functions (k = 20)
//! - ~3.4 MB filter size for 1M nullifiers
//! - [`BLOOM_SIZE`] = 3,400,000 bytes (27,200,000 bits)
//! - [`NUM_HASH_FNS`] = 20
//! - Hash function: `BLAKE3::derive_key("Ochra v1 nullifier-bloom-hash-{i}", nullifier)` for `i` in `0..20`

use crate::{NullifierError, Result};

/// Bloom filter size in bytes (~3.4 MB).
pub const BLOOM_SIZE: usize = 3_400_000;

/// Bloom filter size in bits (27,200,000).
pub const BLOOM_SIZE_BITS: usize = BLOOM_SIZE * 8;

/// Number of hash functions.
pub const NUM_HASH_FNS: usize = 20;

/// Context string prefix for nullifier bloom hash functions.
const BLOOM_HASH_CONTEXT_PREFIX: &str = "Ochra v1 nullifier-bloom-hash-";

/// A Bloom-filter-based nullifier set for double-spend detection.
///
/// Each nullifier is hashed through [`NUM_HASH_FNS`] independent hash functions
/// (domain-separated BLAKE3) to produce bit positions. A nullifier is "present"
/// if all bits are set. False positives are possible but false negatives are not.
pub struct NullifierSet {
    /// The bit array backing the Bloom filter.
    bit_array: Vec<u8>,
    /// Number of nullifiers inserted.
    count: usize,
}

impl NullifierSet {
    /// Create a new empty nullifier set with default parameters.
    pub fn new() -> Self {
        Self {
            bit_array: vec![0u8; BLOOM_SIZE],
            count: 0,
        }
    }

    /// Insert a nullifier into the set.
    ///
    /// Sets all `k` bit positions for this nullifier.
    pub fn insert(&mut self, nullifier: &[u8; 32]) {
        let positions = Self::hash_positions(nullifier);
        for pos in positions {
            let byte_idx = pos / 8;
            let bit_idx = pos % 8;
            self.bit_array[byte_idx] |= 1 << bit_idx;
        }
        self.count += 1;
    }

    /// Check whether a nullifier is possibly present in the set.
    ///
    /// Returns `true` if all `k` hash positions are set (may be a false positive).
    /// Returns `false` if at least one position is unset (definitely not present).
    pub fn contains(&self, nullifier: &[u8; 32]) -> bool {
        let positions = Self::hash_positions(nullifier);
        positions.iter().all(|&pos| {
            let byte_idx = pos / 8;
            let bit_idx = pos % 8;
            (self.bit_array[byte_idx] >> bit_idx) & 1 == 1
        })
    }

    /// Insert a nullifier, checking for double-spend first.
    ///
    /// # Errors
    ///
    /// - [`NullifierError::DoubleSpend`] if the nullifier is already present
    pub fn insert_checked(&mut self, nullifier: &[u8; 32]) -> Result<()> {
        if self.contains(nullifier) {
            return Err(NullifierError::DoubleSpend);
        }
        self.insert(nullifier);
        Ok(())
    }

    /// Return the number of nullifiers inserted.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Compute the BLAKE3 hash of the entire Bloom filter state.
    ///
    /// Used for epoch state snapshots.
    pub fn state_hash(&self) -> [u8; 32] {
        ochra_crypto::blake3::hash(&self.bit_array)
    }

    /// Clear all entries, resetting the filter.
    pub fn clear(&mut self) {
        self.bit_array.fill(0);
        self.count = 0;
    }

    /// Get the raw bit array (for serialization/gossip).
    pub fn as_bytes(&self) -> &[u8] {
        &self.bit_array
    }

    /// Load from a raw byte slice.
    pub fn from_bytes(data: &[u8], count: usize) -> Self {
        Self {
            bit_array: data.to_vec(),
            count,
        }
    }

    /// Return the estimated false positive rate at the current load.
    pub fn false_positive_rate(&self) -> f64 {
        let k = NUM_HASH_FNS as f64;
        let m = BLOOM_SIZE_BITS as f64;
        let n = self.count as f64;
        (1.0 - (-k * n / m).exp()).powf(k)
    }

    /// Compute `k` bit positions for a nullifier using domain-separated BLAKE3.
    ///
    /// Each hash function uses: `BLAKE3::derive_key("Ochra v1 nullifier-bloom-hash-{i}", nullifier)`
    fn hash_positions(nullifier: &[u8; 32]) -> [usize; NUM_HASH_FNS] {
        let mut positions = [0usize; NUM_HASH_FNS];
        for (i, pos) in positions.iter_mut().enumerate() {
            // Build per-function context string
            // These are protocol-internal (not from the registered list)
            let context = format!("{BLOOM_HASH_CONTEXT_PREFIX}{i}");
            let hash = ochra_crypto::blake3::derive_key(&context, nullifier);
            // Use first 8 bytes as u64, modulo bit count
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&hash[..8]);
            let val = u64::from_le_bytes(buf);
            *pos = (val as usize) % BLOOM_SIZE_BITS;
        }
        positions
    }
}

impl Default for NullifierSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_contains() {
        let mut set = NullifierSet::new();
        let nullifier = [0x42u8; 32];

        assert!(!set.contains(&nullifier));
        set.insert(&nullifier);
        assert!(set.contains(&nullifier));
        assert_eq!(set.count(), 1);
    }

    #[test]
    fn test_insert_checked_double_spend() {
        let mut set = NullifierSet::new();
        let nullifier = [0x42u8; 32];

        set.insert_checked(&nullifier).expect("first insert");
        let result = set.insert_checked(&nullifier);
        assert!(result.is_err());
    }

    #[test]
    fn test_different_nullifiers() {
        let mut set = NullifierSet::new();
        let n1 = [0x01u8; 32];
        let n2 = [0x02u8; 32];

        set.insert(&n1);
        assert!(set.contains(&n1));
        assert!(!set.contains(&n2));
    }

    #[test]
    fn test_clear() {
        let mut set = NullifierSet::new();
        let nullifier = [0x42u8; 32];
        set.insert(&nullifier);
        assert!(set.contains(&nullifier));

        set.clear();
        assert!(!set.contains(&nullifier));
        assert_eq!(set.count(), 0);
    }

    #[test]
    fn test_state_hash_changes_on_insert() {
        let mut set = NullifierSet::new();
        let hash_before = set.state_hash();
        set.insert(&[0x42u8; 32]);
        let hash_after = set.state_hash();
        assert_ne!(hash_before, hash_after);
    }

    #[test]
    fn test_false_positive_rate_zero_when_empty() {
        let set = NullifierSet::new();
        assert_eq!(set.false_positive_rate(), 0.0);
    }

    #[test]
    fn test_from_bytes_roundtrip() {
        let mut set = NullifierSet::new();
        set.insert(&[0x42u8; 32]);
        let bytes = set.as_bytes().to_vec();
        let count = set.count();

        let restored = NullifierSet::from_bytes(&bytes, count);
        assert!(restored.contains(&[0x42u8; 32]));
        assert_eq!(restored.count(), count);
    }

    #[test]
    fn test_bloom_size_constants() {
        assert_eq!(BLOOM_SIZE, 3_400_000);
        assert_eq!(BLOOM_SIZE_BITS, 27_200_000);
        assert_eq!(NUM_HASH_FNS, 20);
    }
}
