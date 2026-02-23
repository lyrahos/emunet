//! zk-PoR (Proof of Retrievability) circuit interface (Section 31.2).
//!
//! Provides an interface for generating and verifying zero-knowledge proofs
//! that a node actually holds the data chunks it claims to store.
//!
//! In v1, this is a stub that produces deterministic placeholder proofs.
//! The full Groth16 circuit will be implemented with the ZK infrastructure.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{PowError, Result};

/// Input data for a Proof of Retrievability.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PorProofInput {
    /// Merkle root of the chunk tree.
    pub chunk_merkle_root: [u8; 32],
    /// Indices of the challenged chunks.
    pub chunk_indices: Vec<u32>,
    /// BLAKE3 hashes of the challenged chunks.
    pub chunk_hashes: Vec<[u8; 32]>,
}

/// A serialized PoR proof (stub in v1, Groth16 in production).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedProof {
    /// The proof bytes.
    pub bytes: Vec<u8>,
}

/// Public inputs for PoR proof verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PorPublicInputs {
    /// Merkle root of the chunk tree.
    pub chunk_merkle_root: [u8; 32],
    /// Indices of the challenged chunks.
    pub chunk_indices: Vec<u32>,
    /// BLAKE3 hashes of the challenged chunks.
    pub chunk_hashes: Vec<[u8; 32]>,
}

/// Generate a Proof of Retrievability for the given input.
///
/// In v1, this produces a deterministic stub proof based on a BLAKE3 hash.
///
/// # Errors
///
/// - [`PowError::ProofError`] if the input is malformed
pub fn generate_por_proof(input: &PorProofInput) -> Result<SerializedProof> {
    if input.chunk_indices.len() != input.chunk_hashes.len() {
        return Err(PowError::ProofError(
            "chunk_indices and chunk_hashes must have the same length".to_string(),
        ));
    }
    if input.chunk_indices.is_empty() {
        return Err(PowError::ProofError(
            "at least one chunk must be challenged".to_string(),
        ));
    }
    if input.chunk_merkle_root == [0u8; 32] {
        return Err(PowError::ProofError(
            "chunk merkle root must be non-zero".to_string(),
        ));
    }

    // Stub proof: BLAKE3 hash of all inputs concatenated
    let mut hasher_input = Vec::new();
    hasher_input.extend_from_slice(&input.chunk_merkle_root);
    for idx in &input.chunk_indices {
        hasher_input.extend_from_slice(&idx.to_le_bytes());
    }
    for hash in &input.chunk_hashes {
        hasher_input.extend_from_slice(hash);
    }

    let proof_hash = blake3::hash(&hasher_input);

    Ok(SerializedProof {
        bytes: proof_hash.to_vec(),
    })
}

/// Verify a Proof of Retrievability against public inputs.
///
/// In v1, this recomputes the stub proof and compares.
///
/// # Returns
///
/// `true` if the proof is valid, `false` otherwise.
pub fn verify_por_proof(
    proof: &SerializedProof,
    public_inputs: &PorPublicInputs,
) -> bool {
    if public_inputs.chunk_indices.len() != public_inputs.chunk_hashes.len() {
        return false;
    }
    if public_inputs.chunk_indices.is_empty() {
        return false;
    }
    if public_inputs.chunk_merkle_root == [0u8; 32] {
        return false;
    }

    // Recompute expected stub proof
    let mut hasher_input = Vec::new();
    hasher_input.extend_from_slice(&public_inputs.chunk_merkle_root);
    for idx in &public_inputs.chunk_indices {
        hasher_input.extend_from_slice(&idx.to_le_bytes());
    }
    for hash in &public_inputs.chunk_hashes {
        hasher_input.extend_from_slice(hash);
    }

    let expected = blake3::hash(&hasher_input);
    proof.bytes == expected.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify() {
        let input = PorProofInput {
            chunk_merkle_root: [0xAA; 32],
            chunk_indices: vec![0, 5, 10],
            chunk_hashes: vec![[0x11; 32], [0x22; 32], [0x33; 32]],
        };

        let proof = generate_por_proof(&input).expect("generate");

        let public_inputs = PorPublicInputs {
            chunk_merkle_root: input.chunk_merkle_root,
            chunk_indices: input.chunk_indices,
            chunk_hashes: input.chunk_hashes,
        };

        assert!(verify_por_proof(&proof, &public_inputs));
    }

    #[test]
    fn test_wrong_merkle_root_fails() {
        let input = PorProofInput {
            chunk_merkle_root: [0xAA; 32],
            chunk_indices: vec![0],
            chunk_hashes: vec![[0x11; 32]],
        };

        let proof = generate_por_proof(&input).expect("generate");

        let wrong_inputs = PorPublicInputs {
            chunk_merkle_root: [0xBB; 32],
            chunk_indices: vec![0],
            chunk_hashes: vec![[0x11; 32]],
        };

        assert!(!verify_por_proof(&proof, &wrong_inputs));
    }

    #[test]
    fn test_mismatched_lengths_rejected() {
        let input = PorProofInput {
            chunk_merkle_root: [0xAA; 32],
            chunk_indices: vec![0, 1],
            chunk_hashes: vec![[0x11; 32]], // only 1
        };

        assert!(generate_por_proof(&input).is_err());
    }

    #[test]
    fn test_empty_chunks_rejected() {
        let input = PorProofInput {
            chunk_merkle_root: [0xAA; 32],
            chunk_indices: vec![],
            chunk_hashes: vec![],
        };

        assert!(generate_por_proof(&input).is_err());
    }

    #[test]
    fn test_zero_merkle_root_rejected() {
        let input = PorProofInput {
            chunk_merkle_root: [0u8; 32],
            chunk_indices: vec![0],
            chunk_hashes: vec![[0x11; 32]],
        };

        assert!(generate_por_proof(&input).is_err());
    }

    #[test]
    fn test_proof_deterministic() {
        let input = PorProofInput {
            chunk_merkle_root: [0xAA; 32],
            chunk_indices: vec![0, 5],
            chunk_hashes: vec![[0x11; 32], [0x22; 32]],
        };

        let proof1 = generate_por_proof(&input).expect("proof1");
        let proof2 = generate_por_proof(&input).expect("proof2");
        assert_eq!(proof1.bytes, proof2.bytes);
    }
}
