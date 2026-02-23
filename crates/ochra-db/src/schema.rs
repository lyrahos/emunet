//! SQL schema definitions (Section 27).

/// Complete schema for Ochra v1 database.
pub const SCHEMA_V1: &str = r#"
-- ============================================================
-- Section 27.1: Identity & Contacts
-- ============================================================

CREATE TABLE IF NOT EXISTS pik (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    pik_hash BLOB NOT NULL,
    encrypted_private_key BLOB NOT NULL,
    argon2id_salt BLOB NOT NULL,
    argon2id_nonce BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    profile_key BLOB NOT NULL
);

CREATE TABLE IF NOT EXISTS contacts (
    pik_hash BLOB PRIMARY KEY,
    display_name TEXT NOT NULL,
    profile_key BLOB NOT NULL,
    added_at INTEGER NOT NULL,
    last_seen_epoch INTEGER NOT NULL,
    is_blocked INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS recovery_contacts (
    contact_pik BLOB PRIMARY KEY,
    dkg_share BLOB NOT NULL,
    enrolled_at INTEGER NOT NULL,
    last_heartbeat_epoch INTEGER NOT NULL
);

-- ============================================================
-- Section 27.2: Spaces & Memberships
-- ============================================================

CREATE TABLE IF NOT EXISTS spaces (
    group_id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    icon BLOB,
    template TEXT NOT NULL,
    accent_color TEXT,
    my_role TEXT NOT NULL,
    owner_pik BLOB NOT NULL,
    publish_policy TEXT NOT NULL DEFAULT 'creators_only',
    invite_permission TEXT NOT NULL DEFAULT 'host_only',
    owner_pct INTEGER NOT NULL DEFAULT 10,
    pub_pct INTEGER NOT NULL DEFAULT 70,
    abr_pct INTEGER NOT NULL DEFAULT 20,
    member_count INTEGER NOT NULL DEFAULT 0,
    joined_at INTEGER NOT NULL,
    last_activity_at INTEGER NOT NULL,
    mls_group_state BLOB,
    pinned INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS space_members (
    group_id BLOB NOT NULL REFERENCES spaces(group_id) ON DELETE CASCADE,
    pik_hash BLOB NOT NULL,
    display_name TEXT,
    role TEXT NOT NULL,
    joined_at INTEGER NOT NULL,
    PRIMARY KEY (group_id, pik_hash)
);

CREATE TABLE IF NOT EXISTS invites (
    invite_hash BLOB PRIMARY KEY,
    group_id BLOB NOT NULL REFERENCES spaces(group_id),
    creator_flag INTEGER NOT NULL DEFAULT 0,
    uses_limit INTEGER,
    uses_consumed INTEGER NOT NULL DEFAULT 0,
    ttl_days INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    descriptor_key BLOB
);

CREATE INDEX IF NOT EXISTS idx_invites_group ON invites(group_id);
CREATE INDEX IF NOT EXISTS idx_invites_expires ON invites(expires_at);

-- ============================================================
-- Section 27.3: Content & Catalog
-- ============================================================

CREATE TABLE IF NOT EXISTS content_catalog (
    content_hash BLOB PRIMARY KEY,
    group_id BLOB NOT NULL REFERENCES spaces(group_id),
    title TEXT NOT NULL,
    description TEXT,
    tags TEXT,
    pricing TEXT NOT NULL,
    creator_pik BLOB NOT NULL,
    successor_hash BLOB,
    key_commitment BLOB NOT NULL,
    total_size_bytes INTEGER NOT NULL,
    chunk_count INTEGER NOT NULL,
    force_macro INTEGER NOT NULL DEFAULT 0,
    published_at INTEGER NOT NULL,
    is_tombstoned INTEGER NOT NULL DEFAULT 0,
    tombstoned_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_catalog_group ON content_catalog(group_id);

-- ============================================================
-- Section 27.4: Wallet & Economy
-- ============================================================

CREATE TABLE IF NOT EXISTS wallet_tokens (
    token_id BLOB PRIMARY KEY,
    amount INTEGER NOT NULL,
    nullifier BLOB NOT NULL UNIQUE,
    minted_at INTEGER NOT NULL,
    spent INTEGER NOT NULL DEFAULT 0,
    spent_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_wallet_unspent ON wallet_tokens(spent) WHERE spent = 0;

CREATE TABLE IF NOT EXISTS purchase_receipts (
    content_hash BLOB NOT NULL,
    receipt_secret BLOB NOT NULL,
    tier_type TEXT NOT NULL,
    price_paid INTEGER NOT NULL,
    purchased_at INTEGER NOT NULL,
    expires_at INTEGER,
    last_republished_epoch INTEGER NOT NULL,
    PRIMARY KEY (content_hash, receipt_secret)
);

CREATE TABLE IF NOT EXISTS transaction_history (
    tx_hash BLOB PRIMARY KEY,
    tx_type TEXT NOT NULL,
    amount INTEGER NOT NULL,
    counterparty_pik BLOB,
    note_ciphertext BLOB,
    content_hash BLOB,
    epoch INTEGER NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tx_epoch ON transaction_history(epoch);

CREATE TABLE IF NOT EXISTS vys_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    current_vys REAL NOT NULL DEFAULT 0.0,
    reward_per_token_paid INTEGER NOT NULL DEFAULT 0,
    pending_rewards INTEGER NOT NULL DEFAULT 0,
    last_claim_epoch INTEGER
);

-- ============================================================
-- Section 27.5: ABR & Storage
-- ============================================================

CREATE TABLE IF NOT EXISTS abr_chunks (
    chunk_id BLOB PRIMARY KEY,
    content_hash BLOB NOT NULL,
    shard_index INTEGER NOT NULL,
    size_bytes INTEGER NOT NULL,
    auth_tag BLOB NOT NULL,
    stored_at INTEGER NOT NULL,
    last_accessed INTEGER NOT NULL,
    fetch_count INTEGER NOT NULL DEFAULT 0,
    hll_replica_est REAL NOT NULL DEFAULT 8.0,
    is_pinned INTEGER NOT NULL DEFAULT 0,
    file_path TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_abr_content ON abr_chunks(content_hash);
CREATE INDEX IF NOT EXISTS idx_abr_eviction ON abr_chunks(is_pinned, fetch_count, last_accessed);

CREATE TABLE IF NOT EXISTS abr_service_receipts (
    receipt_id BLOB PRIMARY KEY,
    chunk_id BLOB NOT NULL,
    bytes_served INTEGER NOT NULL,
    timestamp INTEGER NOT NULL,
    relay_epoch INTEGER NOT NULL,
    requester_ack BLOB NOT NULL,
    server_sig BLOB NOT NULL,
    flushed INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_receipts_unflushed ON abr_service_receipts(flushed) WHERE flushed = 0;

-- ============================================================
-- Section 27.6: Handles & Whisper
-- ============================================================

CREATE TABLE IF NOT EXISTS my_handle (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    handle TEXT NOT NULL,
    handle_signing_sk BLOB NOT NULL,
    handle_signing_pk BLOB NOT NULL,
    registered_at INTEGER NOT NULL,
    last_refreshed INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'active'
);

CREATE TABLE IF NOT EXISTS blocked_handles (
    handle TEXT PRIMARY KEY,
    blocked_at INTEGER NOT NULL
);

-- ============================================================
-- Section 27.7: Settings & Misc
-- ============================================================

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS kademlia_routing (
    node_id BLOB PRIMARY KEY,
    ip_port TEXT NOT NULL,
    last_seen INTEGER NOT NULL,
    bucket_index INTEGER NOT NULL,
    trust_weight REAL NOT NULL DEFAULT 1.0
);

CREATE INDEX IF NOT EXISTS idx_kademlia_bucket ON kademlia_routing(bucket_index);

CREATE TABLE IF NOT EXISTS pending_timelocks (
    action TEXT NOT NULL,
    target_id BLOB NOT NULL,
    initiated_at INTEGER NOT NULL,
    completes_at INTEGER NOT NULL,
    payload BLOB NOT NULL,
    PRIMARY KEY (action, target_id)
);
"#;
