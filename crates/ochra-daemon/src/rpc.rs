//! JSON-RPC server over Unix socket (Section 32).
//!
//! Listens on a Unix domain socket, accepts connections, and dispatches
//! JSON-RPC method calls to the appropriate command handlers.

use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tracing::{debug, error, info, warn};

use crate::commands;
use crate::DaemonState;

/// JSON-RPC request.
#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    /// JSON-RPC version (must be "2.0").
    pub jsonrpc: String,
    /// Request ID.
    pub id: serde_json::Value,
    /// Method name.
    pub method: String,
    /// Parameters.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC success response.
#[derive(Debug, Serialize)]
pub struct RpcResponse {
    /// JSON-RPC version.
    pub jsonrpc: String,
    /// Request ID.
    pub id: serde_json::Value,
    /// Result or error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

/// JSON-RPC error object (Section 29).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RpcError {
    /// Error code per Section 29.
    pub code: i32,
    /// Error name.
    pub message: String,
    /// Optional structured data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcResponse {
    /// Create a success response.
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: serde_json::Value, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

impl RpcError {
    // Standard JSON-RPC errors (Section 29.2)

    /// Parse error (-32700).
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "PARSE_ERROR".to_string(),
            data: None,
        }
    }

    /// Invalid request (-32600).
    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "INVALID_REQUEST".to_string(),
            data: None,
        }
    }

    /// Method not found (-32601).
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: "METHOD_NOT_FOUND".to_string(),
            data: Some(serde_json::json!({"method": method})),
        }
    }

    /// Invalid params (-32602).
    pub fn invalid_params(detail: &str) -> Self {
        Self {
            code: -32602,
            message: "INVALID_PARAMS".to_string(),
            data: Some(serde_json::json!({"detail": detail})),
        }
    }

    /// Internal error (-32603).
    pub fn internal_error(detail: &str) -> Self {
        Self {
            code: -32603,
            message: "INTERNAL_ERROR".to_string(),
            data: Some(serde_json::json!({"detail": detail})),
        }
    }

    /// Session locked (-32010).
    pub fn session_locked() -> Self {
        Self {
            code: -32010,
            message: "SESSION_LOCKED".to_string(),
            data: None,
        }
    }

    /// Wrong password (-32011).
    pub fn wrong_password() -> Self {
        Self {
            code: -32011,
            message: "WRONG_PASSWORD".to_string(),
            data: None,
        }
    }

    /// PIK not initialized (-32013).
    pub fn pik_not_initialized() -> Self {
        Self {
            code: -32013,
            message: "PIK_NOT_INITIALIZED".to_string(),
            data: None,
        }
    }

    /// Insufficient balance (-32040).
    pub fn insufficient_balance(required: u64, available: u64) -> Self {
        Self {
            code: -32040,
            message: "INSUFFICIENT_BALANCE".to_string(),
            data: Some(serde_json::json!({"required": required, "available": available})),
        }
    }

    /// Not host (-32060).
    pub fn not_host() -> Self {
        Self {
            code: -32060,
            message: "NOT_HOST".to_string(),
            data: None,
        }
    }
}

/// The RPC server.
pub struct RpcServer {
    state: Arc<DaemonState>,
    socket_path: PathBuf,
}

impl RpcServer {
    /// Create a new RPC server.
    pub fn new(state: Arc<DaemonState>, socket_path: PathBuf) -> Self {
        Self { state, socket_path }
    }

    /// Run the server, accepting connections.
    pub async fn run(&self) -> anyhow::Result<()> {
        // Remove stale socket file
        let _ = std::fs::remove_file(&self.socket_path);

        let listener = UnixListener::bind(&self.socket_path)?;
        info!("IPC server listening on {:?}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let state = self.state.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(state, stream).await {
                            warn!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Accept error: {}", e);
                }
            }
        }
    }
}

