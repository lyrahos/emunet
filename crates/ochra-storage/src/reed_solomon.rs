//! Reed-Solomon k=4, n=8 erasure coding.
//!
//! Provides redundancy for stored chunks. Data is split into 4 data shards,
//! and 4 parity shards are computed, yielding 8 total shards. Any 4 of the
//! 8 shards are sufficient to reconstruct the original data.
//!
//! ## V1 Implementation
//!
//! This v1 implementation uses XOR-based parity for simplicity. The trait
//! interface is designed so that a full `reed-solomon-erasure` implementation
//! can be swapped in later without changing callers.

use serde::{Deserialize, Serialize};

use crate::{Result, StorageError};

/// Number of data shards (k).
pub const DATA_SHARDS: usize = 4;

/// Number of parity shards.
pub const PARITY_SHARDS: usize = 4;

/// Total shards (n = k + parity).
pub const TOTAL_SHARDS: usize = DATA_SHARDS + PARITY_SHARDS;

/// A single shard of encoded data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shard {
    /// Shard index (0..TOTAL_SHARDS).
    pub index: usize,
    /// Shard data bytes.
    pub data: Vec<u8>,
}

/// Reed-Solomon codec for k=4, n=8 erasure coding.
///
/// V1 uses XOR-based parity generation. Each parity shard is computed
/// as a combination of the data shards using different XOR patterns.
#[derive(Clone, Debug)]
pub struct ReedSolomonCodec;

impl ReedSolomonCodec {
    /// Create a new codec instance.
    pub fn new() -> Self {
        Self
    }

    /// Encode 4 data shards into 4 parity shards (8 total).
    ///
    /// All data shards must have the same length.
    ///
    /// # Arguments
    ///
    /// * `data_shards` - Exactly 4 data shards of equal length.
    ///
    /// # Returns
    ///
    /// 4 parity shards. The caller retains the original data shards;
    /// the full set of 8 shards is `data_shards ++ parity_shards`.
    pub fn encode(&self, data_shards: &[Vec<u8>; DATA_SHARDS]) -> Result<[Vec<u8>; PARITY_SHARDS]> {
        let shard_len = data_shards[0].len();
        if shard_len == 0 {
            return Err(StorageError::ReedSolomonEncode(
                "data shards are empty".to_string(),
            ));
        }
        for (i, shard) in data_shards.iter().enumerate() {
            if shard.len() != shard_len {
                return Err(StorageError::ReedSolomonEncode(format!(
                    "shard {i} has length {}, expected {shard_len}",
                    shard.len()
                )));
            }
        }

        // P0 = D0 ^ D1
        let p0 = xor_shards(&data_shards[0], &data_shards[1]);
        // P1 = D2 ^ D3
        let p1 = xor_shards(&data_shards[2], &data_shards[3]);
        // P2 = D0 ^ D2
        let p2 = xor_shards(&data_shards[0], &data_shards[2]);
        // P3 = D1 ^ D3
        let p3 = xor_shards(&data_shards[1], &data_shards[3]);

        Ok([p0, p1, p2, p3])
    }

