//! VOPRF blind token issuance protocol.
//!
//! Implements the client and server sides of the VOPRF-based minting protocol.
//! The client blinds a token serial, the server evaluates the blinded element,
//! and the client unblinds the result to obtain a valid minted token.
//!
//! ## Protocol Flow
//!
//! 1. Client: `blind(amount)` -> `(BlindedToken, BlindState)`
//! 2. Server: `evaluate(blinded, server_key)` -> `EvaluatedToken`
//! 3. Client: `unblind(evaluated, state)` -> `UnblindedToken`

use ochra_crypto::blake3;
use ochra_crypto::voprf::{self, BlindState, BlindedElement, EvaluatedElement, VoprfServerKey};
use serde::{Deserialize, Serialize};

use crate::{Denomination, MintError, Result};

/// A blinded token ready to be sent to the server for evaluation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlindedToken {
    /// The blinded element bytes.
    pub blinded_element: Vec<u8>,
    /// The denomination of the token being minted.
    pub denomination: Denomination,
}

/// An evaluated token returned by the server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvaluatedToken {
    /// The evaluated element bytes.
    pub evaluated_element: Vec<u8>,
}

/// An unblinded token — the final minted token held by the client.
#[derive(Clone, Debug)]
pub struct UnblindedToken {
    /// The token serial (random, client-generated).
    pub serial: [u8; 32],
    /// The VOPRF output (proof of valid minting).
    pub voprf_output: Vec<u8>,
    /// Denomination in micro-seeds.
    pub denomination: Denomination,
    /// Spend secret (random, client-held).
    pub spend_secret: [u8; 32],
}

/// Client-side blind state preserved between `blind` and `unblind` calls.
pub struct MintBlindState {
    /// The token serial.
    serial: [u8; 32],
    /// The VOPRF blind state.
    blind_state: BlindState,
    /// Denomination in micro-seeds.
    denomination: Denomination,
    /// Spend secret (random, client-generated).
    spend_secret: [u8; 32],
}

/// Client-side VOPRF minting operations.
pub struct MintClient;

/// Server-side VOPRF minting operations.
pub struct MintServer;

impl MintClient {
    /// Blind a token for the given amount.
    ///
    /// Generates a random serial and spend secret, blinds the serial with
    /// the denomination, and returns the blinded token plus the state needed
    /// to unblind later.
    ///
    /// # Errors
    ///
    /// - [`MintError::InvalidDenomination`] if amount is zero
    /// - [`MintError::Voprf`] if the underlying VOPRF operation fails
    pub fn blind(amount: Denomination) -> Result<(BlindedToken, MintBlindState)> {
        if amount == 0 {
            return Err(MintError::InvalidDenomination(0));
        }

        // Generate random serial and spend secret
        let mut serial = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut serial);
        let mut spend_secret = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut spend_secret);

        // Build VOPRF input: serial || denomination
        let denom_bytes = amount.to_le_bytes();
        let input = blake3::encode_multi_field(&[&serial[..], &denom_bytes]);

        // Blind
        let (blinded_element, blind_state) =
            voprf::blind(&input).map_err(|e| MintError::Voprf(e.to_string()))?;

        let blinded_token = BlindedToken {
            blinded_element: blinded_element.bytes,
            denomination: amount,
        };

        let state = MintBlindState {
            serial,
            blind_state,
            denomination: amount,
            spend_secret,
        };

        Ok((blinded_token, state))
    }

    /// Unblind an evaluated token to obtain the final minted token.
    ///
    /// # Errors
    ///
    /// - [`MintError::Voprf`] if the unblinding operation fails
    pub fn unblind(
        evaluated: &EvaluatedToken,
        state: &MintBlindState,
    ) -> Result<UnblindedToken> {
        let eval_element = EvaluatedElement {
            bytes: evaluated.evaluated_element.clone(),
        };

        let output =
            voprf::finalize(&state.blind_state, &eval_element)
                .map_err(|e| MintError::Voprf(e.to_string()))?;

        Ok(UnblindedToken {
            serial: state.serial,
            voprf_output: output.bytes,
            denomination: state.denomination,
            spend_secret: state.spend_secret,
        })
    }
}

