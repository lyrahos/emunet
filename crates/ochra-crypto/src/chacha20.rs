//! ChaCha20-Poly1305 AEAD encryption (RFC 8439).
//!
//! Used for payload encryption, Whisper messages, Guardian packets,
//! and PIK-at-rest encryption.

use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    ChaCha20Poly1305, Key, Nonce,
};

use crate::{CryptoError, Result};

/// Nonce size for ChaCha20-Poly1305 (96 bits = 12 bytes).
pub const NONCE_SIZE: usize = 12;

/// Key size for ChaCha20-Poly1305 (256 bits = 32 bytes).
pub const KEY_SIZE: usize = 32;

/// Authentication tag size (128 bits = 16 bytes).
pub const TAG_SIZE: usize = 16;

/// Encrypt data with ChaCha20-Poly1305.
///
/// # Arguments
///
/// * `key` - 32-byte encryption key
/// * `nonce` - 12-byte nonce (must never be reused with the same key)
/// * `plaintext` - Data to encrypt
/// * `aad` - Additional authenticated data (not encrypted, but authenticated)
///
/// # Returns
///
/// Ciphertext with appended 16-byte authentication tag.
pub fn encrypt(key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(nonce);

    cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CryptoError::AeadDecryption)
}

/// Decrypt data with ChaCha20-Poly1305.
///
/// # Arguments
///
/// * `key` - 32-byte encryption key
/// * `nonce` - 12-byte nonce
/// * `ciphertext` - Ciphertext with appended authentication tag
/// * `aad` - Additional authenticated data (must match what was used during encryption)
///
/// # Returns
///
/// Decrypted plaintext, or error if authentication fails.
pub fn decrypt(key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE], ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = Nonce::from_slice(nonce);

    cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| CryptoError::AeadDecryption)
}

/// Encrypt data without additional authenticated data.
pub fn encrypt_no_aad(key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE], plaintext: &[u8]) -> Result<Vec<u8>> {
    encrypt(key, nonce, plaintext, &[])
}

/// Decrypt data without additional authenticated data.
pub fn decrypt_no_aad(key: &[u8; KEY_SIZE], nonce: &[u8; NONCE_SIZE], ciphertext: &[u8]) -> Result<Vec<u8>> {
    decrypt(key, nonce, ciphertext, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; KEY_SIZE];
        let nonce = [0x01u8; NONCE_SIZE];
        let plaintext = b"Hello, Ochra!";
        let aad = b"associated data";

        let ciphertext = encrypt(&key, &nonce, plaintext, aad).expect("encrypt");
        let decrypted = decrypt(&key, &nonce, &ciphertext, aad).expect("decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_no_aad_roundtrip() {
        let key = [0x42u8; KEY_SIZE];
        let nonce = [0x01u8; NONCE_SIZE];
        let plaintext = b"test data";

        let ciphertext = encrypt_no_aad(&key, &nonce, plaintext).expect("encrypt");
        let decrypted = decrypt_no_aad(&key, &nonce, &ciphertext).expect("decrypt");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ciphertext_has_tag() {
        let key = [0x42u8; KEY_SIZE];
        let nonce = [0x01u8; NONCE_SIZE];
        let plaintext = b"test";

        let ciphertext = encrypt_no_aad(&key, &nonce, plaintext).expect("encrypt");
        assert_eq!(ciphertext.len(), plaintext.len() + TAG_SIZE);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1 = [0x01u8; KEY_SIZE];
        let key2 = [0x02u8; KEY_SIZE];
        let nonce = [0x01u8; NONCE_SIZE];

        let ciphertext = encrypt_no_aad(&key1, &nonce, b"test").expect("encrypt");
        assert!(decrypt_no_aad(&key2, &nonce, &ciphertext).is_err());
    }

    #[test]
    fn test_wrong_nonce_fails() {
        let key = [0x01u8; KEY_SIZE];
        let nonce1 = [0x01u8; NONCE_SIZE];
        let nonce2 = [0x02u8; NONCE_SIZE];

        let ciphertext = encrypt_no_aad(&key, &nonce1, b"test").expect("encrypt");
        assert!(decrypt_no_aad(&key, &nonce2, &ciphertext).is_err());
    }

    #[test]
    fn test_wrong_aad_fails() {
        let key = [0x01u8; KEY_SIZE];
        let nonce = [0x01u8; NONCE_SIZE];

        let ciphertext = encrypt(&key, &nonce, b"test", b"aad1").expect("encrypt");
        assert!(decrypt(&key, &nonce, &ciphertext, b"aad2").is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let key = [0x01u8; KEY_SIZE];
        let nonce = [0x01u8; NONCE_SIZE];

        let mut ciphertext = encrypt_no_aad(&key, &nonce, b"test").expect("encrypt");
        if let Some(byte) = ciphertext.first_mut() {
            *byte ^= 0xFF;
        }
        assert!(decrypt_no_aad(&key, &nonce, &ciphertext).is_err());
    }

    #[test]
    fn test_empty_plaintext() {
        let key = [0x42u8; KEY_SIZE];
        let nonce = [0x01u8; NONCE_SIZE];

        let ciphertext = encrypt_no_aad(&key, &nonce, b"").expect("encrypt");
        assert_eq!(ciphertext.len(), TAG_SIZE);

        let decrypted = decrypt_no_aad(&key, &nonce, &ciphertext).expect("decrypt");
        assert!(decrypted.is_empty());
    }
}
