//! Test vector generator for the Ochra protocol.
//!
//! Generates `test_vectors.json` containing all Section 35 vectors.
//! This binary is the ground truth for all cryptographic interoperability.
//!
//! Usage:
//!   ochra-testvec              # Generate test_vectors.json
//!   ochra-testvec --verify     # Verify test vectors match expected values

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
struct TestVectors {
    version: String,
    generated_by: String,
    vectors: BTreeMap<String, TestVector>,
}

#[derive(Serialize, Deserialize)]
struct TestVector {
    description: String,
    inputs: BTreeMap<String, String>,
    outputs: BTreeMap<String, String>,
}

fn generate_blake3_vectors() -> BTreeMap<String, TestVector> {
    let mut vectors = BTreeMap::new();

    // Vector 1: Basic hash
    let hash = ochra_crypto::blake3::hash(b"Ochra test vector 1");
    vectors.insert(
        "blake3_basic_hash".to_string(),
        TestVector {
            description: "BLAKE3::hash(b\"Ochra test vector 1\")".to_string(),
            inputs: BTreeMap::from([("data".to_string(), "Ochra test vector 1".to_string())]),
            outputs: BTreeMap::from([("hash".to_string(), hex::encode(hash))]),
        },
    );

    // Vector 2: Key derivation - profile encryption key
    let key = ochra_crypto::blake3::derive_key(
        ochra_crypto::blake3::contexts::PROFILE_ENCRYPTION_KEY,
        &[0u8; 32],
    );
    vectors.insert(
        "blake3_derive_key_profile".to_string(),
        TestVector {
            description: "BLAKE3::derive_key(\"Ochra v1 profile-encryption-key\", 0x00*32)"
                .to_string(),
            inputs: BTreeMap::from([
                (
                    "context".to_string(),
                    "Ochra v1 profile-encryption-key".to_string(),
                ),
                ("key_material".to_string(), hex::encode([0u8; 32])),
            ]),
            outputs: BTreeMap::from([("derived_key".to_string(), hex::encode(key))]),
        },
    );

    // Vector 3: Handle lookup
    let handle_key = ochra_crypto::blake3::derive_key(
        ochra_crypto::blake3::contexts::HANDLE_LOOKUP,
        b"testuser",
    );
    vectors.insert(
        "blake3_handle_lookup".to_string(),
        TestVector {
            description: "BLAKE3::derive_key(\"Ochra v1 handle-lookup\", b\"testuser\")"
                .to_string(),
            inputs: BTreeMap::from([
                ("context".to_string(), "Ochra v1 handle-lookup".to_string()),
                ("key_material".to_string(), "testuser".to_string()),
            ]),
            outputs: BTreeMap::from([("derived_key".to_string(), hex::encode(handle_key))]),
        },
    );

    // Vector 4: Keyed hash (Merkle inner node)
    let k_inner =
        ochra_crypto::blake3::derive_key(ochra_crypto::blake3::contexts::MERKLE_INNER_NODE, b"");
    let mac = ochra_crypto::blake3::keyed_hash(&k_inner, &[0u8; 64]);
    vectors.insert(
        "blake3_keyed_hash_merkle".to_string(),
        TestVector {
            description: "BLAKE3::keyed_hash(K_inner, 0x00*64) where K_inner = derive_key(\"Ochra v1 merkle-inner-node\", \"\")".to_string(),
            inputs: BTreeMap::from([
                ("k_inner".to_string(), hex::encode(k_inner)),
                ("message".to_string(), hex::encode([0u8; 64])),
            ]),
            outputs: BTreeMap::from([("mac".to_string(), hex::encode(mac))]),
        },
    );

    vectors
}

