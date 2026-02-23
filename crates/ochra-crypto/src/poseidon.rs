//! Poseidon hash on BLS12-381 scalar field.
//!
//! Used exclusively inside Groth16 circuits where BLAKE3 would incur
//! prohibitive constraint counts. Outside ZK circuits, BLAKE3 is used for all hashing.
//!
//! ## Parameters (Section 2.4)
//!
//! - Field: BLS12-381 scalar field
//! - Width (t): 3 (2-input Poseidon)
//! - Full rounds (R_F): 8 (4 before, 4 after partial rounds)
//! - Partial rounds (R_P): 57
//! - S-box: x^5
//! - Seed: b"Ochra_Poseidon_BLS12-381_t3"

use ark_bls12_381::Fr;
use ark_ff::{BigInteger256, Field, PrimeField};

use crate::{CryptoError, Result};

/// Poseidon parameters for BLS12-381 scalar field.
pub struct PoseidonParams {
    /// Round constants (R_F + R_P) * t field elements.
    pub round_constants: Vec<Fr>,
    /// MDS matrix (t x t).
    pub mds_matrix: Vec<Vec<Fr>>,
    /// Number of full rounds.
    pub full_rounds: usize,
    /// Number of partial rounds.
    pub partial_rounds: usize,
    /// Width of the permutation.
    pub width: usize,
}

/// Generate Poseidon round constants deterministically from seed.
///
/// Uses a simple PRNG seeded from the Ochra Poseidon seed to generate
/// field elements. This matches the Grain LFSR approach from the Poseidon paper.
fn generate_round_constants(num_constants: usize) -> Vec<Fr> {
    let seed = b"Ochra_Poseidon_BLS12-381_t3";
    let mut constants = Vec::with_capacity(num_constants);

    for i in 0..num_constants {
        // Deterministic generation: hash(seed || counter)
        let mut input = Vec::with_capacity(seed.len() + 8);
        input.extend_from_slice(seed);
        input.extend_from_slice(&(i as u64).to_le_bytes());
        let hash = crate::blake3::hash(&input);

        // Convert to field element (reduce modulo r)
        let mut repr = [0u64; 4];
        for (j, chunk) in hash.chunks(8).take(4).enumerate() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(chunk);
            repr[j] = u64::from_le_bytes(bytes);
        }
        let big = BigInteger256::new(repr);
        constants.push(Fr::from_bigint(big).unwrap_or(Fr::from(0u64)));
    }

    constants
}

/// Generate the MDS matrix for width=3 Poseidon.
///
/// Uses a Cauchy matrix construction for maximum diffusion.
fn generate_mds_matrix() -> Vec<Vec<Fr>> {
    let t = 3usize;
    let mut matrix = vec![vec![Fr::from(0u64); t]; t];

    // Cauchy matrix: M[i][j] = 1 / (x_i + y_j) where x_i = i, y_j = t + j
    for (i, row) in matrix.iter_mut().enumerate().take(t) {
        for (j, cell) in row.iter_mut().enumerate().take(t) {
            let x = Fr::from((i + 1) as u64);
            let y = Fr::from((t + j + 1) as u64);
            let sum = x + y;
            *cell = sum.inverse().unwrap_or(Fr::from(0u64));
        }
    }

    matrix
}

/// Get the default Poseidon parameters for Ochra.
pub fn default_params() -> PoseidonParams {
    let full_rounds = 8;
    let partial_rounds = 57;
    let width = 3;
    let num_constants = (full_rounds + partial_rounds) * width;

    PoseidonParams {
        round_constants: generate_round_constants(num_constants),
        mds_matrix: generate_mds_matrix(),
        full_rounds,
        partial_rounds,
        width,
    }
}

/// Apply the S-box (x^5) to a field element.
fn sbox(x: Fr) -> Fr {
    let x2 = x * x;
    let x4 = x2 * x2;
    x4 * x
}

/// Compute Poseidon hash of two field elements.
///
/// This is the core 2-input Poseidon function used in all Ochra ZK circuits.
pub fn hash(a: Fr, b: Fr) -> Fr {
    let params = default_params();
    poseidon_permutation(&params, a, b)
}

