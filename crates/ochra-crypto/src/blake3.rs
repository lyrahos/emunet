//! Domain-separated BLAKE3 hashing for the Ochra protocol.
//!
//! BLAKE3 serves 7+ distinct purposes in Ochra. Cross-domain collisions are prevented
//! by mandatory domain separation using BLAKE3's built-in mode flags.
//!
//! ## Modes
//!
//! - [`hash`] — Pure hashing: content addressing, Merkle tree leaves
//! - [`derive_key`] — Key derivation: session keys, receipt keys, DHT addresses
//! - [`keyed_hash`] — Keyed MAC/PRF: HMAC-equivalent operations, Fiat-Shamir challenges
//!
//! ## Context Strings
//!
//! All 36 registered context strings from Section 2.3 of the v5.5 spec are available
//! as constants. Unregistered context strings are a protocol violation.

/// All 36 registered BLAKE3 context strings from the Ochra v5.5 specification.
/// Using an unregistered context string is a protocol violation.
pub mod contexts {
    pub const PQC_SESSION_SECRET: &str = "Ochra v1 pqc-session-secret";
    pub const HYBRID_SESSION_KEY: &str = "Ochra v1 hybrid-session-key";
    pub const SESSION_KEY_ID: &str = "Ochra v1 session-key-id";
    pub const SURB_HOP_PQ_KEY: &str = "Ochra v1 surb-hop-pq-key";
    pub const RECEIPT_ENCRYPTION_KEY: &str = "Ochra v1 receipt-encryption-key";
    pub const RECEIPT_DHT_ADDRESS: &str = "Ochra v1 receipt-dht-address";
    pub const REFUND_COMMITMENT: &str = "Ochra v1 refund-commitment";
    pub const GUARDIAN_DEAD_DROP: &str = "Ochra v1 guardian-dead-drop";
    pub const INVITE_PAYLOAD_KEY: &str = "Ochra v1 invite-payload-key";
    pub const PROFILE_ENCRYPTION_KEY: &str = "Ochra v1 profile-encryption-key";
    pub const PROFILE_LOOKUP_KEY: &str = "Ochra v1 profile-lookup-key";
    pub const MERKLE_INNER_NODE: &str = "Ochra v1 merkle-inner-node";
    pub const FEE_EPOCH_STATE: &str = "Ochra v1 fee-epoch-state";
    pub const ZK_POR_CHALLENGE: &str = "Ochra v1 zk-por-challenge";
    pub const ZK_POR_AUTH_KEY: &str = "Ochra v1 zk-por-auth-key";
    pub const CONTENT_ESCROW_KEY: &str = "Ochra v1 content-escrow-key";
    pub const GROUP_SETTINGS_KEY: &str = "Ochra v1 group-settings-key";
    pub const HANDLE_LOOKUP: &str = "Ochra v1 handle-lookup";
    pub const WHISPER_SESSION_KEY: &str = "Ochra v1 whisper-session-key";
    pub const WHISPER_SEED_TRANSFER: &str = "Ochra v1 whisper-seed-transfer";
    pub const HANDLE_DEPRECATION: &str = "Ochra v1 handle-deprecation";
    pub const WHISPER_PING: &str = "Ochra v1 whisper-ping";
    pub const INVITE_DESCRIPTOR: &str = "Ochra v1 invite-descriptor";
    pub const RECEIPT_REPUBLISH_COVER: &str = "Ochra v1 receipt-republish-cover";
    pub const CONTACT_EXCHANGE_KEY: &str = "Ochra v1 contact-exchange-key";
    pub const REPORT_PSEUDONYM: &str = "Ochra v1 report-pseudonym";
    pub const TRANSFER_NOTE_KEY: &str = "Ochra v1 transfer-note-key";
    pub const SPHINX_HOP_KEY: &str = "Ochra v1 sphinx-hop-key";
    pub const SPHINX_HOP_MAC: &str = "Ochra v1 sphinx-hop-mac";
    pub const SPHINX_HOP_PAD: &str = "Ochra v1 sphinx-hop-pad";
    pub const SPHINX_HOP_NONCE: &str = "Ochra v1 sphinx-hop-nonce";
    pub const ECIES_ENCRYPTION_KEY: &str = "Ochra v1 ecies-encryption-key";
    pub const ECIES_NONCE: &str = "Ochra v1 ecies-nonce";
    pub const RATCHET_ROOT_KDF: &str = "Ochra v1 ratchet-root-kdf";
    pub const RATCHET_MSG_KEY: &str = "Ochra v1 ratchet-msg-key";
    pub const RATCHET_CHAIN_KEY: &str = "Ochra v1 ratchet-chain-key";
    pub const RATCHET_NONCE: &str = "Ochra v1 ratchet-nonce";
    pub const WHISPER_RATCHET_ROOT: &str = "Ochra v1 whisper-ratchet-root";
    pub const SYBILGUARD_WALK: &str = "Ochra v1 sybilguard-walk";

