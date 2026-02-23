//! BEP 44 mutable and immutable record storage.
//!
//! Implements DHT record storage following BEP 44 conventions:
//!
//! - **Immutable records**: keyed by `BLAKE3::hash(value)`. Anyone can publish,
//!   but the key is content-addressed so tampering is detectable.
//! - **Mutable records**: keyed by `BLAKE3::hash(public_key || salt)`. Only the
//!   holder of the corresponding Ed25519 signing key can update the record.
//!   Updates must have a strictly increasing sequence number.
//!
//! Records are stored in a [`RecordStore`] with automatic expiration.

use std::collections::HashMap;
use std::time::{Duration, Instant};


use crate::{DhtError, Result, MAX_RECORD_SIZE};

/// Default record time-to-live (2 hours).
const DEFAULT_TTL_SECS: u64 = 7200;

/// A DHT record, either mutable or immutable.
#[derive(Clone, Debug)]
pub enum DhtRecord {
    /// Immutable record: content-addressed by `BLAKE3::hash(value)`.
    Immutable {
        /// The raw value bytes.
        value: Vec<u8>,
    },
    /// Mutable record: signed by the publisher's Ed25519 key.
    Mutable {
        /// The publisher's Ed25519 public key (32 bytes).
        public_key: [u8; 32],
        /// Optional salt for key derivation (up to 64 bytes).
        salt: Vec<u8>,
        /// Monotonically increasing sequence number.
        seq: u64,
        /// The raw value bytes.
        value: Vec<u8>,
        /// Ed25519 signature over `(salt || seq || value)`.
        signature: [u8; 64],
    },
}

impl DhtRecord {
    /// Compute the storage key for this record.
    ///
    /// - Immutable: `BLAKE3::hash(value)`
    /// - Mutable: `BLAKE3::hash(public_key || salt)`
    pub fn storage_key(&self) -> [u8; 32] {
        match self {
            DhtRecord::Immutable { value } => ochra_crypto::blake3::hash(value),
            DhtRecord::Mutable {
                public_key, salt, ..
            } => {
                let mut input = Vec::with_capacity(32 + salt.len());
                input.extend_from_slice(public_key);
                input.extend_from_slice(salt);
                ochra_crypto::blake3::hash(&input)
            }
        }
    }

    /// Return the value bytes of this record.
    pub fn value(&self) -> &[u8] {
        match self {
            DhtRecord::Immutable { value } => value,
            DhtRecord::Mutable { value, .. } => value,
        }
    }

    /// Return the size of the value in bytes.
    pub fn value_len(&self) -> usize {
        self.value().len()
    }

    /// Validate the record.
    ///
    /// For immutable records, checks the size constraint.
    /// For mutable records, also verifies the Ed25519 signature.
    pub fn validate(&self) -> Result<()> {
        // Size check for all records.
        if self.value_len() > MAX_RECORD_SIZE {
            return Err(DhtError::RecordTooLarge {
                size: self.value_len(),
                max: MAX_RECORD_SIZE,
            });
        }

        match self {
            DhtRecord::Immutable { .. } => Ok(()),
            DhtRecord::Mutable {
                public_key,
                salt,
                seq,
                value,
                signature,
            } => {
                // Verify the Ed25519 signature over (salt || seq || value).
                let vk = ochra_crypto::ed25519::VerifyingKey::from_bytes(public_key)
                    .map_err(DhtError::Crypto)?;
                let sig = ochra_crypto::ed25519::Signature::from_bytes(signature);

                let signed_data = build_signed_data(salt, *seq, value);
                vk.verify(&signed_data, &sig)
                    .map_err(|_| DhtError::InvalidSignature)?;

                Ok(())
            }
        }
    }
}

/// Build the byte string that is signed for mutable records: `salt || seq_be || value`.
fn build_signed_data(salt: &[u8], seq: u64, value: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(salt.len() + 8 + value.len());
    data.extend_from_slice(salt);
    data.extend_from_slice(&seq.to_be_bytes());
    data.extend_from_slice(value);
    data
}

/// Internal storage entry wrapping a record with metadata.
#[derive(Clone, Debug)]
struct StoreEntry {
    /// The DHT record.
    record: DhtRecord,
    /// When this entry was stored.
    stored_at: Instant,
    /// Time-to-live duration for this entry.
    ttl: Duration,
}

