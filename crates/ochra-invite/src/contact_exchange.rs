//! Contact exchange token system for establishing bidirectional contacts.
//!
//! A contact exchange token allows two parties to establish a mutual contact
//! relationship. The token contains the creator's identity information and
//! cryptographic keys needed to initiate secure communication.
//!
//! ## Token Contents
//!
//! A [`ContactExchangeToken`] contains:
//! - `pik_hash`: BLAKE3 hash of the creator's PIK public key
//! - `profile_key`: 256-bit key for encrypted profile lookup
//! - `display_name`: human-readable display name
//! - `x25519_pk`: X25519 public key for key exchange
//! - `signature`: Ed25519 signature over the token fields
//!
//! ## Token Lifecycle
//!
//! 1. **Generate**: Creator calls [`generate_token`] with their identity info.
//! 2. **Share**: Token is encoded as base64 and shared out-of-band.
//! 3. **Redeem**: Recipient calls [`redeem_token`] to validate and extract
//!    contact info.

use serde::{Deserialize, Serialize};

use crate::{InviteError, Result};

/// A contact exchange token for establishing bidirectional contacts.
///
/// This token is signed by the creator's PIK and contains all information
/// needed to add the creator as a contact.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContactExchangeToken {
    /// BLAKE3 hash of the creator's PIK public key.
    pub pik_hash: [u8; 32],
    /// 256-bit profile key for encrypted profile lookup.
    pub profile_key: [u8; 32],
    /// Human-readable display name.
    pub display_name: String,
    /// X25519 public key for key exchange.
    pub x25519_pk: [u8; 32],
    /// Ed25519 signature over `(pik_hash || profile_key || display_name || x25519_pk)`.
    pub signature: Vec<u8>,
}

/// Validated contact information extracted from a redeemed token.
#[derive(Clone, Debug)]
pub struct ContactInfo {
    /// BLAKE3 hash of the contact's PIK public key.
    pub pik_hash: [u8; 32],
    /// Profile key for encrypted profile lookup.
    pub profile_key: [u8; 32],
    /// Display name.
    pub display_name: String,
    /// X25519 public key for key exchange.
    pub x25519_pk: [u8; 32],
}

/// Generate a contact exchange token.
///
/// Creates a token containing the creator's identity information, signed
/// with their Ed25519 PIK signing key.
///
/// # Arguments
///
/// * `signing_key` - The creator's Ed25519 signing key (PIK)
/// * `profile_key` - The creator's 256-bit profile key
/// * `display_name` - Human-readable display name
/// * `x25519_pk` - The creator's X25519 public key for key exchange
pub fn generate_token(
    signing_key: &ochra_crypto::ed25519::SigningKey,
    profile_key: [u8; 32],
    display_name: &str,
    x25519_pk: [u8; 32],
) -> ContactExchangeToken {
    let pik_hash = ochra_crypto::blake3::hash(&signing_key.verifying_key().to_bytes());

    let signed_data = build_signed_data(&pik_hash, &profile_key, display_name, &x25519_pk);
    let signature = signing_key.sign(&signed_data);

    ContactExchangeToken {
        pik_hash,
        profile_key,
        display_name: display_name.to_string(),
        x25519_pk,
        signature: signature.to_bytes().to_vec(),
    }
}

/// Redeem (validate and parse) a contact exchange token.
///
/// Verifies the Ed25519 signature over the token fields using the provided
/// verifying key. The caller must supply the verifying key corresponding to
/// the `pik_hash` in the token (looked up from the network or local store).
///
/// # Arguments
///
/// * `token` - The contact exchange token to validate
/// * `verifying_key` - The Ed25519 verifying key of the token creator
///
/// # Returns
///
/// The validated [`ContactInfo`] extracted from the token.
pub fn redeem_token(
    token: &ContactExchangeToken,
    verifying_key: &ochra_crypto::ed25519::VerifyingKey,
) -> Result<ContactInfo> {
    // Verify that the pik_hash matches the provided verifying key.
    let expected_hash = ochra_crypto::blake3::hash(&verifying_key.to_bytes());
    if expected_hash != token.pik_hash {
        return Err(InviteError::InvalidToken(
            "pik_hash does not match verifying key".to_string(),
        ));
    }

    // Verify the signature.
    if token.signature.len() != 64 {
        return Err(InviteError::InvalidToken(
            "invalid signature length".to_string(),
        ));
    }
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&token.signature);
    let signature = ochra_crypto::ed25519::Signature::from_bytes(&sig_bytes);

    let signed_data = build_signed_data(
        &token.pik_hash,
        &token.profile_key,
        &token.display_name,
        &token.x25519_pk,
    );

    verifying_key
        .verify(&signed_data, &signature)
        .map_err(|_| InviteError::InvalidSignature)?;

    Ok(ContactInfo {
        pik_hash: token.pik_hash,
        profile_key: token.profile_key,
        display_name: token.display_name.clone(),
        x25519_pk: token.x25519_pk,
    })
}

