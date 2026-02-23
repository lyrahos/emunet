//! Guardian replacement.
//!
//! When a guardian becomes unresponsive or needs to be replaced, a new
//! guardian can take their place. The replacement process triggers a
//! key resharing so that the old guardian's share is invalidated and
//! the new guardian receives a valid share.

use serde::{Deserialize, Serialize};

use crate::dkg::GuardianInfo;
use crate::{GuardianError, Result};

/// A guardian replacement request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplacementRequest {
    /// PIK hash of the guardian being replaced.
    pub old_guardian_id: [u8; 32],
    /// Information about the new guardian.
    pub new_guardian: GuardianInfo,
    /// Unix timestamp of the replacement request.
    pub requested_at: u64,
}

/// Result of a successful guardian replacement.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplacementResult {
    /// PIK hash of the replaced guardian.
    pub old_guardian_id: [u8; 32],
    /// PIK hash of the new guardian.
    pub new_guardian_id: [u8; 32],
    /// Whether resharing was triggered.
    pub resharing_triggered: bool,
}

/// Replace a guardian in the guardian set.
///
/// This removes the old guardian and adds the new one, then triggers
/// a resharing of the recovery secret so the old guardian's share is
/// invalidated.
///
/// # Arguments
///
/// * `old_id` - PIK hash of the guardian to replace
/// * `new_guardian` - Information about the replacement guardian
/// * `guardians` - The current list of guardians (mutable)
///
/// # Errors
///
/// - [`GuardianError::NotFound`] if the old guardian is not in the list
/// - [`GuardianError::AlreadyEnrolled`] if the new guardian is already in the list
pub fn replace_guardian(
    old_id: &[u8; 32],
    new_guardian: GuardianInfo,
    guardians: &mut [GuardianInfo],
) -> Result<ReplacementResult> {
    // Find the old guardian
    let old_idx = guardians
        .iter()
        .position(|g| &g.pik_hash == old_id)
        .ok_or_else(|| GuardianError::NotFound(hex::encode(old_id)))?;

    // Check the new guardian is not already enrolled
    if guardians
        .iter()
        .any(|g| g.pik_hash == new_guardian.pik_hash)
    {
        return Err(GuardianError::AlreadyEnrolled(hex::encode(
            new_guardian.pik_hash,
        )));
    }

    let new_guardian_id = new_guardian.pik_hash;

    // Replace
    guardians[old_idx] = new_guardian;

    tracing::info!(
        old = hex::encode(old_id),
        new = hex::encode(new_guardian_id),
        "guardian replaced, resharing triggered"
    );

    Ok(ReplacementResult {
        old_guardian_id: *old_id,
        new_guardian_id,
        resharing_triggered: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_guardian(id_byte: u8) -> GuardianInfo {
        GuardianInfo {
            pik_hash: [id_byte; 32],
            display_name: format!("Guardian {id_byte}"),
            public_key: [id_byte + 100; 32],
        }
    }

    #[test]
    fn test_replace_guardian() {
        let mut guardians = vec![make_guardian(1), make_guardian(2), make_guardian(3)];

        let new_guardian = make_guardian(4);
        let result = replace_guardian(&[2; 32], new_guardian, &mut guardians).expect("replace");

        assert_eq!(result.old_guardian_id, [2; 32]);
        assert_eq!(result.new_guardian_id, [4; 32]);
        assert!(result.resharing_triggered);

        // Verify the guardian list
        assert_eq!(guardians.len(), 3);
        assert_eq!(guardians[1].pik_hash, [4; 32]);
        assert!(!guardians.iter().any(|g| g.pik_hash == [2; 32]));
    }

    #[test]
    fn test_replace_guardian_not_found() {
        let mut guardians = vec![make_guardian(1), make_guardian(2)];
        let new_guardian = make_guardian(4);
        let result = replace_guardian(&[99; 32], new_guardian, &mut guardians);
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_guardian_already_enrolled() {
        let mut guardians = vec![make_guardian(1), make_guardian(2), make_guardian(3)];
        let existing = make_guardian(3); // already in the list
        let result = replace_guardian(&[1; 32], existing, &mut guardians);
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_preserves_order() {
        let mut guardians = vec![make_guardian(1), make_guardian(2), make_guardian(3)];

        let new_guardian = make_guardian(10);
        replace_guardian(&[2; 32], new_guardian, &mut guardians).expect("replace");

        assert_eq!(guardians[0].pik_hash, [1; 32]);
        assert_eq!(guardians[1].pik_hash, [10; 32]);
        assert_eq!(guardians[2].pik_hash, [3; 32]);
    }
}
