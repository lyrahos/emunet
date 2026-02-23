//! Multi-record chunking for large DHT values.
//!
//! When a value exceeds the [`MAX_RECORD_SIZE`](crate::MAX_RECORD_SIZE) limit
//! (1000 bytes), it is split into multiple chunks. Each chunk is stored as a
//! separate DHT record, and a manifest record contains the list of chunk hashes
//! needed for reassembly.
//!
//! ## Chunk Format
//!
//! Each chunk is stored as an immutable DHT record (content-addressed by its hash).
//! The manifest record contains:
//! - `total_chunks`: number of chunks
//! - `total_size`: total size of the original value
//! - `chunk_hashes`: ordered list of 32-byte BLAKE3 hashes of each chunk
//!
//! ## Reassembly
//!
//! To reassemble:
//! 1. Retrieve the manifest record.
//! 2. For each chunk hash, retrieve the corresponding immutable record.
//! 3. Concatenate chunks in order.
//! 4. Verify the reassembled value against the expected total size.

use serde::{Deserialize, Serialize};

use crate::{DhtError, Result, MAX_RECORD_SIZE};

/// Overhead per chunk for framing (index, total, etc.). We keep a safety
/// margin so that chunk data plus any serialization overhead fits within
/// [`MAX_RECORD_SIZE`].
const CHUNK_OVERHEAD: usize = 48;

/// Maximum data payload per chunk.
const CHUNK_DATA_SIZE: usize = MAX_RECORD_SIZE - CHUNK_OVERHEAD;

/// A single chunk of a split record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk {
    /// Zero-based index of this chunk.
    pub index: u32,
    /// Total number of chunks in the split.
    pub total: u32,
    /// The chunk data payload.
    pub data: Vec<u8>,
}

/// A manifest describing the chunks of a split record.
///
/// This manifest itself is stored as a DHT record (must fit within
/// [`MAX_RECORD_SIZE`]).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkManifest {
    /// Total number of chunks.
    pub total_chunks: u32,
    /// Total size of the original value in bytes.
    pub total_size: u64,
    /// Ordered list of BLAKE3 hashes, one per chunk.
    pub chunk_hashes: Vec<[u8; 32]>,
}

/// Split a large value into chunks suitable for DHT storage.
///
/// Returns a list of [`Chunk`]s. Each chunk's data will be at most
/// [`CHUNK_DATA_SIZE`] bytes, ensuring the serialized chunk fits within
/// [`MAX_RECORD_SIZE`].
///
/// If the value fits within a single record, returns a single chunk.
pub fn split_record(value: &[u8]) -> Vec<Chunk> {
    if value.is_empty() {
        return vec![Chunk {
            index: 0,
            total: 1,
            data: Vec::new(),
        }];
    }

    let chunks_needed = value.len().div_ceil(CHUNK_DATA_SIZE);
    let total = chunks_needed as u32;

    let mut chunks = Vec::with_capacity(chunks_needed);
    for (i, chunk_data) in value.chunks(CHUNK_DATA_SIZE).enumerate() {
        chunks.push(Chunk {
            index: i as u32,
            total,
            data: chunk_data.to_vec(),
        });
    }

    chunks
}

/// Build a [`ChunkManifest`] from a list of chunks.
///
/// Computes the BLAKE3 hash of each chunk's data to build the hash list.
pub fn build_manifest(chunks: &[Chunk], total_size: u64) -> ChunkManifest {
    let chunk_hashes: Vec<[u8; 32]> = chunks
        .iter()
        .map(|c| ochra_crypto::blake3::hash(&c.data))
        .collect();

    ChunkManifest {
        total_chunks: chunks.len() as u32,
        total_size,
        chunk_hashes,
    }
}

/// Reassemble a value from its constituent chunks.
///
/// The chunks are sorted by index and concatenated. The manifest is used to
/// verify completeness and integrity.
///
/// # Errors
///
/// Returns [`DhtError::MissingChunk`] if any chunk is missing.
/// Returns [`DhtError::Serialization`] if a chunk's data hash does not match
/// the manifest.
pub fn assemble_record(manifest: &ChunkManifest, chunks: &[Chunk]) -> Result<Vec<u8>> {
    // Sort chunks by index.
    let mut sorted_chunks = chunks.to_vec();
    sorted_chunks.sort_by_key(|c| c.index);

    // Verify we have all chunks.
    if sorted_chunks.len() != manifest.total_chunks as usize {
        // Find the first missing chunk.
        for i in 0..manifest.total_chunks {
            if !sorted_chunks.iter().any(|c| c.index == i) {
                return Err(DhtError::MissingChunk {
                    index: i,
                    total: manifest.total_chunks,
                });
            }
        }
    }

    // Verify each chunk's hash matches the manifest.
    let mut result = Vec::with_capacity(manifest.total_size as usize);
    for (i, chunk) in sorted_chunks.iter().enumerate() {
        if i >= manifest.chunk_hashes.len() {
            return Err(DhtError::MissingChunk {
                index: chunk.index,
                total: manifest.total_chunks,
            });
        }

        let expected_hash = &manifest.chunk_hashes[i];
        let actual_hash = ochra_crypto::blake3::hash(&chunk.data);
        if actual_hash != *expected_hash {
            return Err(DhtError::Serialization(format!(
                "chunk {} hash mismatch: expected {}, got {}",
                i,
                hex::encode(expected_hash),
                hex::encode(actual_hash),
            )));
        }

        result.extend_from_slice(&chunk.data);
    }

    // Verify total size.
    if result.len() as u64 != manifest.total_size {
        return Err(DhtError::Serialization(format!(
            "reassembled size {} does not match expected {}",
            result.len(),
            manifest.total_size,
        )));
    }

    Ok(result)
}