impl MintServer {
    /// Evaluate a blinded token using the server key.
    ///
    /// The server computes the VOPRF evaluation without learning the
    /// client's token serial.
    ///
    /// # Errors
    ///
    /// - [`MintError::Voprf`] if the evaluation fails
    pub fn evaluate(
        blinded: &BlindedToken,
        server_key: &VoprfServerKey,
    ) -> Result<EvaluatedToken> {
        let blinded_element = BlindedElement {
            bytes: blinded.blinded_element.clone(),
        };

        let evaluated = server_key
            .evaluate(&blinded_element)
            .map_err(|e| MintError::Voprf(e.to_string()))?;

        Ok(EvaluatedToken {
            evaluated_element: evaluated.bytes,
        })
    }
}

impl UnblindedToken {
    /// Derive the nullifier for this token.
    ///
    /// `nullifier = BLAKE3::hash(serial || spend_secret)`
    pub fn nullifier(&self) -> [u8; 32] {
        let mut input = [0u8; 64];
        input[..32].copy_from_slice(&self.serial);
        input[32..].copy_from_slice(&self.spend_secret);
        blake3::hash(&input)
    }

    /// Derive the token commitment.
    ///
    /// `commitment = BLAKE3::hash(serial || voprf_output || denomination_le)`
    pub fn commitment(&self) -> [u8; 32] {
        let denom_bytes = self.denomination.to_le_bytes();
        let fields = blake3::encode_multi_field(&[
            &self.serial[..],
            &self.voprf_output,
            &denom_bytes,
        ]);
        blake3::hash(&fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blind_evaluate_unblind() {
        let server_key = VoprfServerKey::generate().expect("generate key");
        let denomination = 1_000_000u64;

        // Client blinds
        let (blinded, state) = MintClient::blind(denomination).expect("blind");
        assert_eq!(blinded.denomination, denomination);

        // Server evaluates
        let evaluated = MintServer::evaluate(&blinded, &server_key).expect("evaluate");

        // Client unblinds
        let token = MintClient::unblind(&evaluated, &state).expect("unblind");
        assert_eq!(token.denomination, denomination);
        assert_eq!(token.voprf_output.len(), 32);
    }

    #[test]
    fn test_zero_denomination_rejected() {
        let result = MintClient::blind(0);
        assert!(result.is_err());
    }

    #[test]
    fn test_nullifier_deterministic() {
        let server_key = VoprfServerKey::generate().expect("generate key");
        let (blinded, state) = MintClient::blind(100).expect("blind");
        let evaluated = MintServer::evaluate(&blinded, &server_key).expect("evaluate");
        let token = MintClient::unblind(&evaluated, &state).expect("unblind");

        let n1 = token.nullifier();
        let n2 = token.nullifier();
        assert_eq!(n1, n2);
    }

    #[test]
    fn test_commitment_deterministic() {
        let server_key = VoprfServerKey::generate().expect("generate key");
        let (blinded, state) = MintClient::blind(100).expect("blind");
        let evaluated = MintServer::evaluate(&blinded, &server_key).expect("evaluate");
        let token = MintClient::unblind(&evaluated, &state).expect("unblind");

        let c1 = token.commitment();
        let c2 = token.commitment();
        assert_eq!(c1, c2);
        assert_ne!(c1, [0u8; 32]);
    }

    #[test]
    fn test_different_tokens_different_nullifiers() {
        let server_key = VoprfServerKey::generate().expect("generate key");

        let (b1, s1) = MintClient::blind(100).expect("blind1");
        let (b2, s2) = MintClient::blind(100).expect("blind2");

        let e1 = MintServer::evaluate(&b1, &server_key).expect("eval1");
        let e2 = MintServer::evaluate(&b2, &server_key).expect("eval2");

        let t1 = MintClient::unblind(&e1, &s1).expect("unblind1");
        let t2 = MintClient::unblind(&e2, &s2).expect("unblind2");

        assert_ne!(t1.nullifier(), t2.nullifier());
    }
}
