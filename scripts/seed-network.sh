#!/usr/bin/env bash
#
# seed-network.sh - Start a local 3-node Ochra test network.
#
# Each node gets its own temp data directory with a dedicated SQLite database
# and runs on a distinct port (9001, 9002, 9003). All nodes are pre-bootstrapped
# to discover each other.
#
# Usage:
#   ./scripts/seed-network.sh          # uses debug build (default)
#   ./scripts/seed-network.sh release  # uses release build
#
# Press Ctrl-C to shut down all three daemon processes and clean up.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Determine build profile
PROFILE="${1:-debug}"
DAEMON_BIN="$PROJECT_ROOT/target/$PROFILE/ochra-daemon"

if [[ ! -x "$DAEMON_BIN" ]]; then
  echo "Error: daemon binary not found at $DAEMON_BIN"
  echo "Build it first with:"
  if [[ "$PROFILE" == "release" ]]; then
    echo "  cargo build --release -p ochra-daemon"
  else
    echo "  cargo build -p ochra-daemon"
  fi
  exit 1
fi

# Ports for the three nodes
PORTS=(9001 9002 9003)

# Create temporary data directories
TEMP_DIRS=()
PIDS=()

cleanup() {
  echo ""
  echo "Shutting down seed network..."

  for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      echo "  Stopping daemon (PID $pid)"
      kill "$pid" 2>/dev/null || true
    fi
  done

  # Wait briefly for graceful shutdown, then force-kill stragglers
  sleep 1
  for pid in "${PIDS[@]}"; do
    if kill -0 "$pid" 2>/dev/null; then
      echo "  Force-killing daemon (PID $pid)"
      kill -9 "$pid" 2>/dev/null || true
    fi
  done

  for dir in "${TEMP_DIRS[@]}"; do
    if [[ -d "$dir" ]]; then
      echo "  Removing $dir"
      rm -rf "$dir"
    fi
  done

  echo "Seed network stopped."
}

trap cleanup EXIT INT TERM

echo "============================================"
echo "  Ochra Seed Network (3 nodes)"
echo "============================================"
echo ""
echo "Binary:  $DAEMON_BIN"
echo "Ports:   ${PORTS[*]}"
echo ""

# Build the bootstrap nodes list (TOML array format)
BOOTSTRAP_NODES='["127.0.0.1:9001", "127.0.0.1:9002", "127.0.0.1:9003"]'

for i in "${!PORTS[@]}"; do
  PORT="${PORTS[$i]}"
  NODE_NUM=$((i + 1))

  # Create a temp directory for this node
  DATA_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ochra-node${NODE_NUM}-XXXXXX")"
  TEMP_DIRS+=("$DATA_DIR")

  # Write a TOML config file for this node
  CONFIG_FILE="$DATA_DIR/config.toml"
  cat > "$CONFIG_FILE" <<EOF
[network]
listen_port = ${PORT}
bootstrap_nodes = ${BOOTSTRAP_NODES}
max_connections = 256
relay_enabled = true

[storage]
data_dir = "${DATA_DIR}"
earning_level = "medium"

[identity]
session_timeout_minutes = 60

[privacy]
cover_traffic_enabled = false
relay_country_diversity = false

[advanced]
log_level = "debug"
EOF

  echo "Node $NODE_NUM:"
  echo "  Port:      $PORT"
  echo "  Data dir:  $DATA_DIR"
  echo "  Config:    $CONFIG_FILE"
  echo ""

  # Start the daemon with OCHRA_DATA_DIR pointing to our temp directory
  OCHRA_DATA_DIR="$DATA_DIR" \
  RUST_LOG="ochra=debug" \
    "$DAEMON_BIN" &

  PIDS+=($!)
  echo "  PID:       ${PIDS[$i]}"
  echo ""

  # Small delay to stagger startup
  sleep 0.5
done

echo "============================================"
echo "  All 3 nodes running. Press Ctrl-C to stop."
echo "============================================"
echo ""
echo "  Node 1: 127.0.0.1:${PORTS[0]}  (PID ${PIDS[0]})"
echo "  Node 2: 127.0.0.1:${PORTS[1]}  (PID ${PIDS[1]})"
echo "  Node 3: 127.0.0.1:${PORTS[2]}  (PID ${PIDS[2]})"
echo ""

# Wait for all background processes; if any exits, keep waiting for the rest
wait