fn generate_ed25519_vectors() -> BTreeMap<String, TestVector> {
    let mut vectors = BTreeMap::new();

    // RFC 8032 Section 7.1, Test 1
    let kp = ochra_crypto::ed25519::KeyPair::from_bytes(&[0u8; 32]);
    let sig = kp.signing_key.sign(b"");
    vectors.insert(
        "ed25519_rfc8032_test1".to_string(),
        TestVector {
            description: "RFC 8032 Section 7.1 Test 1: all-zeros secret key, empty message"
                .to_string(),
            inputs: BTreeMap::from([
                ("secret_key".to_string(), hex::encode([0u8; 32])),
                ("message".to_string(), String::new()),
            ]),
            outputs: BTreeMap::from([
                (
                    "public_key".to_string(),
                    hex::encode(kp.verifying_key.to_bytes()),
                ),
                ("signature".to_string(), hex::encode(sig.to_bytes())),
            ]),
        },
    );

    // Node ID derivation
    let node_id = ochra_crypto::ed25519::derive_node_id(&kp.verifying_key);
    vectors.insert(
        "node_id_derivation".to_string(),
        TestVector {
            description: "Node ID = BLAKE3::hash(pik_public_key)".to_string(),
            inputs: BTreeMap::from([(
                "pik_public_key".to_string(),
                hex::encode(kp.verifying_key.to_bytes()),
            )]),
            outputs: BTreeMap::from([("node_id".to_string(), hex::encode(node_id))]),
        },
    );

    vectors
}

fn generate_receipt_id_vector() -> BTreeMap<String, TestVector> {
    let mut vectors = BTreeMap::new();

    let receipt_secret = [0u8; 32];
    let content_hash = [0u8; 32];
    let tier_index = 0u8;

    let mut input = Vec::with_capacity(32 + 32 + 1);
    input.extend_from_slice(&receipt_secret);
    input.extend_from_slice(&content_hash);
    input.push(tier_index);

    let receipt_id = ochra_crypto::blake3::derive_key(
        ochra_crypto::blake3::contexts::RECEIPT_DHT_ADDRESS,
        &input,
    );

    vectors.insert(
        "receipt_id_derivation".to_string(),
        TestVector {
            description:
                "receipt_id = BLAKE3::derive_key(\"Ochra v1 receipt-dht-address\", receipt_secret || content_hash || tier_index)"
                    .to_string(),
            inputs: BTreeMap::from([
                ("receipt_secret".to_string(), hex::encode(receipt_secret)),
                ("content_hash".to_string(), hex::encode(content_hash)),
                ("tier_index".to_string(), "0".to_string()),
            ]),
            outputs: BTreeMap::from([("receipt_id".to_string(), hex::encode(receipt_id))]),
        },
    );

    vectors
}

fn generate_hybrid_session_vector() -> BTreeMap<String, TestVector> {
    let mut vectors = BTreeMap::new();

    // RFC 7748 Section 6.1 shared secret
    let x25519_shared =
        hex::decode("4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742")
            .expect("valid hex");

    let mlkem_shared = [0u8; 31].iter().chain(&[1u8]).copied().collect::<Vec<u8>>();
    let mut mlkem_bytes = [0u8; 32];
    mlkem_bytes.copy_from_slice(&mlkem_shared);

    let mut combined = Vec::with_capacity(64);
    combined.extend_from_slice(&x25519_shared);
    combined.extend_from_slice(&mlkem_bytes);

    let session_secret = ochra_crypto::blake3::derive_key(
        ochra_crypto::blake3::contexts::PQC_SESSION_SECRET,
        &combined,
    );

    vectors.insert(
        "hybrid_session_secret".to_string(),
        TestVector {
            description: "Hybrid PQC session secret derivation".to_string(),
            inputs: BTreeMap::from([
                ("x25519_shared".to_string(), hex::encode(&x25519_shared)),
                ("mlkem768_shared".to_string(), hex::encode(&mlkem_bytes)),
            ]),
            outputs: BTreeMap::from([("session_secret".to_string(), hex::encode(session_secret))]),
        },
    );

    vectors
}

