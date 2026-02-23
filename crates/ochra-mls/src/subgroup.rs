//! Subgroup/Channel management within an MLS parent group.
//!
//! Subgroups allow partitioning a large MLS group into smaller channels,
//! each with its own membership and key schedule. Subgroup members must
//! be a subset of the parent group's members.

use ochra_crypto::blake3;
use serde::{Deserialize, Serialize};

use crate::{MlsError, Result, MAX_GROUP_SIZE};

/// A subgroup (channel) within a parent MLS group.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Subgroup {
    /// Unique identifier for this subgroup.
    pub subgroup_id: [u8; 32],
    /// The parent group's identifier.
    pub parent_group_id: [u8; 32],
    /// Member IDs in this subgroup (must be a subset of the parent group).
    pub members: Vec<[u8; 32]>,
    /// Current epoch for this subgroup's key schedule.
    pub epoch: u64,
    /// Subgroup-specific epoch secret.
    epoch_secret: [u8; 32],
}

/// Create a new subgroup within a parent group.
///
/// The subgroup derives its own key schedule from the parent group ID
/// and subgroup ID, providing key separation between channels.
///
/// # Arguments
///
/// * `parent_group_id` - The parent group's 32-byte identifier.
/// * `subgroup_id` - The 32-byte subgroup identifier.
/// * `creator_id` - The member ID of the subgroup creator.
pub fn create_subgroup(
    parent_group_id: [u8; 32],
    subgroup_id: [u8; 32],
    creator_id: [u8; 32],
) -> Subgroup {
    let input = blake3::encode_multi_field(&[&parent_group_id, &subgroup_id, &creator_id]);
    let epoch_secret = blake3::derive_key(blake3::contexts::GROUP_SETTINGS_KEY, &input);

    Subgroup {
        subgroup_id,
        parent_group_id,
        members: vec![creator_id],
        epoch: 0,
        epoch_secret,
    }
}

/// Add a member to a subgroup.
///
/// The member must not already be in the subgroup. Caller is responsible
/// for verifying the member is in the parent group.
///
/// # Arguments
///
/// * `subgroup` - The subgroup to modify.
/// * `member_id` - The member ID to add.
pub fn add_member(subgroup: &mut Subgroup, member_id: [u8; 32]) -> Result<()> {
    if subgroup.members.iter().any(|m| m == &member_id) {
        return Err(MlsError::MemberExists(hex::encode(member_id)));
    }
    if subgroup.members.len() >= MAX_GROUP_SIZE {
        return Err(MlsError::GroupFull {
            max: MAX_GROUP_SIZE,
        });
    }

    subgroup.members.push(member_id);
    subgroup.epoch += 1;

    // Derive new epoch secret.
    let epoch_bytes = subgroup.epoch.to_le_bytes();
    let input = blake3::encode_multi_field(&[&subgroup.epoch_secret, &member_id, &epoch_bytes]);
    subgroup.epoch_secret = blake3::derive_key(blake3::contexts::GROUP_SETTINGS_KEY, &input);

    tracing::debug!(
        subgroup_id = hex::encode(subgroup.subgroup_id),
        member = hex::encode(member_id),
        epoch = subgroup.epoch,
        "added member to subgroup"
    );

    Ok(())
}

/// Remove a member from a subgroup.
///
/// # Arguments
///
/// * `subgroup` - The subgroup to modify.
/// * `member_id` - The member ID to remove.
pub fn remove_member(subgroup: &mut Subgroup, member_id: &[u8; 32]) -> Result<()> {
    let idx = subgroup
        .members
        .iter()
        .position(|m| m == member_id)
        .ok_or_else(|| MlsError::MemberNotFound(hex::encode(member_id)))?;

    if subgroup.members.len() == 1 {
        return Err(MlsError::GroupEmpty);
    }

    subgroup.members.remove(idx);
    subgroup.epoch += 1;

    // Derive new epoch secret excluding the removed member.
    let epoch_bytes = subgroup.epoch.to_le_bytes();
    let input = blake3::encode_multi_field(&[&subgroup.epoch_secret, member_id, &epoch_bytes]);
    subgroup.epoch_secret = blake3::derive_key(blake3::contexts::GROUP_SETTINGS_KEY, &input);

    tracing::debug!(
        subgroup_id = hex::encode(subgroup.subgroup_id),
        member = hex::encode(member_id),
        epoch = subgroup.epoch,
        "removed member from subgroup"
    );

    Ok(())
}

