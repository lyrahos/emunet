//! P2P transfer notes.
//!
//! Transfer notes enable private peer-to-peer Seed transfers. The note
//! is encrypted to the recipient's public key and includes the amount
//! and an optional message.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{Result, SpendError};

/// A P2P transfer note encrypted to a recipient.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransferNote {
    /// Hint for the sender (e.g., truncated PIK hash). Not a full identifier.
    pub from_hint: [u8; 8],
    /// Amount in micro-seeds.
    pub amount: u64,
    /// Encrypted message: `BLAKE3::derive_key("Ochra v1 transfer-note-key", recipient_pk)` + XOR.
    ///
    /// In v1, this is a simplified symmetric encryption for the message payload.
    /// In production, this uses ECIES to the recipient's X25519 key.
    pub encrypted_message: Vec<u8>,
}

/// Create a transfer note for a recipient.
///
/// # Arguments
///
/// * `recipient_pk` - The recipient's public key (32 bytes)
/// * `amount` - Amount in micro-seeds
/// * `message` - Optional human-readable message
///
/// # Errors
///
/// - [`SpendError::InvalidProof`] if amount is zero
/// - [`SpendError::InvalidProof`] if recipient_pk is all zeros
pub fn create_transfer_note(
    recipient_pk: &[u8; 32],
    amount: u64,
    message: &str,
) -> Result<TransferNote> {
    if amount == 0 {
        return Err(SpendError::InvalidProof(
            "transfer amount must be non-zero".to_string(),
        ));
    }
    if recipient_pk == &[0u8; 32] {
        return Err(SpendError::InvalidProof(
            "recipient public key must be non-zero".to_string(),
        ));
    }

    // Derive encryption key from recipient's public key
    let enc_key = blake3::derive_key(blake3::contexts::TRANSFER_NOTE_KEY, recipient_pk);

    // Encrypt the payload: amount (8 bytes LE) || message (UTF-8)
    let mut plaintext = Vec::with_capacity(8 + message.len());
    plaintext.extend_from_slice(&amount.to_le_bytes());
    plaintext.extend_from_slice(message.as_bytes());

    // Simple XOR-stream encryption using BLAKE3 XOF for v1
    let mut keystream = vec![0u8; plaintext.len()];
    blake3::hash_xof(&enc_key, &mut keystream);
    let encrypted_message: Vec<u8> = plaintext
        .iter()
        .zip(keystream.iter())
        .map(|(p, k)| p ^ k)
        .collect();

    // From hint: first 8 bytes of a random tag (not identifying)
    let mut from_hint = [0u8; 8];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut from_hint);

    Ok(TransferNote {
        from_hint,
        amount,
        encrypted_message,
    })
}

/// Decrypt a transfer note using the recipient's secret key.
///
/// In v1, the "secret key" is the same as the public key for the symmetric
/// derivation. In production, this uses the X25519 private key.
///
/// # Arguments
///
/// * `note` - The transfer note to decrypt
/// * `recipient_sk` - The recipient's secret key (32 bytes)
///
/// # Returns
///
/// A tuple of `(amount_microseed, message_string)`.
///
/// # Errors
///
/// - [`SpendError::CryptoError`] if decryption produces invalid UTF-8
pub fn decrypt_transfer_note(
    note: &TransferNote,
    recipient_sk: &[u8; 32],
) -> Result<(u64, String)> {
    // Derive the same encryption key
    let enc_key = blake3::derive_key(blake3::contexts::TRANSFER_NOTE_KEY, recipient_sk);

    // Decrypt with XOR stream
    let mut keystream = vec![0u8; note.encrypted_message.len()];
    blake3::hash_xof(&enc_key, &mut keystream);
    let plaintext: Vec<u8> = note
        .encrypted_message
        .iter()
        .zip(keystream.iter())
        .map(|(c, k)| c ^ k)
        .collect();

    if plaintext.len() < 8 {
        return Err(SpendError::CryptoError(
            "decrypted payload too short".to_string(),
        ));
    }

    let mut amount_bytes = [0u8; 8];
    amount_bytes.copy_from_slice(&plaintext[..8]);
    let amount = u64::from_le_bytes(amount_bytes);

    let message = String::from_utf8(plaintext[8..].to_vec())
        .map_err(|e| SpendError::CryptoError(format!("invalid UTF-8 in decrypted message: {e}")))?;

    Ok((amount, message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_decrypt() {
        let recipient_pk = [0x42; 32];
        let amount = 1_000_000u64;
        let message = "Thanks for the content!";

        let note = create_transfer_note(&recipient_pk, amount, message).expect("create");
        assert_eq!(note.amount, amount);

        // In v1, pk == sk for symmetric derivation
        let (dec_amount, dec_msg) = decrypt_transfer_note(&note, &recipient_pk).expect("decrypt");
        assert_eq!(dec_amount, amount);
        assert_eq!(dec_msg, message);
    }

    #[test]
    fn test_create_transfer_note_zero_amount() {
        assert!(create_transfer_note(&[0x42; 32], 0, "test").is_err());
    }

    #[test]
    fn test_create_transfer_note_zero_pk() {
        assert!(create_transfer_note(&[0u8; 32], 1000, "test").is_err());
    }

    #[test]
    fn test_wrong_key_produces_garbage() {
        let recipient_pk = [0x42; 32];
        let note = create_transfer_note(&recipient_pk, 1000, "hello").expect("create");

        let wrong_key = [0x99; 32];
        let result = decrypt_transfer_note(&note, &wrong_key);
        // May produce garbage amount or invalid UTF-8
        if let Ok((amount, _)) = result {
            assert_ne!(amount, 1000);
        }
        // Err case: invalid UTF-8 is expected
    }

    #[test]
    fn test_empty_message() {
        let recipient_pk = [0x42; 32];
        let note = create_transfer_note(&recipient_pk, 500, "").expect("create");
        let (amount, msg) = decrypt_transfer_note(&note, &recipient_pk).expect("decrypt");
        assert_eq!(amount, 500);
        assert_eq!(msg, "");
    }
}