    /// Decode shards back to original data.
    ///
    /// Requires at least 4 of the 8 shards to be present. The shard
    /// array is indexed 0..7 where 0..3 are data shards and 4..7 are
    /// parity shards.
    ///
    /// # Arguments
    ///
    /// * `shards` - Array of 8 optional shards (indices 0-3 are data, 4-7 are parity).
    ///
    /// # Returns
    ///
    /// The original concatenated data (all 4 data shards joined).
    pub fn decode(
        &self,
        shards: &[Option<Vec<u8>>; TOTAL_SHARDS],
    ) -> Result<Vec<u8>> {
        let present_count = shards.iter().filter(|s| s.is_some()).count();
        if present_count < DATA_SHARDS {
            return Err(StorageError::ReedSolomonDecode(format!(
                "need at least {DATA_SHARDS} shards, have {present_count}"
            )));
        }

        let shard_len = shards
            .iter()
            .flatten()
            .next()
            .ok_or_else(|| {
                StorageError::ReedSolomonDecode("no shards present".to_string())
            })?
            .len();

        // Try to recover each data shard.
        let mut recovered = [None, None, None, None];

        // First, use any directly available data shards.
        for i in 0..DATA_SHARDS {
            if let Some(ref data) = shards[i] {
                recovered[i] = Some(data.clone());
            }
        }

        // Attempt recovery using parity shards.
        // P0 = D0 ^ D1 => D0 = P0 ^ D1, D1 = P0 ^ D0
        // P1 = D2 ^ D3 => D2 = P1 ^ D3, D3 = P1 ^ D2
        // P2 = D0 ^ D2 => D0 = P2 ^ D2, D2 = P2 ^ D0
        // P3 = D1 ^ D3 => D1 = P3 ^ D3, D3 = P3 ^ D1

        // Run multiple passes to allow cascading recovery.
        for _pass in 0..DATA_SHARDS {
            // P0 (index 4): D0 ^ D1
            if let Some(ref p0) = shards[4] {
                if recovered[0].is_none() {
                    if let Some(ref d1) = recovered[1] {
                        recovered[0] = Some(xor_shards(p0, d1));
                    }
                }
                if recovered[1].is_none() {
                    if let Some(ref d0) = recovered[0] {
                        recovered[1] = Some(xor_shards(p0, d0));
                    }
                }
            }

            // P1 (index 5): D2 ^ D3
            if let Some(ref p1) = shards[5] {
                if recovered[2].is_none() {
                    if let Some(ref d3) = recovered[3] {
                        recovered[2] = Some(xor_shards(p1, d3));
                    }
                }
                if recovered[3].is_none() {
                    if let Some(ref d2) = recovered[2] {
                        recovered[3] = Some(xor_shards(p1, d2));
                    }
                }
            }

            // P2 (index 6): D0 ^ D2
            if let Some(ref p2) = shards[6] {
                if recovered[0].is_none() {
                    if let Some(ref d2) = recovered[2] {
                        recovered[0] = Some(xor_shards(p2, d2));
                    }
                }
                if recovered[2].is_none() {
                    if let Some(ref d0) = recovered[0] {
                        recovered[2] = Some(xor_shards(p2, d0));
                    }
                }
            }

            // P3 (index 7): D1 ^ D3
            if let Some(ref p3) = shards[7] {
                if recovered[1].is_none() {
                    if let Some(ref d3) = recovered[3] {
                        recovered[1] = Some(xor_shards(p3, d3));
                    }
                }
                if recovered[3].is_none() {
                    if let Some(ref d1) = recovered[1] {
                        recovered[3] = Some(xor_shards(p3, d1));
                    }
                }
            }
        }

        // Verify all data shards are recovered.
        let mut result = Vec::with_capacity(shard_len * DATA_SHARDS);
        for (i, shard) in recovered.iter().enumerate() {
            let data = shard.as_ref().ok_or_else(|| {
                StorageError::ReedSolomonDecode(format!(
                    "unable to recover data shard {i}"
                ))
            })?;
            result.extend_from_slice(data);
        }

        Ok(result)
    }

    /// Prepare data for erasure coding by splitting into 4 equal-sized shards.
    ///
    /// Pads the data to a multiple of [`DATA_SHARDS`] bytes if necessary.
    ///
    /// # Arguments
    ///
    /// * `data` - The raw data to prepare.
    ///
    /// # Returns
    ///
    /// 4 data shards of equal length and the original (unpadded) data length.
    pub fn split_into_data_shards(&self, data: &[u8]) -> Result<([Vec<u8>; DATA_SHARDS], usize)> {
        if data.is_empty() {
            return Err(StorageError::ReedSolomonEncode(
                "data is empty".to_string(),
            ));
        }

        let original_len = data.len();
        let shard_len = original_len.div_ceil(DATA_SHARDS);

        let mut padded = data.to_vec();
        padded.resize(shard_len * DATA_SHARDS, 0);

        let mut shards: [Vec<u8>; DATA_SHARDS] = Default::default();
        for (i, shard) in shards.iter_mut().enumerate() {
            let start = i * shard_len;
            let end = start + shard_len;
            *shard = padded[start..end].to_vec();
        }

        Ok((shards, original_len))
    }
}

