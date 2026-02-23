//! Integration test: Full user lifecycle flow.
//!
//! Exercises the complete identity -> content -> economy pipeline:
//! 1. Create PIK (Ed25519 identity key pair)
//! 2. Derive node ID and encrypt PIK at rest
//! 3. Create a Space (group) and persist it
//! 4. Publish content (chunk, Merkle tree, catalog entry)
//! 5. Simulate a purchase (wallet debit, revenue distribution)
//! 6. Verify wallet balance and earnings math
//!
//! This test uses only the library crates (ochra-crypto, ochra-db,
//! ochra-types, ochra-storage, ochra-revenue) without requiring a
//! running daemon process.

use ochra_crypto::argon2id;
use ochra_crypto::blake3;
use ochra_crypto::chacha20;
use ochra_crypto::ed25519;
use ochra_crypto::x25519;
use ochra_db::queries::{content, spaces, wallet};
use ochra_revenue::splits;
use ochra_storage::chunker;

/// Simulated timestamp for deterministic testing.
const TEST_TIMESTAMP: u64 = 1_700_000_000;

/// Test password for PIK-at-rest encryption.
const TEST_PASSWORD: &[u8] = b"correct horse battery staple";

/// Content pricing: 10 Seeds = 10 * 100_000_000 micro-seeds.
const CONTENT_PRICE_MICRO: u64 = 10 * ochra_types::MICRO_SEEDS_PER_SEED;

