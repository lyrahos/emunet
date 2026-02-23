//! Groth16/BLS12-381 proving and verification infrastructure.
//!
//! All ZK proofs in Ochra use Groth16 over BLS12-381. This module provides
//! the infrastructure for proof generation and verification.
//!
//! ## Performance Targets
//!
//! - Proof size: 192 bytes
//! - Verification time: < 2ms
//! - Desktop proving (2^16 constraints): ~3-5s

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::{
    Groth16, PreparedVerifyingKey, Proof, ProvingKey, VerifyingKey,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;

use crate::{CryptoError, Result};

/// Proof size in bytes for Groth16/BLS12-381.
pub const PROOF_SIZE: usize = 192;

/// A serialized Groth16 proof.
#[derive(Clone, Debug)]
pub struct SerializedProof {
    pub bytes: Vec<u8>,
}

/// A serialized verification key.
#[derive(Clone, Debug)]
pub struct SerializedVerifyingKey {
    pub bytes: Vec<u8>,
}

/// A serialized proving key.
#[derive(Clone, Debug)]
pub struct SerializedProvingKey {
    pub bytes: Vec<u8>,
}

/// Generate proving and verification keys for a circuit.
///
/// This is used during the trusted setup ceremony. In production, keys are
/// generated once and distributed with the binary.
pub fn setup<C: ConstraintSynthesizer<Fr>>(
    circuit: C,
) -> Result<(SerializedProvingKey, SerializedVerifyingKey)> {
    let mut rng = rand::rngs::OsRng;
    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(circuit, &mut rng)
        .map_err(|e| CryptoError::Proof(e.to_string()))?;

    let mut pk_bytes = Vec::new();
    pk.serialize_compressed(&mut pk_bytes)
        .map_err(|e| CryptoError::Serialization(e.to_string()))?;

    let mut vk_bytes = Vec::new();
    vk.serialize_compressed(&mut vk_bytes)
        .map_err(|e| CryptoError::Serialization(e.to_string()))?;

    Ok((
        SerializedProvingKey { bytes: pk_bytes },
        SerializedVerifyingKey { bytes: vk_bytes },
    ))
}

/// Generate a Groth16 proof.
pub fn prove<C: ConstraintSynthesizer<Fr>>(
    circuit: C,
    proving_key: &SerializedProvingKey,
) -> Result<SerializedProof> {
    let mut rng = rand::rngs::OsRng;

    let pk = ProvingKey::<Bls12_381>::deserialize_compressed(&*proving_key.bytes)
        .map_err(|e| CryptoError::Serialization(e.to_string()))?;

    let proof = Groth16::<Bls12_381>::prove(&pk, circuit, &mut rng)
        .map_err(|e| CryptoError::Proof(e.to_string()))?;

    let mut proof_bytes = Vec::new();
    proof
        .serialize_compressed(&mut proof_bytes)
        .map_err(|e| CryptoError::Serialization(e.to_string()))?;

    Ok(SerializedProof { bytes: proof_bytes })
}

/// Verify a Groth16 proof.
pub fn verify(
    proof: &SerializedProof,
    verifying_key: &SerializedVerifyingKey,
    public_inputs: &[Fr],
) -> Result<bool> {
    let vk = VerifyingKey::<Bls12_381>::deserialize_compressed(&*verifying_key.bytes)
        .map_err(|e| CryptoError::Serialization(e.to_string()))?;

    let pvk = PreparedVerifyingKey::from(vk);

    let proof = Proof::<Bls12_381>::deserialize_compressed(&*proof.bytes)
        .map_err(|e| CryptoError::Serialization(e.to_string()))?;

    Groth16::<Bls12_381>::verify_with_processed_vk(&pvk, public_inputs, &proof)
        .map_err(|e| CryptoError::Proof(e.to_string()))
}

/// A simple test circuit for validating the Groth16 infrastructure.
///
/// Proves knowledge of `a` and `b` such that `a * b = c` where `c` is public.
#[derive(Clone)]
pub struct MultiplyCircuit {
    pub a: Option<Fr>,
    pub b: Option<Fr>,
}

impl ConstraintSynthesizer<Fr> for MultiplyCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> std::result::Result<(), SynthesisError> {
        let a_val = self.a.unwrap_or(Fr::from(0u64));
        let b_val = self.b.unwrap_or(Fr::from(0u64));
        let c_val = a_val * b_val;

        // Allocate private inputs
        let a_var = cs.new_witness_variable(|| Ok(a_val))?;
        let b_var = cs.new_witness_variable(|| Ok(b_val))?;

        // Allocate public input
        let c_var = cs.new_input_variable(|| Ok(c_val))?;

        // Enforce: a * b = c
        cs.enforce_constraint(
            ark_relations::lc!() + a_var,
            ark_relations::lc!() + b_var,
            ark_relations::lc!() + c_var,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiply_circuit_prove_verify() {
        let a = Fr::from(3u64);
        let b = Fr::from(7u64);
        let c = a * b; // = 21

        // Setup
        let setup_circuit = MultiplyCircuit {
            a: Some(a),
            b: Some(b),
        };
        let (pk, vk) = setup(setup_circuit).expect("setup");

        // Prove
        let prove_circuit = MultiplyCircuit {
            a: Some(a),
            b: Some(b),
        };
        let proof = prove(prove_circuit, &pk).expect("prove");

        // Verify
        let public_inputs = vec![c];
        let result = verify(&proof, &vk, &public_inputs).expect("verify");
        assert!(result);
    }

    #[test]
    fn test_wrong_public_input_fails() {
        let a = Fr::from(3u64);
        let b = Fr::from(7u64);

        let setup_circuit = MultiplyCircuit {
            a: Some(a),
            b: Some(b),
        };
        let (pk, vk) = setup(setup_circuit).expect("setup");

        let prove_circuit = MultiplyCircuit {
            a: Some(a),
            b: Some(b),
        };
        let proof = prove(prove_circuit, &pk).expect("prove");

        // Wrong public input (22 instead of 21)
        let wrong_inputs = vec![Fr::from(22u64)];
        let result = verify(&proof, &vk, &wrong_inputs).expect("verify");
        assert!(!result);
    }

    #[test]
    fn test_proof_serialization_size() {
        let a = Fr::from(5u64);
        let b = Fr::from(11u64);

        let setup_circuit = MultiplyCircuit {
            a: Some(a),
            b: Some(b),
        };
        let (pk, _vk) = setup(setup_circuit).expect("setup");

        let prove_circuit = MultiplyCircuit {
            a: Some(a),
            b: Some(b),
        };
        let proof = prove(prove_circuit, &pk).expect("prove");

        // Groth16/BLS12-381 compressed proof should be around 128 bytes (compressed)
        // or 192 bytes (uncompressed). Compressed is typical.
        assert!(
            proof.bytes.len() <= PROOF_SIZE,
            "proof size {} exceeds expected {}",
            proof.bytes.len(),
            PROOF_SIZE
        );
    }
}
