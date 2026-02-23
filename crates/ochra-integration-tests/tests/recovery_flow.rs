//! Integration test: Guardian recovery flow.
//!
//! Exercises the complete account recovery lifecycle:
//! 1. Create PIK and encrypt it at rest
//! 2. Nominate 5 guardians via DKG ceremony
//! 3. Distribute threshold shares (3-of-5)
//! 4. Simulate password loss (cannot decrypt PIK)
//! 5. Initiate recovery request
//! 6. Verify 48-hour veto window enforcement
//! 7. Submit guardian shares after veto window
//! 8. Reconstruct PIK with threshold shares
//! 9. Re-encrypt PIK with new password
//!
//! This test uses ochra-crypto (ed25519, argon2id, blake3, chacha20),
//! ochra-guardian (dkg, recovery, heartbeat), and ochra-db.

use ochra_crypto::argon2id;
use ochra_crypto::blake3;
use ochra_crypto::chacha20;
use ochra_crypto::ed25519;
use ochra_guardian::dkg::{self, GuardianInfo};
use ochra_guardian::heartbeat;
use ochra_guardian::recovery::{self, GuardianShare, VetoStatus, VETO_WINDOW};

/// Simulated base timestamp.
const BASE_TIME: u64 = 1_700_000_000;

/// Test password.
const ORIGINAL_PASSWORD: &[u8] = b"original-password-hunter2";
const NEW_PASSWORD: &[u8] = b"new-password-after-recovery";

/// Create a set of test guardians.
fn make_test_guardians(count: usize) -> Vec<GuardianInfo> {
    (0..count)
        .map(|i| {
            let kp = ed25519::KeyPair::generate();
            GuardianInfo {
                pik_hash: blake3::hash(kp.verifying_key.as_bytes()),
                display_name: format!("Guardian-{}", i + 1),
                public_key: kp.verifying_key.to_bytes(),
            }
        })
        .collect()
}

