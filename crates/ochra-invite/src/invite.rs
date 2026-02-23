//! Invite link creation and parsing (`ochra://invite` URLs).
//!
//! Provides helpers for encoding invite descriptors as URIs and decoding them
//! back.

use crate::{InviteDescriptor, InviteError, Result};

/// The URI scheme for Ochra invites.
const INVITE_SCHEME: &str = "ochra://invite/";

/// Encode an invite descriptor as an `ochra://invite/<base64>` URI.
pub fn encode_invite_uri(descriptor: &InviteDescriptor) -> String {
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        descriptor.secret,
    );
    format!("{INVITE_SCHEME}{encoded}")
}

/// Parse an `ochra://invite/<base64>` URI back into an `InviteDescriptor`.
pub fn decode_invite_uri(uri: &str) -> Result<InviteDescriptor> {
    let payload = uri
        .strip_prefix(INVITE_SCHEME)
        .ok_or_else(|| InviteError::InvalidUrl("missing ochra://invite/ prefix".to_string()))?;

    let bytes = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        payload,
    )
    .map_err(|e| InviteError::InvalidUrl(format!("base64 decode error: {e}")))?;

    let secret: [u8; 32] = bytes
        .try_into()
        .map_err(|_| InviteError::InvalidUrl("secret must be 32 bytes".to_string()))?;

    Ok(InviteDescriptor::from_secret(secret))
}