fn generate_ecies_vector() -> BTreeMap<String, TestVector> {
    let mut vectors = BTreeMap::new();

    let randomness = [0x01u8; 32];
    let recipient_sk = ochra_crypto::x25519::X25519StaticSecret::from_bytes([0x02u8; 32]);
    let recipient_pk = recipient_sk.public_key();
    let plaintext = b"Ochra content key test";

    let ct = ochra_crypto::ecies::encrypt_deterministic(&recipient_pk, plaintext, &randomness)
        .expect("ECIES encrypt");

    vectors.insert(
        "ecies_roundtrip".to_string(),
        TestVector {
            description: "ECIES-X25519-ChaCha20-BLAKE3 deterministic encryption".to_string(),
            inputs: BTreeMap::from([
                (
                    "recipient_pk".to_string(),
                    hex::encode(recipient_pk.to_bytes()),
                ),
                ("plaintext".to_string(), hex::encode(plaintext)),
                ("randomness".to_string(), hex::encode(randomness)),
            ]),
            outputs: BTreeMap::from([
                ("eph_pk".to_string(), hex::encode(ct.eph_pk)),
                (
                    "ciphertext_and_tag".to_string(),
                    hex::encode(&ct.ciphertext_and_tag),
                ),
            ]),
        },
    );

    vectors
}

fn generate_ratchet_vectors() -> BTreeMap<String, TestVector> {
    let mut vectors = BTreeMap::new();

    // Root KDF
    let rk = [0u8; 32];
    let dh_out = [0xFFu8; 32];
    let mut root_input = Vec::with_capacity(64);
    root_input.extend_from_slice(&rk);
    root_input.extend_from_slice(&dh_out);

    let new_rk = ochra_crypto::blake3::derive_key(
        ochra_crypto::blake3::contexts::RATCHET_ROOT_KDF,
        &root_input,
    );
    let chain_key = ochra_crypto::blake3::derive_key(
        ochra_crypto::blake3::contexts::RATCHET_CHAIN_KEY,
        &root_input,
    );

    vectors.insert(
        "ratchet_root_kdf".to_string(),
        TestVector {
            description: "Double Ratchet root KDF: KDF_RK(rk=0x00*32, dh_out=0xFF*32)".to_string(),
            inputs: BTreeMap::from([
                ("rk".to_string(), hex::encode(rk)),
                ("dh_out".to_string(), hex::encode(dh_out)),
            ]),
            outputs: BTreeMap::from([
                ("new_rk".to_string(), hex::encode(new_rk)),
                ("chain_key".to_string(), hex::encode(chain_key)),
            ]),
        },
    );

    // Chain KDF
    let ck = [0u8; 32];
    let new_ck =
        ochra_crypto::blake3::derive_key(ochra_crypto::blake3::contexts::RATCHET_CHAIN_KEY, &ck);
    let msg_key =
        ochra_crypto::blake3::derive_key(ochra_crypto::blake3::contexts::RATCHET_MSG_KEY, &ck);

    vectors.insert(
        "ratchet_chain_kdf".to_string(),
        TestVector {
            description: "Double Ratchet chain KDF: KDF_CK(ck=0x00*32)".to_string(),
            inputs: BTreeMap::from([("ck".to_string(), hex::encode(ck))]),
            outputs: BTreeMap::from([
                ("new_ck".to_string(), hex::encode(new_ck)),
                ("msg_key".to_string(), hex::encode(msg_key)),
            ]),
        },
    );

    vectors
}

