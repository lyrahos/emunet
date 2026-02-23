//! Integration test: Ephemeral Whisper messaging end-to-end.
//!
//! Exercises the complete Whisper session lifecycle:
//! 1. Create two separate identity contexts (two "nodes")
//! 2. Initialize PIK keypairs for both
//! 3. Establish a Whisper session via X25519 key exchange
//! 4. Derive session keys using domain-separated BLAKE3
//! 5. Encrypt/decrypt messages with ChaCha20-Poly1305
//! 6. Simulate a Seed transfer via Whisper
//! 7. Reveal identity (sign session transcript with PIK)
//! 8. Close session and verify cleanup
//!
//! This test exercises ochra-crypto (ed25519, x25519, blake3, chacha20),
//! ochra-types (Whisper types), and ochra-db (wallet) without a daemon.

use ochra_crypto::blake3;
use ochra_crypto::chacha20;
use ochra_crypto::ed25519;
use ochra_crypto::x25519;

/// Simulated timestamp for tests.
const TEST_TIMESTAMP: u64 = 1_700_000_000;

/// Represents one side of a Whisper session for testing.
struct WhisperNode {
    /// Ed25519 PIK keypair.
    pik: ed25519::KeyPair,
    /// PIK hash (identity).
    pik_hash: [u8; 32],
    /// X25519 static keypair for session establishment.
    x25519_sk: x25519::X25519StaticSecret,
    x25519_pk: x25519::X25519PublicKey,
    /// Database connection (per-node wallet).
    db: rusqlite::Connection,
}

impl WhisperNode {
    fn new() -> Self {
        let pik = ed25519::KeyPair::generate();
        let pik_hash = blake3::hash(pik.verifying_key.as_bytes());
        let x25519_sk = x25519::X25519StaticSecret::random();
        let x25519_pk = x25519_sk.public_key();
        let db = ochra_db::open_memory().expect("open in-memory DB");
        Self {
            pik,
            pik_hash,
            x25519_sk,
            x25519_pk,
            db,
        }
    }
}

/// Derive Whisper session keys from a shared secret.
///
/// Returns (encryption_key, nonce_prefix) using domain-separated BLAKE3.
fn derive_whisper_session_keys(
    shared_secret: &[u8; 32],
    session_id: &[u8; 16],
) -> ([u8; 32], [u8; 12]) {
    let key_material = blake3::encode_multi_field(&[shared_secret.as_slice(), session_id]);
    let enc_key = blake3::derive_key(
        ochra_crypto::blake3::contexts::WHISPER_SESSION_KEY,
        &key_material,
    );
    let nonce_full = blake3::derive_key(
        ochra_crypto::blake3::contexts::WHISPER_RATCHET_ROOT,
        &key_material,
    );
    let mut nonce_prefix = [0u8; 12];
    nonce_prefix.copy_from_slice(&nonce_full[..12]);
    (enc_key, nonce_prefix)
}

/// Derive a per-message nonce from a prefix and sequence number.
fn message_nonce(prefix: &[u8; 12], sequence: u64) -> [u8; 12] {
    let mut nonce = *prefix;
    let seq_bytes = sequence.to_le_bytes();
    // XOR the sequence into the last 8 bytes of the nonce.
    for i in 0..8 {
        nonce[4 + i] ^= seq_bytes[i];
    }
    nonce
}

