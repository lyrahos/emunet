//! Integration test: Sphinx circuit construction, rotation, and cover traffic.
//!
//! Exercises the complete onion routing lifecycle:
//! 1. Create relay descriptors with distinct network properties
//! 2. Build a 3-hop Sphinx circuit via CircuitBuilder
//! 3. Verify hop key derivation (each hop gets unique keys)
//! 4. Verify circuit identity and lifetime management
//! 5. Test relay selection with constraint enforcement
//! 6. Verify fixed 8192-byte packet size for cover traffic
//! 7. Test cover traffic generation and detection
//! 8. Test circuit rotation readiness
//!
//! This test uses ochra-onion (circuit, relay, cover) and
//! ochra-crypto (x25519, blake3) without any network I/O.

use std::collections::HashSet;

use ochra_crypto::x25519;
use ochra_onion::circuit::{self, CircuitBuilder, HopKeys};
use ochra_onion::cover::{
    self, CoverTrafficConfig, CoverTrafficGenerator, DEFAULT_COVER_INTERVAL_MS,
    MAX_COVER_INTERVAL_MS, MIN_COVER_INTERVAL_MS,
};
use ochra_onion::relay::{RelayCache, RelaySelector, SelectionConstraints};
use ochra_onion::{CIRCUIT_HOPS, CIRCUIT_LIFETIME_SECS, SPHINX_PACKET_SIZE};
use ochra_types::network::RelayDescriptor;

/// Create a relay descriptor with unique identity and network properties.
fn make_relay(id_byte: u8, ip: &str, as_num: u32, country: [u8; 2], score: f32) -> RelayDescriptor {
    let secret = x25519::X25519StaticSecret::random();
    let pk = secret.public_key();
    RelayDescriptor {
        node_id: [id_byte; 32],
        pik_hash: [id_byte; 32],
        x25519_pk: pk.to_bytes(),
        mlkem768_ek: vec![0u8; 1184],
        relay_epoch: 1,
        posrv_score: score,
        ip_addr: ip.to_string(),
        as_number: as_num,
        country_code: country,
        bandwidth_cap_mbps: 100,
        uptime_epochs: 100,
        sig: [0u8; 64],
    }
}

/// Create a relay with a real X25519 keypair for DH testing.
fn make_relay_with_dh(id_byte: u8) -> (RelayDescriptor, x25519::X25519StaticSecret) {
    let secret = x25519::X25519StaticSecret::random();
    let pk = secret.public_key();
    let descriptor = RelayDescriptor {
        node_id: [id_byte; 32],
        pik_hash: [id_byte; 32],
        x25519_pk: pk.to_bytes(),
        mlkem768_ek: vec![0u8; 1184],
        relay_epoch: 1,
        posrv_score: 1.0,
        ip_addr: format!("10.0.{}.1:4433", id_byte),
        as_number: u32::from(id_byte) * 100,
        country_code: [b'U', b'S'],
        bandwidth_cap_mbps: 100,
        uptime_epochs: 100,
        sig: [0u8; 64],
    };
    (descriptor, secret)
}