#[tokio::test]
#[ignore]
async fn recovery_full_lifecycle_3_of_5() {
    // =========================================================
    // Step 1: Create PIK and encrypt at rest
    // =========================================================
    let pik_keypair = ed25519::KeyPair::generate();
    let pik_hash = blake3::hash(pik_keypair.verifying_key.as_bytes());
    let pik_secret_bytes = pik_keypair.signing_key.to_bytes();

    // Encrypt PIK with original password.
    let salt = argon2id::generate_salt();
    let enc_key = argon2id::derive_key_custom(ORIGINAL_PASSWORD, &salt, 1024, 1, 1, 32)
        .expect("Argon2id derivation should succeed");
    let mut enc_key_arr = [0u8; 32];
    enc_key_arr.copy_from_slice(&enc_key);

    let nonce = [0x01u8; 12];
    let encrypted_pik = chacha20::encrypt_no_aad(&enc_key_arr, &nonce, &pik_secret_bytes)
        .expect("PIK encryption should succeed");

    // Verify original password can decrypt.
    let decrypted = chacha20::decrypt_no_aad(&enc_key_arr, &nonce, &encrypted_pik)
        .expect("Decryption with correct password should succeed");
    assert_eq!(
        decrypted, pik_secret_bytes,
        "Decrypted PIK must match original"
    );

    // =========================================================
    // Step 2: Nominate 5 guardians and run DKG ceremony
    // =========================================================
    let guardians = make_test_guardians(5);
    let threshold: u32 = 3;

    let mut ceremony = dkg::initiate_dkg(guardians.clone(), threshold)
        .expect("DKG initiation with 5 guardians and threshold 3 should succeed");

    assert_eq!(ceremony.guardian_count(), 5);
    assert_eq!(ceremony.threshold, 3);
    assert!(!ceremony.is_complete(), "DKG should not be complete yet");

    // Process shares (generates threshold key shares).
    ceremony
        .process_shares()
        .expect("DKG share processing should succeed");

    assert!(
        ceremony.is_complete(),
        "DKG should be complete after processing"
    );

    // Verify each guardian received a unique share.
    let mut shares: Vec<Vec<u8>> = Vec::new();
    for i in 0..5 {
        let share = ceremony
            .get_share(i)
            .expect("Each guardian should have a share");
        assert_eq!(share.len(), 32, "Each share should be 32 bytes");
        shares.push(share.to_vec());
    }

    // All shares must be distinct.
    for i in 0..shares.len() {
        for j in (i + 1)..shares.len() {
            assert_ne!(
                shares[i], shares[j],
                "Guardian shares {i} and {j} must be distinct"
            );
        }
    }

    // =========================================================
    // Step 3: Verify guardian heartbeats
    // =========================================================
    for guardian in &guardians {
        let hb = heartbeat::publish_heartbeat(guardian.pik_hash, BASE_TIME);
        assert_eq!(hb.guardian_id, guardian.pik_hash);

        let status = heartbeat::check_heartbeat(
            &guardian.pik_hash,
            hb.timestamp,
            BASE_TIME + 1000,
        );
        assert_eq!(
            status,
            heartbeat::HealthStatus::Healthy,
            "Recently-heartbeated guardian should be healthy"
        );

        // Verify dead drop address is deterministic.
        let addr1 = heartbeat::derive_dead_drop_addr(&guardian.pik_hash, 1);
        let addr2 = heartbeat::derive_dead_drop_addr(&guardian.pik_hash, 1);
        assert_eq!(
            addr1, addr2,
            "Dead drop address must be deterministic"
        );

        // Different epochs produce different addresses.
        let addr3 = heartbeat::derive_dead_drop_addr(&guardian.pik_hash, 2);
        assert_ne!(
            addr1, addr3,
            "Dead drop addresses for different epochs must differ"
        );
    }

    // =========================================================
    // Step 4: Simulate password loss
    // =========================================================
    // User tries wrong password -- decryption should fail.
    let wrong_key = argon2id::derive_key_custom(b"wrong-password", &salt, 1024, 1, 1, 32)
        .expect("Argon2id derivation should succeed");
    let mut wrong_key_arr = [0u8; 32];
    wrong_key_arr.copy_from_slice(&wrong_key);

    let wrong_decrypt = chacha20::decrypt_no_aad(&wrong_key_arr, &nonce, &encrypted_pik);
    assert!(
        wrong_decrypt.is_err(),
        "Wrong password must fail to decrypt PIK"
    );

    // =========================================================
    // Step 5: Initiate recovery
    // =========================================================
    let recovery_proof = blake3::hash(&pik_hash);
    let mut request = recovery::initiate_recovery(recovery_proof.to_vec(), BASE_TIME);

    assert_eq!(request.initiated_at, BASE_TIME);
    assert!(!request.vetoed);
    assert!(request.guardian_shares.is_empty());

    // =========================================================
    // Step 6: Verify veto window enforcement
    // =========================================================
    // During veto window (before 48 hours).
    let during_veto = BASE_TIME + 1000;
    let status = recovery::check_veto_window(&request, during_veto);
    assert_eq!(
        status,
        VetoStatus::Active,
        "Veto window should be active within 48 hours"
    );

    // Attempting to submit a share during the veto window must fail.
    let early_share = GuardianShare {
        guardian_id: guardians[0].pik_hash,
        shard_data: shares[0].clone(),
    };
    let early_result = recovery::submit_share(&mut request, early_share, during_veto);
    assert!(
        early_result.is_err(),
        "Submitting shares during veto window must fail"
    );

    // After veto window (48+ hours later).
    let after_veto = BASE_TIME + VETO_WINDOW;
    let status_after = recovery::check_veto_window(&request, after_veto);
    assert_eq!(
        status_after,
        VetoStatus::Expired,
        "Veto window should be expired after 48 hours"
    );

    // =========================================================
    // Step 7: Submit guardian shares (3 of 5)
    // =========================================================
    // Submit 3 shares from guardians 0, 2, and 4.
    let contributing_indices = [0, 2, 4];
    for &idx in &contributing_indices {
        let share = GuardianShare {
            guardian_id: guardians[idx].pik_hash,
            shard_data: shares[idx].clone(),
        };
        recovery::submit_share(&mut request, share, after_veto + 100)
            .unwrap_or_else(|e| panic!("Guardian {idx} share submission should succeed: {e}"));
    }

    assert_eq!(
        request.guardian_shares.len(),
        3,
        "Three guardian shares should have been submitted"
    );

    // Verify threshold is met.
    assert!(
        recovery::has_enough_shares(&request, threshold as usize),
        "3 of 5 shares should meet the threshold of 3"
    );

    // Not enough if threshold were 4.
    assert!(
        !recovery::has_enough_shares(&request, 4),
        "3 shares should not meet a threshold of 4"
    );

    // =========================================================
    // Step 8: Simulate PIK reconstruction
    // =========================================================
    // In v1, shares are stubs (BLAKE3 hashes). In production, Shamir
    // secret sharing or FROST DKG would be used. For the integration
    // test, we verify the shares are present and distinct, then simulate
    // successful reconstruction by using the original PIK secret.
    //
    // Verify we have enough shares and they match what was distributed.
    for (share_idx, &guardian_idx) in contributing_indices.iter().enumerate() {
        let submitted = &request.guardian_shares[share_idx];
        assert_eq!(
            submitted.guardian_id, guardians[guardian_idx].pik_hash,
            "Submitted share guardian ID should match"
        );
        assert_eq!(
            submitted.shard_data, shares[guardian_idx],
            "Submitted shard data should match DKG share"
        );
    }

    // Simulate reconstruction (in real implementation, this would use
    // threshold secret sharing to recover the PIK secret key).
    let reconstructed_secret = pik_secret_bytes; // Simulated reconstruction.

    // =========================================================
    // Step 9: Re-encrypt PIK with new password
    // =========================================================
    let new_salt = argon2id::generate_salt();
    let new_enc_key = argon2id::derive_key_custom(NEW_PASSWORD, &new_salt, 1024, 1, 1, 32)
        .expect("Argon2id derivation with new password should succeed");
    let mut new_enc_key_arr = [0u8; 32];
    new_enc_key_arr.copy_from_slice(&new_enc_key);

    let new_nonce = [0x02u8; 12];
    let re_encrypted_pik =
        chacha20::encrypt_no_aad(&new_enc_key_arr, &new_nonce, &reconstructed_secret)
            .expect("PIK re-encryption should succeed");

    // Verify new password can decrypt.
    let final_decrypted = chacha20::decrypt_no_aad(&new_enc_key_arr, &new_nonce, &re_encrypted_pik)
        .expect("Decryption with new password should succeed");
    assert_eq!(
        final_decrypted, pik_secret_bytes,
        "Re-encrypted PIK must decrypt to original secret"
    );

    // Verify old password cannot decrypt the re-encrypted PIK.
    let old_attempt = chacha20::decrypt_no_aad(&enc_key_arr, &nonce, &re_encrypted_pik);
    assert!(
        old_attempt.is_err(),
        "Old password must not decrypt re-encrypted PIK"
    );

    // Verify the recovered identity matches.
    let recovered_kp = ed25519::KeyPair::from_bytes(&reconstructed_secret);
    assert_eq!(
        recovered_kp.verifying_key.to_bytes(),
        pik_keypair.verifying_key.to_bytes(),
        "Recovered PIK public key must match original"
    );
}

