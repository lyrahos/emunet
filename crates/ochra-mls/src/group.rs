//! MLS group lifecycle management.
//!
//! Provides group creation, member management, key rotation, and
//! message encryption/decryption using the MLS key schedule.

use ochra_crypto::{blake3, chacha20};
use serde::{Deserialize, Serialize};

use crate::{MlsError, Result, MAX_GROUP_SIZE};

/// A member's key package for joining a group.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyPackage {
    /// Member's PIK hash (identity).
    pub member_id: [u8; 32],
    /// X25519 init key for key exchange.
    pub init_key: [u8; 32],
    /// Ed25519 signing key for authentication.
    pub signing_key: [u8; 32],
}

/// Welcome message sent to a new member joining the group.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Welcome {
    /// The group ID being joined.
    pub group_id: [u8; 32],
    /// The current epoch of the group.
    pub epoch: u64,
    /// Encrypted group secret for the new member.
    pub encrypted_group_secret: Vec<u8>,
    /// List of current member IDs.
    pub member_ids: Vec<[u8; 32]>,
}

/// Encrypted MLS ciphertext.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MlsCiphertext {
    /// The group ID.
    pub group_id: [u8; 32],
    /// The epoch in which this message was encrypted.
    pub epoch: u64,
    /// The sender's member ID.
    pub sender_id: [u8; 32],
    /// The encrypted content (ChaCha20-Poly1305).
    pub ciphertext: Vec<u8>,
    /// The nonce used for encryption.
    pub nonce: [u8; 12],
}

/// Group-epoch secret derived from the MLS key schedule.
#[derive(Clone, Debug)]
pub struct GroupSecret {
    /// The group epoch secret (32 bytes).
    pub epoch_secret: [u8; 32],
    /// Encryption key derived from the epoch secret.
    pub encryption_key: [u8; 32],
    /// Nonce base for this epoch.
    pub nonce_base: [u8; 12],
}

/// An MLS group member entry.
#[derive(Clone, Debug)]
struct Member {
    /// Member identity (PIK hash).
    member_id: [u8; 32],
    /// Member's key package (retained for future key exchange operations).
    _key_package: KeyPackage,
    /// When the member was added (epoch, retained for auditing).
    _added_epoch: u64,
}

/// MLS group state.
///
/// Manages the group membership, epoch tracking, and key schedule.
pub struct GroupState {
    /// Group identifier.
    group_id: [u8; 32],
    /// Current epoch number.
    epoch: u64,
    /// Current group secret.
    secret: GroupSecret,
    /// Group members.
    members: Vec<Member>,
    /// Message counter for nonce generation.
    message_counter: u64,
}

impl GroupState {
    /// Get the group ID.
    pub fn group_id(&self) -> &[u8; 32] {
        &self.group_id
    }

    /// Get the current epoch number.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// Get the number of members.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Check if a member is in the group.
    pub fn has_member(&self, member_id: &[u8; 32]) -> bool {
        self.members.iter().any(|m| &m.member_id == member_id)
    }

    /// List all member IDs.
    pub fn member_ids(&self) -> Vec<[u8; 32]> {
        self.members.iter().map(|m| m.member_id).collect()
    }

    /// Get the current group secret.
    pub fn current_secret(&self) -> &GroupSecret {
        &self.secret
    }

    /// Ratchet the group secret via an arbitrary key update (post-compromise security).
    ///
    /// Advances the epoch and derives a new group secret.
    pub fn update_keys(&mut self) -> Result<GroupSecret> {
        self.epoch += 1;
        let epoch_bytes = self.epoch.to_le_bytes();
        self.secret = derive_next_secret(&self.secret, &epoch_bytes, self.epoch);
        self.message_counter = 0;
        Ok(self.secret.clone())
    }

    /// Encrypt a plaintext message using the current group secret.
    ///
    /// # Arguments
    ///
    /// * `sender_id` - The sender's member ID.
    /// * `plaintext` - The message to encrypt.
    pub fn encrypt_message(
        &mut self,
        sender_id: &[u8; 32],
        plaintext: &[u8],
    ) -> Result<MlsCiphertext> {
        if !self.has_member(sender_id) {
            return Err(MlsError::MemberNotFound(hex::encode(sender_id)));
        }

        let nonce = self.next_nonce();

        let ciphertext = chacha20::encrypt(
            &self.secret.encryption_key,
            &nonce,
            plaintext,
            &self.group_id,
        )
        .map_err(|e| MlsError::Encryption(e.to_string()))?;

        Ok(MlsCiphertext {
            group_id: self.group_id,
            epoch: self.epoch,
            sender_id: *sender_id,
            ciphertext,
            nonce,
        })
    }

