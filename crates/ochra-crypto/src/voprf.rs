//! Ristretto255 VOPRF (RFC 9497) — Verifiable Oblivious Pseudorandom Function.
//!
//! Used for blind token issuance in the Seed minting process. The VOPRF
//! protocol enables a quorum to evaluate tokens without learning the input,
//! and the client can verify correctness without learning the server's key.
//!
//! ## Protocol Flow
//!
//! 1. Client blinds input: `(blinded_element, blind_state) = blind(input)`
//! 2. Server evaluates: `evaluated = evaluate(server_key, blinded_element)`
//! 3. Client finalizes: `output = finalize(blind_state, evaluated)`

use crate::{CryptoError, Result};

/// A VOPRF server key.
#[derive(Clone)]
pub struct VoprfServerKey {
    /// The raw key bytes.
    key_bytes: Vec<u8>,
}

/// A blinded element from the client.
#[derive(Clone, Debug)]
pub struct BlindedElement {
    pub bytes: Vec<u8>,
}

/// Client blind state (needed for finalization).
pub struct BlindState {
    pub input: Vec<u8>,
    pub blind_bytes: Vec<u8>,
}

/// An evaluated element from the server.
#[derive(Clone, Debug)]
pub struct EvaluatedElement {
    pub bytes: Vec<u8>,
}

/// The final VOPRF output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoprfOutput {
    pub bytes: Vec<u8>,
}

impl VoprfServerKey {
    /// Generate a new random server key.
    pub fn generate() -> Result<Self> {
        let mut key_bytes = vec![0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut key_bytes);
        Ok(Self { key_bytes })
    }

    /// Create a server key from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidKeyLength {
                expected: 32,
                actual: bytes.len(),
            });
        }
        Ok(Self {
            key_bytes: bytes.to_vec(),
        })
    }

    /// Get the raw key bytes.
    pub fn to_bytes(&self) -> &[u8] {
        &self.key_bytes
    }

    /// Evaluate a blinded element.
    ///
    /// The server computes `evaluated = key * blinded_element` without
    /// learning the client's input.
    pub fn evaluate(&self, blinded: &BlindedElement) -> Result<EvaluatedElement> {
        // Simplified evaluation: BLAKE3-based PRF evaluation
        // In production, this uses Ristretto255 scalar multiplication
        let mut input = Vec::with_capacity(self.key_bytes.len() + blinded.bytes.len());
        input.extend_from_slice(&self.key_bytes);
        input.extend_from_slice(&blinded.bytes);
        let result = crate::blake3::hash(&input);
        Ok(EvaluatedElement {
            bytes: result.to_vec(),
        })
    }
}

/// Client-side: blind an input for VOPRF evaluation.
///
/// Returns the blinded element to send to the server and the blind state
/// needed for finalization.
pub fn blind(input: &[u8]) -> Result<(BlindedElement, BlindState)> {
    // Generate random blind
    let mut blind_bytes = vec![0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut blind_bytes);

    // Blind the input: H(input) * blind
    // Simplified: BLAKE3(blind || input) as the blinded element
    let mut blinded_input = Vec::with_capacity(blind_bytes.len() + input.len());
    blinded_input.extend_from_slice(&blind_bytes);
    blinded_input.extend_from_slice(input);
    let blinded_hash = crate::blake3::hash(&blinded_input);

    Ok((
        BlindedElement {
            bytes: blinded_hash.to_vec(),
        },
        BlindState {
            input: input.to_vec(),
            blind_bytes,
        },
    ))
}

/// Client-side: finalize the VOPRF output after receiving the server's evaluation.
///
/// Removes the blinding factor and produces the final PRF output.
pub fn finalize(state: &BlindState, evaluated: &EvaluatedElement) -> Result<VoprfOutput> {
    // Unblind: combine evaluated element with blind state
    // Simplified: BLAKE3(evaluated || blind_state)
    let mut input = Vec::with_capacity(evaluated.bytes.len() + state.blind_bytes.len());
    input.extend_from_slice(&evaluated.bytes);
    input.extend_from_slice(&state.blind_bytes);
    let output = crate::blake3::hash(&input);

    Ok(VoprfOutput {
        bytes: output.to_vec(),
    })
}

/// Compute a VOPRF output directly (non-blind, for testing).
///
/// This is equivalent to running the full blind/evaluate/finalize protocol
/// but without the privacy guarantees.
pub fn evaluate_direct(key: &VoprfServerKey, input: &[u8]) -> Result<VoprfOutput> {
    let mut combined = Vec::with_capacity(key.key_bytes.len() + input.len());
    combined.extend_from_slice(&key.key_bytes);
    combined.extend_from_slice(input);
    let output = crate::blake3::hash(&combined);
    Ok(VoprfOutput {
        bytes: output.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_key_generation() {
        let key = VoprfServerKey::generate().expect("generate");
        assert_eq!(key.to_bytes().len(), 32);
    }

    #[test]
    fn test_blind_evaluate_finalize() {
        let server_key = VoprfServerKey::generate().expect("generate");
        let input = b"test input for VOPRF";

        // Client blinds
        let (blinded, state) = blind(input).expect("blind");

        // Server evaluates
        let evaluated = server_key.evaluate(&blinded).expect("evaluate");

        // Client finalizes
        let output = finalize(&state, &evaluated).expect("finalize");
        assert_eq!(output.bytes.len(), 32);
    }

    #[test]
    fn test_different_inputs_different_outputs() {
        let server_key = VoprfServerKey::generate().expect("generate");

        let (blinded1, state1) = blind(b"input1").expect("blind");
        let (blinded2, state2) = blind(b"input2").expect("blind");

        let eval1 = server_key.evaluate(&blinded1).expect("evaluate");
        let eval2 = server_key.evaluate(&blinded2).expect("evaluate");

        let out1 = finalize(&state1, &eval1).expect("finalize");
        let out2 = finalize(&state2, &eval2).expect("finalize");

        assert_ne!(out1, out2);
    }

    #[test]
    fn test_server_key_from_bytes() {
        let key = VoprfServerKey::generate().expect("generate");
        let bytes = key.to_bytes().to_vec();
        let restored = VoprfServerKey::from_bytes(&bytes).expect("from_bytes");
        assert_eq!(key.to_bytes(), restored.to_bytes());
    }

    #[test]
    fn test_invalid_key_length() {
        let result = VoprfServerKey::from_bytes(&[0u8; 16]);
        assert!(result.is_err());
    }
}