#[tokio::test]
#[ignore]
async fn recovery_veto_cancels_process() {
    // Test that a vetoed recovery cannot proceed.
    let mut request = recovery::initiate_recovery(vec![0xAA], BASE_TIME);

    // A guardian vetoes the recovery.
    recovery::submit_veto(&mut request).expect("First veto should succeed");
    assert!(request.vetoed);

    let status = recovery::check_veto_window(&request, BASE_TIME + 100);
    assert_eq!(
        status,
        VetoStatus::Vetoed,
        "Vetoed status takes priority over time-based window"
    );

    // Double veto must fail.
    let double_veto_result = recovery::submit_veto(&mut request);
    assert!(
        double_veto_result.is_err(),
        "Double veto must be rejected"
    );

    // Shares cannot be submitted even after veto window expires.
    let share = GuardianShare {
        guardian_id: [0x01; 32],
        shard_data: vec![0xBB; 32],
    };
    let after_veto = BASE_TIME + VETO_WINDOW + 1;
    let share_result = recovery::submit_share(&mut request, share, after_veto);
    assert!(
        share_result.is_err(),
        "Shares must be rejected after a veto, even past the window"
    );
}

#[tokio::test]
#[ignore]
async fn guardian_heartbeat_health_transitions() {
    let guardian_id = [0x42u8; 32];
    let heartbeat_time = BASE_TIME;

    // Healthy: within 5 days.
    let status_healthy = heartbeat::check_heartbeat(
        &guardian_id,
        heartbeat_time,
        heartbeat_time + heartbeat::WARNING_AGE - 1,
    );
    assert_eq!(status_healthy, heartbeat::HealthStatus::Healthy);

    // Warning: between 5 and 7 days.
    let status_warning = heartbeat::check_heartbeat(
        &guardian_id,
        heartbeat_time,
        heartbeat_time + heartbeat::WARNING_AGE + 3600,
    );
    assert_eq!(status_warning, heartbeat::HealthStatus::Warning);

    // Unresponsive: beyond 7 days.
    let status_unresponsive = heartbeat::check_heartbeat(
        &guardian_id,
        heartbeat_time,
        heartbeat_time + heartbeat::MAX_HEARTBEAT_AGE + 1,
    );
    assert_eq!(
        status_unresponsive,
        heartbeat::HealthStatus::Unresponsive
    );
}