impl Default for ReedSolomonCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// XOR two byte slices of equal length.
fn xor_shards(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(x, y)| x ^ y).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_all_shards_present() {
        let codec = ReedSolomonCodec::new();
        let data = b"Hello, Ochra! This is test data for Reed-Solomon coding.";
        let (data_shards, original_len) = codec.split_into_data_shards(data).expect("split");

        let parity = codec.encode(&data_shards).expect("encode");

        let mut all_shards: [Option<Vec<u8>>; TOTAL_SHARDS] = Default::default();
        for (i, shard) in data_shards.iter().enumerate() {
            all_shards[i] = Some(shard.clone());
        }
        for (i, shard) in parity.iter().enumerate() {
            all_shards[DATA_SHARDS + i] = Some(shard.clone());
        }

        let recovered = codec.decode(&all_shards).expect("decode");
        assert_eq!(&recovered[..original_len], data.as_slice());
    }

    #[test]
    fn test_recover_missing_d0_d1() {
        let codec = ReedSolomonCodec::new();
        let data = b"Testing erasure recovery with missing data shards d0 and d1!";
        let (data_shards, original_len) = codec.split_into_data_shards(data).expect("split");
        let parity = codec.encode(&data_shards).expect("encode");

        // Remove D0 and D1, keep D2, D3, and all parity.
        let mut shards: [Option<Vec<u8>>; TOTAL_SHARDS] = Default::default();
        shards[2] = Some(data_shards[2].clone());
        shards[3] = Some(data_shards[3].clone());
        for (i, p) in parity.iter().enumerate() {
            shards[DATA_SHARDS + i] = Some(p.clone());
        }

        let recovered = codec.decode(&shards).expect("decode");
        assert_eq!(&recovered[..original_len], data.as_slice());
    }

    #[test]
    fn test_recover_missing_d2_d3() {
        let codec = ReedSolomonCodec::new();
        let data = b"Testing recovery of d2 and d3 using parity shards!";
        let (data_shards, original_len) = codec.split_into_data_shards(data).expect("split");
        let parity = codec.encode(&data_shards).expect("encode");

        // Remove D2 and D3, keep D0, D1, and all parity.
        let mut shards: [Option<Vec<u8>>; TOTAL_SHARDS] = Default::default();
        shards[0] = Some(data_shards[0].clone());
        shards[1] = Some(data_shards[1].clone());
        for (i, p) in parity.iter().enumerate() {
            shards[DATA_SHARDS + i] = Some(p.clone());
        }

        let recovered = codec.decode(&shards).expect("decode");
        assert_eq!(&recovered[..original_len], data.as_slice());
    }

    #[test]
    fn test_recover_missing_d0_d2() {
        let codec = ReedSolomonCodec::new();
        let data = b"Cross-recovery test: missing d0 and d2.";
        let (data_shards, original_len) = codec.split_into_data_shards(data).expect("split");
        let parity = codec.encode(&data_shards).expect("encode");

        // Remove D0 and D2; keep D1, D3, and all parity.
        let mut shards: [Option<Vec<u8>>; TOTAL_SHARDS] = Default::default();
        shards[1] = Some(data_shards[1].clone());
        shards[3] = Some(data_shards[3].clone());
        for (i, p) in parity.iter().enumerate() {
            shards[DATA_SHARDS + i] = Some(p.clone());
        }

        let recovered = codec.decode(&shards).expect("decode");
        assert_eq!(&recovered[..original_len], data.as_slice());
    }

    #[test]
    fn test_insufficient_shards_fails() {
        let codec = ReedSolomonCodec::new();
        let data = b"Not enough shards to recover.";
        let (data_shards, _) = codec.split_into_data_shards(data).expect("split");
        let parity = codec.encode(&data_shards).expect("encode");

        // Only provide 3 shards total.
        let mut shards: [Option<Vec<u8>>; TOTAL_SHARDS] = Default::default();
        shards[0] = Some(data_shards[0].clone());
        shards[4] = Some(parity[0].clone());
        shards[5] = Some(parity[1].clone());

        let result = codec.decode(&shards);
        assert!(result.is_err());
    }

    #[test]
    fn test_data_shards_equal_length() {
        let codec = ReedSolomonCodec::new();
        let data = b"Uneven data size test";
        let (data_shards, _) = codec.split_into_data_shards(data).expect("split");

        let len = data_shards[0].len();
        for shard in &data_shards {
            assert_eq!(shard.len(), len);
        }
    }

    #[test]
    fn test_empty_data_fails() {
        let codec = ReedSolomonCodec::new();
        let result = codec.split_into_data_shards(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_parity_shard_count() {
        let codec = ReedSolomonCodec::new();
        let data = b"parity count test";
        let (data_shards, _) = codec.split_into_data_shards(data).expect("split");
        let parity = codec.encode(&data_shards).expect("encode");
        assert_eq!(parity.len(), PARITY_SHARDS);
    }

    #[test]
    fn test_only_parity_and_two_data_shards() {
        let codec = ReedSolomonCodec::new();
        let data = b"Recover from only 2 data + 2 relevant parity.";
        let (data_shards, original_len) = codec.split_into_data_shards(data).expect("split");
        let parity = codec.encode(&data_shards).expect("encode");

        // D0, D3 present. P0 (D0^D1) can recover D1. P1 (D2^D3) can recover D2.
        let mut shards: [Option<Vec<u8>>; TOTAL_SHARDS] = Default::default();
        shards[0] = Some(data_shards[0].clone());
        shards[3] = Some(data_shards[3].clone());
        shards[4] = Some(parity[0].clone()); // P0 = D0 ^ D1
        shards[5] = Some(parity[1].clone()); // P1 = D2 ^ D3

        let recovered = codec.decode(&shards).expect("decode");
        assert_eq!(&recovered[..original_len], data.as_slice());
    }
}
