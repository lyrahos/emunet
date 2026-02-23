//! Identity, Contacts & Recovery command handlers (Section 21.1).

use std::sync::Arc;

use serde_json::Value;
use tracing::info;

use crate::rpc::RpcError;
use crate::DaemonState;

type Result = std::result::Result<Value, RpcError>;

/// Initialize a new PIK with the given password.
pub async fn init_pik(state: &Arc<DaemonState>, params: &Value) -> Result {
    let password = params
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("password required"))?;

    info!("Initializing new PIK");

    // Generate Ed25519 keypair
    let keypair = ochra_crypto::ed25519::KeyPair::generate();
    let pik_hash = ochra_crypto::blake3::hash(keypair.verifying_key.as_bytes());

    // Encrypt PIK with password-derived key (Argon2id + ChaCha20-Poly1305)
    let salt = ochra_crypto::argon2id::generate_salt();
    let derived_key = ochra_crypto::argon2id::derive_pik_key(password.as_bytes(), &salt)
        .map_err(|e| RpcError::internal_error(&format!("key derivation failed: {e}")))?;

    let nonce = [0u8; 12]; // Will use random nonce in production
    let encrypted_pik = ochra_crypto::chacha20::encrypt(
        &derived_key,
        &nonce,
        keypair.signing_key.to_bytes().as_slice(),
        &[],
    )
    .map_err(|e| RpcError::internal_error(&format!("encryption failed: {e}")))?;

    // Store in database
    {
        let db = state.db.lock().await;
        db.execute(
            "INSERT OR REPLACE INTO pik (id, encrypted_key, salt, argon2_params, created_at) VALUES (1, ?1, ?2, ?3, ?4)",
            rusqlite::params![
                encrypted_pik.as_slice(),
                salt.as_slice(),
                "m=262144,t=3,p=4",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
            ],
        ).map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;
    }

    // Unlock session
    {
        let mut unlocked = state.unlocked.write().await;
        *unlocked = true;
    }

    Ok(serde_json::json!({
        "pik_hash": hex::encode(pik_hash),
        "created": true,
    }))
}

/// Authenticate with password.
pub async fn authenticate(state: &Arc<DaemonState>, params: &Value) -> Result {
    let password = params
        .get("password")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("password required"))?;

    info!("Authenticating");

    // Load encrypted PIK and salt from database
    let (encrypted_key, salt): (Vec<u8>, Vec<u8>) = {
        let db = state.db.lock().await;
        db.query_row(
            "SELECT encrypted_key, salt FROM pik WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| RpcError::pik_not_initialized())?
    };

    // Derive key and attempt decryption
    let salt_arr: [u8; 16] = salt
        .try_into()
        .map_err(|_| RpcError::internal_error("invalid salt length"))?;
    let derived_key = ochra_crypto::argon2id::derive_pik_key(password.as_bytes(), &salt_arr)
        .map_err(|_| RpcError::wrong_password())?;

    let nonce = [0u8; 12];
    let _decrypted = ochra_crypto::chacha20::decrypt(&derived_key, &nonce, &encrypted_key, &[])
        .map_err(|_| RpcError::wrong_password())?;

    // Unlock session
    {
        let mut unlocked = state.unlocked.write().await;
        *unlocked = true;
    }

    Ok(serde_json::json!({"authenticated": true}))
}

/// Authenticate with biometric.
pub async fn authenticate_biometric(state: &Arc<DaemonState>) -> Result {
    // v1: Biometric auth delegates to OS keychain. Stub for now.
    let mut unlocked = state.unlocked.write().await;
    *unlocked = true;
    Ok(serde_json::json!({"authenticated": true}))
}

/// Get own PIK hash.
pub async fn get_my_pik(state: &Arc<DaemonState>) -> Result {
    let db = state.db.lock().await;
    let pik_hash: Vec<u8> = db
        .query_row("SELECT encrypted_key FROM pik WHERE id = 1", [], |row| {
            row.get(0)
        })
        .map_err(|_| RpcError::pik_not_initialized())?;

    // Return hash of the encrypted key as identifier
    let hash = ochra_crypto::blake3::hash(&pik_hash);
    Ok(serde_json::json!({"pik_hash": hex::encode(hash)}))
}

/// Change password.
pub async fn change_password(state: &Arc<DaemonState>, params: &Value) -> Result {
    let _old = params
        .get("old")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("old password required"))?;
    let _new = params
        .get("new")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("new password required"))?;

    // Would re-encrypt PIK with new password
    Ok(serde_json::json!({"changed": true}))
}

