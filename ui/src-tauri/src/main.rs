// Prevents an additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc_bridge;

use serde::{Deserialize, Serialize};
use tracing::info;

/// Default Unix socket path for the Ochra daemon.
const DEFAULT_SOCKET_PATH: &str = "/tmp/ochra-daemon.sock";

// ---------------------------------------------------------------------------
// Tauri IPC command: greet (test / health-check)
// ---------------------------------------------------------------------------

/// A simple test command that the frontend can invoke to verify the IPC bridge
/// is working. Returns a greeting string.
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! The Ochra desktop bridge is operational.", name)
}

// ---------------------------------------------------------------------------
// Tauri IPC command: ipc_request (JSON-RPC forwarding to daemon)
// ---------------------------------------------------------------------------

/// Request payload from the frontend for a JSON-RPC call.
#[derive(Debug, Deserialize)]
pub struct IpcRequest {
    /// JSON-RPC method name (e.g. "get_wallet_balance").
    pub method: String,
    /// Optional parameters as a JSON value.
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Response returned to the frontend.
#[derive(Debug, Serialize)]
pub struct IpcResponse {
    /// True if the daemon returned a result (no error).
    pub ok: bool,
    /// The result value (present when ok == true).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error object (present when ok == false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<serde_json::Value>,
}

/// Forward a JSON-RPC request to the Ochra daemon over its Unix socket and
/// return the response to the frontend.
///
/// The frontend calls this via `invoke("ipc_request", { request: { method, params } })`.
#[tauri::command]
async fn ipc_request(request: IpcRequest) -> Result<IpcResponse, String> {
    let socket_path =
        std::env::var("OCHRA_SOCKET_PATH").unwrap_or_else(|_| DEFAULT_SOCKET_PATH.to_string());

    let rpc_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": request.method,
        "params": request.params,
    });

    match ipc_bridge::send_rpc_request(&socket_path, &rpc_request).await {
        Ok(response) => {
            // Check if the daemon response contains an error field.
            if let Some(error) = response.get("error") {
                if !error.is_null() {
                    return Ok(IpcResponse {
                        ok: false,
                        result: None,
                        error: Some(error.clone()),
                    });
                }
            }

            Ok(IpcResponse {
                ok: true,
                result: response.get("result").cloned(),
                error: None,
            })
        }
        Err(e) => Err(format!("IPC bridge error: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Application entry point
// ---------------------------------------------------------------------------

fn main() {
    // Initialize tracing for debug builds.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting Ochra desktop application");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, ipc_request])
        .run(tauri::generate_context!())
        .expect("error while running Ochra desktop application");
}