#[tokio::test]
#[ignore]
async fn circuit_construction_three_hops() {
    // =========================================================
    // Step 1: Create 3 relays with X25519 keys
    // =========================================================
    let (r1, _sk1) = make_relay_with_dh(1);
    let (r2, _sk2) = make_relay_with_dh(2);
    let (r3, _sk3) = make_relay_with_dh(3);

    // =========================================================
    // Step 2: Build circuit
    // =========================================================
    let circuit = CircuitBuilder::new()
        .add_relay(r1)
        .expect("add entry relay")
        .add_relay(r2)
        .expect("add middle relay")
        .add_relay(r3)
        .expect("add exit relay")
        .build()
        .expect("build circuit");

    // =========================================================
    // Step 3: Verify circuit structure
    // =========================================================
    assert_eq!(
        circuit.hops().len(),
        CIRCUIT_HOPS,
        "Circuit must have exactly {} hops",
        CIRCUIT_HOPS
    );
    assert_eq!(CIRCUIT_HOPS, 3, "CIRCUIT_HOPS constant must be 3");

    // Verify hop order: entry, middle, exit.
    assert_eq!(
        circuit.entry_hop().node_id,
        [1u8; 32],
        "Entry hop should be relay 1"
    );
    assert_eq!(
        circuit.middle_hop().node_id,
        [2u8; 32],
        "Middle hop should be relay 2"
    );
    assert_eq!(
        circuit.exit_hop().node_id,
        [3u8; 32],
        "Exit hop should be relay 3"
    );

    // =========================================================
    // Step 4: Verify per-hop key derivation
    // =========================================================
    let h1_keys = &circuit.entry_hop().keys;
    let h2_keys = &circuit.middle_hop().keys;
    let h3_keys = &circuit.exit_hop().keys;

    // Each hop must have different encryption keys.
    assert_ne!(
        h1_keys.hop_key, h2_keys.hop_key,
        "Entry and middle hop keys must differ"
    );
    assert_ne!(
        h2_keys.hop_key, h3_keys.hop_key,
        "Middle and exit hop keys must differ"
    );
    assert_ne!(
        h1_keys.hop_key, h3_keys.hop_key,
        "Entry and exit hop keys must differ"
    );

    // Within each hop, all derived keys must be distinct.
    verify_keys_distinct(h1_keys, "entry");
    verify_keys_distinct(h2_keys, "middle");
    verify_keys_distinct(h3_keys, "exit");

    // =========================================================
    // Step 5: Verify circuit identity
    // =========================================================
    let circuit_id = circuit.circuit_id();
    assert_eq!(circuit_id.len(), 16, "Circuit ID must be 16 bytes");

    // Circuit IDs should be unique between circuits.
    let (r1b, _) = make_relay_with_dh(4);
    let (r2b, _) = make_relay_with_dh(5);
    let (r3b, _) = make_relay_with_dh(6);

    let circuit2 = CircuitBuilder::new()
        .add_relay(r1b)
        .expect("add relay")
        .add_relay(r2b)
        .expect("add relay")
        .add_relay(r3b)
        .expect("add relay")
        .build()
        .expect("build circuit");

    assert_ne!(
        circuit.circuit_id(),
        circuit2.circuit_id(),
        "Different circuits must have different IDs"
    );

    // =========================================================
    // Step 6: Verify circuit is not expired (freshly created)
    // =========================================================
    assert!(
        !circuit.is_expired(),
        "Freshly created circuit must not be expired"
    );
    assert!(
        !circuit::needs_rotation(&circuit),
        "Fresh circuit should not need rotation"
    );
    assert!(
        circuit.remaining_secs() > 0,
        "Fresh circuit should have remaining lifetime"
    );
    assert!(
        circuit.remaining_secs() <= CIRCUIT_LIFETIME_SECS,
        "Remaining seconds should not exceed lifetime"
    );
    assert_eq!(
        CIRCUIT_LIFETIME_SECS, 600,
        "Circuit lifetime should be 10 minutes (600 seconds)"
    );
}

/// Verify that all derived keys within a hop are distinct from each other.
fn verify_keys_distinct(keys: &HopKeys, label: &str) {
    assert_ne!(
        keys.hop_key, keys.hop_mac,
        "{label} hop: key and MAC must differ"
    );
    assert_ne!(
        keys.hop_key, keys.hop_pad,
        "{label} hop: key and pad must differ"
    );
    assert_ne!(
        keys.hop_mac, keys.hop_pad,
        "{label} hop: MAC and pad must differ"
    );
}

#[tokio::test]
#[ignore]
async fn circuit_builder_error_handling() {
    // Too many relays.
    let (r1, _) = make_relay_with_dh(1);
    let (r2, _) = make_relay_with_dh(2);
    let (r3, _) = make_relay_with_dh(3);
    let (r4, _) = make_relay_with_dh(4);

    let result = CircuitBuilder::new()
        .add_relay(r1)
        .expect("add")
        .add_relay(r2)
        .expect("add")
        .add_relay(r3)
        .expect("add")
        .add_relay(r4);

    assert!(
        result.is_err(),
        "Adding 4th relay to 3-hop circuit should fail"
    );

    // Too few relays.
    let (r1b, _) = make_relay_with_dh(5);

    let result2 = CircuitBuilder::new().add_relay(r1b).expect("add").build();

    assert!(
        result2.is_err(),
        "Building circuit with only 1 relay should fail"
    );

    // Empty builder should fail to build.
    let result3 = CircuitBuilder::new().build();
    assert!(
        result3.is_err(),
        "Building circuit with 0 relays should fail"
    );
}

