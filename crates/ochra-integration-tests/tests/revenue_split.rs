//! Integration test: Economic correctness of revenue splits.
//!
//! Exercises the complete revenue lifecycle:
//! 1. Create a Space with specific revenue split (60/30/10)
//! 2. Publish content with multiple pricing tiers
//! 3. Simulate purchases at each tier
//! 4. Verify revenue distribution matches the split configuration
//! 5. Test immutable split enforcement (30-day timelock)
//! 6. Verify rounding correctness across edge cases
//! 7. Test default split configuration
//!
//! This test uses ochra-revenue (splits), ochra-db (wallet, spaces,
//! content), ochra-crypto (blake3, ed25519), and ochra-types.

use ochra_crypto::blake3;
use ochra_crypto::ed25519;
use ochra_db::queries::{content, wallet};
use ochra_revenue::splits::{self, RevenueSplitConfig, TIMELOCK_SECONDS};

/// Base timestamp for test scenarios.
const BASE_TIME: u64 = 1_700_000_000;

/// Helper: set up a Space with a given split in the database.
fn setup_space_with_split(
    conn: &rusqlite::Connection,
    group_id: &[u8; 32],
    owner_pik: &[u8; 32],
    split: &RevenueSplitConfig,
) {
    // Insert space with split percentages stored as defaults.
    conn.execute(
        "INSERT INTO spaces (group_id, name, template, my_role, owner_pik, owner_pct, pub_pct, abr_pct, joined_at, last_activity_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        rusqlite::params![
            group_id.as_slice(),
            "Revenue Test Space",
            "storefront",
            "host",
            owner_pik.as_slice(),
            split.host_pct as i64,
            split.creator_pct as i64,
            split.network_pct as i64,
            BASE_TIME as i64,
        ],
    )
    .expect("Space insertion should succeed");
}

/// Helper: record a purchase and distribute revenue.
fn simulate_purchase(
    conn: &rusqlite::Connection,
    purchase_amount: u64,
    split: &RevenueSplitConfig,
    tx_id: u8,
) -> (u64, u64, u64) {
    // Record the purchase transaction.
    let tx_hash = blake3::hash(&[tx_id; 32]);
    wallet::record_transaction(conn, &tx_hash, "purchase", purchase_amount, 1, BASE_TIME + u64::from(tx_id) * 100)
        .expect("Transaction recording should succeed");

    // Distribute revenue according to split.
    let (host_share, creator_share, network_share) = splits::distribute(purchase_amount, split)
        .expect("Revenue distribution should succeed");

    (host_share, creator_share, network_share)
}

