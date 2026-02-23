// Prevents an additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ipc_bridge;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

// ---------------------------------------------------------------------------
// Daemon socket path resolution (mirrors ochra-daemon config.rs logic)
// ---------------------------------------------------------------------------

/// Resolve the daemon socket path.
///
/// Priority: `$OCHRA_SOCKET_PATH` > `$OCHRA_DATA_DIR/daemon.sock` > platform default.
fn resolve_socket_path() -> String {
    if let Ok(p) = std::env::var("OCHRA_SOCKET_PATH") {
        return p;
    }

    let data_dir = if let Ok(dir) = std::env::var("OCHRA_DATA_DIR") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        #[cfg(target_os = "macos")]
        {
            PathBuf::from(home).join("Library/Application Support/Ochra")
        }
        #[cfg(not(target_os = "macos"))]
        {
            PathBuf::from(home).join(".ochra")
        }
    } else {
        PathBuf::from("/tmp/ochra")
    };

    data_dir.join("daemon.sock").to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// Daemon lifecycle management
// ---------------------------------------------------------------------------

/// Try to spawn the daemon process if the socket doesn't exist.
///
/// Returns the child process handle so we can kill it on exit.
fn maybe_spawn_daemon(socket_path: &str) -> Option<std::process::Child> {
    // If socket already exists, daemon is likely running.
    if std::path::Path::new(socket_path).exists() {
        info!("Daemon socket already exists at {}", socket_path);
        return None;
    }

    info!("Daemon socket not found, attempting to spawn ochra-daemon");

    // Try to find the daemon binary: next to the Tauri binary, or on PATH.
    let daemon_bin = find_daemon_binary();

    match std::process::Command::new(&daemon_bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => {
            info!("Spawned ochra-daemon (pid {})", child.id());
            Some(child)
        }
        Err(e) => {
            warn!(
                "Could not spawn ochra-daemon (tried '{}'): {}",
                daemon_bin.display(),
                e
            );
            None
        }
    }
}

/// Locate the daemon binary.
fn find_daemon_binary() -> PathBuf {
    // 1. Check next to the current executable (bundled app scenario)
    if let Ok(exe) = std::env::current_exe() {
        let sibling = exe.with_file_name("ochra-daemon");
        if sibling.exists() {
            return sibling;
        }
    }

    // 2. Check CARGO_TARGET_DIR / target dir (dev scenario)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let target_bin = parent.join("ochra-daemon");
            if target_bin.exists() {
                return target_bin;
            }
        }
    }

    // 3. Fall back to PATH
    PathBuf::from("ochra-daemon")
}

/// Block until the daemon socket appears or timeout is reached.
fn wait_for_socket(socket_path: &str, timeout: Duration) -> bool {
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(100);

    while start.elapsed() < timeout {
        if std::path::Path::new(socket_path).exists() {
            info!("Daemon socket ready at {}", socket_path);
            return true;
        }
        std::thread::sleep(poll_interval);
    }

    error!(
        "Timed out waiting for daemon socket at {} ({:.1}s)",
        socket_path,
        timeout.as_secs_f64()
    );
    false
}

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
    let socket_path = resolve_socket_path();

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

    let socket_path = resolve_socket_path();
    info!("Daemon socket path: {}", socket_path);

    // Spawn daemon if not already running.
    let daemon_child: Mutex<Option<std::process::Child>> =
        Mutex::new(maybe_spawn_daemon(&socket_path));

    // Wait for socket to appear (up to 10s).
    if !std::path::Path::new(&socket_path).exists() {
        if !wait_for_socket(&socket_path, Duration::from_secs(10)) {
            warn!("Daemon socket not available — UI will show connection errors");
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet, ipc_request])
        .on_event(move |_app, event| {
            if let tauri::RunEvent::Exit = event {
                // Kill the daemon we spawned on exit.
                if let Ok(mut guard) = daemon_child.lock() {
                    if let Some(ref mut child) = *guard {
                        info!("Shutting down daemon (pid {})", child.id());
                        let _ = child.kill();
                        let _ = child.wait();
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Ochra desktop application");
}
