//! JSON-RPC client that connects to the Ochra daemon over a Unix domain
//! socket and forwards requests from the Tauri frontend.
//!
//! The daemon speaks newline-delimited JSON-RPC 2.0 (one request per line,
//! one response per line). This module handles the connection lifecycle,
//! serialization, and deserialization.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{debug, error};

/// Send a single JSON-RPC request to the daemon and return the parsed
/// response.
///
/// # Arguments
///
/// * `socket_path` - Path to the daemon's Unix socket (e.g. `/tmp/ochra-daemon.sock`).
/// * `request`     - A complete JSON-RPC 2.0 request as a `serde_json::Value`.
///
/// # Errors
///
/// Returns an error if the connection fails, the write fails, or the
/// response cannot be parsed.
pub async fn send_rpc_request(
    socket_path: &str,
    request: &serde_json::Value,
) -> Result<serde_json::Value, IpcBridgeError> {
    // Connect to the daemon socket.
    let stream = UnixStream::connect(socket_path).await.map_err(|e| {
        error!(
            "Failed to connect to daemon socket at {}: {}",
            socket_path, e
        );
        IpcBridgeError::ConnectionFailed {
            path: socket_path.to_string(),
            reason: e.to_string(),
        }
    })?;

    debug!("Connected to daemon socket at {}", socket_path);

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Serialize the request to a single line of JSON, terminated by newline.
    let mut request_json = serde_json::to_string(request)
        .map_err(|e| IpcBridgeError::SerializationFailed(e.to_string()))?;
    request_json.push('\n');

    // Send the request.
    writer
        .write_all(request_json.as_bytes())
        .await
        .map_err(|e| {
            error!("Failed to write request to daemon: {}", e);
            IpcBridgeError::WriteFailed(e.to_string())
        })?;
    writer
        .flush()
        .await
        .map_err(|e| IpcBridgeError::WriteFailed(e.to_string()))?;

    debug!("Sent RPC request to daemon");

    // Read the response (one line).
    let mut response_line = String::new();
    let bytes_read = reader.read_line(&mut response_line).await.map_err(|e| {
        error!("Failed to read response from daemon: {}", e);
        IpcBridgeError::ReadFailed(e.to_string())
    })?;

    if bytes_read == 0 {
        return Err(IpcBridgeError::DaemonDisconnected);
    }

    // Parse the JSON response.
    let response: serde_json::Value = serde_json::from_str(&response_line).map_err(|e| {
        error!("Failed to parse daemon response: {}", e);
        IpcBridgeError::ParseFailed {
            reason: e.to_string(),
            raw: response_line.clone(),
        }
    })?;

    debug!("Received RPC response from daemon");

    Ok(response)
}

/// Errors that can occur during IPC communication with the daemon.
#[derive(Debug, thiserror::Error)]
pub enum IpcBridgeError {
    /// Failed to connect to the daemon socket.
    #[error("Failed to connect to daemon at '{path}': {reason}")]
    ConnectionFailed { path: String, reason: String },

    /// Failed to serialize the request.
    #[error("Failed to serialize RPC request: {0}")]
    SerializationFailed(String),

    /// Failed to write to the socket.
    #[error("Failed to write to daemon socket: {0}")]
    WriteFailed(String),

    /// Failed to read from the socket.
    #[error("Failed to read from daemon socket: {0}")]
    ReadFailed(String),

    /// The daemon closed the connection unexpectedly.
    #[error("Daemon disconnected unexpectedly (EOF)")]
    DaemonDisconnected,

    /// Failed to parse the daemon's response as JSON.
    #[error("Failed to parse daemon response: {reason} (raw: {raw})")]
    ParseFailed { reason: String, raw: String },
}