#[tokio::test]
#[ignore]
async fn revenue_split_custom_60_30_10() {
    // =========================================================
    // Setup: Space with Host=60%, Creator=30%, Network=10%
    // =========================================================
    let conn = ochra_db::open_memory().expect("open DB");
    let kp = ed25519::KeyPair::generate();
    let owner_pik = blake3::hash(kp.verifying_key.as_bytes());
    let group_id = blake3::derive_key(
        ochra_crypto::blake3::contexts::GROUP_SETTINGS_KEY,
        &owner_pik,
    );

    let split = RevenueSplitConfig {
        host_pct: 60,
        creator_pct: 30,
        network_pct: 10,
    };
    splits::validate_split(&split).expect("60/30/10 split should be valid");

    setup_space_with_split(&conn, &group_id, &owner_pik, &split);

    // =========================================================
    // Publish content with multiple pricing tiers
    // =========================================================
    let pricing_json = serde_json::json!([
        {"tier_type": "permanent", "price_seeds": 10_000_000_000_u64},
        {"tier_type": "rental", "price_seeds": 2_000_000_000_u64, "rental_days": 30}
    ])
    .to_string();

    let content_hash = blake3::hash(b"test-content-for-revenue");
    content::insert(
        &conn,
        &content_hash,
        &group_id,
        "Revenue Test Content",
        Some("Content for revenue split testing"),
        &pricing_json,
        &owner_pik,
        &blake3::hash(b"key-commitment"),
        1024 * 1024,
        1,
        BASE_TIME + 100,
    )
    .expect("Content insertion should succeed");

    // =========================================================
    // Purchase at permanent tier (10 Seeds)
    // =========================================================
    let permanent_price = 10 * ochra_types::MICRO_SEEDS_PER_SEED;
    let (host_share, creator_share, network_share) =
        simulate_purchase(&conn, permanent_price, &split, 1);

    // Verify exact amounts.
    assert_eq!(
        host_share,
        permanent_price * 60 / 100,
        "Host should receive 60% of permanent price"
    );
    assert_eq!(
        network_share,
        permanent_price * 10 / 100,
        "Network should receive 10% of permanent price"
    );
    assert_eq!(
        creator_share,
        permanent_price - host_share - network_share,
        "Creator should receive remainder (30%)"
    );
    assert_eq!(
        host_share + creator_share + network_share,
        permanent_price,
        "All shares must sum to purchase price"
    );

    // =========================================================
    // Purchase at rental tier (2 Seeds)
    // =========================================================
    let rental_price = 2 * ochra_types::MICRO_SEEDS_PER_SEED;
    let (host_r, creator_r, network_r) =
        simulate_purchase(&conn, rental_price, &split, 2);

    assert_eq!(host_r, rental_price * 60 / 100);
    assert_eq!(network_r, rental_price * 10 / 100);
    assert_eq!(creator_r, rental_price - host_r - network_r);
    assert_eq!(host_r + creator_r + network_r, rental_price);

    // =========================================================
    // Verify cumulative earnings
    // =========================================================
    let total_revenue = permanent_price + rental_price;
    let total_host = host_share + host_r;
    let total_creator = creator_share + creator_r;
    let total_network = network_share + network_r;

    assert_eq!(
        total_host + total_creator + total_network,
        total_revenue,
        "Cumulative shares must equal total revenue"
    );

    // Host should have exactly 60% of total.
    assert_eq!(
        total_host,
        total_revenue * 60 / 100,
        "Cumulative host share must be 60%"
    );
}

#[tokio::test]
#[ignore]
async fn revenue_split_default_10_70_20() {
    // Test the default split from the spec.
    let default = splits::DEFAULT_SPLIT;
    assert_eq!(default.host_pct, 10);
    assert_eq!(default.creator_pct, 70);
    assert_eq!(default.network_pct, 20);
    splits::validate_split(&default).expect("Default split should be valid");

    // Distribute 100 Seeds.
    let amount = 100 * ochra_types::MICRO_SEEDS_PER_SEED;
    let (host, creator, network) = splits::distribute(amount, &default)
        .expect("Distribution should succeed");

    assert_eq!(host, amount * 10 / 100, "Host should get 10%");
    assert_eq!(network, amount * 20 / 100, "Network should get 20%");
    assert_eq!(creator, amount - host - network, "Creator gets remainder");
    assert_eq!(host + creator + network, amount, "Shares must sum to total");
}

#[tokio::test]
#[ignore]
async fn revenue_split_rounding_correctness() {
    // Test rounding with amounts that don't divide evenly.
    let split = RevenueSplitConfig {
        host_pct: 33,
        creator_pct: 34,
        network_pct: 33,
    };
    splits::validate_split(&split).expect("33/34/33 split should be valid");

    // Amount that produces rounding: 100 micro-seeds.
    let amount = 100u64;
    let (host, creator, network) = splits::distribute(amount, &split)
        .expect("Distribution should succeed");

    // Host: 100 * 33 / 100 = 33.
    assert_eq!(host, 33, "Host should get floor(33%)");
    // Network: 100 * 33 / 100 = 33.
    assert_eq!(network, 33, "Network should get floor(33%)");
    // Creator: remainder = 100 - 33 - 33 = 34.
    assert_eq!(creator, 34, "Creator should get remainder");
    assert_eq!(
        host + creator + network,
        amount,
        "Must sum to total (no loss)"
    );

    // Odd amount: 1 micro-seed.
    let (h1, c1, n1) = splits::distribute(1, &split)
        .expect("Distribution of 1 should succeed");
    assert_eq!(
        h1 + c1 + n1,
        1,
        "Distributing 1 micro-seed must not lose or gain"
    );

    // Large amount.
    let large = u64::MAX / 200; // Avoid overflow.
    let (hl, cl, nl) = splits::distribute(large, &split)
        .expect("Large distribution should succeed");
    assert_eq!(
        hl + cl + nl,
        large,
        "Large amount shares must sum to total"
    );
}