#[tokio::test]
#[ignore]
async fn whisper_session_full_lifecycle() {
    // =========================================================
    // Step 1: Create two nodes (Alice and Bob)
    // =========================================================
    let alice = WhisperNode::new();
    let bob = WhisperNode::new();

    assert_ne!(
        alice.pik_hash, bob.pik_hash,
        "Alice and Bob must have distinct PIK hashes"
    );

    // =========================================================
    // Step 2: X25519 key exchange to establish shared secret
    // =========================================================
    // Alice computes shared secret with Bob's public key.
    let alice_shared = alice.x25519_sk.diffie_hellman(&bob.x25519_pk);
    // Bob computes shared secret with Alice's public key.
    let bob_shared = bob.x25519_sk.diffie_hellman(&alice.x25519_pk);

    assert_eq!(
        alice_shared.as_bytes(),
        bob_shared.as_bytes(),
        "DH shared secrets must be identical"
    );

    // =========================================================
    // Step 3: Derive session keys
    // =========================================================
    let mut session_id = [0u8; 16];
    // Derive session ID from both PIK hashes.
    let session_id_full = blake3::derive_key(
        ochra_crypto::blake3::contexts::SESSION_KEY_ID,
        &blake3::encode_multi_field(&[&alice.pik_hash, &bob.pik_hash]),
    );
    session_id.copy_from_slice(&session_id_full[..16]);

    let (alice_key, alice_nonce_prefix) =
        derive_whisper_session_keys(alice_shared.as_bytes(), &session_id);
    let (bob_key, bob_nonce_prefix) =
        derive_whisper_session_keys(bob_shared.as_bytes(), &session_id);

    assert_eq!(
        alice_key, bob_key,
        "Both sides must derive the same encryption key"
    );
    assert_eq!(
        alice_nonce_prefix, bob_nonce_prefix,
        "Both sides must derive the same nonce prefix"
    );

    // =========================================================
    // Step 4: Alice sends a text message to Bob
    // =========================================================
    let message_1 = b"Hello Bob, this is a Whisper message!";
    let sequence_1: u64 = 1;
    let nonce_1 = message_nonce(&alice_nonce_prefix, sequence_1);

    let ciphertext_1 = chacha20::encrypt_no_aad(&alice_key, &nonce_1, message_1)
        .expect("Message encryption should succeed");

    // Verify ciphertext is not plaintext.
    assert_ne!(
        ciphertext_1.as_slice(),
        message_1.as_slice(),
        "Ciphertext must differ from plaintext"
    );
    assert_eq!(
        ciphertext_1.len(),
        message_1.len() + chacha20::TAG_SIZE,
        "Ciphertext must include 16-byte auth tag"
    );

    // Bob decrypts.
    let decrypted_1 = chacha20::decrypt_no_aad(&bob_key, &nonce_1, &ciphertext_1)
        .expect("Message decryption should succeed");
    assert_eq!(
        decrypted_1.as_slice(),
        message_1.as_slice(),
        "Decrypted message must match original"
    );

    // =========================================================
    // Step 5: Bob replies to Alice
    // =========================================================
    let message_2 = b"Hi Alice! I received your Whisper.";
    let sequence_2: u64 = 2;
    let nonce_2 = message_nonce(&bob_nonce_prefix, sequence_2);

    // Verify nonces are unique per sequence.
    assert_ne!(nonce_1, nonce_2, "Per-message nonces must be unique");

    let ciphertext_2 = chacha20::encrypt_no_aad(&bob_key, &nonce_2, message_2)
        .expect("Reply encryption should succeed");

    let decrypted_2 = chacha20::decrypt_no_aad(&alice_key, &nonce_2, &ciphertext_2)
        .expect("Reply decryption should succeed");
    assert_eq!(
        decrypted_2.as_slice(),
        message_2.as_slice(),
        "Alice must correctly decrypt Bob's reply"
    );

    // =========================================================
    // Step 6: Seed transfer via Whisper
    // =========================================================
    // Alice sends 5 Seeds to Bob.
    let transfer_amount: u64 = 5 * ochra_types::MICRO_SEEDS_PER_SEED;

    // Seed Alice's wallet.
    ochra_db::queries::wallet::insert_token(
        &alice.db,
        &[0xA1; 16],
        10 * ochra_types::MICRO_SEEDS_PER_SEED,
        &[0xB1; 32],
        TEST_TIMESTAMP,
    )
    .expect("Alice token insertion should succeed");

    // Spend from Alice's wallet.
    ochra_db::queries::wallet::spend_token(&alice.db, &[0xA1; 16], TEST_TIMESTAMP + 100)
        .expect("Alice token spend should succeed");

    // Derive transfer note key from session.
    let transfer_key = blake3::derive_key(
        ochra_crypto::blake3::contexts::WHISPER_SEED_TRANSFER,
        &blake3::encode_multi_field(&[
            alice_shared.as_bytes().as_slice(),
            &session_id,
            &transfer_amount.to_le_bytes(),
        ]),
    );

    // Create and encrypt the transfer payload.
    // Encode the PIK hash as a byte array string for the JSON payload.
    let pik_hash_str: String = alice.pik_hash.iter().map(|b| format!("{b:02x}")).collect();
    let transfer_payload = serde_json::json!({
        "type": "seed_transfer",
        "amount_micro": transfer_amount,
        "sender_pik_hash": pik_hash_str,
        "timestamp": TEST_TIMESTAMP + 100,
    })
    .to_string();

    let transfer_nonce = message_nonce(&alice_nonce_prefix, 3);
    let encrypted_transfer = chacha20::encrypt_no_aad(
        &alice_key,
        &transfer_nonce,
        transfer_payload.as_bytes(),
    )
    .expect("Transfer payload encryption should succeed");

    // Bob decrypts the transfer.
    let decrypted_transfer = chacha20::decrypt_no_aad(
        &bob_key,
        &transfer_nonce,
        &encrypted_transfer,
    )
    .expect("Transfer payload decryption should succeed");
    let parsed: serde_json::Value =
        serde_json::from_slice(&decrypted_transfer).expect("Transfer JSON should parse");
    assert_eq!(
        parsed["amount_micro"],
        serde_json::json!(transfer_amount),
        "Decrypted transfer amount must match"
    );

    // Credit Bob's wallet.
    ochra_db::queries::wallet::insert_token(
        &bob.db,
        &[0xC1; 16],
        transfer_amount,
        &[0xD1; 32],
        TEST_TIMESTAMP + 100,
    )
    .expect("Bob token insertion should succeed");

    // Verify balances.
    let alice_balance = ochra_db::queries::wallet::balance(&alice.db)
        .expect("Alice balance query should succeed");
    assert_eq!(alice_balance, 0, "Alice should have zero balance after spend");

    let bob_balance = ochra_db::queries::wallet::balance(&bob.db)
        .expect("Bob balance query should succeed");
    assert_eq!(
        bob_balance, transfer_amount,
        "Bob should have received the transfer amount"
    );

    // Verify the transfer note key is deterministic.
    let transfer_key_2 = blake3::derive_key(
        ochra_crypto::blake3::contexts::WHISPER_SEED_TRANSFER,
        &blake3::encode_multi_field(&[
            bob_shared.as_bytes().as_slice(),
            &session_id,
            &transfer_amount.to_le_bytes(),
        ]),
    );
    assert_eq!(
        transfer_key, transfer_key_2,
        "Transfer key must be deterministic from both sides"
    );

    // =========================================================
    // Step 7: Identity reveal
    // =========================================================
    // Alice signs the session transcript to prove her identity.
    let transcript_hash = blake3::hash(&blake3::encode_multi_field(&[
        &session_id,
        &alice.pik_hash,
        &bob.pik_hash,
    ]));
    let reveal_sig = alice.pik.signing_key.sign(&transcript_hash);

    // Bob verifies Alice's identity reveal.
    alice
        .pik
        .verifying_key
        .verify(&transcript_hash, &reveal_sig)
        .expect("Alice's identity reveal signature must verify");

    // Bob reciprocates the reveal.
    let bob_reveal_sig = bob.pik.signing_key.sign(&transcript_hash);
    bob.pik
        .verifying_key
        .verify(&transcript_hash, &bob_reveal_sig)
        .expect("Bob's identity reveal signature must verify");

    // Cross-verification: Alice cannot use Bob's signature as her own.
    assert!(
        alice
            .pik
            .verifying_key
            .verify(&transcript_hash, &bob_reveal_sig)
            .is_err(),
        "Bob's signature must not verify under Alice's key"
    );

    // =========================================================
    // Step 8: Session cleanup and verification
    // =========================================================
    // After session close, old nonces must not work with new sessions.
    let new_session_id_full = blake3::derive_key(
        ochra_crypto::blake3::contexts::SESSION_KEY_ID,
        &[0xFF; 32], // Different key material
    );
    let mut new_session_id = [0u8; 16];
    new_session_id.copy_from_slice(&new_session_id_full[..16]);

    let (new_key, _new_nonce) =
        derive_whisper_session_keys(alice_shared.as_bytes(), &new_session_id);
    assert_ne!(
        new_key, alice_key,
        "New session must derive different encryption key"
    );

    // Old ciphertext must not decrypt with new session key.
    let decrypt_result = chacha20::decrypt_no_aad(&new_key, &nonce_1, &ciphertext_1);
    assert!(
        decrypt_result.is_err(),
        "Old ciphertext must fail decryption with new session key"
    );
}