#[tokio::test]
#[ignore]
async fn full_lifecycle_identity_to_economy() {
    // =========================================================
    // Step 1: Generate PIK (Platform Identity Key)
    // =========================================================
    let pik_keypair = ed25519::KeyPair::generate();
    let pik_hash = blake3::hash(pik_keypair.verifying_key.as_bytes());
    let node_id = ed25519::derive_node_id(&pik_keypair.verifying_key);

    // Verify node ID is a BLAKE3 hash of the public key.
    assert_eq!(
        node_id,
        blake3::hash(pik_keypair.verifying_key.as_bytes()),
        "Node ID must equal BLAKE3(pik_public_key)"
    );

    // =========================================================
    // Step 2: Encrypt PIK at rest with Argon2id-derived key
    // =========================================================
    let salt = argon2id::generate_salt();
    // Use small Argon2id params for test speed (production uses 256MB).
    let encryption_key =
        argon2id::derive_key_custom(TEST_PASSWORD, &salt, 1024, 1, 1, 32)
            .expect("Argon2id key derivation should succeed");
    let mut enc_key_arr = [0u8; 32];
    enc_key_arr.copy_from_slice(&encryption_key);

    let nonce = [0x01u8; 12];
    let pik_secret_bytes = pik_keypair.signing_key.to_bytes();
    let encrypted_pik = chacha20::encrypt_no_aad(&enc_key_arr, &nonce, &pik_secret_bytes)
        .expect("PIK encryption should succeed");

    // Verify we can decrypt back to the original key.
    let decrypted_pik = chacha20::decrypt_no_aad(&enc_key_arr, &nonce, &encrypted_pik)
        .expect("PIK decryption should succeed");
    assert_eq!(
        decrypted_pik, pik_secret_bytes,
        "Decrypted PIK must match original secret key bytes"
    );

    // Restore the keypair from decrypted bytes.
    let mut restored_secret = [0u8; 32];
    restored_secret.copy_from_slice(&decrypted_pik);
    let restored_kp = ed25519::KeyPair::from_bytes(&restored_secret);
    assert_eq!(
        restored_kp.verifying_key.to_bytes(),
        pik_keypair.verifying_key.to_bytes(),
        "Restored PIK public key must match original"
    );

    // =========================================================
    // Step 3: Open database, create a Space, seed the wallet
    // =========================================================
    let conn = ochra_db::open_memory().expect("In-memory DB should open");

    // Generate an X25519 keypair for session negotiation.
    let _x25519_sk = x25519::X25519StaticSecret::random();

    // Create a Space (group).
    let group_id: [u8; 32] = blake3::derive_key(
        ochra_crypto::blake3::contexts::GROUP_SETTINGS_KEY,
        &pik_hash,
    );
    spaces::insert(
        &conn,
        &group_id,
        "Ochra Test Store",
        "storefront",
        "host",
        &pik_hash,
        TEST_TIMESTAMP,
    )
    .expect("Space insertion should succeed");

    // Verify the Space was created.
    let all_spaces = spaces::list(&conn).expect("Space listing should succeed");
    assert_eq!(all_spaces.len(), 1, "There should be exactly one Space");
    assert_eq!(all_spaces[0].name, "Ochra Test Store");

    // Seed the buyer wallet with tokens (simulate minting).
    let buyer_initial_balance: u64 = 100 * ochra_types::MICRO_SEEDS_PER_SEED;
    wallet::insert_token(
        &conn,
        &[0xA1; 16],
        buyer_initial_balance,
        &[0xB1; 32],
        TEST_TIMESTAMP,
    )
    .expect("Token insertion should succeed");

    let balance_before = wallet::balance(&conn).expect("Balance query should succeed");
    assert_eq!(
        balance_before, buyer_initial_balance,
        "Initial wallet balance must equal seeded amount"
    );

    // =========================================================
    // Step 4: Publish content (chunk + Merkle tree + catalog)
    // =========================================================
    // Create test content (small for speed; real content would be MB+).
    let content_data = b"This is premium Ochra content for the integration test. \
        It exercises chunking, Merkle tree construction, and the full \
        publish-to-catalog pipeline.";

    let split_result =
        chunker::split_content(content_data).expect("Content splitting should succeed");

    assert!(
        !split_result.chunks.is_empty(),
        "Split result must contain at least one chunk"
    );
    assert_eq!(
        split_result.chunks[0].data.as_slice(),
        content_data.as_slice(),
        "Single-chunk content data must match original"
    );

    // Verify Merkle root is deterministic.
    let split_result_2 =
        chunker::split_content(content_data).expect("Second split should succeed");
    assert_eq!(
        split_result.content_hash, split_result_2.content_hash,
        "Merkle root must be deterministic for identical content"
    );

    // Verify Merkle proof for each chunk.
    for (i, leaf_hash) in split_result.leaf_hashes.iter().enumerate() {
        let proof = chunker::generate_merkle_proof(&split_result.leaf_hashes, i)
            .expect("Merkle proof generation should succeed");
        assert!(
            chunker::verify_merkle_proof(
                &split_result.content_hash,
                leaf_hash,
                &proof,
                i as u32,
            ),
            "Merkle proof for chunk {i} must verify against root"
        );
    }

    // Compute the key commitment (hash of a random decryption key).
    let decryption_key = blake3::derive_key(
        ochra_crypto::blake3::contexts::CONTENT_ESCROW_KEY,
        &content_data[..],
    );
    let key_commitment = blake3::hash(&decryption_key);

    // Create the pricing JSON.
    let pricing_json = serde_json::json!([{
        "tier_type": "permanent",
        "price_seeds": CONTENT_PRICE_MICRO,
    }])
    .to_string();

    // Sign the content manifest.
    let manifest_data = blake3::encode_multi_field(&[
        &split_result.content_hash,
        &group_id,
        b"Premium Content",
        &key_commitment,
    ]);
    let manifest_sig = pik_keypair.signing_key.sign(&manifest_data);

    // Verify the signature.
    pik_keypair
        .verifying_key
        .verify(&manifest_data, &manifest_sig)
        .expect("Content manifest signature must verify");

    // Insert into catalog.
    content::insert(
        &conn,
        &split_result.content_hash,
        &group_id,
        "Premium Content",
        Some("A test content item"),
        &pricing_json,
        &pik_hash,
        &key_commitment,
        content_data.len() as u64,
        split_result.chunks.len() as u32,
        TEST_TIMESTAMP + 100,
    )
    .expect("Content catalog insertion should succeed");

    // Verify the content appears in the catalog.
    let catalog = content::list_by_space(&conn, &group_id)
        .expect("Catalog listing should succeed");
    assert_eq!(catalog.len(), 1, "Catalog should contain one item");
    assert_eq!(catalog[0].title, "Premium Content");
    assert_eq!(catalog[0].total_size_bytes, content_data.len() as u64);

    // =========================================================
    // Step 5: Simulate a purchase
    // =========================================================
    // Buyer spends a token to purchase the content.
    wallet::spend_token(&conn, &[0xA1; 16], TEST_TIMESTAMP + 200)
        .expect("Token spend should succeed");

    // Record the purchase transaction.
    let tx_hash = blake3::hash(&blake3::encode_multi_field(&[
        &split_result.content_hash,
        &pik_hash,
        &CONTENT_PRICE_MICRO.to_le_bytes(),
    ]));
    wallet::record_transaction(
        &conn,
        &tx_hash,
        "purchase",
        CONTENT_PRICE_MICRO,
        1,
        TEST_TIMESTAMP + 200,
    )
    .expect("Transaction recording should succeed");

    // =========================================================
    // Step 6: Revenue distribution using the Space's split
    // =========================================================
    // Use default split: host=10%, creator=70%, network=20%.
    let split_config = splits::RevenueSplitConfig {
        host_pct: 10,
        creator_pct: 70,
        network_pct: 20,
    };
    splits::validate_split(&split_config)
        .expect("Default split configuration must be valid");

    let (host_share, creator_share, network_share) =
        splits::distribute(CONTENT_PRICE_MICRO, &split_config)
            .expect("Revenue distribution should succeed");

    // Verify the math: all shares must sum to the purchase price.
    assert_eq!(
        host_share + creator_share + network_share,
        CONTENT_PRICE_MICRO,
        "Revenue shares must sum to total purchase price"
    );

    // Verify percentages (exact for this amount).
    assert_eq!(
        host_share,
        CONTENT_PRICE_MICRO * 10 / 100,
        "Host share should be 10%"
    );
    assert_eq!(
        network_share,
        CONTENT_PRICE_MICRO * 20 / 100,
        "Network share should be 20%"
    );
    // Creator gets remainder (handles rounding).
    assert_eq!(
        creator_share,
        CONTENT_PRICE_MICRO - host_share - network_share,
        "Creator share should be the remainder"
    );

    // Credit the creator's earnings by inserting tokens for each share.
    wallet::insert_token(
        &conn,
        &[0xC1; 16],
        creator_share,
        &[0xD1; 32],
        TEST_TIMESTAMP + 300,
    )
    .expect("Creator earning token insertion should succeed");

    // =========================================================
    // Step 7: Verify final wallet state
    // =========================================================
    // The original buyer token was spent; the creator received earnings.
    let final_balance = wallet::balance(&conn).expect("Final balance query should succeed");
    assert_eq!(
        final_balance, creator_share,
        "Final balance should equal creator's revenue share"
    );

    // Verify transaction history.
    let txs = wallet::recent_transactions(&conn, 10)
        .expect("Transaction history query should succeed");
    assert_eq!(txs.len(), 1, "There should be exactly one transaction");
    assert_eq!(txs[0].tx_type, "purchase");
    assert_eq!(txs[0].amount, CONTENT_PRICE_MICRO);

    // Verify double-spend prevention.
    let double_spend_result = wallet::spend_token(&conn, &[0xA1; 16], TEST_TIMESTAMP + 400);
    assert!(
        double_spend_result.is_err(),
        "Double-spending a token must fail"
    );
}

