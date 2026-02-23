//! Integration test: Network formation and DHT bootstrap.
//!
//! Exercises the complete network bootstrap lifecycle:
//! 1. Create 3 separate nodes with independent routing tables
//! 2. Bootstrap DHT by having each node learn about the others
//! 3. Store and retrieve data via DHT record store
//! 4. Verify Kademlia routing table convergence
//! 5. Test FIND_NODE lookup
//! 6. Test mutable record updates and sequence ordering
//! 7. Test node discovery via closest-node queries
//!
//! This test uses ochra-dht (kademlia, bep44) and ochra-crypto
//! (ed25519, blake3) without any network I/O.

use std::net::SocketAddr;
use std::time::Duration;

use ochra_crypto::blake3;
use ochra_crypto::ed25519;
use ochra_dht::bep44::{self, DhtRecord, RecordStore};
use ochra_dht::kademlia::{AddNodeResult, FindNodeLookup, NodeId, NodeInfo, RoutingTable};

/// Create a test NodeInfo with a deterministic ID and address.
fn make_node_info(id_byte: u8) -> NodeInfo {
    let kp = ed25519::KeyPair::from_bytes(&[id_byte; 32]);
    let node_id = blake3::hash(kp.verifying_key.as_bytes());
    NodeInfo {
        node_id,
        addr: SocketAddr::from(([10, 0, 0, id_byte], 4433 + u16::from(id_byte))),
        pik_public_key: kp.verifying_key.to_bytes(),
        x25519_public_key: [id_byte; 32],
    }
}

/// Create a node info with a specific node ID (for controlled distance tests).
fn make_node_with_id(id: NodeId, port: u16) -> NodeInfo {
    NodeInfo {
        node_id: id,
        addr: SocketAddr::from(([127, 0, 0, 1], port)),
        pik_public_key: [0u8; 32],
        x25519_public_key: [0u8; 32],
    }
}

#[tokio::test]
#[ignore]
async fn network_bootstrap_three_nodes() {
    // =========================================================
    // Step 1: Create 3 separate nodes
    // =========================================================
    let node_a = make_node_info(1);
    let node_b = make_node_info(2);
    let node_c = make_node_info(3);

    // Each node gets its own routing table.
    let mut rt_a = RoutingTable::new(node_a.node_id);
    let mut rt_b = RoutingTable::new(node_b.node_id);
    let mut rt_c = RoutingTable::new(node_c.node_id);

    assert!(rt_a.is_empty(), "Node A routing table should start empty");
    assert!(rt_b.is_empty(), "Node B routing table should start empty");
    assert!(rt_c.is_empty(), "Node C routing table should start empty");

    // =========================================================
    // Step 2: Bootstrap -- each node learns about the others
    // =========================================================
    // Node A learns about B and C.
    let result_ab = rt_a.add_node(node_b.clone());
    assert!(
        matches!(result_ab, AddNodeResult::Inserted),
        "Node A should insert Node B"
    );
    let result_ac = rt_a.add_node(node_c.clone());
    assert!(
        matches!(result_ac, AddNodeResult::Inserted),
        "Node A should insert Node C"
    );

    // Node B learns about A and C.
    let result_ba = rt_b.add_node(node_a.clone());
    assert!(
        matches!(result_ba, AddNodeResult::Inserted),
        "Node B should insert Node A"
    );
    let result_bc = rt_b.add_node(node_c.clone());
    assert!(
        matches!(result_bc, AddNodeResult::Inserted),
        "Node B should insert Node C"
    );

    // Node C learns about A and B.
    let result_ca = rt_c.add_node(node_a.clone());
    assert!(
        matches!(result_ca, AddNodeResult::Inserted),
        "Node C should insert Node A"
    );
    let result_cb = rt_c.add_node(node_b.clone());
    assert!(
        matches!(result_cb, AddNodeResult::Inserted),
        "Node C should insert Node B"
    );

    // =========================================================
    // Step 3: Verify routing table convergence
    // =========================================================
    assert_eq!(rt_a.len(), 2, "Node A should know 2 peers");
    assert_eq!(rt_b.len(), 2, "Node B should know 2 peers");
    assert_eq!(rt_c.len(), 2, "Node C should know 2 peers");

    // Each node's routing table should not contain itself.
    assert_eq!(
        rt_a.local_id(),
        &node_a.node_id,
        "Node A local ID should match"
    );

    // Adding self should be ignored.
    let self_result = rt_a.add_node(node_a.clone());
    assert!(
        matches!(self_result, AddNodeResult::Ignored),
        "Adding self to routing table should be ignored"
    );

    // =========================================================
    // Step 4: Find closest nodes
    // =========================================================
    // From Node A's perspective, find closest to Node B's ID.
    let closest_to_b = rt_a.find_closest(&node_b.node_id, 5);
    assert!(
        !closest_to_b.is_empty(),
        "Should find at least one node close to B"
    );
    assert_eq!(
        closest_to_b[0].node_id, node_b.node_id,
        "Closest node to B's ID should be B itself"
    );

    // Find closest to a random target.
    let random_target = blake3::hash(b"random search target");
    let closest_random = rt_a.find_closest(&random_target, 10);
    assert_eq!(
        closest_random.len(),
        2,
        "Should find both peers when searching for random target"
    );

    // Results should be sorted by XOR distance.
    let d0 = RoutingTable::xor_distance(&closest_random[0].node_id, &random_target);
    let d1 = RoutingTable::xor_distance(&closest_random[1].node_id, &random_target);
    assert!(
        d0 <= d1,
        "Results must be sorted by ascending XOR distance"
    );

    // =========================================================
    // Step 5: XOR distance properties
    // =========================================================
    // Self-distance is zero.
    let self_dist = RoutingTable::xor_distance(&node_a.node_id, &node_a.node_id);
    assert_eq!(
        self_dist,
        [0u8; 32],
        "XOR distance to self must be zero"
    );

    // Symmetry: d(A,B) == d(B,A).
    let d_ab = RoutingTable::xor_distance(&node_a.node_id, &node_b.node_id);
    let d_ba = RoutingTable::xor_distance(&node_b.node_id, &node_a.node_id);
    assert_eq!(
        d_ab, d_ba,
        "XOR distance must be symmetric"
    );

    // Non-zero for different nodes.
    assert_ne!(
        d_ab,
        [0u8; 32],
        "XOR distance between different nodes must be non-zero"
    );

    // =========================================================
    // Step 6: Node update (existing node re-added)
    // =========================================================
    let update_result = rt_a.add_node(node_b.clone());
    assert!(
        matches!(update_result, AddNodeResult::Updated),
        "Re-adding an existing node should update, not insert"
    );
    assert_eq!(
        rt_a.len(),
        2,
        "Table size should not change after update"
    );

    // =========================================================
    // Step 7: Node removal
    // =========================================================
    let removed = rt_a.remove_node(&node_b.node_id);
    assert!(removed.is_some(), "Node B should be removable");
    assert_eq!(rt_a.len(), 1, "Table should have 1 node after removal");

    // Re-add for subsequent tests.
    rt_a.add_node(node_b.clone());
    assert_eq!(rt_a.len(), 2, "Table should have 2 nodes after re-add");
}

