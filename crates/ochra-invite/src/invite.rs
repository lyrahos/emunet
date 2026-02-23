//! Invite link creation and parsing for `ochra://invite` URLs.
//!
//! Invite links encode the information needed to join a Space or the network.
//! They contain the creator's identity, cryptographic keys, and usage policies.
//!
//! ## URL Format
//!
//! ```text
//! ochra://invite/<base64url-encoded-payload>
//! ```
//!
//! The payload is a JSON-serialized [`InviteLink`] struct, base64url-encoded
//! (without padding).
//!
//! ## Invite Policies
//!
//! Invites can be configured with different usage policies:
//! - [`InvitePolicy::SingleUse`] - The invite can be used exactly once
//! - [`InvitePolicy::MultiUse`] - The invite can be used up to N times
//! - [`InvitePolicy::Unlimited`] - The invite can be used without limit

use serde::{Deserialize, Serialize};

use crate::{InviteError, Result};

/// The URI scheme for Ochra invite links.
const INVITE_SCHEME: &str = "ochra://invite/";

/// An invite link containing all information needed to join a Space.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InviteLink {
    /// The group ID (Space ID) this invite is for.
    pub group_id: [u8; 32],
    /// The Space's display name.
    pub space_name: String,
    /// The creator's PIK public key (Ed25519, 32 bytes).
    pub creator_pik: [u8; 32],
    /// The creator's X25519 public key for key exchange.
    pub x25519_pk: [u8; 32],
    /// Ed25519 signature over the invite fields.
    pub signature: Vec<u8>,
    /// Time-to-live in seconds (0 = no expiration).
    pub ttl: u64,
    /// Maximum number of uses (0 = unlimited).
    pub max_uses: u32,
    /// Creation timestamp (Unix seconds).
    pub created_at: u64,
    /// The invite policy.
    pub policy: InvitePolicy,
}

/// Policy governing how many times an invite can be used.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvitePolicy {
    /// The invite can be used exactly once, then expires.
    SingleUse,
    /// The invite can be used up to the specified number of times.
    MultiUse(u32),
    /// The invite can be used without limit.
    Unlimited,
}

impl InvitePolicy {
    /// Return the maximum number of uses, or `None` for unlimited.
    pub fn max_uses(&self) -> Option<u32> {
        match self {
            InvitePolicy::SingleUse => Some(1),
            InvitePolicy::MultiUse(n) => Some(*n),
            InvitePolicy::Unlimited => None,
        }
    }

    /// Check whether the invite can still be used given the current use count.
    pub fn can_use(&self, current_uses: u32) -> bool {
        match self {
            InvitePolicy::SingleUse => current_uses < 1,
            InvitePolicy::MultiUse(n) => current_uses < *n,
            InvitePolicy::Unlimited => true,
        }
    }
}

/// Create an invite link URL.
///
/// Generates an `ochra://invite/<base64>` URL from the invite parameters,
/// signed by the creator's PIK.
///
/// # Arguments
///
/// * `signing_key` - The creator's Ed25519 signing key
/// * `group_id` - The Space's group ID
/// * `space_name` - The Space's display name
/// * `x25519_pk` - The creator's X25519 public key
/// * `policy` - The invite usage policy
/// * `ttl` - Time-to-live in seconds (0 = no expiration)
pub fn create_invite_link(
    signing_key: &ochra_crypto::ed25519::SigningKey,
    group_id: [u8; 32],
    space_name: &str,
    x25519_pk: [u8; 32],
    policy: InvitePolicy,
    ttl: u64,
) -> Result<String> {
    let creator_pik = signing_key.verifying_key().to_bytes();
    let max_uses = policy.max_uses().unwrap_or(0);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Build the data to sign.
    let signed_data = build_invite_signed_data(
        &group_id,
        space_name,
        &creator_pik,
        &x25519_pk,
        ttl,
        max_uses,
        now,
    );
    let signature = signing_key.sign(&signed_data);

    let invite = InviteLink {
        group_id,
        space_name: space_name.to_string(),
        creator_pik,
        x25519_pk,
        signature: signature.to_bytes().to_vec(),
        ttl,
        max_uses,
        created_at: now,
        policy,
    };

    let json =
        serde_json::to_vec(&invite).map_err(|e| InviteError::Serialization(e.to_string()))?;
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &json,
    );

    Ok(format!("{INVITE_SCHEME}{encoded}"))
}

/// Parse and validate an `ochra://invite/<base64>` URL.
///
/// Decodes the invite link and verifies the Ed25519 signature using the
/// creator's PIK embedded in the invite.
///
/// # Arguments
///
/// * `url` - The invite URL to parse
///
/// # Returns
///
/// The validated [`InviteLink`].
pub fn parse_invite_link(url: &str) -> Result<InviteLink> {
    let payload = url
        .strip_prefix(INVITE_SCHEME)
        .ok_or_else(|| InviteError::InvalidUrl("missing ochra://invite/ prefix".to_string()))?;

    let json = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        payload,
    )
    .map_err(|e| InviteError::InvalidUrl(format!("base64 decode error: {}", e)))?;

    let invite: InviteLink = serde_json::from_slice(&json)
        .map_err(|e| InviteError::Malformed(format!("invalid invite JSON: {}", e)))?;

    // Verify the signature.
    if invite.signature.len() != 64 {
        return Err(InviteError::InvalidSignature);
    }

    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&invite.signature);
    let signature = ochra_crypto::ed25519::Signature::from_bytes(&sig_bytes);

    let verifying_key = ochra_crypto::ed25519::VerifyingKey::from_bytes(&invite.creator_pik)
        .map_err(|_| InviteError::InvalidSignature)?;

    let signed_data = build_invite_signed_data(
        &invite.group_id,
        &invite.space_name,
        &invite.creator_pik,
        &invite.x25519_pk,
        invite.ttl,
        invite.max_uses,
        invite.created_at,
    );

    verifying_key
        .verify(&signed_data, &signature)
        .map_err(|_| InviteError::InvalidSignature)?;

    // Check TTL expiration.
    if invite.ttl > 0 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now > invite.created_at.saturating_add(invite.ttl) {
            return Err(InviteError::TtlExpired);
        }
    }

    Ok(invite)
}

