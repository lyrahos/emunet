//! Minting circuit proof (Section 31.1).
//!
//! Provides an interface for generating and verifying Groth16 proofs that
//! attest to the correctness of a minting operation. The proof demonstrates
//! that the minted amount matches the receipts without revealing the
//! individual receipt values.
//!
//! In v1 this is a stub that produces deterministic placeholder proofs.
//! The full Groth16 circuit will be implemented when the ZK infrastructure
//! is complete.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{MintError, Result};

/// Input data for the minting proof circuit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MintingProofInput {
    /// Merkle root of the receipt tree being flushed.
    pub receipt_merkle_root: [u8; 32],
    /// Total amount of micro-seeds to mint.
    pub total_amount: u64,
    /// Epoch number for this minting batch.
    pub epoch: u64,
}

/// A serialized proof (stub in v1, Groth16 in production).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerializedProof {
    /// The proof bytes.
    pub bytes: Vec<u8>,
}

/// Public inputs for proof verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MintingPublicInputs {
    /// Merkle root of the receipt tree.
    pub receipt_merkle_root: [u8; 32],
    /// Total minted amount.
    pub total_amount: u64,
    /// Epoch number.
    pub epoch: u64,
}

/// Generate a minting proof for the given input.
///
/// In v1 this produces a deterministic stub proof based on a BLAKE3 hash
/// of the input data. The proof format is compatible with the verification
/// interface so that the transition to real Groth16 proofs is seamless.
///
/// # Errors
///
/// - [`MintError::ProofError`] if the total amount is zero
/// - [`MintError::ProofError`] if the receipt merkle root is all zeros
pub fn generate_minting_proof(input: &MintingProofInput) -> Result<SerializedProof> {
    if input.total_amount == 0 {
        return Err(MintError::ProofError(
            "total amount must be non-zero".to_string(),
        ));
    }
    if input.receipt_merkle_root == [0u8; 32] {
        return Err(MintError::ProofError(
            "receipt merkle root must be non-zero".to_string(),
        ));
    }

    // Stub proof: BLAKE3::hash(merkle_root || total_amount || epoch)
    let amount_bytes = input.total_amount.to_le_bytes();
    let epoch_bytes = input.epoch.to_le_bytes();
    let fields = blake3::encode_multi_field(&[
        &input.receipt_merkle_root[..],
        &amount_bytes,
        &epoch_bytes,
    ]);
    let proof_hash = blake3::hash(&fields);

    tracing::debug!(
        epoch = input.epoch,
        amount = input.total_amount,
        "generated minting proof (stub)"
    );

    Ok(SerializedProof {
        bytes: proof_hash.to_vec(),
    })
}

/// Verify a minting proof against public inputs.
///
/// In v1 this recomputes the stub proof and compares it to the provided
/// proof bytes.
///
/// # Returns
///
/// `true` if the proof is valid, `false` otherwise.
pub fn verify_minting_proof(
    proof: &SerializedProof,
    public_inputs: &MintingPublicInputs,
) -> bool {
    if public_inputs.total_amount == 0 || public_inputs.receipt_merkle_root == [0u8; 32] {
        return false;
    }

    // Recompute the expected stub proof
    let amount_bytes = public_inputs.total_amount.to_le_bytes();
    let epoch_bytes = public_inputs.epoch.to_le_bytes();
    let fields = blake3::encode_multi_field(&[
        &public_inputs.receipt_merkle_root[..],
        &amount_bytes,
        &epoch_bytes,
    ]);
    let expected = blake3::hash(&fields);

    proof.bytes == expected.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify() {
        let input = MintingProofInput {
            receipt_merkle_root: [0xAA; 32],
            total_amount: 1_000_000,
            epoch: 42,
        };

        let proof = generate_minting_proof(&input).expect("generate proof");

        let public_inputs = MintingPublicInputs {
            receipt_merkle_root: input.receipt_merkle_root,
            total_amount: input.total_amount,
            epoch: input.epoch,
        };

        assert!(verify_minting_proof(&proof, &public_inputs));
    }

    #[test]
    fn test_wrong_amount_fails_verification() {
        let input = MintingProofInput {
            receipt_merkle_root: [0xAA; 32],
            total_amount: 1_000_000,
            epoch: 42,
        };

        let proof = generate_minting_proof(&input).expect("generate proof");

        let wrong_inputs = MintingPublicInputs {
            receipt_merkle_root: [0xAA; 32],
            total_amount: 2_000_000,
            epoch: 42,
        };

        assert!(!verify_minting_proof(&proof, &wrong_inputs));
    }

    #[test]
    fn test_zero_amount_rejected() {
        let input = MintingProofInput {
            receipt_merkle_root: [0xAA; 32],
            total_amount: 0,
            epoch: 42,
        };

        assert!(generate_minting_proof(&input).is_err());
    }

    #[test]
    fn test_zero_merkle_root_rejected() {
        let input = MintingProofInput {
            receipt_merkle_root: [0u8; 32],
            total_amount: 1_000_000,
            epoch: 42,
        };

        assert!(generate_minting_proof(&input).is_err());
    }

    #[test]
    fn test_proof_deterministic() {
        let input = MintingProofInput {
            receipt_merkle_root: [0xBB; 32],
            total_amount: 500_000,
            epoch: 10,
        };

        let proof1 = generate_minting_proof(&input).expect("proof1");
        let proof2 = generate_minting_proof(&input).expect("proof2");
        assert_eq!(proof1.bytes, proof2.bytes);
    }

    #[test]
    fn test_verify_zero_inputs_returns_false() {
        let proof = SerializedProof {
            bytes: vec![0u8; 32],
        };
        let inputs = MintingPublicInputs {
            receipt_merkle_root: [0u8; 32],
            total_amount: 0,
            epoch: 0,
        };
        assert!(!verify_minting_proof(&proof, &inputs));
    }
}