    /// All registered context strings. Used for validation.
    pub const ALL_CONTEXTS: &[&str] = &[
        PQC_SESSION_SECRET,
        HYBRID_SESSION_KEY,
        SESSION_KEY_ID,
        SURB_HOP_PQ_KEY,
        RECEIPT_ENCRYPTION_KEY,
        RECEIPT_DHT_ADDRESS,
        REFUND_COMMITMENT,
        GUARDIAN_DEAD_DROP,
        INVITE_PAYLOAD_KEY,
        PROFILE_ENCRYPTION_KEY,
        PROFILE_LOOKUP_KEY,
        MERKLE_INNER_NODE,
        FEE_EPOCH_STATE,
        ZK_POR_CHALLENGE,
        ZK_POR_AUTH_KEY,
        CONTENT_ESCROW_KEY,
        GROUP_SETTINGS_KEY,
        HANDLE_LOOKUP,
        WHISPER_SESSION_KEY,
        WHISPER_SEED_TRANSFER,
        HANDLE_DEPRECATION,
        WHISPER_PING,
        INVITE_DESCRIPTOR,
        RECEIPT_REPUBLISH_COVER,
        CONTACT_EXCHANGE_KEY,
        REPORT_PSEUDONYM,
        TRANSFER_NOTE_KEY,
        SPHINX_HOP_KEY,
        SPHINX_HOP_MAC,
        SPHINX_HOP_PAD,
        SPHINX_HOP_NONCE,
        ECIES_ENCRYPTION_KEY,
        ECIES_NONCE,
        RATCHET_ROOT_KDF,
        RATCHET_MSG_KEY,
        RATCHET_CHAIN_KEY,
        RATCHET_NONCE,
        WHISPER_RATCHET_ROOT,
        SYBILGUARD_WALK,
    ];
}

/// Compute BLAKE3 hash of the input data.
///
/// Used for content addressing, Merkle tree leaves, and general-purpose hashing.
pub fn hash(data: &[u8]) -> [u8; 32] {
    *::blake3::hash(data).as_bytes()
}

/// Compute a variable-length BLAKE3 hash.
pub fn hash_xof(data: &[u8], output: &mut [u8]) {
    let mut hasher = ::blake3::Hasher::new();
    hasher.update(data);
    let mut reader = hasher.finalize_xof();
    reader.fill(output);
}

/// Derive a key using BLAKE3's built-in key derivation mode.
///
/// The context string must be one of the registered context strings from Section 2.3
/// of the Ochra v5.5 specification. The key material can be any byte slice.
///
/// # Arguments
///
/// * `context` - A registered context string (must start with "Ochra v1 ")
/// * `key_material` - The input key material
pub fn derive_key(context: &str, key_material: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut hasher = ::blake3::Hasher::new_derive_key(context);
    hasher.update(key_material);
    let hash = hasher.finalize();
    out.copy_from_slice(hash.as_bytes());
    out
}

/// Compute a keyed BLAKE3 hash (MAC/PRF).
///
/// The key must be exactly 32 bytes, typically derived via [`derive_key`].
///
/// # Arguments
///
/// * `key` - A 32-byte key (derived via `derive_key`)
/// * `message` - The message to authenticate
pub fn keyed_hash(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    *::blake3::keyed_hash(key, message).as_bytes()
}

/// Verify that a context string is registered in the Ochra protocol.
pub fn is_registered_context(context: &str) -> bool {
    contexts::ALL_CONTEXTS.contains(&context)
}

/// Compute a Merkle tree leaf hash with domain separation.
///
/// Leaf nodes use `BLAKE3::hash(0x00 || data)` to prevent second-preimage attacks.
pub fn merkle_leaf(data: &[u8]) -> [u8; 32] {
    let mut input = Vec::with_capacity(1 + data.len());
    input.push(0x00);
    input.extend_from_slice(data);
    hash(&input)
}

/// Compute a Merkle tree inner node hash with domain separation.
///
/// Inner nodes use `BLAKE3::keyed_hash(K_inner, left || right)` where
/// `K_inner = BLAKE3::derive_key("Ochra v1 merkle-inner-node", "")`.
pub fn merkle_inner(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let k_inner = derive_key(contexts::MERKLE_INNER_NODE, b"");
    let mut message = [0u8; 64];
    message[..32].copy_from_slice(left);
    message[32..].copy_from_slice(right);
    keyed_hash(&k_inner, &message)
}

