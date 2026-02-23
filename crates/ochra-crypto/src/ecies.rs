//! ECIES-X25519-ChaCha20-BLAKE3 encryption (Section 2.5).
//!
//! Used for encrypting content decryption keys to buyer ephemeral public keys
//! in the threshold escrow flow and content key verification circuit.
//!
//! ## Algorithm
//!
//! ```text
//! ECIES.Encrypt(recipient_pk, plaintext; randomness):
//!   1. eph_sk = randomness
//!   2. eph_pk = X25519_basepoint_mult(eph_sk)
//!   3. shared_secret = X25519(eph_sk, recipient_pk)
//!   4. enc_key = BLAKE3::derive_key("Ochra v1 ecies-encryption-key",
//!               shared_secret || eph_pk || recipient_pk)
//!   5. nonce = BLAKE3::derive_key("Ochra v1 ecies-nonce",
//!             shared_secret || eph_pk)[:12]
//!   6. ciphertext = ChaCha20-Poly1305.Encrypt(enc_key, nonce, plaintext, aad=eph_pk)
//!   7. return (eph_pk || ciphertext || tag)
//! ```

use crate::blake3::{self, contexts};
use crate::chacha20;
use crate::x25519::{self, X25519PublicKey, X25519StaticSecret};
use crate::{CryptoError, Result};

/// ECIES ciphertext: ephemeral public key + ciphertext + tag.
pub struct EciesCiphertext {
    /// The ephemeral public key (32 bytes).
    pub eph_pk: [u8; 32],
    /// The ciphertext with appended Poly1305 tag.
    pub ciphertext_and_tag: Vec<u8>,
}

impl EciesCiphertext {
    /// Serialize to bytes: eph_pk || ciphertext || tag.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + self.ciphertext_and_tag.len());
        out.extend_from_slice(&self.eph_pk);
        out.extend_from_slice(&self.ciphertext_and_tag);
        out
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 32 + chacha20::TAG_SIZE {
            return Err(CryptoError::Ecies("ciphertext too short".into()));
        }
        let mut eph_pk = [0u8; 32];
        eph_pk.copy_from_slice(&data[..32]);
        Ok(Self {
            eph_pk,
            ciphertext_and_tag: data[32..].to_vec(),
        })
    }
}

/// Encrypt using ECIES with explicit randomness (for deterministic ECIES in ZK circuits).
///
/// # Arguments
///
/// * `recipient_pk` - Recipient's X25519 public key
/// * `plaintext` - Data to encrypt
/// * `randomness` - 32 bytes of randomness for ephemeral key generation
pub fn encrypt_deterministic(
    recipient_pk: &X25519PublicKey,
    plaintext: &[u8],
    randomness: &[u8; 32],
) -> Result<EciesCiphertext> {
    // Step 1-2: Generate ephemeral keypair
    let eph_pk_bytes = x25519::basepoint_mult(randomness);

    // Step 3: Compute shared secret
    let eph_secret = X25519StaticSecret::from_bytes(*randomness);
    let shared_secret = eph_secret.diffie_hellman(recipient_pk);

    // Step 4: Derive encryption key
    let mut key_material = Vec::with_capacity(32 + 32 + 32);
    key_material.extend_from_slice(shared_secret.as_bytes());
    key_material.extend_from_slice(&eph_pk_bytes);
    key_material.extend_from_slice(recipient_pk.as_bytes());
    let enc_key = blake3::derive_key(contexts::ECIES_ENCRYPTION_KEY, &key_material);

    // Step 5: Derive nonce
    let mut nonce_material = Vec::with_capacity(32 + 32);
    nonce_material.extend_from_slice(shared_secret.as_bytes());
    nonce_material.extend_from_slice(&eph_pk_bytes);
    let nonce_full = blake3::derive_key(contexts::ECIES_NONCE, &nonce_material);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_full[..12]);

    // Step 6: Encrypt with AAD = eph_pk
    let ciphertext_and_tag = chacha20::encrypt(&enc_key, &nonce, plaintext, &eph_pk_bytes)?;

    Ok(EciesCiphertext {
        eph_pk: eph_pk_bytes,
        ciphertext_and_tag,
    })
}

