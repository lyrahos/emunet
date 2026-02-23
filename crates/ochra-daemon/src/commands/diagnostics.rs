//! Diagnostics & Settings command handlers (Section 21.6).

use std::sync::Arc;

use serde_json::Value;

use crate::rpc::RpcError;
use crate::DaemonState;

type Result = std::result::Result<Value, RpcError>;

/// Check for protocol updates.
pub async fn check_protocol_updates(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({
        "update_available": false,
        "current_version": env!("CARGO_PKG_VERSION"),
    }))
}

/// Apply a protocol update.
pub async fn apply_protocol_update(_state: &Arc<DaemonState>) -> Result {
    // v1: Updates via GitHub Releases, not P2P OTA
    Ok(serde_json::json!({"status": "no_update_available"}))
}

/// Get daemon logs.
pub async fn get_daemon_logs(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _level = params
        .get("level")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    // Would return buffered log entries
    Ok(serde_json::json!([]))
}

/// Export diagnostics bundle.
pub async fn export_diagnostics(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({
        "diagnostics": {
            "version": env!("CARGO_PKG_VERSION"),
            "epoch": crate::epoch::current_epoch(),
            "relay_epoch": crate::epoch::current_relay_epoch(),
        }
    }))
}

/// Set theme settings.
pub async fn set_theme_settings(state: &Arc<DaemonState>, params: &Value) -> Result {
    let mode = params
        .get("mode")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("mode required"))?;

    if !["light", "dark", "system"].contains(&mode) {
        return Err(RpcError {
            code: -32125,
            message: "SETTINGS_INVALID".to_string(),
            data: Some(serde_json::json!({"detail": "mode must be light/dark/system"})),
        });
    }

    let db = state.db.lock().await;
    ochra_db::queries::settings::set(&db, "theme_mode", mode)
        .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;

    if let Some(accent) = params.get("accent_color").and_then(|v| v.as_str()) {
        ochra_db::queries::settings::set(&db, "accent_color", accent)
            .map_err(|e| RpcError::internal_error(&format!("db error: {e}")))?;
    }

    Ok(serde_json::json!({"updated": true}))
}

/// Get network stats.
pub async fn get_network_stats(_state: &Arc<DaemonState>) -> Result {
    Ok(serde_json::json!({
        "total_nodes": 0_u32,
        "quorum_size": 0_u32,
        "is_degraded_mode": true,
    }))
}

/// Get cover traffic stats.
pub async fn get_cover_traffic_stats(state: &Arc<DaemonState>) -> Result {
    if !state.config.privacy.cover_traffic_enabled {
        return Err(RpcError {
            code: -32127,
            message: "COVER_TRAFFIC_DISABLED".to_string(),
            data: None,
        });
    }

    Ok(serde_json::json!({
        "packets_sent_24h": 0_u64,
        "current_rate_pps": 0.0_f64,
        "mode": "sleep",
    }))
}

/// Lock the current session.
pub async fn lock_session(state: &Arc<DaemonState>) -> Result {
    let mut unlocked = state.unlocked.write().await;
    *unlocked = false;
    Ok(serde_json::json!({"locked": true}))
}

/// Subscribe to daemon events.
pub async fn subscribe_events(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _filter = params.get("filter");

    // Generate subscription ID
    let mut sub_id = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut sub_id);

    Ok(serde_json::json!({
        "subscription_id": hex::encode(sub_id),
    }))
}

/// Unsubscribe from daemon events.
pub async fn unsubscribe_events(_state: &Arc<DaemonState>, params: &Value) -> Result {
    let _subscription_id = params
        .get("subscription_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| RpcError::invalid_params("subscription_id required"))?;

    Ok(serde_json::json!({"unsubscribed": true}))
}