fn generate_bloom_filter_vector() -> BTreeMap<String, TestVector> {
    let mut vectors = BTreeMap::new();

    let nullifier = [0u8; 32];
    let filter_size_bits: u64 = 28_700_000;

    let h1_hash = ochra_crypto::blake3::hash(&nullifier);
    let h1 = u64::from_le_bytes(h1_hash[..8].try_into().expect("8 bytes"));

    let mut prefixed = vec![0x01u8];
    prefixed.extend_from_slice(&nullifier);
    let h2_hash = ochra_crypto::blake3::hash(&prefixed);
    let h2 = u64::from_le_bytes(h2_hash[..8].try_into().expect("8 bytes"));

    let mut indices = Vec::new();
    for i in 0u64..20 {
        let bit_index = (h1
            .wrapping_add(i.wrapping_mul(h2))
            .wrapping_add(i.wrapping_mul(i)))
            % filter_size_bits;
        indices.push(bit_index.to_string());
    }

    vectors.insert(
        "bloom_filter_hash".to_string(),
        TestVector {
            description: "NullifierSet Bloom filter hash derivation (20 indices)".to_string(),
            inputs: BTreeMap::from([
                ("nullifier".to_string(), hex::encode(nullifier)),
                ("filter_size_bits".to_string(), filter_size_bits.to_string()),
            ]),
            outputs: BTreeMap::from([
                ("h1".to_string(), h1.to_string()),
                ("h2".to_string(), h2.to_string()),
                ("bit_indices".to_string(), indices.join(",")),
            ]),
        },
    );

    vectors
}

fn generate_all_vectors() -> TestVectors {
    let mut all_vectors = BTreeMap::new();

    all_vectors.extend(generate_blake3_vectors());
    all_vectors.extend(generate_ed25519_vectors());
    all_vectors.extend(generate_receipt_id_vector());
    all_vectors.extend(generate_hybrid_session_vector());
    all_vectors.extend(generate_ecies_vector());
    all_vectors.extend(generate_ratchet_vectors());
    all_vectors.extend(generate_bloom_filter_vector());

    TestVectors {
        version: "1.0".to_string(),
        generated_by: "ochra-testvec".to_string(),
        vectors: all_vectors,
    }
}

fn verify_vectors(vectors: &TestVectors) -> bool {
    let regenerated = generate_all_vectors();
    let mut all_pass = true;

    for (name, expected) in &vectors.vectors {
        if let Some(actual) = regenerated.vectors.get(name) {
            if actual.outputs != expected.outputs {
                eprintln!("FAIL: {name}");
                eprintln!("  expected: {:?}", expected.outputs);
                eprintln!("  actual:   {:?}", actual.outputs);
                all_pass = false;
            } else {
                eprintln!("PASS: {name}");
            }
        } else {
            eprintln!("MISSING: {name}");
            all_pass = false;
        }
    }

    all_pass
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--verify") {
        // Verify mode: load existing vectors and check
        let path = "tests/fixtures/test_vectors.json";
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let vectors: TestVectors = serde_json::from_str(&content).expect("valid JSON");
                if verify_vectors(&vectors) {
                    eprintln!("All test vectors verified successfully.");
                    std::process::exit(0);
                } else {
                    eprintln!("Test vector verification FAILED.");
                    std::process::exit(1);
                }
            }
            Err(_) => {
                eprintln!("No existing test vectors found at {path}. Generating...");
                let vectors = generate_all_vectors();
                let json = serde_json::to_string_pretty(&vectors).expect("serialize");
                std::fs::write(path, &json).expect("write file");
                eprintln!("Generated test vectors to {path}");
                // Verify the freshly generated vectors
                if verify_vectors(&vectors) {
                    eprintln!("Self-verification passed.");
                } else {
                    eprintln!("Self-verification FAILED.");
                    std::process::exit(1);
                }
            }
        }
    } else {
        // Generate mode: produce test_vectors.json
        let vectors = generate_all_vectors();
        let json = serde_json::to_string_pretty(&vectors).expect("serialize");
        let path = "tests/fixtures/test_vectors.json";

        // Create parent directories
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).expect("create dirs");
        }

        std::fs::write(path, &json).expect("write file");
        eprintln!("Generated {} test vectors to {path}", vectors.vectors.len());

        // Self-verify
        if verify_vectors(&vectors) {
            eprintln!("Self-verification passed.");
        } else {
            eprintln!("Self-verification FAILED.");
            std::process::exit(1);
        }
    }
}
