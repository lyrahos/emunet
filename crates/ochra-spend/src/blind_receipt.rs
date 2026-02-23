//! Blind receipt token system.
//!
//! Blind receipts are privacy-preserving proofs of purchase. The receipt
//! contains a blinded content hash so that the receipt issuer cannot link
//! the receipt to the specific content purchased.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{SpendError, Result};

/// A blind receipt for a content purchase.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlindReceipt {
    /// Unique receipt identifier.
    pub receipt_id: [u8; 32],
    /// BLAKE3 hash of the blinded content hash.
    pub blinded_content_hash: [u8; 32],
    /// The amount paid in micro-seeds.
    pub amount: u64,
    /// Unix timestamp of issuance.
    pub issued_at: u64,
}

/// Generate a blind receipt for a content purchase.
///
/// # Arguments
///
/// * `content_hash` - The hash of the purchased content
/// * `amount` - The amount paid in micro-seeds
///
/// # Errors
///
/// - [`SpendError::InvalidReceipt`] if the content hash is all zeros
/// - [`SpendError::InvalidReceipt`] if the amount is zero
pub fn generate_receipt(content_hash: &[u8; 32], amount: u64) -> Result<BlindReceipt> {
    if content_hash == &[0u8; 32] {
        return Err(SpendError::InvalidReceipt(
            "content hash must be non-zero".to_string(),
        ));
    }
    if amount == 0 {
        return Err(SpendError::InvalidReceipt(
            "amount must be non-zero".to_string(),
        ));
    }

    // Generate a random blinding factor
    let mut blind_factor = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut blind_factor);

    // Blind the content hash: BLAKE3::hash(content_hash || blind_factor)
    let mut blind_input = [0u8; 64];
    blind_input[..32].copy_from_slice(content_hash);
    blind_input[32..].copy_from_slice(&blind_factor);
    let blinded_content_hash = blake3::hash(&blind_input);

    // Derive receipt ID from the blinded content hash and amount
    let amount_bytes = amount.to_le_bytes();
    let fields = blake3::encode_multi_field(&[
        &blinded_content_hash[..],
        &amount_bytes,
    ]);
    let receipt_id = blake3::hash(&fields);

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(BlindReceipt {
        receipt_id,
        blinded_content_hash,
        amount,
        issued_at: timestamp,
    })
}

/// Verify the basic well-formedness of a blind receipt.
///
/// This checks structural validity (non-zero fields). Full verification
/// requires checking against the receipt ledger.
pub fn verify_receipt(receipt: &BlindReceipt) -> bool {
    if receipt.receipt_id == [0u8; 32] {
        return false;
    }
    if receipt.blinded_content_hash == [0u8; 32] {
        return false;
    }
    if receipt.amount == 0 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_receipt() {
        let content_hash = [0xAA; 32];
        let receipt = generate_receipt(&content_hash, 1_000_000).expect("generate");

        assert_ne!(receipt.receipt_id, [0u8; 32]);
        assert_ne!(receipt.blinded_content_hash, [0u8; 32]);
        assert_eq!(receipt.amount, 1_000_000);
        assert!(receipt.issued_at > 0);
    }

    #[test]
    fn test_generate_receipt_zero_hash() {
        assert!(generate_receipt(&[0u8; 32], 1000).is_err());
    }

    #[test]
    fn test_generate_receipt_zero_amount() {
        assert!(generate_receipt(&[0xAA; 32], 0).is_err());
    }

    #[test]
    fn test_generate_receipt_is_blinded() {
        let content_hash = [0xAA; 32];
        let r1 = generate_receipt(&content_hash, 1000).expect("r1");
        let r2 = generate_receipt(&content_hash, 1000).expect("r2");
        // Different blind factors produce different blinded hashes
        assert_ne!(r1.blinded_content_hash, r2.blinded_content_hash);
    }

    #[test]
    fn test_verify_receipt_valid() {
        let content_hash = [0xAA; 32];
        let receipt = generate_receipt(&content_hash, 1000).expect("generate");
        assert!(verify_receipt(&receipt));
    }

    #[test]
    fn test_verify_receipt_zero_id() {
        let receipt = BlindReceipt {
            receipt_id: [0u8; 32],
            blinded_content_hash: [0xAA; 32],
            amount: 1000,
            issued_at: 12345,
        };
        assert!(!verify_receipt(&receipt));
    }

    #[test]
    fn test_verify_receipt_zero_amount() {
        let receipt = BlindReceipt {
            receipt_id: [0x01; 32],
            blinded_content_hash: [0xAA; 32],
            amount: 0,
            issued_at: 12345,
        };
        assert!(!verify_receipt(&receipt));
    }
}