#[tokio::test]
#[ignore]
async fn whisper_ephemeral_key_exchange() {
    // Test ephemeral (one-shot) key exchange for initial Whisper contact.
    let bob_sk = x25519::X25519StaticSecret::random();
    let bob_pk = bob_sk.public_key();

    let (alice_eph_pk, alice_shared) = x25519::ephemeral_key_exchange(&bob_pk);
    let bob_shared = bob_sk.diffie_hellman(&alice_eph_pk);

    assert_eq!(
        alice_shared.as_bytes(),
        bob_shared.as_bytes(),
        "Ephemeral key exchange must produce matching shared secrets"
    );

    // The ephemeral public key should be unique each time.
    let (alice_eph_pk_2, _) = x25519::ephemeral_key_exchange(&bob_pk);
    assert_ne!(
        alice_eph_pk.to_bytes(),
        alice_eph_pk_2.to_bytes(),
        "Each ephemeral exchange must produce a unique public key"
    );
}

#[tokio::test]
#[ignore]
async fn whisper_message_tamper_detection() {
    let shared_secret = [0x42u8; 32];
    let session_id = [0x01u8; 16];

    let (enc_key, nonce_prefix) = derive_whisper_session_keys(&shared_secret, &session_id);
    let nonce = message_nonce(&nonce_prefix, 1);

    let plaintext = b"Sensitive whisper content";
    let ciphertext = chacha20::encrypt_no_aad(&enc_key, &nonce, plaintext)
        .expect("Encryption should succeed");

    // Tamper with the ciphertext.
    let mut tampered = ciphertext.clone();
    if let Some(byte) = tampered.first_mut() {
        *byte ^= 0xFF;
    }

    let result = chacha20::decrypt_no_aad(&enc_key, &nonce, &tampered);
    assert!(
        result.is_err(),
        "Tampered ciphertext must fail AEAD authentication"
    );

    // Tamper with the auth tag (last 16 bytes).
    let mut tag_tampered = ciphertext.clone();
    let last = tag_tampered.len() - 1;
    tag_tampered[last] ^= 0x01;

    let result2 = chacha20::decrypt_no_aad(&enc_key, &nonce, &tag_tampered);
    assert!(
        result2.is_err(),
        "Tag-tampered ciphertext must fail AEAD authentication"
    );
}