/// Encrypt using ECIES with random ephemeral key.
pub fn encrypt(recipient_pk: &X25519PublicKey, plaintext: &[u8]) -> Result<EciesCiphertext> {
    let mut randomness = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut randomness);
    encrypt_deterministic(recipient_pk, plaintext, &randomness)
}

/// Decrypt an ECIES ciphertext.
///
/// # Arguments
///
/// * `recipient_sk` - Recipient's X25519 static secret
/// * `ciphertext` - The ECIES ciphertext (eph_pk || ciphertext || tag)
pub fn decrypt(recipient_sk: &X25519StaticSecret, ciphertext: &EciesCiphertext) -> Result<Vec<u8>> {
    let eph_pk = X25519PublicKey::from_bytes(ciphertext.eph_pk);
    let recipient_pk = recipient_sk.public_key();

    // Step 1: Compute shared secret
    let shared_secret = recipient_sk.diffie_hellman(&eph_pk);

    // Step 2-3: Derive encryption key
    let mut key_material = Vec::with_capacity(32 + 32 + 32);
    key_material.extend_from_slice(shared_secret.as_bytes());
    key_material.extend_from_slice(&ciphertext.eph_pk);
    key_material.extend_from_slice(recipient_pk.as_bytes());
    let enc_key = blake3::derive_key(contexts::ECIES_ENCRYPTION_KEY, &key_material);

    // Step 4: Derive nonce
    let mut nonce_material = Vec::with_capacity(32 + 32);
    nonce_material.extend_from_slice(shared_secret.as_bytes());
    nonce_material.extend_from_slice(&ciphertext.eph_pk);
    let nonce_full = blake3::derive_key(contexts::ECIES_NONCE, &nonce_material);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&nonce_full[..12]);

    // Step 5: Decrypt with AAD = eph_pk
    chacha20::decrypt(
        &enc_key,
        &nonce,
        &ciphertext.ciphertext_and_tag,
        &ciphertext.eph_pk,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecies_roundtrip() {
        let sk = X25519StaticSecret::random();
        let pk = sk.public_key();

        let plaintext = b"Ochra content key test";
        let ct = encrypt(&pk, plaintext).expect("encrypt");
        let decrypted = decrypt(&sk, &ct).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ecies_deterministic() {
        let sk = X25519StaticSecret::random();
        let pk = sk.public_key();
        let randomness = [0x01u8; 32];

        let ct1 = encrypt_deterministic(&pk, b"test", &randomness).expect("encrypt");
        let ct2 = encrypt_deterministic(&pk, b"test", &randomness).expect("encrypt");

        assert_eq!(ct1.eph_pk, ct2.eph_pk);
        assert_eq!(ct1.ciphertext_and_tag, ct2.ciphertext_and_tag);
    }

    #[test]
    fn test_ecies_wrong_key_fails() {
        let sk1 = X25519StaticSecret::random();
        let sk2 = X25519StaticSecret::random();
        let pk1 = sk1.public_key();

        let ct = encrypt(&pk1, b"test").expect("encrypt");
        assert!(decrypt(&sk2, &ct).is_err());
    }

    #[test]
    fn test_ecies_serialization() {
        let sk = X25519StaticSecret::random();
        let pk = sk.public_key();

        let ct = encrypt(&pk, b"test data").expect("encrypt");
        let bytes = ct.to_bytes();
        let restored = EciesCiphertext::from_bytes(&bytes).expect("deserialize");

        let decrypted = decrypt(&sk, &restored).expect("decrypt");
        assert_eq!(decrypted, b"test data");
    }

    #[test]
    fn test_ecies_test_vector() {
        // Section 35.7: Deterministic ECIES with known randomness
        let randomness = [0x01u8; 32];
        let recipient_sk = X25519StaticSecret::from_bytes([0x02u8; 32]);
        let recipient_pk = recipient_sk.public_key();
        let plaintext = b"Ochra content key test";

        let ct = encrypt_deterministic(&recipient_pk, plaintext, &randomness).expect("encrypt");
        let decrypted = decrypt(&recipient_sk, &ct).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_ecies_empty_plaintext() {
        let sk = X25519StaticSecret::random();
        let pk = sk.public_key();

        let ct = encrypt(&pk, b"").expect("encrypt");
        let decrypted = decrypt(&sk, &ct).expect("decrypt");
        assert!(decrypted.is_empty());
    }
}