#[tokio::test]
#[ignore]
async fn revenue_split_extreme_configurations() {
    // 100% creator split.
    let all_creator = RevenueSplitConfig {
        host_pct: 0,
        creator_pct: 100,
        network_pct: 0,
    };
    splits::validate_split(&all_creator).expect("0/100/0 should be valid");

    let amount = 500 * ochra_types::MICRO_SEEDS_PER_SEED;
    let (h, c, n) = splits::distribute(amount, &all_creator)
        .expect("Distribution should succeed");
    assert_eq!(h, 0, "Host gets nothing");
    assert_eq!(c, amount, "Creator gets everything");
    assert_eq!(n, 0, "Network gets nothing");

    // 100% host split.
    let all_host = RevenueSplitConfig {
        host_pct: 100,
        creator_pct: 0,
        network_pct: 0,
    };
    splits::validate_split(&all_host).expect("100/0/0 should be valid");

    let (h2, c2, n2) = splits::distribute(amount, &all_host)
        .expect("Distribution should succeed");
    assert_eq!(h2, amount, "Host gets everything");
    assert_eq!(c2, 0, "Creator gets nothing");
    assert_eq!(n2, 0, "Network gets nothing");

    // 100% network split.
    let all_network = RevenueSplitConfig {
        host_pct: 0,
        creator_pct: 0,
        network_pct: 100,
    };
    splits::validate_split(&all_network).expect("0/0/100 should be valid");

    let (h3, c3, n3) = splits::distribute(amount, &all_network)
        .expect("Distribution should succeed");
    assert_eq!(h3, 0);
    // Creator gets remainder = amount - 0 - amount = 0.
    assert_eq!(c3, 0, "Creator gets nothing with 0%");
    assert_eq!(n3, amount, "Network gets everything");
}

#[tokio::test]
#[ignore]
async fn revenue_split_validation_failures() {
    // Sum > 100.
    let over = RevenueSplitConfig {
        host_pct: 40,
        creator_pct: 40,
        network_pct: 30,
    };
    assert!(
        splits::validate_split(&over).is_err(),
        "Split summing to 110 should fail"
    );

    // Sum < 100.
    let under = RevenueSplitConfig {
        host_pct: 10,
        creator_pct: 10,
        network_pct: 10,
    };
    assert!(
        splits::validate_split(&under).is_err(),
        "Split summing to 30 should fail"
    );

    // Zero amount distribution.
    let valid_split = splits::DEFAULT_SPLIT;
    let zero_result = splits::distribute(0, &valid_split);
    assert!(
        zero_result.is_err(),
        "Distributing zero amount should fail"
    );
}

#[tokio::test]
#[ignore]
async fn revenue_split_timelock_enforcement() {
    // =========================================================
    // Test the 30-day timelock for split changes
    // =========================================================
    let current = splits::DEFAULT_SPLIT;
    let proposed = RevenueSplitConfig {
        host_pct: 60,
        creator_pct: 30,
        network_pct: 10,
    };

    let proposal = splits::propose_split_change(&current, proposed.clone(), BASE_TIME)
        .expect("Proposal should succeed");

    assert_eq!(proposal.proposed_at, BASE_TIME);
    assert_eq!(
        proposal.effective_at,
        BASE_TIME + TIMELOCK_SECONDS,
        "Effective time must be 30 days after proposal"
    );

    // Not effective immediately.
    assert!(
        !splits::is_effective(&proposal, BASE_TIME),
        "Proposal should not be effective immediately"
    );

    // Not effective 29 days later.
    assert!(
        !splits::is_effective(&proposal, BASE_TIME + TIMELOCK_SECONDS - 1),
        "Proposal should not be effective before 30 days"
    );

    // Effective at exactly 30 days.
    assert!(
        splits::is_effective(&proposal, BASE_TIME + TIMELOCK_SECONDS),
        "Proposal should be effective at exactly 30 days"
    );

    // Effective after 30 days.
    assert!(
        splits::is_effective(&proposal, BASE_TIME + TIMELOCK_SECONDS + 1),
        "Proposal should be effective after 30 days"
    );

    // Verify the timelock constant.
    assert_eq!(
        TIMELOCK_SECONDS,
        30 * 24 * 3600,
        "Timelock should be exactly 30 days in seconds"
    );
}

