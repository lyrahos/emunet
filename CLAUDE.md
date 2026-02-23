# CLAUDE.md — Ochra Project Instructions

## What is Ochra?
A decentralized, privacy-first P2P content distribution network with a built-in
cryptocurrency economy (Seeds). See `docs/` for the full specifications.

## Architecture
- **Daemon (Rust):** `crates/` — 20 library crates + 1 binary crate (`ochra-daemon`)
- **UI (TypeScript/React):** `ui/` — Tauri v2 desktop app
- **Circuits (Rust):** `circuits/` — Groth16/BLS12-381 ZK proof circuits

## Build Commands
```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Run test vector verification
cargo run --bin ochra-testvec -- --verify

# Build UI
cd ui && pnpm install && pnpm dev        # dev server
cd ui && pnpm tauri build                 # production build
```

## Key Files
- `docs/Ochra_v5_5_Unified_Technical_Specification.md` — Protocol spec (source of truth)
- `docs/Ochra_v5_5_Human_Interface_Specification.md` — UI spec (source of truth)
- `crates/ochra-types/src/` — Shared data structures (Section 22 of spec)
- `crates/ochra-daemon/src/commands/` — IPC command handlers (Section 21 of spec)

## Coding Rules
- All protocol-internal names match the v5.5 spec exactly
- User-facing names use the terminology mapping from HIS Section 1
- No `unwrap()` in library code — use `?` or `expect("reason")`
- No `unsafe` without `// SAFETY:` comment
- BLAKE3 context strings must be registered in Section 2.3 — never invent new ones
- All IPC errors must map to Section 29 error codes
- Micro-seeds are u64 (1 Seed = 100,000,000 micro-seeds)
- Run `cargo fmt` and `cargo clippy` before committing

## Milestone Tracking
GitHub Milestones correspond 1:1 with build phases. Check the current milestone
before starting work. Dependencies are:
Phase 1 → (none), Phase 3 → 1, Phase 4 → 1,3, etc.
See Section 24 of the tech spec for the full dependency graph.