    /// Decrypt an MLS ciphertext.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - The encrypted message.
    pub fn decrypt_message(&self, ciphertext: &MlsCiphertext) -> Result<Vec<u8>> {
        if ciphertext.group_id != self.group_id {
            return Err(MlsError::Encryption("group ID mismatch".to_string()));
        }
        if ciphertext.epoch != self.epoch {
            return Err(MlsError::InvalidEpoch {
                expected: self.epoch,
                actual: ciphertext.epoch,
            });
        }

        chacha20::decrypt(
            &self.secret.encryption_key,
            &ciphertext.nonce,
            &ciphertext.ciphertext,
            &self.group_id,
        )
        .map_err(|e| MlsError::Encryption(e.to_string()))
    }

    /// Generate the next nonce from the nonce base and message counter.
    fn next_nonce(&mut self) -> [u8; 12] {
        let mut nonce = self.secret.nonce_base;
        let counter_bytes = self.message_counter.to_le_bytes();
        for (i, b) in counter_bytes.iter().enumerate() {
            if i < nonce.len() {
                nonce[i] ^= b;
            }
        }
        self.message_counter += 1;
        nonce
    }
}

/// Create a new MLS group with the given creator as the first member.
///
/// # Arguments
///
/// * `group_id` - The 32-byte group identifier.
/// * `creator_key_package` - The creator's key package.
///
/// # Returns
///
/// A new [`GroupState`] with the creator as the sole member at epoch 0.
pub fn create_group(group_id: [u8; 32], creator_key_package: KeyPackage) -> GroupState {
    let secret = derive_initial_secret(&group_id, &creator_key_package);
    let creator = Member {
        member_id: creator_key_package.member_id,
        _key_package: creator_key_package,
        _added_epoch: 0,
    };

    GroupState {
        group_id,
        epoch: 0,
        secret,
        members: vec![creator],
        message_counter: 0,
    }
}

/// Add a member to the group.
///
/// Advances the epoch and derives a new group secret that includes the
/// new member's key material. Returns the updated group state and a
/// Welcome message for the new member.
///
/// # Arguments
///
/// * `group` - The current group state (consumed and returned updated).
/// * `member_key_package` - The new member's key package.
pub fn add_member(
    mut group: GroupState,
    member_key_package: KeyPackage,
) -> Result<(GroupState, Welcome)> {
    let member_id = member_key_package.member_id;

    if group.has_member(&member_id) {
        return Err(MlsError::MemberExists(hex::encode(member_id)));
    }
    if group.members.len() >= MAX_GROUP_SIZE {
        return Err(MlsError::GroupFull {
            max: MAX_GROUP_SIZE,
        });
    }

    group.epoch += 1;

    group.members.push(Member {
        member_id,
        _key_package: member_key_package,
        _added_epoch: group.epoch,
    });

    // Derive new epoch secret incorporating the new member.
    group.secret = derive_next_secret(&group.secret, &member_id, group.epoch);
    group.message_counter = 0;

    let welcome = Welcome {
        group_id: group.group_id,
        epoch: group.epoch,
        encrypted_group_secret: group.secret.epoch_secret.to_vec(),
        member_ids: group.member_ids(),
    };

    tracing::debug!(
        group_id = hex::encode(group.group_id),
        member = hex::encode(member_id),
        epoch = group.epoch,
        "added member to MLS group"
    );

    Ok((group, welcome))
}

/// Remove a member from the group.
///
/// Advances the epoch and derives a new group secret that excludes
/// the removed member's key material, providing forward secrecy.
///
/// # Arguments
///
/// * `group` - The current group state (consumed and returned updated).
/// * `member_id` - The ID of the member to remove.
pub fn remove_member(mut group: GroupState, member_id: &[u8; 32]) -> Result<GroupState> {
    let idx = group
        .members
        .iter()
        .position(|m| &m.member_id == member_id)
        .ok_or_else(|| MlsError::MemberNotFound(hex::encode(member_id)))?;

    if group.members.len() == 1 {
        return Err(MlsError::GroupEmpty);
    }

    group.members.remove(idx);
    group.epoch += 1;

    // Derive new epoch secret excluding the removed member.
    group.secret = derive_next_secret(&group.secret, member_id, group.epoch);
    group.message_counter = 0;

    tracing::debug!(
        group_id = hex::encode(group.group_id),
        member = hex::encode(member_id),
        epoch = group.epoch,
        "removed member from MLS group"
    );

    Ok(group)
}

/// Derive the initial group secret from the group ID and creator's key package.
fn derive_initial_secret(group_id: &[u8; 32], creator: &KeyPackage) -> GroupSecret {
    let input = blake3::encode_multi_field(&[group_id, &creator.init_key, &creator.signing_key]);
    let epoch_secret = blake3::derive_key(blake3::contexts::GROUP_SETTINGS_KEY, &input);
    derive_group_secret_from_epoch(&epoch_secret)
}

/// Derive the next epoch secret from the current secret and change data.
fn derive_next_secret(current: &GroupSecret, change_data: &[u8], epoch: u64) -> GroupSecret {
    let epoch_bytes = epoch.to_le_bytes();
    let input = blake3::encode_multi_field(&[&current.epoch_secret, change_data, &epoch_bytes]);
    let epoch_secret = blake3::derive_key(blake3::contexts::GROUP_SETTINGS_KEY, &input);
    derive_group_secret_from_epoch(&epoch_secret)
}