#[tokio::test]
#[ignore]
async fn dht_record_store_immutable() {
    // =========================================================
    // Store and retrieve immutable DHT records
    // =========================================================
    let mut store = RecordStore::new();
    assert!(store.is_empty(), "Store should start empty");

    // Create immutable records.
    let data_a = b"Hello, Ochra DHT!".to_vec();
    let data_b = b"Another record in the DHT".to_vec();

    let record_a = bep44::create_immutable_record(data_a.clone())
        .expect("Immutable record creation should succeed");
    let record_b = bep44::create_immutable_record(data_b.clone())
        .expect("Immutable record creation should succeed");

    // Storage key is content-addressed (BLAKE3(value)).
    let key_a = record_a.storage_key();
    let key_b = record_b.storage_key();
    assert_eq!(
        key_a,
        blake3::hash(&data_a),
        "Immutable record key must equal BLAKE3(value)"
    );
    assert_ne!(key_a, key_b, "Different records must have different keys");

    // Store records.
    store.put(record_a).expect("Store record A should succeed");
    store.put(record_b).expect("Store record B should succeed");

    assert_eq!(store.len(), 2, "Store should contain 2 records");

    // Retrieve records.
    let retrieved_a = store.get(&key_a).expect("Record A should be retrievable");
    assert_eq!(
        retrieved_a.value(),
        data_a.as_slice(),
        "Retrieved value must match original"
    );

    let retrieved_b = store.get(&key_b).expect("Record B should be retrievable");
    assert_eq!(
        retrieved_b.value(),
        data_b.as_slice(),
        "Retrieved value must match original"
    );

    // Non-existent key returns None.
    let missing = store.get(&[0xFFu8; 32]);
    assert!(missing.is_none(), "Non-existent key should return None");

    // List all keys.
    let keys = store.keys();
    assert_eq!(keys.len(), 2, "Should have 2 keys");
    assert!(keys.contains(&key_a), "Keys should contain key A");
    assert!(keys.contains(&key_b), "Keys should contain key B");
}