impl StoreEntry {
    /// Check if this entry has expired.
    fn is_expired(&self) -> bool {
        self.stored_at.elapsed() > self.ttl
    }
}

/// In-memory record store with expiration support.
///
/// Stores DHT records keyed by their storage key. Mutable records enforce
/// sequence number ordering: a `put` with a lower or equal sequence number
/// than the existing record is rejected.
pub struct RecordStore {
    /// Records indexed by storage key.
    entries: HashMap<[u8; 32], StoreEntry>,
    /// Default TTL for new records.
    default_ttl: Duration,
}

impl RecordStore {
    /// Create a new record store with the default TTL.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl: Duration::from_secs(DEFAULT_TTL_SECS),
        }
    }

    /// Create a new record store with a custom default TTL.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            default_ttl: ttl,
        }
    }

    /// Store a record. Validates the record before storing.
    ///
    /// For mutable records, enforces that the sequence number is strictly
    /// greater than any existing record at the same key.
    pub fn put(&mut self, record: DhtRecord) -> Result<()> {
        record.validate()?;

        let key = record.storage_key();

        // For mutable records, check sequence number ordering.
        if let DhtRecord::Mutable { seq, .. } = &record {
            if let Some(existing) = self.entries.get(&key) {
                if !existing.is_expired() {
                    if let DhtRecord::Mutable {
                        seq: existing_seq, ..
                    } = &existing.record
                    {
                        if *seq <= *existing_seq {
                            return Err(DhtError::StaleSequence {
                                got: *seq,
                                have: *existing_seq,
                            });
                        }
                    }
                }
            }
        }

        self.entries.insert(
            key,
            StoreEntry {
                record,
                stored_at: Instant::now(),
                ttl: self.default_ttl,
            },
        );

        Ok(())
    }

    /// Retrieve a record by its storage key.
    ///
    /// Returns `None` if the record does not exist or has expired.
    pub fn get(&self, key: &[u8; 32]) -> Option<&DhtRecord> {
        self.entries.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(&entry.record)
            }
        })
    }

    /// Remove expired records from the store.
    ///
    /// Returns the number of records removed.
    pub fn expire(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|_, entry| !entry.is_expired());
        let removed = before - self.entries.len();
        if removed > 0 {
            tracing::debug!("Expired {removed} DHT records");
        }
        removed
    }

    /// Return the number of (non-expired) records in the store.
    pub fn len(&self) -> usize {
        self.entries.values().filter(|e| !e.is_expired()).count()
    }

    /// Return whether the store is empty (ignoring expired entries).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return all non-expired storage keys.
    pub fn keys(&self) -> Vec<[u8; 32]> {
        self.entries
            .iter()
            .filter(|(_, e)| !e.is_expired())
            .map(|(k, _)| *k)
            .collect()
    }
}

impl Default for RecordStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a signed mutable record.
///
/// # Arguments
///
/// * `signing_key` - The Ed25519 signing key for the record publisher
/// * `salt` - Optional salt for key derivation
/// * `seq` - Monotonically increasing sequence number
/// * `value` - The record value (must be <= [`MAX_RECORD_SIZE`] bytes)
pub fn create_mutable_record(
    signing_key: &ochra_crypto::ed25519::SigningKey,
    salt: &[u8],
    seq: u64,
    value: Vec<u8>,
) -> Result<DhtRecord> {
    if value.len() > MAX_RECORD_SIZE {
        return Err(DhtError::RecordTooLarge {
            size: value.len(),
            max: MAX_RECORD_SIZE,
        });
    }

    let public_key = signing_key.verifying_key().to_bytes();
    let signed_data = build_signed_data(salt, seq, &value);
    let signature = signing_key.sign(&signed_data).to_bytes();

    Ok(DhtRecord::Mutable {
        public_key,
        salt: salt.to_vec(),
        seq,
        value,
        signature,
    })
}

