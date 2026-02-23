//! Argon2id password hashing and Proof-of-Work.
//!
//! Used for:
//! - PIK-at-rest key derivation (m=256MB, t=3, p=4)
//! - Publishing Proof-of-Work
//! - Handle registration Proof-of-Work

use argon2::{Algorithm, Argon2, Params, Version};

use crate::{CryptoError, Result};

/// Default Argon2id parameters for PIK derivation.
/// m=256MB, t=3 iterations, p=4 parallelism lanes.
pub const PIK_M_COST: u32 = 262144; // 256 * 1024 = 256 MB in KiB
pub const PIK_T_COST: u32 = 3;
pub const PIK_P_COST: u32 = 4;
pub const PIK_OUTPUT_LEN: usize = 32;

/// Derive a key from a password using Argon2id with PIK parameters.
///
/// This is used to derive the encryption key for PIK-at-rest storage.
/// Parameters: m=256MB, t=3, p=4 (Section 2.1 of the v5.5 spec).
///
/// # Arguments
///
/// * `password` - User's password
/// * `salt` - Random 16-byte salt (stored alongside encrypted PIK)
pub fn derive_pik_key(password: &[u8], salt: &[u8]) -> Result<[u8; PIK_OUTPUT_LEN]> {
    let params = Params::new(PIK_M_COST, PIK_T_COST, PIK_P_COST, Some(PIK_OUTPUT_LEN))
        .map_err(|e| CryptoError::Argon2(e.to_string()))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut output = [0u8; PIK_OUTPUT_LEN];
    argon2
        .hash_password_into(password, salt, &mut output)
        .map_err(|e| CryptoError::Argon2(e.to_string()))?;

    Ok(output)
}

/// Derive a key with custom Argon2id parameters.
///
/// Used for Proof-of-Work and other applications where different parameters
/// are needed.
pub fn derive_key_custom(
    password: &[u8],
    salt: &[u8],
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
    output_len: usize,
) -> Result<Vec<u8>> {
    let params = Params::new(m_cost, t_cost, p_cost, Some(output_len))
        .map_err(|e| CryptoError::Argon2(e.to_string()))?;

    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

    let mut output = vec![0u8; output_len];
    argon2
        .hash_password_into(password, salt, &mut output)
        .map_err(|e| CryptoError::Argon2(e.to_string()))?;

    Ok(output)
}

/// Verify an Argon2id Proof-of-Work.
///
/// The work is valid if the first `difficulty` bits of the output are zero.
///
/// # Arguments
///
/// * `data` - The data being committed to (e.g., content hash, handle)
/// * `nonce` - The nonce found by the miner
/// * `difficulty` - Number of leading zero bits required
/// * `m_cost` - Memory cost in KiB
/// * `t_cost` - Time cost (iterations)
/// * `p_cost` - Parallelism
pub fn verify_pow(
    data: &[u8],
    nonce: &[u8],
    difficulty: u32,
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<bool> {
    let hash = derive_key_custom(data, nonce, m_cost, t_cost, p_cost, 32)?;
    Ok(count_leading_zero_bits(&hash) >= difficulty)
}

/// Count the number of leading zero bits in a byte slice.
fn count_leading_zero_bits(data: &[u8]) -> u32 {
    let mut count = 0u32;
    for byte in data {
        if *byte == 0 {
            count += 8;
        } else {
            count += byte.leading_zeros();
            break;
        }
    }
    count
}

/// Generate a random 16-byte salt for Argon2id.
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut salt);
    salt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_key_custom_deterministic() {
        let password = b"test password";
        let salt = b"1234567890123456"; // 16 bytes

        // Use small parameters for testing
        let key1 = derive_key_custom(password, salt, 1024, 1, 1, 32).expect("derive");
        let key2 = derive_key_custom(password, salt, 1024, 1, 1, 32).expect("derive");
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_key_different_passwords() {
        let salt = b"1234567890123456";
        let key1 = derive_key_custom(b"pass1", salt, 1024, 1, 1, 32).expect("derive");
        let key2 = derive_key_custom(b"pass2", salt, 1024, 1, 1, 32).expect("derive");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_key_different_salts() {
        let password = b"password";
        let key1 =
            derive_key_custom(password, b"salt111111111111", 1024, 1, 1, 32).expect("derive");
        let key2 =
            derive_key_custom(password, b"salt222222222222", 1024, 1, 1, 32).expect("derive");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_count_leading_zero_bits() {
        assert_eq!(count_leading_zero_bits(&[0x00, 0x00, 0xFF]), 16);
        assert_eq!(count_leading_zero_bits(&[0x00, 0x80, 0xFF]), 8);
        assert_eq!(count_leading_zero_bits(&[0x00, 0x01, 0xFF]), 15);
        assert_eq!(count_leading_zero_bits(&[0x80]), 0);
        assert_eq!(count_leading_zero_bits(&[0x40]), 1);
        assert_eq!(count_leading_zero_bits(&[0x00]), 8);
    }

    #[test]
    fn test_generate_salt() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        assert_ne!(salt1, salt2);
        assert_eq!(salt1.len(), 16);
    }

    #[test]
    fn test_pow_verification() {
        // With difficulty 0, any input should pass
        let result = verify_pow(b"data", b"nonce12345678901", 0, 1024, 1, 1).expect("verify");
        assert!(result);
    }
}