#[tokio::test]
#[ignore]
async fn hop_key_derivation_deterministic() {
    // Verify that derive_hop_keys is deterministic.
    let shared_secret = [0x42u8; 32];
    let keys1 = circuit::derive_hop_keys(&shared_secret);
    let keys2 = circuit::derive_hop_keys(&shared_secret);

    assert_eq!(
        keys1.hop_key, keys2.hop_key,
        "Hop key derivation must be deterministic"
    );
    assert_eq!(keys1.hop_mac, keys2.hop_mac);
    assert_eq!(keys1.hop_pad, keys2.hop_pad);
    assert_eq!(keys1.hop_nonce, keys2.hop_nonce);

    // Different shared secrets produce different keys.
    let keys3 = circuit::derive_hop_keys(&[0x43u8; 32]);
    assert_ne!(
        keys1.hop_key, keys3.hop_key,
        "Different shared secrets must produce different keys"
    );

    // Nonce is exactly 12 bytes (from first 12 bytes of BLAKE3 output).
    assert_eq!(
        keys1.hop_nonce.len(),
        12,
        "Hop nonce must be exactly 12 bytes"
    );
}

#[tokio::test]
#[ignore]
async fn relay_selection_with_constraints() {
    // =========================================================
    // Create a diverse relay pool
    // =========================================================
    let relays = vec![
        make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 2.0),
        make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 3.0),
        make_relay(3, "10.0.3.1:4433", 300, [b'J', b'P'], 1.5),
        make_relay(4, "10.0.4.1:4433", 400, [b'G', b'B'], 4.0),
        make_relay(5, "10.0.5.1:4433", 500, [b'F', b'R'], 2.5),
    ];

    let cache = RelayCache::from_descriptors(relays);
    assert_eq!(cache.len(), 5, "Cache should contain 5 relays");

    // Select 3 relays for a circuit.
    let selector = RelaySelector::new();
    let selected = selector
        .select_relays(&cache)
        .expect("Relay selection should succeed");

    assert_eq!(
        selected.len(),
        CIRCUIT_HOPS,
        "Should select exactly 3 relays"
    );

    // All selected relays must be unique.
    let selected_ids: HashSet<[u8; 32]> = selected.iter().map(|r| r.node_id).collect();
    assert_eq!(
        selected_ids.len(),
        CIRCUIT_HOPS,
        "Selected relays must be unique"
    );

    // No two relays in the same /24 subnet.
    let subnets: HashSet<String> = selected
        .iter()
        .map(|r| {
            let parts: Vec<&str> = r
                .ip_addr
                .split(':')
                .next()
                .unwrap_or("")
                .split('.')
                .collect();
            format!("{}.{}.{}", parts[0], parts[1], parts[2])
        })
        .collect();
    assert_eq!(
        subnets.len(),
        CIRCUIT_HOPS,
        "No two relays should share a /24 subnet"
    );

    // No two relays with the same AS number.
    let as_numbers: HashSet<u32> = selected.iter().map(|r| r.as_number).collect();
    assert_eq!(
        as_numbers.len(),
        CIRCUIT_HOPS,
        "No two relays should share an AS number"
    );
}

#[tokio::test]
#[ignore]
async fn relay_selection_as_exclusion() {
    // Verify AS number exclusion constraint.
    let relays = vec![
        make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0),
        make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 2.0),
        make_relay(3, "10.0.3.1:4433", 300, [b'J', b'P'], 3.0),
        make_relay(4, "10.0.4.1:4433", 400, [b'G', b'B'], 4.0),
    ];

    let cache = RelayCache::from_descriptors(relays);

    let mut constraints = SelectionConstraints::default();
    constraints.excluded_as_numbers.insert(100); // Exclude AS 100

    let selector = RelaySelector::with_constraints(constraints);
    let selected = selector
        .select_relays(&cache)
        .expect("Selection should succeed");

    for relay in &selected {
        assert_ne!(
            relay.as_number, 100,
            "Excluded AS number should not appear in selection"
        );
    }
}

#[tokio::test]
#[ignore]
async fn relay_selection_insufficient_relays() {
    // Not enough relays for a 3-hop circuit.
    let relays = vec![
        make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0),
        make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 2.0),
    ];

    let cache = RelayCache::from_descriptors(relays);
    let selector = RelaySelector::new();
    let result = selector.select_relays(&cache);

    assert!(
        result.is_err(),
        "Selection with fewer than 3 relays should fail"
    );
}