/// Check whether a value needs chunking (exceeds single-record size).
pub fn needs_chunking(value: &[u8]) -> bool {
    value.len() > CHUNK_DATA_SIZE
}

/// Return the maximum data payload size per chunk.
pub fn max_chunk_data_size() -> usize {
    CHUNK_DATA_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_value_single_chunk() {
        let value = b"hello, small value";
        let chunks = split_record(value);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].index, 0);
        assert_eq!(chunks[0].total, 1);
        assert_eq!(chunks[0].data, value);
    }

    #[test]
    fn test_empty_value() {
        let chunks = split_record(b"");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].data.is_empty());
    }

    #[test]
    fn test_exact_chunk_boundary() {
        let value = vec![0xABu8; CHUNK_DATA_SIZE];
        let chunks = split_record(&value);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].data.len(), CHUNK_DATA_SIZE);
    }

    #[test]
    fn test_multiple_chunks() {
        let value = vec![0x42u8; CHUNK_DATA_SIZE * 3 + 100];
        let chunks = split_record(&value);
        assert_eq!(chunks.len(), 4);

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i as u32);
            assert_eq!(chunk.total, 4);
        }

        // First 3 chunks should be full
        assert_eq!(chunks[0].data.len(), CHUNK_DATA_SIZE);
        assert_eq!(chunks[1].data.len(), CHUNK_DATA_SIZE);
        assert_eq!(chunks[2].data.len(), CHUNK_DATA_SIZE);
        // Last chunk has remainder
        assert_eq!(chunks[3].data.len(), 100);
    }

    #[test]
    fn test_split_and_reassemble() {
        let value: Vec<u8> = (0..5000u32).flat_map(|i| i.to_le_bytes()).collect();
        let chunks = split_record(&value);
        let manifest = build_manifest(&chunks, value.len() as u64);

        let reassembled = assemble_record(&manifest, &chunks).expect("assemble");
        assert_eq!(reassembled, value);
    }

    #[test]
    fn test_reassemble_out_of_order() {
        let value = vec![0xABu8; CHUNK_DATA_SIZE * 2 + 50];
        let mut chunks = split_record(&value);
        let manifest = build_manifest(&chunks, value.len() as u64);

        // Reverse chunk order
        chunks.reverse();

        let reassembled = assemble_record(&manifest, &chunks).expect("assemble");
        assert_eq!(reassembled, value);
    }

    #[test]
    fn test_missing_chunk() {
        let value = vec![0xABu8; CHUNK_DATA_SIZE * 3];
        let chunks = split_record(&value);
        let manifest = build_manifest(&chunks, value.len() as u64);

        // Remove the middle chunk
        let partial: Vec<Chunk> = chunks.into_iter().filter(|c| c.index != 1).collect();

        let result = assemble_record(&manifest, &partial);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(DhtError::MissingChunk { index: 1, total: 3 })
        ));
    }

    #[test]
    fn test_tampered_chunk() {
        let value = vec![0xABu8; CHUNK_DATA_SIZE * 2];
        let mut chunks = split_record(&value);
        let manifest = build_manifest(&chunks, value.len() as u64);

        // Tamper with the first chunk's data
        chunks[0].data[0] ^= 0xFF;

        let result = assemble_record(&manifest, &chunks);
        assert!(result.is_err());
    }

    #[test]
    fn test_needs_chunking() {
        assert!(!needs_chunking(&vec![0u8; 100]));
        assert!(!needs_chunking(&vec![0u8; CHUNK_DATA_SIZE]));
        assert!(needs_chunking(&vec![0u8; CHUNK_DATA_SIZE + 1]));
    }

    #[test]
    fn test_manifest_hashes() {
        let value = b"test data for manifest";
        let chunks = split_record(value);
        let manifest = build_manifest(&chunks, value.len() as u64);

        assert_eq!(manifest.total_chunks, 1);
        assert_eq!(manifest.total_size, value.len() as u64);
        assert_eq!(manifest.chunk_hashes.len(), 1);
        assert_eq!(manifest.chunk_hashes[0], ochra_crypto::blake3::hash(value));
    }
}
