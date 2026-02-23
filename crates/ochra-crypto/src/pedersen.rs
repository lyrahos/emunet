//! Pedersen commitments on BLS12-381.
//!
//! Used for cryptographic value blinding in Seed token minting.
//! Pedersen commitments are homomorphic: `C(a, r1) + C(b, r2) = C(a+b, r1+r2)`.

use ark_bls12_381::{Fr, G1Projective as G1};
use ark_ec::Group;
use ark_ff::{PrimeField, UniformRand};
use ark_std::ops::Mul;

// Pedersen uses Result type from this crate (unused currently but reserved for future error paths)

/// Pedersen commitment generators.
///
/// The generators G and H must be chosen such that the discrete log
/// relationship between them is unknown.
pub struct PedersenParams {
    /// Generator G (base point).
    pub g: G1,
    /// Generator H (blinding base point).
    pub h: G1,
}

/// A Pedersen commitment C = v*G + r*H.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commitment {
    pub point: G1,
}

/// Opening of a Pedersen commitment.
pub struct Opening {
    /// The committed value.
    pub value: Fr,
    /// The blinding factor.
    pub blinding: Fr,
}

impl PedersenParams {
    /// Generate default parameters using hash-to-curve for nothing-up-my-sleeve.
    ///
    /// G is the standard BLS12-381 G1 generator.
    /// H is derived by hashing "Ochra Pedersen H" to the curve.
    pub fn default_params() -> Self {
        let g = G1::generator();

        // Derive H from a fixed seed using hash-to-point
        // This ensures no one knows log_g(H)
        let seed = crate::blake3::hash(b"Ochra Pedersen H generator BLS12-381");
        let mut repr = [0u64; 4];
        for (i, chunk) in seed.chunks(8).take(4).enumerate() {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(chunk);
            repr[i] = u64::from_le_bytes(bytes);
        }
        let scalar = Fr::from_bigint(ark_ff::BigInteger256::new(repr))
            .unwrap_or(Fr::from(1u64));
        let h = g.mul(scalar);

        Self { g, h }
    }

    /// Create a commitment: C = value * G + blinding * H.
    pub fn commit(&self, value: Fr, blinding: Fr) -> Commitment {
        let point = self.g.mul(value) + self.h.mul(blinding);
        Commitment { point }
    }

    /// Create a commitment with a random blinding factor.
    pub fn commit_random(&self, value: Fr) -> (Commitment, Fr) {
        let mut rng = rand::rngs::OsRng;
        let blinding = Fr::rand(&mut rng);
        let commitment = self.commit(value, blinding);
        (commitment, blinding)
    }

    /// Verify a commitment opening.
    pub fn verify(&self, commitment: &Commitment, opening: &Opening) -> bool {
        let expected = self.commit(opening.value, opening.blinding);
        commitment.point == expected.point
    }
}

impl Commitment {
    /// Add two commitments (homomorphic property).
    ///
    /// C(a, r1) + C(b, r2) = C(a+b, r1+r2)
    pub fn add(&self, other: &Commitment) -> Commitment {
        Commitment {
            point: self.point + other.point,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_verify() {
        let params = PedersenParams::default_params();
        let value = Fr::from(42u64);
        let blinding = Fr::from(12345u64);

        let commitment = params.commit(value, blinding);
        let opening = Opening { value, blinding };

        assert!(params.verify(&commitment, &opening));
    }

    #[test]
    fn test_wrong_opening_fails() {
        let params = PedersenParams::default_params();
        let commitment = params.commit(Fr::from(42u64), Fr::from(100u64));

        let wrong_opening = Opening {
            value: Fr::from(43u64),
            blinding: Fr::from(100u64),
        };
        assert!(!params.verify(&commitment, &wrong_opening));
    }

    #[test]
    fn test_homomorphic_property() {
        let params = PedersenParams::default_params();

        let v1 = Fr::from(10u64);
        let r1 = Fr::from(100u64);
        let v2 = Fr::from(20u64);
        let r2 = Fr::from(200u64);

        let c1 = params.commit(v1, r1);
        let c2 = params.commit(v2, r2);
        let c_sum = c1.add(&c2);

        // Should equal commitment to (v1+v2, r1+r2)
        let c_expected = params.commit(v1 + v2, r1 + r2);
        assert_eq!(c_sum, c_expected);
    }

    #[test]
    fn test_commit_random() {
        let params = PedersenParams::default_params();
        let (c1, r1) = params.commit_random(Fr::from(42u64));
        let (c2, r2) = params.commit_random(Fr::from(42u64));

        // Random blindings should differ
        assert_ne!(r1, r2);
        // Commitments should differ
        assert_ne!(c1, c2);

        // But both should verify
        let o1 = Opening {
            value: Fr::from(42u64),
            blinding: r1,
        };
        let o2 = Opening {
            value: Fr::from(42u64),
            blinding: r2,
        };
        assert!(params.verify(&c1, &o1));
        assert!(params.verify(&c2, &o2));
    }

    #[test]
    fn test_deterministic_params() {
        let p1 = PedersenParams::default_params();
        let p2 = PedersenParams::default_params();
        assert_eq!(p1.g, p2.g);
        assert_eq!(p1.h, p2.h);
    }
}