/// Handle a single client connection.
async fn handle_connection(
    state: Arc<DaemonState>,
    stream: tokio::net::UnixStream,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            break; // EOF
        }

        let response = match serde_json::from_str::<RpcRequest>(&line) {
            Ok(request) => dispatch_request(state.clone(), request).await,
            Err(_) => RpcResponse::error(serde_json::Value::Null, RpcError::parse_error()),
        };

        let mut response_json = serde_json::to_string(&response)?;
        response_json.push('\n');
        writer.write_all(response_json.as_bytes()).await?;
        writer.flush().await?;
    }

    Ok(())
}

/// Dispatch a JSON-RPC request to the appropriate command handler.
async fn dispatch_request(state: Arc<DaemonState>, request: RpcRequest) -> RpcResponse {
    let id = request.id.clone();
    let method = request.method.as_str();

    debug!("Dispatching RPC method: {}", method);

    // Check if method requires authentication
    let requires_auth = !matches!(method, "init_pik" | "authenticate" | "authenticate_biometric");

    if requires_auth {
        let unlocked = state.unlocked.read().await;
        if !*unlocked {
            // Allow some diagnostic commands even when locked
            if !matches!(
                method,
                "get_daemon_logs" | "export_diagnostics" | "lock_session" | "check_protocol_updates"
            ) {
                return RpcResponse::error(id, RpcError::session_locked());
            }
        }
    }

    let result = match method {
        // Identity commands (Section 21.1)
        "init_pik" => commands::identity::init_pik(&state, &request.params).await,
        "authenticate" => commands::identity::authenticate(&state, &request.params).await,
        "authenticate_biometric" => commands::identity::authenticate_biometric(&state).await,
        "get_my_pik" => commands::identity::get_my_pik(&state).await,
        "change_password" => commands::identity::change_password(&state, &request.params).await,
        "update_display_name" => {
            commands::identity::update_display_name(&state, &request.params).await
        }
        "enroll_biometric" => commands::identity::enroll_biometric(&state).await,
        "export_revocation_certificate" => {
            commands::identity::export_revocation_certificate(&state).await
        }
        "export_user_data" => commands::identity::export_user_data(&state).await,
        "nominate_guardian" => commands::identity::nominate_guardian(&state, &request.params).await,
        "replace_guardian" => commands::identity::replace_guardian(&state, &request.params).await,
        "get_guardian_health" => commands::identity::get_guardian_health(&state).await,
        "initiate_recovery" => {
            commands::identity::initiate_recovery(&state, &request.params).await
        }
        "veto_recovery" => commands::identity::veto_recovery(&state, &request.params).await,
        "add_contact" => commands::identity::add_contact(&state, &request.params).await,
        "remove_contact" => commands::identity::remove_contact(&state, &request.params).await,
        "generate_contact_token" => {
            commands::identity::generate_contact_token(&state, &request.params).await
        }
        "get_contacts" => commands::identity::get_contacts(&state).await,

        // Network commands (Section 21.2)
        "get_my_groups" => commands::network::get_my_groups(&state).await,
        "create_group" => commands::network::create_group(&state, &request.params).await,
        "join_group" => commands::network::join_group(&state, &request.params).await,
        "leave_group" => commands::network::leave_group(&state, &request.params).await,
        "kick_member" => commands::network::kick_member(&state, &request.params).await,
        "generate_invite" => commands::network::generate_invite(&state, &request.params).await,
        "revoke_invite" => commands::network::revoke_invite(&state, &request.params).await,
        "get_active_invites" => commands::network::get_active_invites(&state, &request.params).await,
        "get_group_members" => commands::network::get_group_members(&state, &request.params).await,
        "grant_publisher_role" => {
            commands::network::grant_publisher_role(&state, &request.params).await
        }
        "revoke_publisher_role" => {
            commands::network::revoke_publisher_role(&state, &request.params).await
        }
        "grant_moderator_role" => {
            commands::network::grant_moderator_role(&state, &request.params).await
        }
        "revoke_moderator_role" => {
            commands::network::revoke_moderator_role(&state, &request.params).await
        }
        "transfer_group_ownership" => {
            commands::network::transfer_group_ownership(&state, &request.params).await
        }
        "veto_ownership_transfer" => {
            commands::network::veto_ownership_transfer(&state, &request.params).await
        }
        "update_group_settings" => {
            commands::network::update_group_settings(&state, &request.params).await
        }
        "update_group_profile" => {
            commands::network::update_group_profile(&state, &request.params).await
        }
        "create_subgroup" => commands::network::create_subgroup(&state, &request.params).await,
        "get_subgroup_members" => {
            commands::network::get_subgroup_members(&state, &request.params).await
        }
        "mls_grant_subgroup_access" => {
            commands::network::mls_grant_subgroup_access(&state, &request.params).await
        }
        "mls_revoke_subgroup_access" => {
            commands::network::mls_revoke_subgroup_access(&state, &request.params).await
        }
        "preview_layout_manifest" => {
            commands::network::preview_layout_manifest(&state, &request.params).await
        }
        "update_group_layout_manifest" => {
            commands::network::update_group_layout_manifest(&state, &request.params).await
        }
        "get_onion_circuit_health" => commands::network::get_onion_circuit_health(&state).await,
        "set_group_notification_settings" => {
            commands::network::set_group_notification_settings(&state, &request.params).await
        }
        "get_space_stats" => commands::network::get_space_stats(&state, &request.params).await,
        "get_space_activity" => {
            commands::network::get_space_activity(&state, &request.params).await
        }
        "get_content_reports" => {
            commands::network::get_content_reports(&state, &request.params).await
        }
        "dismiss_content_report" => {
            commands::network::dismiss_content_report(&state, &request.params).await
        }
        "report_content" => commands::network::report_content(&state, &request.params).await,
        "owner_tombstone_content" => {
            commands::network::owner_tombstone_content(&state, &request.params).await
        }

        // Economy commands (Section 21.3)
        "get_oracle_twap" => commands::economy::get_oracle_twap(&state).await,
        "get_wallet_balance" => commands::economy::get_wallet_balance(&state).await,
        "get_purchase_history" => commands::economy::get_purchase_history(&state).await,
        "send_funds" => commands::economy::send_funds(&state, &request.params).await,
        "force_flush_receipts" => {
            commands::economy::force_flush_receipts(&state, &request.params).await
        }
        "init_tls_notary_share" => {
            commands::economy::init_tls_notary_share(&state, &request.params).await
        }
        "propose_revenue_split" => {
            commands::economy::propose_revenue_split(&state, &request.params).await
        }
        "get_earnings_breakdown" => {
            commands::economy::get_earnings_breakdown(&state, &request.params).await
        }
        "claim_vys_rewards" => commands::economy::claim_vys_rewards(&state).await,
        "request_anonymous_refund" => {
            commands::economy::request_anonymous_refund(&state, &request.params).await
        }
        "get_collateral_ratio" => commands::economy::get_collateral_ratio(&state).await,
        "get_circulating_supply" => commands::economy::get_circulating_supply(&state).await,

        // File IO commands (Section 21.4)
        "get_store_catalog" => commands::file_io::get_store_catalog(&state, &request.params).await,
        "search_catalog" => commands::file_io::search_catalog(&state, &request.params).await,
        "publish_file" => commands::file_io::publish_file(&state, &request.params).await,
        "set_content_pricing" => {
            commands::file_io::set_content_pricing(&state, &request.params).await
        }
        "purchase_content" => commands::file_io::purchase_content(&state, &request.params).await,
        "redownload_content" => {
            commands::file_io::redownload_content(&state, &request.params).await
        }
        "get_purchase_receipts" => commands::file_io::get_purchase_receipts(&state).await,
        "get_access_status" => {
            commands::file_io::get_access_status(&state, &request.params).await
        }
        "download_file" => commands::file_io::download_file(&state, &request.params).await,
        "pause_download" => commands::file_io::pause_download(&state, &request.params).await,
        "get_abr_telemetry" => commands::file_io::get_abr_telemetry(&state).await,
        "update_earning_settings" => {
            commands::file_io::update_earning_settings(&state, &request.params).await
        }
        "pin_content" => commands::file_io::pin_content(&state, &request.params).await,
        "unpin_content" => commands::file_io::unpin_content(&state, &request.params).await,
        "submit_zk_por_proof" => commands::file_io::submit_zk_por_proof(&state).await,

        // Whisper commands (Section 21.5)
        "register_handle" => commands::whisper::register_handle(&state, &request.params).await,
        "deprecate_handle" => commands::whisper::deprecate_handle(&state, &request.params).await,
        "get_my_handle" => commands::whisper::get_my_handle(&state).await,
        "resolve_handle" => commands::whisper::resolve_handle(&state, &request.params).await,
        "check_handle_availability" => {
            commands::whisper::check_handle_availability(&state, &request.params).await
        }
        "change_handle" => commands::whisper::change_handle(&state, &request.params).await,
        "start_whisper" => commands::whisper::start_whisper(&state, &request.params).await,
        "send_whisper" => commands::whisper::send_whisper(&state, &request.params).await,
        "send_whisper_seeds" => {
            commands::whisper::send_whisper_seeds(&state, &request.params).await
        }
        "reveal_identity" => commands::whisper::reveal_identity(&state, &request.params).await,
        "close_whisper" => commands::whisper::close_whisper(&state, &request.params).await,
        "block_whisper" => commands::whisper::block_whisper(&state, &request.params).await,
        "get_active_whispers" => commands::whisper::get_active_whispers(&state).await,
        "get_whisper_throttle_status" => {
            commands::whisper::get_whisper_throttle_status(&state, &request.params).await
        }
        "send_typing_indicator" => {
            commands::whisper::send_typing_indicator(&state, &request.params).await
        }
        "send_read_ack" => commands::whisper::send_read_ack(&state, &request.params).await,

        // Diagnostics commands (Section 21.6)
        "check_protocol_updates" => commands::diagnostics::check_protocol_updates(&state).await,
        "apply_protocol_update" => commands::diagnostics::apply_protocol_update(&state).await,
        "get_daemon_logs" => commands::diagnostics::get_daemon_logs(&state, &request.params).await,
        "export_diagnostics" => commands::diagnostics::export_diagnostics(&state).await,
        "set_theme_settings" => {
            commands::diagnostics::set_theme_settings(&state, &request.params).await
        }
        "get_network_stats" => commands::diagnostics::get_network_stats(&state).await,
        "get_cover_traffic_stats" => commands::diagnostics::get_cover_traffic_stats(&state).await,
        "lock_session" => commands::diagnostics::lock_session(&state).await,

        // Event subscription (Section 21.7)
        "subscribe_events" => commands::diagnostics::subscribe_events(&state, &request.params).await,
        "unsubscribe_events" => {
            commands::diagnostics::unsubscribe_events(&state, &request.params).await
        }

        // Dev-only commands
        "dev_set_oracle_rate" => commands::economy::dev_set_oracle_rate(&state, &request.params).await,

        _ => Err(RpcError::method_not_found(method)),
    };

    match result {
        Ok(value) => RpcResponse::success(id, value),
        Err(err) => RpcResponse::error(id, err),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_error_codes() {
        let err = RpcError::session_locked();
        assert_eq!(err.code, -32010);
        assert_eq!(err.message, "SESSION_LOCKED");

        let err = RpcError::insufficient_balance(100, 50);
        assert_eq!(err.code, -32040);

        let err = RpcError::method_not_found("unknown");
        assert_eq!(err.code, -32601);
    }

    #[test]
    fn test_rpc_response_success() {
        let resp = RpcResponse::success(
            serde_json::json!(1),
            serde_json::json!({"balance": 1000}),
        );
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_rpc_response_error() {
        let resp = RpcResponse::error(
            serde_json::json!(1),
            RpcError::internal_error("test"),
        );
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
    }
}