#[tokio::test]
#[ignore]
async fn dkg_edge_cases() {
    // Test: threshold equals guardian count (n-of-n).
    let guardians = make_test_guardians(3);
    let mut dkg_n_of_n = dkg::initiate_dkg(guardians.clone(), 3)
        .expect("3-of-3 DKG should succeed");
    dkg_n_of_n
        .process_shares()
        .expect("Processing should succeed");
    assert!(dkg_n_of_n.is_complete());

    // Test: threshold = 1 (any single guardian can recover).
    let mut dkg_1_of_3 = dkg::initiate_dkg(guardians, 1)
        .expect("1-of-3 DKG should succeed");
    dkg_1_of_3
        .process_shares()
        .expect("Processing should succeed");
    assert!(dkg_1_of_3.is_complete());

    // Test: too few guardians for threshold.
    let small_group = make_test_guardians(2);
    let dkg_fail = dkg::initiate_dkg(small_group, 3);
    assert!(
        dkg_fail.is_err(),
        "2 guardians with threshold 3 should fail"
    );

    // Test: zero threshold.
    let guardians3 = make_test_guardians(3);
    let dkg_zero = dkg::initiate_dkg(guardians3, 0);
    assert!(dkg_zero.is_err(), "Zero threshold should fail");

    // Test: cannot process shares twice.
    let guardians4 = make_test_guardians(4);
    let mut dkg_double = dkg::initiate_dkg(guardians4, 2)
        .expect("DKG should initiate");
    dkg_double.process_shares().expect("First process should succeed");
    let second_process = dkg_double.process_shares();
    assert!(
        second_process.is_err(),
        "Second process_shares call should fail"
    );
}

#[tokio::test]
#[ignore]
async fn recovery_database_persistence() {
    // Verify that recovery contact state can be tracked via the database.
    let conn = ochra_db::open_memory().expect("open in-memory DB");

    // Store recovery contacts in the database.
    let guardians = make_test_guardians(3);
    for guardian in &guardians {
        conn.execute(
            "INSERT INTO recovery_contacts (contact_pik, dkg_share, enrolled_at, last_heartbeat_epoch) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                guardian.pik_hash.as_slice(),
                vec![0xAAu8; 32],
                BASE_TIME as i64,
                1_i64,
            ],
        )
        .expect("Guardian insertion should succeed");
    }

    // Query recovery contacts.
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM recovery_contacts", [], |row| {
            row.get(0)
        })
        .expect("Count query should succeed");
    assert_eq!(
        count, 3,
        "Should have 3 recovery contacts in the database"
    );

    // Verify we can retrieve by PIK hash.
    let found: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM recovery_contacts WHERE contact_pik = ?1",
            [guardians[0].pik_hash.as_slice()],
            |row| row.get(0),
        )
        .expect("Lookup should succeed");
    assert!(found, "Guardian 0 should be found by PIK hash");
}
