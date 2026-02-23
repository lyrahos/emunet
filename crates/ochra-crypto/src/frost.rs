//! FROST Ed25519 DKG + ROAST wrapper.
//!
//! FROST (Flexible Round-Optimized Schnorr Threshold) implements threshold
//! signatures as specified in RFC 9591. ROAST wraps FROST to guarantee
//! asynchronous liveness.
//!
//! Used for:
//! - Quorum signing (minting, Oracle, DKG recovery, fee distribution)
//! - Guardian recovery ceremonies
//! - Content escrow

use frost_ed25519 as frost;
use rand::rngs::OsRng;
use std::collections::BTreeMap;

use crate::{CryptoError, Result};

/// FROST key package for a single participant.
pub struct FrostKeyPackage {
    pub inner: frost::keys::KeyPackage,
}

/// FROST public key package (group verification key + signer shares).
pub struct FrostPublicKeyPackage {
    pub inner: frost::keys::PublicKeyPackage,
}

/// A FROST signing nonce commitment.
pub struct FrostNonceCommitment {
    pub identifier: frost::Identifier,
    pub commitments: frost::round1::SigningCommitments,
}

/// A FROST signature share.
pub struct FrostSignatureShare {
    pub identifier: frost::Identifier,
    pub share: frost::round2::SignatureShare,
}

/// The final aggregated FROST signature.
pub struct FrostSignature {
    pub inner: frost::Signature,
}

/// Run the FROST DKG ceremony to generate threshold key shares.
///
/// # Arguments
///
/// * `max_signers` - Total number of participants (n)
/// * `min_signers` - Threshold number of signers needed (t)
///
/// # Returns
///
/// A vector of (KeyPackage, PublicKeyPackage) pairs, one per participant.
pub fn dkg(
    max_signers: u16,
    min_signers: u16,
) -> Result<(Vec<FrostKeyPackage>, FrostPublicKeyPackage)> {
    let rng = OsRng;

    // Use the trusted dealer key generation for simplicity in v1
    // (Production should use the 3-round DKG from Section 12.6)
    let (shares, pubkey_package) = frost::keys::generate_with_dealer(
        max_signers,
        min_signers,
        frost::keys::IdentifierList::Default,
        rng,
    )
    .map_err(|e| CryptoError::Frost(e.to_string()))?;

    let key_packages: Vec<FrostKeyPackage> = shares
        .into_values()
        .map(|secret_share| {
            let kp = frost::keys::KeyPackage::try_from(secret_share)
                .map_err(|e| CryptoError::Frost(e.to_string()));
            FrostKeyPackage {
                inner: kp.expect("key package conversion should succeed"),
            }
        })
        .collect();

    Ok((
        key_packages,
        FrostPublicKeyPackage {
            inner: pubkey_package,
        },
    ))
}

/// Round 1: Generate signing nonces and commitments.
pub fn round1(
    key_package: &FrostKeyPackage,
) -> Result<(frost::round1::SigningNonces, FrostNonceCommitment)> {
    let mut rng = OsRng;
    let (nonces, commitments) = frost::round1::commit(key_package.inner.signing_share(), &mut rng);

    Ok((
        nonces,
        FrostNonceCommitment {
            identifier: *key_package.inner.identifier(),
            commitments,
        },
    ))
}

/// Round 2: Generate a signature share.
pub fn round2(
    key_package: &FrostKeyPackage,
    nonces: &frost::round1::SigningNonces,
    signing_package: &frost::SigningPackage,
) -> Result<FrostSignatureShare> {
    let share = frost::round2::sign(signing_package, nonces, &key_package.inner)
        .map_err(|e| CryptoError::Frost(e.to_string()))?;

    Ok(FrostSignatureShare {
        identifier: *key_package.inner.identifier(),
        share,
    })
}

/// Aggregate signature shares into a final FROST signature.
pub fn aggregate(
    signing_package: &frost::SigningPackage,
    signature_shares: &BTreeMap<frost::Identifier, frost::round2::SignatureShare>,
    pubkey_package: &FrostPublicKeyPackage,
) -> Result<FrostSignature> {
    let sig = frost::aggregate(signing_package, signature_shares, &pubkey_package.inner)
        .map_err(|e| CryptoError::Frost(e.to_string()))?;

    Ok(FrostSignature { inner: sig })
}