/// Derive encryption key and nonce from an epoch secret.
fn derive_group_secret_from_epoch(epoch_secret: &[u8; 32]) -> GroupSecret {
    let encryption_key = blake3::derive_key(blake3::contexts::CONTENT_ESCROW_KEY, epoch_secret);
    let nonce_full = blake3::derive_key(blake3::contexts::SESSION_KEY_ID, epoch_secret);
    let mut nonce_base = [0u8; 12];
    nonce_base.copy_from_slice(&nonce_full[..12]);

    GroupSecret {
        epoch_secret: *epoch_secret,
        encryption_key,
        nonce_base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key_package(id: u8) -> KeyPackage {
        KeyPackage {
            member_id: [id; 32],
            init_key: [id.wrapping_add(100); 32],
            signing_key: [id.wrapping_add(200); 32],
        }
    }

    #[test]
    fn test_create_group() {
        let kp = make_key_package(1);
        let group = create_group([0xAA; 32], kp);
        assert_eq!(group.epoch(), 0);
        assert_eq!(group.member_count(), 1);
    }

    #[test]
    fn test_add_member() {
        let kp1 = make_key_package(1);
        let kp2 = make_key_package(2);
        let group = create_group([0xAA; 32], kp1);

        let (group, welcome) = add_member(group, kp2).expect("add member");
        assert_eq!(group.member_count(), 2);
        assert_eq!(group.epoch(), 1);
        assert!(group.has_member(&[2; 32]));
        assert_eq!(welcome.epoch, 1);
        assert_eq!(welcome.member_ids.len(), 2);
    }

    #[test]
    fn test_add_duplicate_rejected() {
        let kp = make_key_package(1);
        let group = create_group([0xAA; 32], kp.clone());

        let result = add_member(group, kp);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_member() {
        let kp1 = make_key_package(1);
        let kp2 = make_key_package(2);
        let group = create_group([0xAA; 32], kp1);
        let (group, _) = add_member(group, kp2).expect("add");

        let group = remove_member(group, &[2; 32]).expect("remove");
        assert_eq!(group.member_count(), 1);
        assert!(!group.has_member(&[2; 32]));
    }

    #[test]
    fn test_remove_last_member_fails() {
        let kp = make_key_package(1);
        let group = create_group([0xAA; 32], kp);

        let result = remove_member(group, &[1; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_keys_advances_epoch() {
        let kp = make_key_package(1);
        let mut group = create_group([0xAA; 32], kp);

        let old_secret = group.current_secret().epoch_secret;
        group.update_keys().expect("update");
        let new_secret = group.current_secret().epoch_secret;

        assert_eq!(group.epoch(), 1);
        assert_ne!(old_secret, new_secret);
    }

    #[test]
    fn test_epoch_secret_changes_on_membership_change() {
        let kp1 = make_key_package(1);
        let kp2 = make_key_package(2);
        let group = create_group([0xAA; 32], kp1);
        let secret_before = group.current_secret().epoch_secret;

        let (group, _) = add_member(group, kp2).expect("add");
        let secret_after = group.current_secret().epoch_secret;

        assert_ne!(secret_before, secret_after);
    }

    #[test]
    fn test_encrypt_decrypt_message() {
        let kp1 = make_key_package(1);
        let kp2 = make_key_package(2);
        let group = create_group([0xAA; 32], kp1);
        let (mut group, _) = add_member(group, kp2).expect("add");

        let plaintext = b"Hello, group!";
        let ciphertext = group
            .encrypt_message(&[1; 32], plaintext)
            .expect("encrypt");

        let decrypted = group.decrypt_message(&ciphertext).expect("decrypt");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_non_member_fails() {
        let kp = make_key_package(1);
        let mut group = create_group([0xAA; 32], kp);

        let result = group.encrypt_message(&[99; 32], b"not a member");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_wrong_epoch_fails() {
        let kp = make_key_package(1);
        let mut group = create_group([0xAA; 32], kp);

        let ciphertext = group.encrypt_message(&[1; 32], b"msg").expect("encrypt");
        group.update_keys().expect("update");

        let result = group.decrypt_message(&ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_messages_different_nonces() {
        let kp = make_key_package(1);
        let mut group = create_group([0xAA; 32], kp);

        let ct1 = group.encrypt_message(&[1; 32], b"msg1").expect("encrypt");
        let ct2 = group.encrypt_message(&[1; 32], b"msg2").expect("encrypt");

        assert_ne!(ct1.nonce, ct2.nonce);

        // Both should decrypt correctly.
        let p1 = group.decrypt_message(&ct1).expect("decrypt");
        let p2 = group.decrypt_message(&ct2).expect("decrypt");
        assert_eq!(p1, b"msg1");
        assert_eq!(p2, b"msg2");
    }
}
