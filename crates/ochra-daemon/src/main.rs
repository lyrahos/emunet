//! ochra-daemon: the main Ochra network daemon.
//!
//! Single OS process running a Tokio async runtime. The UI communicates
//! with the daemon via JSON-RPC over Unix socket (Section 32).

mod commands;
mod config;
mod epoch;
mod events;
mod rpc;

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::{error, info, warn};

use crate::config::DaemonConfig;
use crate::events::EventBus;
use crate::rpc::RpcServer;

/// Daemon-wide shared state.
pub struct DaemonState {
    /// Database connection.
    pub db: Arc<tokio::sync::Mutex<rusqlite::Connection>>,
    /// Configuration.
    pub config: DaemonConfig,
    /// Event bus for pushing events to subscribers.
    pub event_bus: EventBus,
    /// Whether the session is unlocked (PIK decrypted).
    pub unlocked: Arc<RwLock<bool>>,
    /// Shutdown signal sender.
    pub shutdown_tx: broadcast::Sender<()>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("ochra=info".parse()?),
        )
        .init();

    info!("Ochra daemon starting");

    // 1. Load config
    let config = DaemonConfig::load()?;
    let data_dir = config.data_dir();

    // Ensure data directory exists
    std::fs::create_dir_all(&data_dir)?;

    // 2. Open database
    let db_path = data_dir.join("ochra.db");
    let conn = if db_path.exists() {
        ochra_db::open(&db_path)?
    } else {
        ochra_db::open(&db_path)?
    };
    let db = Arc::new(tokio::sync::Mutex::new(conn));

    // 3. Create event bus
    let event_bus = EventBus::new(1000);

    // 4. Create shutdown channel
    let (shutdown_tx, _shutdown_rx) = broadcast::channel(1);

    // 5. Build daemon state
    let state = Arc::new(DaemonState {
        db,
        config,
        event_bus,
        unlocked: Arc::new(RwLock::new(false)),
        shutdown_tx: shutdown_tx.clone(),
    });

    // 6. Start IPC server
    let socket_path = data_dir.join("daemon.sock");
    let rpc_server = RpcServer::new(state.clone(), socket_path.clone());

    info!("Starting JSON-RPC server on {:?}", socket_path);

    // 7. Emit DaemonStarted event
    state.event_bus.emit(events::Event {
        event_type: "DaemonStarted".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        payload: serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
        }),
    });

    // 8. Run the RPC server until shutdown
    let mut shutdown_rx = shutdown_tx.subscribe();
    tokio::select! {
        result = rpc_server.run() => {
            if let Err(e) = result {
                error!("RPC server error: {}", e);
            }
        }
        _ = shutdown_rx.recv() => {
            info!("Shutdown signal received");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Ctrl-C received, shutting down");
        }
    }

    // Graceful shutdown
    info!("Daemon shutting down gracefully");

    // Clean up socket file
    let _ = std::fs::remove_file(&socket_path);

    info!("Daemon stopped");
    Ok(())
}