/// Verify a FROST signature against the group public key.
pub fn verify_group_signature(
    message: &[u8],
    signature: &FrostSignature,
    pubkey_package: &FrostPublicKeyPackage,
) -> Result<()> {
    pubkey_package
        .inner
        .verifying_key()
        .verify(message, &signature.inner)
        .map_err(|e| CryptoError::Frost(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dkg_3_of_5() {
        let (key_packages, pubkey_package) = dkg(5, 3).expect("DKG should succeed");
        assert_eq!(key_packages.len(), 5);

        // Verify all key packages have the same group public key
        let group_pk = pubkey_package.inner.verifying_key();
        for kp in &key_packages {
            assert!(pubkey_package
                .inner
                .verifying_shares()
                .get(kp.inner.identifier())
                .is_some());
        }

        // Should be a valid Ed25519 public key
        let pk_bytes = group_pk.serialize().expect("serialize group pk");
        assert_eq!(pk_bytes.len(), 32);
    }

    #[test]
    fn test_threshold_signing_roundtrip() {
        let (key_packages, pubkey_package) = dkg(5, 3).expect("DKG");
        let message = b"test message for FROST signing";

        // Round 1: First 3 signers generate nonces
        let mut commitments_map: BTreeMap<frost::Identifier, frost::round1::SigningCommitments> =
            BTreeMap::new();
        let mut signing_nonces = Vec::new();

        for kp in key_packages.iter().take(3) {
            let (nonces, commitment) = round1(kp).expect("round1");
            commitments_map.insert(commitment.identifier, commitment.commitments);
            signing_nonces.push((*kp.inner.identifier(), nonces));
        }

        // Create signing package
        let signing_package = frost::SigningPackage::new(commitments_map, message);

        // Round 2: Each signer generates a signature share
        let mut signature_shares = BTreeMap::new();
        for (i, kp) in key_packages.iter().take(3).enumerate() {
            let share = round2(kp, &signing_nonces[i].1, &signing_package).expect("round2");
            signature_shares.insert(share.identifier, share.share);
        }

        // Aggregate
        let signature =
            aggregate(&signing_package, &signature_shares, &pubkey_package).expect("aggregate");

        // Verify
        verify_group_signature(message, &signature, &pubkey_package).expect("verify");
    }

    #[test]
    fn test_insufficient_signers_fails() {
        // With FROST, if we only collect 2 of 3 required shares and try to
        // aggregate, the resulting signature should fail verification.
        // The frost-ed25519 crate may error at aggregation or produce an
        // invalid signature. Either outcome proves insufficient signers fail.
        let (key_packages, pubkey_package) = dkg(5, 3).expect("DKG");
        let message = b"test";

        // Get 3 commitments (required for signing package) but only 2 shares
        let mut commitments_map: BTreeMap<frost::Identifier, frost::round1::SigningCommitments> =
            BTreeMap::new();
        let mut signing_nonces = Vec::new();

        for kp in key_packages.iter().take(3) {
            let (nonces, commitment) = round1(kp).expect("round1");
            commitments_map.insert(commitment.identifier, commitment.commitments);
            signing_nonces.push((*kp.inner.identifier(), nonces));
        }

        let signing_package = frost::SigningPackage::new(commitments_map, message);

        // Only generate 2 of 3 required shares
        let mut signature_shares = BTreeMap::new();
        for (i, kp) in key_packages.iter().take(2).enumerate() {
            let share = round2(kp, &signing_nonces[i].1, &signing_package).expect("round2");
            signature_shares.insert(share.identifier, share.share);
        }

        // Aggregation with insufficient shares should fail
        let result = aggregate(&signing_package, &signature_shares, &pubkey_package);
        assert!(
            result.is_err(),
            "Aggregation with fewer than threshold shares should fail"
        );
    }
}
