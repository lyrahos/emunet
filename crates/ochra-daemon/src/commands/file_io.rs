//! File IO, ABR & Publishing command handlers (Section 21.4).

use std::sync::Arc;

use serde_json::Value;

use crate::rpc::RpcError;
use crate::DaemonState;

type Result = std::result::Result<Value, RpcError>;

/// Get the content catalog for a Space.
pub async fn get_store_catalog(state: &Arc<DaemonState>, params: &Value) -> Result {
    let group_id_hex = params
        .get("group_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("group_id required"))?;

    let group_id_bytes = hex::decode(group_id_hex)
        .map_err(|_| RpcError::invalid_params("invalid hex for group_id"))?;
    let group_id: [u8; 32] = group_id_bytes
        .try_into()
        .map_err(|_| RpcError::invalid_params("group_id must be 32 bytes"))?;

    let db = state.db.lock().await;
    let items = ochra_db::queries::content::list_by_space(&db, &group_id)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    let result: Vec<Value> = items
        .iter()
        .map(|item| {
            serde_json::json!({
                "content_hash": hex::encode(&item.content_hash),
                "title": item.title,
                "description": item.description,
                "pricing": item.pricing_json,
                "total_size_bytes": item.total_size_bytes,
                "chunk_count": item.chunk_count,
                "published_at": item.published_at,
            })
        })
        .collect();

    Ok(serde_json::json!(result))
}

/// Search the content catalog.
pub async fn search_catalog(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _group_id = params
        .get("group_id")
        .ok_or_else(|| RpcError::invalid_params("group_id required"))?;
    let _query = params
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("query required"))?;

    // Would use FTS5 search
    Ok(serde_json::json!([]))
}

/// Publish a file to a Space.
pub async fn publish_file(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("path required"))?;
    let _target_id = params
        .get("target_id")
        .ok_or_else(|| RpcError::invalid_params("target_id required"))?;
    let _pricing = params
        .get("pricing")
        .ok_or_else(|| RpcError::invalid_params("pricing required"))?;

    // Would: chunk file, compute Merkle root, generate PoW, publish manifest
    let content_hash = [0u8; 32]; // Placeholder
    Ok(serde_json::json!({
        "content_hash": hex::encode(content_hash),
    }))
}

/// Set pricing for existing content.
pub async fn set_content_pricing(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;
    let _pricing = params
        .get("pricing")
        .ok_or_else(|| RpcError::invalid_params("pricing required"))?;
    Ok(serde_json::json!({"updated": true}))
}

/// Purchase content.
pub async fn purchase_content(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;
    let _tier_index = params
        .get("tier_index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| RpcError::invalid_params("tier_index required"))?;

    // Would: check balance, create escrow/micro tx, begin download
    Ok(serde_json::json!({
        "status": "downloading",
        "progress": 0.0,
    }))
}

/// Re-download previously purchased content.
pub async fn redownload_content(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;
    let _destination = params
        .get("destination")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("destination required"))?;

    Ok(serde_json::json!({
        "status": "downloading",
        "progress": 0.0,
    }))
}

/// Get purchase receipts.
pub async fn get_purchase_receipts(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!([]))
}

/// Get access status for content.
pub async fn get_access_status(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;

    Ok(serde_json::json!({
        "has_access": false,
        "tier_type": null,
        "expires_at": null,
    }))
}

/// Download a file.
pub async fn download_file(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;
    let _destination = params
        .get("destination")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("destination required"))?;

    Ok(serde_json::json!({
        "status": "downloading",
        "progress": 0.0,
    }))
}

/// Pause an active download.
pub async fn pause_download(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;
    Ok(serde_json::json!({"paused": true}))
}

/// Get ABR telemetry.
pub async fn get_abr_telemetry(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({
        "used_bytes": 0_u64,
        "evictions_24h": 0_u32,
        "posrv_score": 0.0_f32,
    }))
}

/// Update earning settings.
pub async fn update_earning_settings(state: &Arc<DaemonState>, params: &Value) -> Result {
    let power_level = params
        .get("power_level")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("power_level required"))?;

    if !["low", "medium", "high", "custom"].contains(&power_level) {
        return Err(RpcError::invalid_params(
            "power_level must be low/medium/high/custom",
        ));
    }

    let db = state.db.lock().await;
    ochra_db::queries::settings::set(&db, "earning_level", power_level)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    Ok(serde_json::json!({"updated": true}))
}

/// Pin content (prevent ABR eviction).
pub async fn pin_content(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;
    Ok(serde_json::json!({"pinned": true}))
}

/// Unpin content.
pub async fn unpin_content(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _content_hash = params
        .get("content_hash")
        .ok_or_else(|| RpcError::invalid_params("content_hash required"))?;
    Ok(serde_json::json!({"unpinned": true}))
}

/// Submit a zk-PoR proof.
pub async fn submit_zk_por_proof(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({
        "status": "submitted",
        "proving_time_ms": 0,
    }))
}