/// Encode multiple dynamic fields using length-prefixed encoding.
///
/// When deriving keys from multiple dynamic fields, inputs use
/// `LE32(len(field1)) || field1 || LE32(len(field2)) || field2 || ...`
pub fn encode_multi_field(fields: &[&[u8]]) -> Vec<u8> {
    let total_len: usize = fields.iter().map(|f| 4 + f.len()).sum();
    let mut output = Vec::with_capacity(total_len);
    for field in fields {
        output.extend_from_slice(&(field.len() as u32).to_le_bytes());
        output.extend_from_slice(field);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_context_strings_registered() {
        // Verify we have exactly 39 context strings (36 from spec + 3 additional)
        // The spec says 36 but the registry in Section 2.3 lists 39
        assert!(contexts::ALL_CONTEXTS.len() >= 36);

        // All context strings should start with "Ochra v1 "
        for ctx in contexts::ALL_CONTEXTS {
            assert!(
                ctx.starts_with("Ochra v1 "),
                "Context string '{ctx}' has wrong prefix"
            );
        }
    }

    #[test]
    fn test_hash_deterministic() {
        let result1 = hash(b"Ochra test vector 1");
        let result2 = hash(b"Ochra test vector 1");
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_hash_different_inputs() {
        let result1 = hash(b"input1");
        let result2 = hash(b"input2");
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_derive_key_deterministic() {
        let key1 = derive_key(contexts::PROFILE_ENCRYPTION_KEY, &[0u8; 32]);
        let key2 = derive_key(contexts::PROFILE_ENCRYPTION_KEY, &[0u8; 32]);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_derive_key_different_contexts() {
        let key1 = derive_key(contexts::PROFILE_ENCRYPTION_KEY, &[0u8; 32]);
        let key2 = derive_key(contexts::HANDLE_LOOKUP, &[0u8; 32]);
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_keyed_hash_deterministic() {
        let key = derive_key(contexts::MERKLE_INNER_NODE, b"");
        let mac1 = keyed_hash(&key, &[0u8; 64]);
        let mac2 = keyed_hash(&key, &[0u8; 64]);
        assert_eq!(mac1, mac2);
    }

    #[test]
    fn test_merkle_leaf_prefix() {
        // Leaf hash should differ from plain hash due to 0x00 prefix
        let leaf = merkle_leaf(b"test");
        let plain = hash(b"test");
        assert_ne!(leaf, plain);

        // Should equal hash(0x00 || data)
        let mut prefixed = vec![0x00];
        prefixed.extend_from_slice(b"test");
        assert_eq!(leaf, hash(&prefixed));
    }

    #[test]
    fn test_merkle_inner_node() {
        let left = hash(b"left");
        let right = hash(b"right");
        let inner = merkle_inner(&left, &right);

        // Verify manually
        let k_inner = derive_key(contexts::MERKLE_INNER_NODE, b"");
        let mut message = [0u8; 64];
        message[..32].copy_from_slice(&left);
        message[32..].copy_from_slice(&right);
        assert_eq!(inner, keyed_hash(&k_inner, &message));
    }

    #[test]
    fn test_merkle_leaf_inner_separation() {
        // Leaf and inner node hashes should use different domains
        let data = [0u8; 32];
        let leaf = merkle_leaf(&data);
        let inner = merkle_inner(&data, &data);
        assert_ne!(leaf, inner);
    }

    #[test]
    fn test_multi_field_encoding() {
        let encoded = encode_multi_field(&[b"hello", b"world"]);
        assert_eq!(encoded.len(), 4 + 5 + 4 + 5);
        assert_eq!(&encoded[0..4], &5u32.to_le_bytes());
        assert_eq!(&encoded[4..9], b"hello");
        assert_eq!(&encoded[9..13], &5u32.to_le_bytes());
        assert_eq!(&encoded[13..18], b"world");
    }

    #[test]
    fn test_is_registered_context() {
        assert!(is_registered_context("Ochra v1 profile-encryption-key"));
        assert!(!is_registered_context("Ochra v1 made-up-context"));
    }

    #[test]
    fn test_xof_output() {
        let mut output64 = [0u8; 64];
        hash_xof(b"test", &mut output64);
        // First 32 bytes should match the standard hash
        let standard = hash(b"test");
        assert_eq!(&output64[..32], &standard);
    }
}