impl Subgroup {
    /// Check if a member is in this subgroup.
    pub fn has_member(&self, member_id: &[u8; 32]) -> bool {
        self.members.iter().any(|m| m == member_id)
    }

    /// Get the number of members.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get the current epoch secret (for key derivation).
    pub fn epoch_secret(&self) -> &[u8; 32] {
        &self.epoch_secret
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_subgroup() {
        let parent_id = [0xAA; 32];
        let subgroup_id = [0xBB; 32];
        let creator_id = [0x01; 32];

        let sg = create_subgroup(parent_id, subgroup_id, creator_id);

        assert_eq!(sg.subgroup_id, subgroup_id);
        assert_eq!(sg.parent_group_id, parent_id);
        assert_eq!(sg.members.len(), 1);
        assert_eq!(sg.members[0], creator_id);
        assert_eq!(sg.epoch, 0);
    }

    #[test]
    fn test_add_member_to_subgroup() {
        let mut sg = create_subgroup([0xAA; 32], [0xBB; 32], [0x01; 32]);
        add_member(&mut sg, [0x02; 32]).expect("add");

        assert_eq!(sg.member_count(), 2);
        assert!(sg.has_member(&[0x02; 32]));
        assert_eq!(sg.epoch, 1);
    }

    #[test]
    fn test_add_duplicate_fails() {
        let mut sg = create_subgroup([0xAA; 32], [0xBB; 32], [0x01; 32]);
        let result = add_member(&mut sg, [0x01; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_member_from_subgroup() {
        let mut sg = create_subgroup([0xAA; 32], [0xBB; 32], [0x01; 32]);
        add_member(&mut sg, [0x02; 32]).expect("add");

        remove_member(&mut sg, &[0x02; 32]).expect("remove");
        assert_eq!(sg.member_count(), 1);
        assert!(!sg.has_member(&[0x02; 32]));
    }

    #[test]
    fn test_remove_nonexistent_fails() {
        let mut sg = create_subgroup([0xAA; 32], [0xBB; 32], [0x01; 32]);
        let result = remove_member(&mut sg, &[0xFF; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_last_member_fails() {
        let mut sg = create_subgroup([0xAA; 32], [0xBB; 32], [0x01; 32]);
        let result = remove_member(&mut sg, &[0x01; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_epoch_secret_changes_on_membership() {
        let mut sg = create_subgroup([0xAA; 32], [0xBB; 32], [0x01; 32]);
        let secret_before = *sg.epoch_secret();

        add_member(&mut sg, [0x02; 32]).expect("add");
        let secret_after = *sg.epoch_secret();

        assert_ne!(secret_before, secret_after);
    }

    #[test]
    fn test_different_subgroups_different_secrets() {
        let sg1 = create_subgroup([0xAA; 32], [0xBB; 32], [0x01; 32]);
        let sg2 = create_subgroup([0xAA; 32], [0xCC; 32], [0x01; 32]);

        assert_ne!(sg1.epoch_secret(), sg2.epoch_secret());
    }

    #[test]
    fn test_subgroup_serde_roundtrip() {
        let sg = create_subgroup([0xAA; 32], [0xBB; 32], [0x01; 32]);
        let json = serde_json::to_string(&sg).expect("serialize");
        let restored: Subgroup = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(sg.subgroup_id, restored.subgroup_id);
        assert_eq!(sg.parent_group_id, restored.parent_group_id);
        assert_eq!(sg.members, restored.members);
        assert_eq!(sg.epoch, restored.epoch);
    }
}