#[tokio::test]
#[ignore]
async fn relay_cache_operations() {
    let mut cache = RelayCache::new();
    assert!(cache.is_empty());

    // Add relays.
    let r1 = make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 1.0);
    cache.add(r1);
    assert_eq!(cache.len(), 1);

    // Update existing relay (same node_id).
    let r1_updated = make_relay(1, "10.0.1.1:4433", 100, [b'U', b'S'], 5.0);
    cache.add(r1_updated);
    assert_eq!(cache.len(), 1, "Update should not increase count");
    assert!(
        (cache.all()[0].posrv_score - 5.0).abs() < f32::EPSILON,
        "Score should be updated"
    );

    // Add more relays.
    cache.add(make_relay(2, "10.0.2.1:4433", 200, [b'D', b'E'], 2.0));
    cache.add(make_relay(3, "10.0.3.1:4433", 300, [b'J', b'P'], 0.5));
    assert_eq!(cache.len(), 3);

    // Filter by minimum score.
    let filtered = cache.filter_by_min_score(1.0);
    assert_eq!(
        filtered.len(),
        2,
        "Only relays with score >= 1.0 should pass"
    );

    // Remove a relay.
    cache.remove(&[2u8; 32]);
    assert_eq!(cache.len(), 2, "Cache should have 2 relays after removal");
}

#[tokio::test]
#[ignore]
async fn sphinx_packet_size_fixed() {
    // Verify the fixed 8192-byte Sphinx packet size.
    assert_eq!(
        SPHINX_PACKET_SIZE, 8192,
        "Sphinx packet size must be exactly 8192 bytes"
    );

    // Cover traffic packets must be exactly this size.
    let config = CoverTrafficConfig::default();
    let generator = CoverTrafficGenerator::new(config, [0xAAu8; 32]);

    let packet = generator
        .generate_packet()
        .expect("Cover packet generation should succeed");

    assert_eq!(
        packet.len(),
        SPHINX_PACKET_SIZE,
        "Cover traffic packet must be exactly {} bytes",
        SPHINX_PACKET_SIZE
    );

    // A second packet should also be the same size (consistency check).
    let packet2 = generator
        .generate_packet()
        .expect("Second packet generation should succeed");
    assert_eq!(packet2.len(), SPHINX_PACKET_SIZE);
}

#[tokio::test]
#[ignore]
async fn cover_traffic_generation_and_detection() {
    let exit_secret = [0xBBu8; 32];
    let config = CoverTrafficConfig::default();
    let generator = CoverTrafficGenerator::new(config, exit_secret);

    assert!(
        generator.is_enabled(),
        "Cover traffic should be enabled by default"
    );

    // =========================================================
    // Generate cover packet
    // =========================================================
    let packet = generator
        .generate_packet()
        .expect("Cover packet should generate");

    assert_eq!(packet.len(), SPHINX_PACKET_SIZE);

    // =========================================================
    // Exit relay detection: verify cover token at offset 512
    // =========================================================
    let cover_token = generator.cover_token();
    let expected_token = cover::derive_cover_token(&exit_secret);
    assert_eq!(
        cover_token, expected_token,
        "Cover token must match derivation"
    );

    // The token should be embedded at offset 512 in the packet.
    let token_offset = 512;
    assert!(
        cover::is_cover_traffic(&packet, &cover_token, token_offset),
        "Generated cover packet should be detectable at offset 512"
    );

    // Wrong token should not detect as cover traffic.
    let wrong_token = cover::derive_cover_token(&[0xCCu8; 32]);
    assert!(
        !cover::is_cover_traffic(&packet, &wrong_token, token_offset),
        "Wrong token should not detect as cover traffic"
    );

    // Real data packet should not be detected as cover.
    let real_packet = vec![0x42u8; SPHINX_PACKET_SIZE];
    assert!(
        !cover::is_cover_traffic(&real_packet, &cover_token, token_offset),
        "Real data should not be detected as cover traffic"
    );
}