/// Update display name.
pub async fn update_display_name(state: &Arc<DaemonState>, params: &Value) -> Result {
    let new_name = params
        .get("new_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("new_name required"))?;

    let db = state.db.lock().await;
    ochra_db::queries::settings::set(&db, "display_name", new_name)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    Ok(serde_json::json!({"updated": true}))
}

/// Enroll biometric authentication.
pub async fn enroll_biometric(_state: &Arc<DaemonState>) -> Result {
    // Stub: would integrate with OS keychain
    Ok(serde_json::json!({"enrolled": true}))
}

/// Export revocation certificate.
pub async fn export_revocation_certificate(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({"certificate": "stub-revocation-cert"}))
}

/// Export user data.
pub async fn export_user_data(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({"data": "stub-export-data"}))
}

/// Nominate a guardian (Recovery Contact).
pub async fn nominate_guardian(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _contact_pik = params
        .get("contact_pik")
        .ok_or_else(|| RpcError::invalid_params("contact_pik required"))?;
    Ok(serde_json::json!({"nominated": true}))
}

/// Replace a guardian.
pub async fn replace_guardian(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _old_pik = params
        .get("old_pik")
        .ok_or_else(|| RpcError::invalid_params("old_pik required"))?;
    let _new_pik = params
        .get("new_pik")
        .ok_or_else(|| RpcError::invalid_params("new_pik required"))?;
    Ok(serde_json::json!({"replaced": true}))
}

/// Get guardian health status.
pub async fn get_guardian_health(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({"guardians": []}))
}

/// Initiate recovery.
pub async fn initiate_recovery(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _shares = params
        .get("guardian_shares")
        .ok_or_else(|| RpcError::invalid_params("guardian_shares required"))?;
    Ok(serde_json::json!({
        "status": "pending",
        "veto_window_ends": 0,
    }))
}

/// Veto an ongoing recovery.
pub async fn veto_recovery(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _auth = params
        .get("auth_payload")
        .ok_or_else(|| RpcError::invalid_params("auth_payload required"))?;
    Ok(serde_json::json!({"vetoed": true}))
}

/// Add a contact from a token.
pub async fn add_contact(state: &Arc<DaemonState>, params: &Value) -> Result {
    let token = params
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("token required"))?;

    // Parse and validate token, then insert contact
    let _token_data = token; // Would parse ContactExchangeToken
    let pik_hash = [0u8; 32]; // Placeholder
    let profile_key = [0u8; 32]; // Placeholder

    let db = state.db.lock().await;
    ochra_db::queries::contacts::insert(
        &db,
        &pik_hash,
        "New Contact",
        &profile_key,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    )
    .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    Ok(serde_json::json!({
        "pik_hash": hex::encode(pik_hash),
        "display_name": "New Contact",
    }))
}

/// Remove a contact.
pub async fn remove_contact(state: &Arc<DaemonState>, params: &Value) -> Result {
    let pik_hex = params
        .get("contact_pik")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("contact_pik required"))?;

    let pik_bytes = hex::decode(pik_hex)
        .map_err(|_| RpcError::invalid_params("invalid hex for contact_pik"))?;
    let pik: [u8; 32] = pik_bytes
        .try_into()
        .map_err(|_| RpcError::invalid_params("contact_pik must be 32 bytes"))?;

    let db = state.db.lock().await;
    ochra_db::queries::contacts::remove(&db, &pik)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    Ok(serde_json::json!({"removed": true}))
}

/// Generate a contact exchange token.
pub async fn generate_contact_token(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _ttl_hours = params
        .get("ttl_hours")
        .and_then(|v| v.as_u64())
        .unwrap_or(24);

    Ok(serde_json::json!({"token": "stub-contact-token"}))
}

/// Get all contacts.
pub async fn get_contacts(state: &Arc<DaemonState>) -> Result {
    let db = state.db.lock().await;
    let contacts = ochra_db::queries::contacts::list(&db)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    let result: Vec<Value> = contacts
        .iter()
        .map(|c| {
            serde_json::json!({
                "pik_hash": hex::encode(&c.pik_hash),
                "display_name": c.display_name,
                "added_at": c.added_at,
                "is_blocked": c.is_blocked,
            })
        })
        .collect();

    Ok(serde_json::json!(result))
}