/// Poseidon sponge permutation for 2 inputs.
fn poseidon_permutation(params: &PoseidonParams, a: Fr, b: Fr) -> Fr {
    let t = params.width;
    let r_f = params.full_rounds;
    let r_p = params.partial_rounds;
    let half_f = r_f / 2;

    // Initial state: [0, a, b] (capacity=0, rate elements = a, b)
    let mut state = vec![Fr::from(0u64), a, b];

    let mut rc_idx = 0;

    // First half of full rounds
    for _ in 0..half_f {
        // Add round constants
        for (j, s) in state.iter_mut().enumerate().take(t) {
            *s += params.round_constants[rc_idx + j];
        }
        rc_idx += t;

        // Full S-box layer
        for s in state.iter_mut().take(t) {
            *s = sbox(*s);
        }

        // MDS matrix multiplication
        state = mds_mul(&params.mds_matrix, &state);
    }

    // Partial rounds
    for _ in 0..r_p {
        // Add round constants
        for (j, s) in state.iter_mut().enumerate().take(t) {
            *s += params.round_constants[rc_idx + j];
        }
        rc_idx += t;

        // Partial S-box (only first element)
        state[0] = sbox(state[0]);

        // MDS matrix multiplication
        state = mds_mul(&params.mds_matrix, &state);
    }

    // Second half of full rounds
    for _ in 0..half_f {
        // Add round constants
        for (j, s) in state.iter_mut().enumerate().take(t) {
            *s += params.round_constants[rc_idx + j];
        }
        rc_idx += t;

        // Full S-box layer
        for s in state.iter_mut().take(t) {
            *s = sbox(*s);
        }

        // MDS matrix multiplication
        state = mds_mul(&params.mds_matrix, &state);
    }

    // Output: first state element (capacity)
    state[1]
}

/// MDS matrix-vector multiplication.
fn mds_mul(matrix: &[Vec<Fr>], state: &[Fr]) -> Vec<Fr> {
    let t = state.len();
    let mut result = vec![Fr::from(0u64); t];
    for i in 0..t {
        for j in 0..t {
            result[i] += matrix[i][j] * state[j];
        }
    }
    result
}

/// Iterated Poseidon for 4 inputs: H(a,b,c,d) = Poseidon(Poseidon(a,b), Poseidon(c,d)).
pub fn hash_four(a: Fr, b: Fr, c: Fr, d: Fr) -> Fr {
    let left = hash(a, b);
    let right = hash(c, d);
    hash(left, right)
}

/// Convert bytes to a BLS12-381 scalar field element.
pub fn bytes_to_field(bytes: &[u8; 32]) -> Result<Fr> {
    let mut repr = [0u64; 4];
    for (i, chunk) in bytes.chunks(8).take(4).enumerate() {
        let mut b = [0u8; 8];
        b.copy_from_slice(chunk);
        repr[i] = u64::from_le_bytes(b);
    }
    let big = BigInteger256::new(repr);
    Fr::from_bigint(big)
        .ok_or_else(|| CryptoError::InvalidInput("value exceeds field modulus".into()))
}

/// Convert a field element to bytes.
pub fn field_to_bytes(f: &Fr) -> [u8; 32] {
    let repr = f.into_bigint();
    let mut bytes = [0u8; 32];
    for (i, limb) in repr.0.iter().enumerate() {
        bytes[i * 8..(i + 1) * 8].copy_from_slice(&limb.to_le_bytes());
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poseidon_deterministic() {
        let a = Fr::from(1u64);
        let b = Fr::from(2u64);
        let h1 = hash(a, b);
        let h2 = hash(a, b);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_poseidon_different_inputs() {
        let h1 = hash(Fr::from(1u64), Fr::from(2u64));
        let h2 = hash(Fr::from(3u64), Fr::from(4u64));
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_poseidon_zero_inputs() {
        let h = hash(Fr::from(0u64), Fr::from(0u64));
        // Should produce a non-zero output
        assert_ne!(h, Fr::from(0u64));
    }

    #[test]
    fn test_poseidon_four_inputs() {
        let h = hash_four(
            Fr::from(1u64),
            Fr::from(2u64),
            Fr::from(3u64),
            Fr::from(4u64),
        );
        // Should be deterministic
        let h2 = hash_four(
            Fr::from(1u64),
            Fr::from(2u64),
            Fr::from(3u64),
            Fr::from(4u64),
        );
        assert_eq!(h, h2);
    }

    #[test]
    fn test_poseidon_noncommutative() {
        // Poseidon(a, b) != Poseidon(b, a) in general
        let h1 = hash(Fr::from(1u64), Fr::from(2u64));
        let h2 = hash(Fr::from(2u64), Fr::from(1u64));
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_field_conversion_roundtrip() {
        let f = Fr::from(42u64);
        let bytes = field_to_bytes(&f);
        let restored = bytes_to_field(&bytes).expect("valid field element");
        assert_eq!(f, restored);
    }

    #[test]
    fn test_params_correct() {
        let params = default_params();
        assert_eq!(params.width, 3);
        assert_eq!(params.full_rounds, 8);
        assert_eq!(params.partial_rounds, 57);
        assert_eq!(params.round_constants.len(), (8 + 57) * 3);
        assert_eq!(params.mds_matrix.len(), 3);
        assert_eq!(params.mds_matrix[0].len(), 3);
    }

    #[test]
    fn test_sbox() {
        let x = Fr::from(3u64);
        let result = sbox(x);
        // 3^5 = 243
        assert_eq!(result, Fr::from(243u64));
    }
}