#[tokio::test]
#[ignore]
async fn cover_traffic_timing_parameters() {
    // Default config.
    let default_config = CoverTrafficConfig::default();
    assert_eq!(
        default_config.mean_interval_ms, DEFAULT_COVER_INTERVAL_MS,
        "Default mean interval should be 500ms"
    );
    assert!(default_config.enabled);

    // Clamping: too low.
    let low_config = CoverTrafficConfig::new(10);
    assert_eq!(
        low_config.mean_interval_ms, MIN_COVER_INTERVAL_MS,
        "Below-minimum should be clamped to {}ms",
        MIN_COVER_INTERVAL_MS
    );

    // Clamping: too high.
    let high_config = CoverTrafficConfig::new(100_000);
    assert_eq!(
        high_config.mean_interval_ms, MAX_COVER_INTERVAL_MS,
        "Above-maximum should be clamped to {}ms",
        MAX_COVER_INTERVAL_MS
    );

    // Disabled config.
    let disabled_config = CoverTrafficConfig::disabled();
    assert!(!disabled_config.enabled);

    // Poisson delay computation bounds.
    for u in [0.01, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99] {
        let delay = cover::next_cover_delay_ms(500, u);
        assert!(
            delay >= MIN_COVER_INTERVAL_MS,
            "Delay {delay}ms must be >= {}ms for u={u}",
            MIN_COVER_INTERVAL_MS
        );
        assert!(
            delay <= MAX_COVER_INTERVAL_MS,
            "Delay {delay}ms must be <= {}ms for u={u}",
            MAX_COVER_INTERVAL_MS
        );
    }
}

#[tokio::test]
#[ignore]
async fn cover_traffic_secret_rotation() {
    let config = CoverTrafficConfig::default();
    let mut generator = CoverTrafficGenerator::new(config, [0x01u8; 32]);

    let token1 = generator.cover_token();

    // After circuit rotation, update the exit secret.
    generator.set_exit_secret([0x02u8; 32]);
    let token2 = generator.cover_token();

    assert_ne!(
        token1, token2,
        "Cover token must change when exit secret is rotated"
    );

    // Packets generated after rotation should use the new token.
    let new_packet = generator
        .generate_packet()
        .expect("Packet generation should succeed after rotation");

    assert!(
        cover::is_cover_traffic(&new_packet, &token2, 512),
        "New packet should use updated cover token"
    );
    assert!(
        !cover::is_cover_traffic(&new_packet, &token1, 512),
        "New packet should not match old cover token"
    );
}

#[tokio::test]
#[ignore]
async fn cover_traffic_disablement() {
    let config = CoverTrafficConfig::default();
    let mut generator = CoverTrafficGenerator::new(config, [0x01u8; 32]);

    assert!(generator.is_enabled());

    generator.set_config(CoverTrafficConfig::disabled());
    assert!(
        !generator.is_enabled(),
        "Generator should be disabled after config update"
    );

    // Re-enable.
    generator.set_config(CoverTrafficConfig::default());
    assert!(generator.is_enabled());
}

#[tokio::test]
#[ignore]
async fn cover_token_derivation_deterministic() {
    // Same secret should always produce same token.
    let secret = [0xABu8; 32];
    let token1 = cover::derive_cover_token(&secret);
    let token2 = cover::derive_cover_token(&secret);
    assert_eq!(token1, token2, "Token derivation must be deterministic");

    // Different secrets produce different tokens.
    let token3 = cover::derive_cover_token(&[0xCDu8; 32]);
    assert_ne!(
        token1, token3,
        "Different secrets must produce different tokens"
    );
}

#[tokio::test]
#[ignore]
async fn circuit_rotation_readiness() {
    // A fresh circuit should not need rotation.
    let (r1, _) = make_relay_with_dh(1);
    let (r2, _) = make_relay_with_dh(2);
    let (r3, _) = make_relay_with_dh(3);

    let circuit = CircuitBuilder::new()
        .add_relay(r1)
        .expect("add")
        .add_relay(r2)
        .expect("add")
        .add_relay(r3)
        .expect("add")
        .build()
        .expect("build");

    assert!(!circuit::needs_rotation(&circuit));
    assert!(!circuit.is_expired());
    assert!(
        circuit.age_secs() < 5,
        "Fresh circuit should be < 5 seconds old"
    );
    assert!(
        circuit.remaining_secs() > CIRCUIT_LIFETIME_SECS - 5,
        "Fresh circuit should have nearly full lifetime remaining"
    );

    // Verify the ephemeral public key exists and is non-zero.
    let eph_pk = circuit.ephemeral_pk();
    assert_ne!(
        eph_pk.to_bytes(),
        [0u8; 32],
        "Ephemeral public key should not be all zeros"
    );
}