#[tokio::test]
#[ignore]
async fn dht_record_store_mutable() {
    // =========================================================
    // Signed mutable records with sequence ordering
    // =========================================================
    let mut store = RecordStore::new();
    let kp = ed25519::KeyPair::generate();
    let salt = b"test-mutable-salt";

    // Create mutable record with seq=1.
    let record_v1 = bep44::create_mutable_record(
        &kp.signing_key,
        salt,
        1,
        b"value version 1".to_vec(),
    )
    .expect("Mutable record v1 creation should succeed");

    let key = record_v1.storage_key();

    // Verify the storage key derivation.
    let expected_key = {
        let mut input = Vec::new();
        input.extend_from_slice(&kp.verifying_key.to_bytes());
        input.extend_from_slice(salt);
        blake3::hash(&input)
    };
    assert_eq!(
        key, expected_key,
        "Mutable record key must equal BLAKE3(pubkey || salt)"
    );

    // Record validates (signature check).
    record_v1
        .validate()
        .expect("Mutable record v1 should validate");

    store
        .put(record_v1)
        .expect("Store mutable record v1 should succeed");

    // Update to seq=2 should succeed.
    let record_v2 = bep44::create_mutable_record(
        &kp.signing_key,
        salt,
        2,
        b"value version 2".to_vec(),
    )
    .expect("Mutable record v2 creation should succeed");

    store
        .put(record_v2)
        .expect("Store mutable record v2 should succeed");

    let retrieved = store.get(&key).expect("Updated record should exist");
    assert_eq!(
        retrieved.value(),
        b"value version 2",
        "Retrieved value should be the updated version"
    );

    // Stale sequence (seq=1) should be rejected.
    let stale_record = bep44::create_mutable_record(
        &kp.signing_key,
        salt,
        1,
        b"stale attempt".to_vec(),
    )
    .expect("Stale record creation should succeed");

    let stale_result = store.put(stale_record);
    assert!(
        stale_result.is_err(),
        "Stale sequence number must be rejected"
    );

    // Equal sequence (seq=2) should also be rejected.
    let equal_seq_record = bep44::create_mutable_record(
        &kp.signing_key,
        salt,
        2,
        b"equal seq attempt".to_vec(),
    )
    .expect("Equal-seq record creation should succeed");

    let equal_result = store.put(equal_seq_record);
    assert!(
        equal_result.is_err(),
        "Equal sequence number must be rejected"
    );

    // Forward sequence (seq=3) should succeed.
    let record_v3 = bep44::create_mutable_record(
        &kp.signing_key,
        salt,
        3,
        b"value version 3".to_vec(),
    )
    .expect("Mutable record v3 creation should succeed");

    store
        .put(record_v3)
        .expect("Store mutable record v3 should succeed");

    let final_val = store.get(&key).expect("v3 record should exist");
    assert_eq!(final_val.value(), b"value version 3");
}

#[tokio::test]
#[ignore]
async fn dht_record_store_tamper_detection() {
    // Verify that tampered mutable records are rejected.
    let kp = ed25519::KeyPair::generate();

    let mut record = bep44::create_mutable_record(
        &kp.signing_key,
        b"salt",
        1,
        b"original value".to_vec(),
    )
    .expect("Record creation should succeed");

    // Tamper with the value.
    if let DhtRecord::Mutable { ref mut value, .. } = record {
        value[0] ^= 0xFF;
    }

    let result = record.validate();
    assert!(
        result.is_err(),
        "Tampered mutable record must fail validation"
    );

    // A different signer cannot publish to the same key.
    let other_kp = ed25519::KeyPair::generate();
    let other_record = bep44::create_mutable_record(
        &other_kp.signing_key,
        b"salt",
        1,
        b"impersonation attempt".to_vec(),
    )
    .expect("Record creation should succeed");

    // The record validates under its own key but has a different storage key.
    other_record
        .validate()
        .expect("Record validates under other key");
    assert_ne!(
        record.storage_key(),
        other_record.storage_key(),
        "Different signers must produce different storage keys"
    );
}

#[tokio::test]
#[ignore]
async fn dht_record_store_expiration() {
    // Verify TTL-based expiration.
    let mut store = RecordStore::with_ttl(Duration::from_millis(50));

    let record = bep44::create_immutable_record(b"ephemeral data".to_vec())
        .expect("Record creation should succeed");
    let key = record.storage_key();

    store.put(record).expect("Put should succeed");
    assert_eq!(store.len(), 1, "Store should have 1 record");

    // Record is accessible immediately.
    assert!(store.get(&key).is_some(), "Record should be accessible before TTL");

    // Wait for TTL to expire.
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Record should be gone.
    assert!(
        store.get(&key).is_none(),
        "Expired record should not be accessible"
    );

    // Explicit expiration cleanup.
    let expired_count = store.expire();
    assert_eq!(expired_count, 1, "One record should have been expired");
    assert!(store.is_empty(), "Store should be empty after expiration");
}