#[tokio::test]
#[ignore]
async fn pik_generation_and_node_id_derivation() {
    // Verify deterministic key derivation from a known seed.
    let seed = [42u8; 32];
    let kp1 = ed25519::KeyPair::from_bytes(&seed);
    let kp2 = ed25519::KeyPair::from_bytes(&seed);

    assert_eq!(
        kp1.verifying_key.to_bytes(),
        kp2.verifying_key.to_bytes(),
        "Same seed must produce identical PIK public keys"
    );

    // Different seeds produce different keys.
    let kp3 = ed25519::KeyPair::from_bytes(&[43u8; 32]);
    assert_ne!(
        kp1.verifying_key.to_bytes(),
        kp3.verifying_key.to_bytes(),
        "Different seeds must produce different PIK public keys"
    );

    // Node ID derivation is deterministic.
    let node_id_a = ed25519::derive_node_id(&kp1.verifying_key);
    let node_id_b = ed25519::derive_node_id(&kp1.verifying_key);
    assert_eq!(
        node_id_a, node_id_b,
        "Node ID derivation must be deterministic"
    );

    // Different PIKs produce different node IDs.
    let node_id_c = ed25519::derive_node_id(&kp3.verifying_key);
    assert_ne!(
        node_id_a, node_id_c,
        "Different PIKs must produce different node IDs"
    );
}

#[tokio::test]
#[ignore]
async fn content_chunking_and_merkle_verification() {
    // Test with multi-chunk content.
    let large_data = vec![0xABu8; chunker::CHUNK_SIZE * 3 + 500];
    let result = chunker::split_content(&large_data)
        .expect("Multi-chunk splitting should succeed");

    assert_eq!(result.chunks.len(), 4, "3.x chunks should split into 4");
    assert_eq!(result.chunks[0].data.len(), chunker::CHUNK_SIZE);
    assert_eq!(result.chunks[1].data.len(), chunker::CHUNK_SIZE);
    assert_eq!(result.chunks[2].data.len(), chunker::CHUNK_SIZE);
    assert_eq!(result.chunks[3].data.len(), 500);

    // Verify all Merkle proofs.
    for i in 0..result.chunks.len() {
        let proof = chunker::generate_merkle_proof(&result.leaf_hashes, i)
            .expect("Proof generation should succeed");
        assert!(
            chunker::verify_merkle_proof(
                &result.content_hash,
                &result.leaf_hashes[i],
                &proof,
                i as u32,
            ),
            "Merkle proof for chunk {i} must verify"
        );
    }

    // A wrong leaf must not verify.
    let fake_leaf = blake3::merkle_leaf(b"fake data");
    let proof_0 = chunker::generate_merkle_proof(&result.leaf_hashes, 0)
        .expect("Proof generation should succeed");
    assert!(
        !chunker::verify_merkle_proof(&result.content_hash, &fake_leaf, &proof_0, 0),
        "Fake leaf must not pass Merkle verification"
    );
}