/// Build the byte string signed for an invite link.
///
/// Format: `group_id || LE32(space_name.len()) || space_name || creator_pik ||
///          x25519_pk || LE64(ttl) || LE32(max_uses) || LE64(created_at)`
fn build_invite_signed_data(
    group_id: &[u8; 32],
    space_name: &str,
    creator_pik: &[u8; 32],
    x25519_pk: &[u8; 32],
    ttl: u64,
    max_uses: u32,
    created_at: u64,
) -> Vec<u8> {
    let name_bytes = space_name.as_bytes();
    let mut data = Vec::with_capacity(32 + 4 + name_bytes.len() + 32 + 32 + 8 + 4 + 8);
    data.extend_from_slice(group_id);
    data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(name_bytes);
    data.extend_from_slice(creator_pik);
    data.extend_from_slice(x25519_pk);
    data.extend_from_slice(&ttl.to_le_bytes());
    data.extend_from_slice(&max_uses.to_le_bytes());
    data.extend_from_slice(&created_at.to_le_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochra_crypto::ed25519::KeyPair;
    use ochra_crypto::x25519::X25519StaticSecret;

    fn create_test_invite_url() -> (String, KeyPair) {
        let kp = KeyPair::generate();
        let x_secret = X25519StaticSecret::random();
        let x_pk = x_secret.public_key();

        let url = create_invite_link(
            &kp.signing_key,
            [0x42u8; 32],
            "Test Space",
            x_pk.to_bytes(),
            InvitePolicy::MultiUse(10),
            3600,
        )
        .expect("create invite link");

        (url, kp)
    }

    #[test]
    fn test_create_invite_link() {
        let (url, _kp) = create_test_invite_url();
        assert!(url.starts_with(INVITE_SCHEME));
    }

    #[test]
    fn test_parse_invite_link_roundtrip() {
        let (url, _kp) = create_test_invite_url();
        let invite = parse_invite_link(&url).expect("parse invite");
        assert_eq!(invite.group_id, [0x42u8; 32]);
        assert_eq!(invite.space_name, "Test Space");
        assert_eq!(invite.policy, InvitePolicy::MultiUse(10));
        assert_eq!(invite.ttl, 3600);
    }

    #[test]
    fn test_parse_invalid_prefix() {
        let result = parse_invite_link("https://example.com/invite/abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_base64() {
        let result = parse_invite_link("ochra://invite/not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tampered_signature() {
        let (url, _kp) = create_test_invite_url();
        let invite = parse_invite_link(&url).expect("parse");

        // Tamper with the invite.
        let mut tampered = invite;
        tampered.space_name = "Tampered Space".to_string();

        let json = serde_json::to_vec(&tampered).expect("serialize");
        let encoded = base64::Engine::encode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            &json,
        );
        let tampered_url = format!("{INVITE_SCHEME}{encoded}");

        let result = parse_invite_link(&tampered_url);
        assert!(result.is_err());
    }

    #[test]
    fn test_invite_policy_single_use() {
        let policy = InvitePolicy::SingleUse;
        assert_eq!(policy.max_uses(), Some(1));
        assert!(policy.can_use(0));
        assert!(!policy.can_use(1));
    }

    #[test]
    fn test_invite_policy_multi_use() {
        let policy = InvitePolicy::MultiUse(5);
        assert_eq!(policy.max_uses(), Some(5));
        assert!(policy.can_use(0));
        assert!(policy.can_use(4));
        assert!(!policy.can_use(5));
    }

    #[test]
    fn test_invite_policy_unlimited() {
        let policy = InvitePolicy::Unlimited;
        assert_eq!(policy.max_uses(), None);
        assert!(policy.can_use(0));
        assert!(policy.can_use(1_000_000));
    }

    #[test]
    fn test_invite_link_creator_pik() {
        let (url, kp) = create_test_invite_url();
        let invite = parse_invite_link(&url).expect("parse");
        assert_eq!(invite.creator_pik, kp.verifying_key.to_bytes());
    }

    #[test]
    fn test_invite_link_unlimited_ttl() {
        let kp = KeyPair::generate();
        let x_secret = X25519StaticSecret::random();
        let x_pk = x_secret.public_key();

        let url = create_invite_link(
            &kp.signing_key,
            [0x01u8; 32],
            "Eternal Space",
            x_pk.to_bytes(),
            InvitePolicy::Unlimited,
            0, // No TTL
        )
        .expect("create");

        let invite = parse_invite_link(&url).expect("parse");
        assert_eq!(invite.ttl, 0);
        assert_eq!(invite.policy, InvitePolicy::Unlimited);
    }
}