/// Create an immutable record.
///
/// # Arguments
///
/// * `value` - The record value (must be <= [`MAX_RECORD_SIZE`] bytes)
pub fn create_immutable_record(value: Vec<u8>) -> Result<DhtRecord> {
    if value.len() > MAX_RECORD_SIZE {
        return Err(DhtError::RecordTooLarge {
            size: value.len(),
            max: MAX_RECORD_SIZE,
        });
    }

    Ok(DhtRecord::Immutable { value })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ochra_crypto::ed25519::KeyPair;

    #[test]
    fn test_immutable_record_key() {
        let value = b"hello, ochra DHT".to_vec();
        let record = create_immutable_record(value.clone()).expect("create record");
        let key = record.storage_key();
        assert_eq!(key, ochra_crypto::blake3::hash(&value));
    }

    #[test]
    fn test_immutable_record_validate() {
        let record = create_immutable_record(b"test".to_vec()).expect("create record");
        assert!(record.validate().is_ok());
    }

    #[test]
    fn test_immutable_record_too_large() {
        let value = vec![0u8; MAX_RECORD_SIZE + 1];
        let result = create_immutable_record(value);
        assert!(result.is_err());
    }

    #[test]
    fn test_mutable_record_roundtrip() {
        let kp = KeyPair::generate();
        let record = create_mutable_record(
            &kp.signing_key,
            b"test-salt",
            1,
            b"mutable data".to_vec(),
        )
        .expect("create record");

        assert!(record.validate().is_ok());
    }

    #[test]
    fn test_mutable_record_wrong_signature() {
        let kp = KeyPair::generate();
        let mut record = create_mutable_record(
            &kp.signing_key,
            b"salt",
            1,
            b"data".to_vec(),
        )
        .expect("create record");

        // Tamper with the value
        if let DhtRecord::Mutable { ref mut value, .. } = record {
            value[0] ^= 0xFF;
        }

        assert!(record.validate().is_err());
    }

    #[test]
    fn test_mutable_record_storage_key() {
        let kp = KeyPair::generate();
        let salt = b"my-salt";
        let record = create_mutable_record(
            &kp.signing_key,
            salt,
            1,
            b"value".to_vec(),
        )
        .expect("create record");

        let expected_key = {
            let mut input = Vec::new();
            input.extend_from_slice(&kp.verifying_key.to_bytes());
            input.extend_from_slice(salt);
            ochra_crypto::blake3::hash(&input)
        };

        assert_eq!(record.storage_key(), expected_key);
    }

    #[test]
    fn test_record_store_put_get() {
        let mut store = RecordStore::new();
        let record = create_immutable_record(b"test value".to_vec()).expect("create");
        let key = record.storage_key();

        store.put(record).expect("put");
        assert!(store.get(&key).is_some());
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_record_store_get_nonexistent() {
        let store = RecordStore::new();
        assert!(store.get(&[0u8; 32]).is_none());
    }

    #[test]
    fn test_record_store_sequence_ordering() {
        let mut store = RecordStore::new();
        let kp = KeyPair::generate();

        let r1 = create_mutable_record(&kp.signing_key, b"s", 1, b"v1".to_vec())
            .expect("create");
        let r2 = create_mutable_record(&kp.signing_key, b"s", 2, b"v2".to_vec())
            .expect("create");
        let r_stale = create_mutable_record(&kp.signing_key, b"s", 1, b"v_stale".to_vec())
            .expect("create");

        store.put(r1).expect("put r1");
        store.put(r2).expect("put r2");

        // Stale sequence should be rejected
        let result = store.put(r_stale);
        assert!(result.is_err());
        assert!(matches!(result, Err(DhtError::StaleSequence { got: 1, have: 2 })));
    }

    #[test]
    fn test_record_store_expiration() {
        let mut store = RecordStore::with_ttl(Duration::from_millis(1));
        let record = create_immutable_record(b"ephemeral".to_vec()).expect("create");
        let key = record.storage_key();

        store.put(record).expect("put");

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(10));

        assert!(store.get(&key).is_none());
        let expired_count = store.expire();
        assert_eq!(expired_count, 1);
    }

    #[test]
    fn test_record_store_is_empty() {
        let store = RecordStore::new();
        assert!(store.is_empty());
    }

    #[test]
    fn test_record_store_keys() {
        let mut store = RecordStore::new();
        let r1 = create_immutable_record(b"value1".to_vec()).expect("create");
        let r2 = create_immutable_record(b"value2".to_vec()).expect("create");
        let k1 = r1.storage_key();
        let k2 = r2.storage_key();

        store.put(r1).expect("put");
        store.put(r2).expect("put");

        let keys = store.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&k1));
        assert!(keys.contains(&k2));
    }
}