/// Encode a contact exchange token to a base64 string for sharing.
pub fn encode_token(token: &ContactExchangeToken) -> Result<String> {
    let json = serde_json::to_vec(token).map_err(|e| InviteError::Serialization(e.to_string()))?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &json,
    ))
}

/// Decode a contact exchange token from a base64 string.
pub fn decode_token(encoded: &str) -> Result<ContactExchangeToken> {
    let json = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, encoded)
        .map_err(|e| InviteError::Malformed(format!("invalid base64: {}", e)))?;

    serde_json::from_slice(&json)
        .map_err(|e| InviteError::Malformed(format!("invalid token JSON: {}", e)))
}

/// Build the byte string that is signed for a contact exchange token.
///
/// Format: `pik_hash || profile_key || LE32(display_name.len()) || display_name || x25519_pk`
fn build_signed_data(
    pik_hash: &[u8; 32],
    profile_key: &[u8; 32],
    display_name: &str,
    x25519_pk: &[u8; 32],
) -> Vec<u8> {
    let name_bytes = display_name.as_bytes();
    let mut data = Vec::with_capacity(32 + 32 + 4 + name_bytes.len() + 32);
    data.extend_from_slice(pik_hash);
    data.extend_from_slice(profile_key);
    data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(name_bytes);
    data.extend_from_slice(x25519_pk);
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochra_crypto::ed25519::KeyPair;
    use ochra_crypto::x25519::X25519StaticSecret;

    fn make_test_token() -> (ContactExchangeToken, KeyPair) {
        let kp = KeyPair::generate();
        let x_secret = X25519StaticSecret::random();
        let x_pk = x_secret.public_key();

        let token = generate_token(&kp.signing_key, [0xAAu8; 32], "Alice", x_pk.to_bytes());

        (token, kp)
    }

    #[test]
    fn test_generate_token() {
        let (token, kp) = make_test_token();
        assert_eq!(
            token.pik_hash,
            ochra_crypto::blake3::hash(&kp.verifying_key.to_bytes())
        );
        assert_eq!(token.display_name, "Alice");
        assert_eq!(token.profile_key, [0xAAu8; 32]);
        assert_eq!(token.signature.len(), 64);
    }

    #[test]
    fn test_redeem_token_success() {
        let (token, kp) = make_test_token();
        let info = redeem_token(&token, &kp.verifying_key).expect("redeem");
        assert_eq!(info.pik_hash, token.pik_hash);
        assert_eq!(info.display_name, "Alice");
        assert_eq!(info.profile_key, [0xAAu8; 32]);
    }

    #[test]
    fn test_redeem_token_wrong_key() {
        let (token, _kp) = make_test_token();
        let other_kp = KeyPair::generate();
        let result = redeem_token(&token, &other_kp.verifying_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_redeem_token_tampered() {
        let (mut token, kp) = make_test_token();
        token.display_name = "Bob".to_string();
        let result = redeem_token(&token, &kp.verifying_key);
        assert!(result.is_err());
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        let (token, _) = make_test_token();
        let encoded = encode_token(&token).expect("encode");
        let decoded = decode_token(&encoded).expect("decode");
        assert_eq!(decoded.pik_hash, token.pik_hash);
        assert_eq!(decoded.display_name, token.display_name);
        assert_eq!(decoded.x25519_pk, token.x25519_pk);
        assert_eq!(decoded.signature, token.signature);
    }

    #[test]
    fn test_decode_invalid_base64() {
        let result = decode_token("not valid base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_invalid_json() {
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            b"not json",
        );
        let result = decode_token(&encoded);
        assert!(result.is_err());
    }

    #[test]
    fn test_token_signature_length() {
        let (token, _) = make_test_token();
        assert_eq!(token.signature.len(), 64);
    }
}