#[tokio::test]
#[ignore]
async fn find_node_lookup_convergence() {
    // =========================================================
    // Simulate a FIND_NODE lookup with 10 nodes
    // =========================================================
    let local_id = blake3::hash(b"local node");
    let mut rt = RoutingTable::new(local_id);

    // Add 10 nodes to the routing table.
    let mut all_nodes = Vec::new();
    for i in 1..=10u8 {
        let node = make_node_info(i);
        rt.add_node(node.clone());
        all_nodes.push(node);
    }
    assert_eq!(rt.len(), 10, "Routing table should have 10 nodes");

    // Target is a random hash.
    let target = blake3::hash(b"find-this-node");

    // Get seed nodes from routing table.
    let seed_nodes = rt.find_closest(&target, 3);
    assert!(
        !seed_nodes.is_empty(),
        "Should have seed nodes for lookup"
    );

    let mut lookup = FindNodeLookup::new(target, seed_nodes);
    assert!(
        !lookup.is_complete(),
        "Lookup should not be complete initially"
    );

    // Simulate iterative lookup.
    let mut iteration = 0;
    loop {
        let batch = lookup.next_queries();
        if batch.is_empty() {
            break;
        }

        // Simulate responses: each queried node returns 3 closest from our pool.
        for queried in &batch {
            // Find 3 closest to target from the perspective of the queried node.
            let response: Vec<NodeInfo> = all_nodes
                .iter()
                .filter(|n| n.node_id != queried.node_id)
                .take(3)
                .cloned()
                .collect();
            lookup.add_responses(response);
        }

        iteration += 1;
        if iteration > 20 {
            break; // Safety limit.
        }
    }

    assert!(
        lookup.is_complete(),
        "Lookup should converge after iterative querying"
    );

    let results = lookup.results();
    assert!(
        !results.is_empty(),
        "Lookup should produce results"
    );

    // Results should be sorted by distance to target.
    for i in 0..results.len().saturating_sub(1) {
        let d_i = RoutingTable::xor_distance(&results[i].node_id, &target);
        let d_next = RoutingTable::xor_distance(&results[i + 1].node_id, &target);
        assert!(
            d_i <= d_next,
            "Lookup results must be sorted by XOR distance to target"
        );
    }
}

#[tokio::test]
#[ignore]
async fn routing_table_bucket_diversity() {
    // Verify that nodes at different distances land in different buckets.
    let local_id = [0x00u8; 32];
    let rt = RoutingTable::new(local_id);

    // Node with first bit set -> bucket 0 (most distant).
    let mut far_id = [0x00u8; 32];
    far_id[0] = 0x80;
    assert_eq!(
        rt.bucket_index(&far_id),
        Some(0),
        "Node with MSB set should land in bucket 0"
    );

    // Node with only last bit set -> bucket 255 (closest).
    let mut close_id = [0x00u8; 32];
    close_id[31] = 0x01;
    assert_eq!(
        rt.bucket_index(&close_id),
        Some(255),
        "Node with LSB set should land in bucket 255"
    );

    // Self (identical ID) -> no bucket.
    assert_eq!(
        rt.bucket_index(&local_id),
        None,
        "Self ID should not map to any bucket"
    );

    // Bucket index is determined by leading zeros of XOR distance.
    let mut mid_id = [0x00u8; 32];
    mid_id[16] = 0x01;
    let bucket = rt.bucket_index(&mid_id);
    assert_eq!(
        bucket,
        Some(128 + 7),
        "Node with bit 135 set (byte 16, bit 7) should be in bucket 135"
    );
}

#[tokio::test]
#[ignore]
async fn bucket_full_eviction() {
    // Test k-bucket overflow and eviction.
    let local_id = [0x00u8; 32];
    let mut rt = RoutingTable::new(local_id);

    // Fill bucket 0 (nodes with first bit set).
    let k = ochra_dht::K;
    let mut node_ids = Vec::new();
    for i in 0..k {
        let mut id = [0x80u8; 32];
        id[31] = i as u8;
        let node = make_node_with_id(id, 5000 + i as u16);
        let result = rt.add_node(node);
        assert!(
            matches!(result, AddNodeResult::Inserted),
            "Should insert node {i}"
        );
        node_ids.push(id);
    }

    assert_eq!(rt.len(), k, "Routing table should have K nodes");

    // Next insert into the same bucket should report BucketFull.
    let mut overflow_id = [0x80u8; 32];
    overflow_id[31] = k as u8;
    let overflow_node = make_node_with_id(overflow_id, 6000);
    let result = rt.add_node(overflow_node.clone());

    match result {
        AddNodeResult::BucketFull {
            least_recently_seen,
        } => {
            // Simulate failed ping to LRS node -> evict and insert new node.
            rt.evict_and_insert(&least_recently_seen.node_id, overflow_node)
                .expect("Eviction should succeed");
            assert_eq!(
                rt.len(),
                k,
                "Table size should remain K after eviction"
            );
        }
        other => panic!("Expected BucketFull, got {:?}", other),
    }
}
