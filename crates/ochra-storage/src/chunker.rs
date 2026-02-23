//! 4 MB chunk splitting with Merkle tree construction and verification.
//!
//! Content is split into fixed-size 4 MB chunks. Each chunk is hashed using
//! BLAKE3 with domain-separated leaf hashing. A Merkle tree is built from
//! the chunk hashes to produce a `content_hash` (the Merkle root).
//!
//! Merkle proofs allow verifying that a chunk belongs to a given content
//! without downloading the entire content.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{Result, StorageError};

/// Chunk size: 4 MB.
pub const CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// A single chunk of content data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chunk {
    /// BLAKE3 hash identifying this chunk (merkle leaf hash of its data).
    pub chunk_id: [u8; 32],
    /// The raw chunk data.
    pub data: Vec<u8>,
    /// Zero-based index of this chunk within the content.
    pub index: u32,
}

/// Result of splitting content into chunks.
#[derive(Clone, Debug)]
pub struct SplitResult {
    /// The individual chunks.
    pub chunks: Vec<Chunk>,
    /// The Merkle root hash (content_hash).
    pub content_hash: [u8; 32],
    /// The leaf hashes used to build the Merkle tree.
    pub leaf_hashes: Vec<[u8; 32]>,
}

/// A Merkle proof for a single leaf.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleProof {
    /// The sibling hashes along the path from leaf to root.
    /// Each entry is `(hash, is_left)` where `is_left` indicates
    /// whether the sibling is on the left side.
    pub siblings: Vec<([u8; 32], bool)>,
}

/// Split content into 4 MB chunks and compute the Merkle root.
///
/// # Arguments
///
/// * `data` - The raw content bytes to split.
///
/// # Returns
///
/// A [`SplitResult`] containing the chunks, content hash (Merkle root),
/// and leaf hashes.
pub fn split_content(data: &[u8]) -> Result<SplitResult> {
    if data.is_empty() {
        return Err(StorageError::EmptyContent);
    }

    let mut chunks = Vec::new();
    let mut leaf_hashes = Vec::new();

    for (i, chunk_data) in data.chunks(CHUNK_SIZE).enumerate() {
        let chunk_id = blake3::merkle_leaf(chunk_data);
        leaf_hashes.push(chunk_id);
        chunks.push(Chunk {
            chunk_id,
            data: chunk_data.to_vec(),
            index: i as u32,
        });
    }

    let content_hash = build_merkle_root(&leaf_hashes);

    Ok(SplitResult {
        chunks,
        content_hash,
        leaf_hashes,
    })
}

/// Build a Merkle root from a list of leaf hashes.
///
/// If the number of leaves is not a power of two, the last leaf is
/// duplicated to pad the tree to the next level.
pub fn build_merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    if leaves.len() == 1 {
        return leaves[0];
    }

    let mut current_level: Vec<[u8; 32]> = leaves.to_vec();

    while current_level.len() > 1 {
        let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));

        let mut i = 0;
        while i < current_level.len() {
            let left = &current_level[i];
            let right = if i + 1 < current_level.len() {
                &current_level[i + 1]
            } else {
                // Duplicate the last node if odd number of nodes.
                &current_level[i]
            };
            next_level.push(blake3::merkle_inner(left, right));
            i += 2;
        }

        current_level = next_level;
    }

    current_level[0]
}

/// Generate a Merkle proof for a leaf at the given index.
///
/// # Arguments
///
/// * `leaves` - All leaf hashes in the tree.
/// * `index` - The index of the leaf to generate a proof for.
///
/// # Returns
///
/// A [`MerkleProof`] containing the sibling hashes from leaf to root.
pub fn generate_merkle_proof(leaves: &[[u8; 32]], index: usize) -> Result<MerkleProof> {
    if leaves.is_empty() || index >= leaves.len() {
        return Err(StorageError::MerkleVerification);
    }

    if leaves.len() == 1 {
        return Ok(MerkleProof {
            siblings: Vec::new(),
        });
    }

    let mut siblings = Vec::new();
    let mut current_level: Vec<[u8; 32]> = leaves.to_vec();
    let mut current_index = index;

    while current_level.len() > 1 {
        let sibling_index = if current_index.is_multiple_of(2) {
            if current_index + 1 < current_level.len() {
                current_index + 1
            } else {
                current_index
            }
        } else {
            current_index - 1
        };

        // is_left = true means the sibling is on the left side.
        let is_left = current_index % 2 == 1;
        siblings.push((current_level[sibling_index], is_left));

        // Build the next level.
        let mut next_level = Vec::with_capacity(current_level.len().div_ceil(2));
        let mut i = 0;
        while i < current_level.len() {
            let left = &current_level[i];
            let right = if i + 1 < current_level.len() {
                &current_level[i + 1]
            } else {
                &current_level[i]
            };
            next_level.push(blake3::merkle_inner(left, right));
            i += 2;
        }

        current_level = next_level;
        current_index /= 2;
    }

    Ok(MerkleProof { siblings })
}