#[tokio::test]
#[ignore]
async fn revenue_split_identical_change_rejected() {
    // Proposing the same split should fail.
    let current = splits::DEFAULT_SPLIT;
    let identical = RevenueSplitConfig {
        host_pct: 10,
        creator_pct: 70,
        network_pct: 20,
    };

    let result = splits::propose_split_change(&current, identical, BASE_TIME);
    assert!(
        result.is_err(),
        "Proposing identical split should be rejected"
    );
}

#[tokio::test]
#[ignore]
async fn revenue_split_invalid_change_rejected() {
    // Proposing an invalid split (sum != 100) should fail.
    let current = splits::DEFAULT_SPLIT;
    let invalid = RevenueSplitConfig {
        host_pct: 50,
        creator_pct: 50,
        network_pct: 50,
    };

    let result = splits::propose_split_change(&current, invalid, BASE_TIME);
    assert!(
        result.is_err(),
        "Proposing invalid split should be rejected"
    );
}

#[tokio::test]
#[ignore]
async fn revenue_split_database_consistency() {
    // =========================================================
    // Verify split state is correctly persisted in the database
    // =========================================================
    let conn = ochra_db::open_memory().expect("open DB");
    let kp = ed25519::KeyPair::generate();
    let owner_pik = blake3::hash(kp.verifying_key.as_bytes());
    let group_id = blake3::derive_key(
        ochra_crypto::blake3::contexts::GROUP_SETTINGS_KEY,
        &owner_pik,
    );

    let split = RevenueSplitConfig {
        host_pct: 60,
        creator_pct: 30,
        network_pct: 10,
    };

    setup_space_with_split(&conn, &group_id, &owner_pik, &split);

    // Query the stored split values.
    let (stored_host, stored_creator, stored_network): (i64, i64, i64) = conn
        .query_row(
            "SELECT owner_pct, pub_pct, abr_pct FROM spaces WHERE group_id = ?1",
            [group_id.as_slice()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("Split query should succeed");

    assert_eq!(stored_host, 60, "Stored host_pct should be 60");
    assert_eq!(stored_creator, 30, "Stored creator_pct should be 30");
    assert_eq!(stored_network, 10, "Stored network_pct should be 10");

    // Reconstruct the split from DB values and verify it works.
    let db_split = RevenueSplitConfig {
        host_pct: stored_host as u8,
        creator_pct: stored_creator as u8,
        network_pct: stored_network as u8,
    };
    splits::validate_split(&db_split).expect("DB-loaded split should be valid");

    let amount = 1000 * ochra_types::MICRO_SEEDS_PER_SEED;
    let (h, c, n) = splits::distribute(amount, &db_split)
        .expect("Distribution with DB split should succeed");
    assert_eq!(h + c + n, amount, "DB split shares must sum to total");
}

#[tokio::test]
#[ignore]
async fn revenue_split_multiple_purchases() {
    // Verify that multiple purchases accumulate correctly.
    let conn = ochra_db::open_memory().expect("open DB");
    let kp = ed25519::KeyPair::generate();
    let owner_pik = blake3::hash(kp.verifying_key.as_bytes());
    let group_id = blake3::derive_key(
        ochra_crypto::blake3::contexts::GROUP_SETTINGS_KEY,
        &owner_pik,
    );

    let split = RevenueSplitConfig {
        host_pct: 60,
        creator_pct: 30,
        network_pct: 10,
    };

    setup_space_with_split(&conn, &group_id, &owner_pik, &split);

    // Simulate 100 purchases of varying amounts.
    let mut total_host = 0u64;
    let mut total_creator = 0u64;
    let mut total_network = 0u64;
    let mut total_revenue = 0u64;

    for i in 1..=100u8 {
        let amount = u64::from(i) * ochra_types::MICRO_SEEDS_PER_SEED;
        let (h, c, n) = simulate_purchase(&conn, amount, &split, i);
        total_host += h;
        total_creator += c;
        total_network += n;
        total_revenue += amount;
    }

    // Total should sum perfectly.
    assert_eq!(
        total_host + total_creator + total_network,
        total_revenue,
        "Cumulative shares across 100 purchases must equal total revenue"
    );

    // Verify transaction count in DB.
    let tx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transaction_history",
            [],
            |row| row.get(0),
        )
        .expect("Transaction count query should succeed");
    assert_eq!(
        tx_count, 100,
        "Should have 100 transaction records"
    );
}