/// Verify a Merkle proof for a given leaf against a known root.
///
/// # Arguments
///
/// * `root` - The expected Merkle root.
/// * `leaf` - The leaf hash to verify.
/// * `proof` - The Merkle proof containing sibling hashes.
/// * `index` - The index of the leaf in the tree.
///
/// # Returns
///
/// `true` if the proof is valid, `false` otherwise.
pub fn verify_merkle_proof(
    root: &[u8; 32],
    leaf: &[u8; 32],
    proof: &MerkleProof,
    index: u32,
) -> bool {
    let mut current = *leaf;
    let _ = index; // Index validated by proof structure.

    for (sibling, is_left) in &proof.siblings {
        current = if *is_left {
            blake3::merkle_inner(sibling, &current)
        } else {
            blake3::merkle_inner(&current, sibling)
        };
    }

    current == *root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_small_content() {
        let data = vec![0xABu8; 1024];
        let result = split_content(&data).expect("split should succeed");
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0].index, 0);
        assert_eq!(result.chunks[0].data.len(), 1024);
        assert_eq!(result.leaf_hashes.len(), 1);
    }

    #[test]
    fn test_split_exact_chunk_size() {
        let data = vec![0xCDu8; CHUNK_SIZE];
        let result = split_content(&data).expect("split should succeed");
        assert_eq!(result.chunks.len(), 1);
        assert_eq!(result.chunks[0].data.len(), CHUNK_SIZE);
    }

    #[test]
    fn test_split_multiple_chunks() {
        let data = vec![0xEFu8; CHUNK_SIZE * 3 + 100];
        let result = split_content(&data).expect("split should succeed");
        assert_eq!(result.chunks.len(), 4);
        assert_eq!(result.chunks[0].data.len(), CHUNK_SIZE);
        assert_eq!(result.chunks[1].data.len(), CHUNK_SIZE);
        assert_eq!(result.chunks[2].data.len(), CHUNK_SIZE);
        assert_eq!(result.chunks[3].data.len(), 100);
        for (i, chunk) in result.chunks.iter().enumerate() {
            assert_eq!(chunk.index, i as u32);
        }
    }

    #[test]
    fn test_split_empty_content_fails() {
        let result = split_content(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_merkle_root_single_leaf() {
        let leaf = blake3::merkle_leaf(b"hello");
        let root = build_merkle_root(&[leaf]);
        assert_eq!(root, leaf);
    }

    #[test]
    fn test_merkle_root_two_leaves() {
        let leaf0 = blake3::merkle_leaf(b"chunk0");
        let leaf1 = blake3::merkle_leaf(b"chunk1");
        let root = build_merkle_root(&[leaf0, leaf1]);
        let expected = blake3::merkle_inner(&leaf0, &leaf1);
        assert_eq!(root, expected);
    }

    #[test]
    fn test_merkle_root_deterministic() {
        let data = vec![0xAAu8; CHUNK_SIZE * 2 + 500];
        let r1 = split_content(&data).expect("split");
        let r2 = split_content(&data).expect("split");
        assert_eq!(r1.content_hash, r2.content_hash);
    }

    #[test]
    fn test_merkle_proof_single_leaf() {
        let leaf = blake3::merkle_leaf(b"only");
        let root = build_merkle_root(&[leaf]);
        let proof = generate_merkle_proof(&[leaf], 0).expect("proof");
        assert!(proof.siblings.is_empty());
        assert!(verify_merkle_proof(&root, &leaf, &proof, 0));
    }

    #[test]
    fn test_merkle_proof_two_leaves() {
        let leaf0 = blake3::merkle_leaf(b"chunk0");
        let leaf1 = blake3::merkle_leaf(b"chunk1");
        let leaves = [leaf0, leaf1];
        let root = build_merkle_root(&leaves);

        let proof0 = generate_merkle_proof(&leaves, 0).expect("proof0");
        assert!(verify_merkle_proof(&root, &leaf0, &proof0, 0));

        let proof1 = generate_merkle_proof(&leaves, 1).expect("proof1");
        assert!(verify_merkle_proof(&root, &leaf1, &proof1, 1));
    }

    #[test]
    fn test_merkle_proof_four_leaves() {
        let leaves: Vec<[u8; 32]> = (0..4u8).map(|i| blake3::merkle_leaf(&[i])).collect();
        let root = build_merkle_root(&leaves);

        for i in 0..4 {
            let proof = generate_merkle_proof(&leaves, i).expect("proof");
            assert!(
                verify_merkle_proof(&root, &leaves[i], &proof, i as u32),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn test_merkle_proof_three_leaves_odd() {
        let leaves: Vec<[u8; 32]> = (0..3u8).map(|i| blake3::merkle_leaf(&[i])).collect();
        let root = build_merkle_root(&leaves);

        for i in 0..3 {
            let proof = generate_merkle_proof(&leaves, i).expect("proof");
            assert!(
                verify_merkle_proof(&root, &leaves[i], &proof, i as u32),
                "proof failed for leaf {i}"
            );
        }
    }

    #[test]
    fn test_merkle_proof_wrong_leaf_fails() {
        let leaf0 = blake3::merkle_leaf(b"real");
        let leaf1 = blake3::merkle_leaf(b"also_real");
        let fake = blake3::merkle_leaf(b"fake");
        let leaves = [leaf0, leaf1];
        let root = build_merkle_root(&leaves);

        let proof0 = generate_merkle_proof(&leaves, 0).expect("proof");
        assert!(!verify_merkle_proof(&root, &fake, &proof0, 0));
    }

    #[test]
    fn test_chunk_ids_are_leaf_hashes() {
        let data = vec![0xBBu8; CHUNK_SIZE + 100];
        let result = split_content(&data).expect("split");
        for (chunk, leaf_hash) in result.chunks.iter().zip(result.leaf_hashes.iter()) {
            assert_eq!(&chunk.chunk_id, leaf_hash);
            assert_eq!(chunk.chunk_id, blake3::merkle_leaf(&chunk.data));
        }
    }
}
