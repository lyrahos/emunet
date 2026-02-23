# Ochra v1.0 — Engineering Specification for Claude Code

*Implementation plan for the first buildable release of the Ochra protocol*

| **Property** | **Value** |
|---|---|
| Version | 1.0-alpha |
| Status | Implementation-Ready |
| Source Specs | Ochra v5.5 Unified Technical Specification, Ochra v5.5 Human Interface Specification |
| Language (Daemon) | Rust (2021 edition, stable toolchain) |
| Language (UI) | TypeScript + React (Tauri for desktop) |
| Repository | GitHub monorepo |
| Target Platforms (v1) | macOS (arm64, x86_64), Linux (x86_64) |
| Deferred Platforms | Windows, Android, iOS |

---

## 1. Document Purpose

This document translates the Ochra v5.5 Unified Technical Specification and Human Interface Specification into a concrete engineering plan for Claude Code. It defines repository structure, technology choices, build targets, milestone breakdown, coding standards, testing strategy, and CI/CD configuration. Claude Code should treat this document as the authoritative implementation guide.

**Core Principle:** Build the simplest correct implementation of each protocol phase. Prefer correctness over optimization. Every protocol invariant from the v5.5 spec is a hard requirement; every performance target is a soft goal for v1.

---

## 2. Scope: What Ships in v1

v1 is a **functional proof-of-concept** that demonstrates the complete Ochra protocol loop: identity creation → Space formation → content publishing → content purchase → Seed earning → Whisper messaging. It targets desktop (macOS + Linux) with a Tauri/React UI.

### 2.1 Included in v1

| **Category** | **Features** | **Spec Phases** |
|---|---|---|
| Identity | PIK generation, password auth, biometric enrollment (system keychain) | 1 |
| Cryptography | Ed25519, ChaCha20-Poly1305, BLAKE3 (all 36 context strings), X25519, Argon2id, Poseidon hash | 1 |
| ZK Proofs | Groth16/BLS12-381 circuit compilation, proving, verification (all 4 circuits) | 1, 2 |
| Transport | QUIC/TLS 1.3, Sphinx 8192-byte packets, wire protocol envelope, CBOR serialization | 3 |
| DHT | Kademlia + BEP 44, bootstrap, record storage, multi-record chunking | 4 |
| Invites | Anonymous rendezvous, contact exchange tokens, invite links | 5 |
| Storage (ABR) | Chunk storage, Reed-Solomon, LFU-DA eviction, earning levels, service receipts | 6 |
| Onion Routing | 3-hop Sphinx circuits, relay selection, circuit rotation (cover traffic simplified) | 7 |
| Group Encryption | MLS (RFC 9420), Double Ratchet, member add/remove | 8 |
| Reputation | PoSrv scoring, SybilGuard trust graph | 9 |
| Threshold Crypto | FROST DKG + ROAST | 10 |
| Oracle (Phase 1) | TWAP calculation stub (hardcoded for v1, full MPC TLS deferred) | 11-12 |
| Token Minting | VOPRF blind tokens, Groth16 minting proof, CR throttling | 13 |
| Nullifiers | NullifierSet Bloom filter, gossip, double-spend prevention | 14 |
| VYS | Yield share accounting, EpochState, pull-based claims | 15 |
| Publishing | Argon2id-PoW, zk-PoR, content manifests | 16 |
| Transactions | Micro/macro, blind receipts, threshold escrow | 17 |
| Revenue | Revenue splits, 30-day timelocks | 18 |
| Recovery | Guardian DKG, 48-hour veto, dead drop heartbeats | 19-20 |
| Database | SQLite schema (all 18 tables) | 21 |
| IPC | JSON-RPC over Unix socket, all commands, event subscriptions | 22 |
| TypeScript Bindings | Auto-generated from Rust structs | 23 |
| Events | Full event system, daemon lifecycle | 24 |
| UI | Complete Tauri/React desktop app (all HIS screens) | 25-31 |
| Platform | Tauri packaging for macOS + Linux | 32 |

### 2.2 Deferred to v1.1+

| **Feature** | **Reason** |
|---|---|
| Windows build | Platform-specific packaging; straightforward to add after macOS/Linux |
| Mobile (Android/iOS) | Requires React Native/KMP; separate build pipeline |
| Full MPC TLS Oracle | DECO/TLSNotary integration complex; v1 uses a hardcoded Oracle rate with manual override |
| Post-Quantum KEM (ML-KEM-768) | Hybrid X25519+ML-KEM-768 added as upgrade; v1 uses X25519-only for QUIC handshakes |
| ElGamal receipt re-encryption | Anti-fingerprint re-publication; v1 uses simpler receipt blob storage |
| Cover traffic (full Loopix 4-tier) | v1 implements basic cover traffic (single-tier Poisson) for protocol correctness; full 4-tier model deferred |
| OTA P2P binary distribution | v1 uses standard GitHub Releases for updates |
| Trusted Setup ceremony | v1 uses dev ceremony parameters (insecure but functional); production ceremony before mainnet |
| Genesis manifest + token allocation | Deferred to mainnet launch |

### 2.3 Oracle Strategy for v1

The v5.5 spec requires a DECO/TLSNotary MPC TLS oracle querying 5 exchange APIs (Phases 11-12). For v1, this is replaced with:

- **Hardcoded Seed value:** 1 Seed = 1 USD equivalent (micro-seeds: 100,000,000 = 1 USD)
- **Admin override:** A dev-only IPC command `dev_set_oracle_rate(rate: u64)` allows manual TWAP adjustment during testing
- **Circuit Breaker:** Implemented as specified, triggered by staleness of the hardcoded rate (which never goes stale in v1, making it inert until the real oracle is connected)
- **Denomination formula:** Implemented as specified (Section 11.9) — it just receives a constant input

This ensures all downstream economic logic (minting, CR, VYS) runs against the real formulas and can be validated, while deferring the MPC complexity.

---

## 3. Repository Structure

```
ochra/
├── CLAUDE.md                          # Claude Code project instructions (points to this spec)
├── Cargo.toml                         # Workspace root
├── rust-toolchain.toml                # Pin Rust stable version
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                     # Main CI: lint, test, build
│   │   ├── release.yml                # Tagged release builds
│   │   └── security-audit.yml         # cargo-audit, cargo-deny
│   ├── ISSUE_TEMPLATE/
│   │   ├── phase-task.yml             # Template for build phase work
│   │   └── bug-report.yml
│   └── CODEOWNERS
├── crates/
│   ├── ochra-crypto/                  # Phase 1: All cryptographic primitives
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ed25519.rs             # Ed25519 signing/verification
│   │   │   ├── chacha20.rs            # ChaCha20-Poly1305 AEAD
│   │   │   ├── blake3.rs              # Domain-separated BLAKE3 (all 36 context strings)
│   │   │   ├── x25519.rs              # X25519 key agreement
│   │   │   ├── argon2id.rs            # Password hashing + PoW
│   │   │   ├── poseidon.rs            # Poseidon hash (BLS12-381 scalar field)
│   │   │   ├── groth16.rs             # Groth16 proving/verification
│   │   │   ├── pedersen.rs            # Pedersen commitments
│   │   │   ├── ecies.rs               # ECIES encrypt/decrypt (Section 2.5)
│   │   │   ├── voprf.rs               # Ristretto255 VOPRF (RFC 9497)
│   │   │   └── frost.rs               # FROST DKG + ROAST wrapper
│   │   └── tests/
│   │       └── test_vectors.rs        # Section 35 test vectors
│   ├── ochra-testvec/                 # Phase 1: Test vector generator binary
│   │   ├── Cargo.toml
│   │   └── src/main.rs                # Generates test_vectors.json
│   ├── ochra-transport/               # Phase 3: QUIC, Sphinx, wire protocol
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── quic.rs                # QUIC/TLS 1.3 connection management
│   │       ├── sphinx.rs              # Sphinx packet construction/processing
│   │       ├── wire.rs                # ProtocolMessage envelope
│   │       ├── cbor.rs                # CBOR serialization (Section 26.5 key maps)
│   │       └── messages.rs            # All message payload structs (Section 26.4)
│   ├── ochra-dht/                     # Phase 4: Kademlia + BEP 44
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── kademlia.rs            # K=20, α=3, bucket management
│   │       ├── bep44.rs               # Mutable/immutable records
│   │       ├── chunking.rs            # Multi-record chunking (Section 28.3)
│   │       └── bootstrap.rs           # Network bootstrap
│   ├── ochra-invite/                  # Phase 5: Anonymous rendezvous
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── rendezvous.rs          # Introduction + rendezvous protocol
│   │       ├── contact_exchange.rs    # Contact token generation/redemption
│   │       └── invite.rs              # Invite link creation/parsing
│   ├── ochra-storage/                 # Phase 6: ABR, chunking, Reed-Solomon
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chunker.rs             # 4 MB chunk splitting
│   │       ├── reed_solomon.rs        # k=4, n=8 erasure coding
│   │       ├── abr.rs                 # ABR lifecycle, LFU-DA eviction
│   │       ├── earning.rs             # Earning levels (Low/Med/High/Custom)
│   │       └── receipts.rs            # Service receipt generation (Section 14.7)
│   ├── ochra-onion/                   # Phase 7: Sphinx routing, cover traffic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── circuit.rs             # 3-hop circuit construction/rotation
│   │       ├── relay.rs               # Relay selection (Section 4.9)
│   │       ├── cover.rs               # Cover traffic (simplified single-tier for v1)
│   │       └── nat.rs                 # NAT traversal
│   ├── ochra-mls/                     # Phase 8: MLS group encryption
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── group.rs               # MLS group lifecycle
│   │       ├── ratchet.rs             # Double Ratchet for group keys
│   │       └── subgroup.rs            # Subgroup (Channel) management
│   ├── ochra-posrv/                   # Phase 9: Proof of Service
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scoring.rs             # PoSrv formula (Section 9.1)
│   │       └── sybilguard.rs          # Trust graph (Section 9.2)
│   ├── ochra-frost/                   # Phase 10: FROST DKG + ROAST
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dkg.rs                 # DKG ceremony (Section 12.6)
│   │       ├── roast.rs               # ROAST wrapper
│   │       ├── quorum.rs              # Quorum membership + churn dampening
│   │       └── reshare.rs             # Proactive secret resharing (Section 12.8)
│   ├── ochra-oracle/                  # Phases 11-12: Oracle (stub for v1)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── twap.rs                # TWAP calculation
│   │       ├── denomination.rs        # Denomination formula (Section 11.9)
│   │       ├── circuit_breaker.rs     # Circuit Breaker + Emergency Pause
│   │       └── stub.rs                # Hardcoded rate for v1
│   ├── ochra-mint/                    # Phase 13: Token minting
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── voprf_mint.rs          # Blind token issuance
│   │       ├── groth16_mint.rs        # Minting circuit proof (Section 31.1)
│   │       └── cr_throttle.rs         # Collateral Ratio throttling
│   ├── ochra-nullifier/               # Phase 14: Double-spend defense
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── bloom.rs               # NullifierSet Bloom filter (Section 12.4)
│   │       ├── gossip.rs              # Nullifier gossip protocol (Section 12.5)
│   │       └── refund.rs              # Refund commitment tree
│   ├── ochra-vys/                     # Phase 15: Validator Yield Shares
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── accounting.rs          # VYS accumulator (Section 11.8)
│   │       ├── claims.rs              # Pull-based claims + optional ZK
│   │       └── decay.rs               # Decay/slash/CR formula
│   ├── ochra-pow/                     # Phase 16: Proof of Work + zk-PoR
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── argon2id_pow.rs        # Publishing PoW
│   │       └── zk_por.rs              # zk-PoR circuit (Section 31.2)
│   ├── ochra-spend/                   # Phase 17: Transactions
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── micro.rs               # Micro transactions (< 5 Seeds)
│   │       ├── macro_tx.rs            # Macro transactions (≥ 5 Seeds, escrow)
│   │       ├── blind_receipt.rs       # Blind receipt token system
│   │       └── transfer.rs            # P2P transfer notes (Section 11.3)
│   ├── ochra-revenue/                 # Phase 18: Revenue splits
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── splits.rs              # Immutable splits + 30-day timelock proposals
│   ├── ochra-guardian/                # Phases 19-20: Recovery Contacts
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── dkg.rs                 # Guardian DKG ceremony
│   │       ├── heartbeat.rs           # Dead drop heartbeats
│   │       ├── recovery.rs            # 48-hour Dual-Path Cancellation
│   │       └── replacement.rs         # Guardian replacement
│   ├── ochra-db/                      # Phase 21: SQLite database
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── schema.rs              # All 18 tables (Section 27)
│   │   │   ├── migrations/            # SQL migration files
│   │   │   └── queries.rs             # Typed query layer
│   │   └── migrations/
│   │       └── 001_initial.sql        # Complete schema
│   ├── ochra-daemon/                  # Phases 22, 24: Main daemon binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs                # Entry point, daemon lifecycle (Section 32)
│   │       ├── rpc.rs                 # JSON-RPC server (Unix socket)
│   │       ├── commands/              # IPC command handlers (Sections 21.1-21.6)
│   │       │   ├── mod.rs
│   │       │   ├── identity.rs        # init_pik, authenticate, etc.
│   │       │   ├── network.rs         # create_group, join_group, etc.
│   │       │   ├── economy.rs         # get_wallet_balance, send_funds, etc.
│   │       │   ├── file_io.rs         # publish_file, purchase_content, etc.
│   │       │   ├── whisper.rs         # start_whisper, send_whisper, etc.
│   │       │   └── diagnostics.rs     # get_daemon_logs, etc.
│   │       ├── events.rs              # Event emission system (Section 23)
│   │       ├── epoch.rs               # Epoch boundary processing
│   │       └── config.rs              # Configuration file (Section 33)
│   └── ochra-types/                   # Phase 23: Shared types
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── identity.rs            # PikMeta, Contact, PeerProfile, etc.
│           ├── space.rs               # GroupSummary, GroupSettings, etc.
│           ├── content.rs             # ContentManifest, PricingTier, etc.
│           ├── economy.rs             # PurchaseRecord, EarningsReport, etc.
│           ├── whisper.rs             # WhisperSessionSummary, etc.
│           ├── network.rs             # ServiceReceipt, RelayDescriptor, EpochState, etc.
│           ├── events.rs              # All event type definitions
│           └── errors.rs              # Error codes (Section 29)
├── ui/                                # Phases 25-31: Tauri + React frontend
│   ├── package.json
│   ├── tsconfig.json
│   ├── tailwind.config.ts
│   ├── vite.config.ts
│   ├── src-tauri/
│   │   ├── Cargo.toml                 # Tauri Rust backend
│   │   ├── tauri.conf.json
│   │   └── src/
│   │       ├── main.rs                # Tauri entry point
│   │       └── ipc_bridge.rs          # JSON-RPC client → Tauri commands
│   └── src/
│       ├── main.tsx                   # React entry
│       ├── App.tsx                    # Root with routing
│       ├── types/                     # Phase 23: Generated TypeScript types
│       │   └── ochra.ts               # All IPC request/response types
│       ├── hooks/
│       │   ├── useIpc.ts              # JSON-RPC hook
│       │   ├── useEvents.ts           # Event subscription hook
│       │   ├── useBalance.ts          # Wallet balance
│       │   └── useSpaces.ts           # Space list
│       ├── components/
│       │   ├── ui/                    # Design system (Phase 25)
│       │   │   ├── Button.tsx
│       │   │   ├── Card.tsx
│       │   │   ├── Badge.tsx          # Role badges (Host/Creator/Moderator)
│       │   │   ├── Modal.tsx
│       │   │   ├── Slider.tsx         # Earning level slider
│       │   │   ├── BottomSheet.tsx
│       │   │   └── Toast.tsx
│       │   ├── layout/
│       │   │   ├── Sidebar.tsx        # Desktop sidebar nav
│       │   │   ├── BottomNav.tsx      # (future: mobile)
│       │   │   └── Shell.tsx          # App shell with sidebar
│       │   └── space/
│       │       ├── SpaceCard.tsx       # Home screen Space cards
│       │       ├── LayoutRenderer.tsx  # Phase 28: Sandboxed renderer
│       │       ├── StorefrontGrid.tsx
│       │       ├── ForumThread.tsx
│       │       ├── NewsFeed.tsx
│       │       ├── GalleryMosaic.tsx
│       │       └── LibraryList.tsx
│       ├── pages/
│       │   ├── setup/                 # Phase 26: Setup Assistant
│       │   │   ├── Welcome.tsx        # Step 1: Name + password
│       │   │   ├── MeetSeeds.tsx      # Step 2: Animation
│       │   │   ├── EarnSetup.tsx      # Step 3: Earning level
│       │   │   ├── Recovery.tsx       # Step 4: Recovery Contacts
│       │   │   └── Ready.tsx          # Step 5: Summary
│       │   ├── Home.tsx               # Phase 27: Space list
│       │   ├── SpaceBuilder.tsx       # Phase 27: Create/edit Space
│       │   ├── SpaceView.tsx          # Phase 28: Inside a Space
│       │   ├── Dashboard.tsx          # Phase 29.5: Host Dashboard
│       │   ├── People.tsx             # Phase 29.5: Member management
│       │   ├── SpaceSettings.tsx      # Phase 29.5: Host settings
│       │   ├── Seeds.tsx              # Phase 29: Wallet
│       │   ├── Earn.tsx               # Phase 29: Earning settings
│       │   ├── You.tsx                # Phase 29: Personal settings
│       │   ├── Contacts.tsx           # Phase 29: Contact management
│       │   ├── Whisper.tsx            # Phase 29.7: Whisper hub
│       │   ├── WhisperChat.tsx        # Phase 29.7: Conversation view
│       │   ├── Checkout.tsx           # Phase 31: Purchase flow
│       │   ├── PurchaseLibrary.tsx    # Phase 29: My Purchases
│       │   └── RecoverySetup.tsx      # Phase 30: Guardian management
│       └── lib/
│           ├── rpc-client.ts          # JSON-RPC client (Unix socket via Tauri)
│           ├── event-bus.ts           # Event subscription manager
│           ├── terminology.ts         # Protocol → user-facing term mapping (Section 1 HIS)
│           └── format.ts              # Seed formatting, relative dates, etc.
├── circuits/                          # Phase 2: Groth16 circuit definitions
│   ├── minting/                       # Section 31.1 (~45k constraints)
│   │   └── circuit.rs
│   ├── zk_por/                        # Section 31.2 (~150-300k constraints)
│   │   └── circuit.rs
│   ├── refund/                        # Section 31.3 (~50-60k constraints)
│   │   └── circuit.rs
│   ├── content_key/                   # Section 31.4 (~20k constraints)
│   │   └── circuit.rs
│   └── params/                        # Dev ceremony parameters (NOT production)
│       └── README.md
├── scripts/
│   ├── dev-setup.sh                   # Install all dev dependencies
│   ├── generate-types.sh              # Rust → TypeScript type generation
│   ├── dev-ceremony.sh                # Generate dev trusted setup params
│   └── seed-network.sh                # Spin up a local 3-node test network
├── tests/
│   ├── integration/                   # Cross-crate integration tests
│   │   ├── full_loop.rs               # Identity → Space → publish → purchase → earn
│   │   ├── whisper_e2e.rs             # End-to-end Whisper session
│   │   └── recovery_flow.rs           # Guardian recovery
│   └── fixtures/
│       └── test_vectors.json          # Generated by ochra-testvec
└── docs/
    ├── Ochra_v5_5_Unified_Technical_Specification.md
    ├── Ochra_v5_5_Human_Interface_Specification.md
    └── architecture.md                # High-level architecture diagram
```

---

## 4. Technology Stack

### 4.1 Daemon (Rust)

| **Dependency** | **Purpose** | **Crate** |
|---|---|---|
| Ed25519 | Signatures | `ed25519-dalek` |
| X25519 | Key agreement | `x25519-dalek` |
| ChaCha20-Poly1305 | AEAD | `chacha20poly1305` |
| BLAKE3 | Hashing, KDF, MAC | `blake3` |
| Argon2id | Password hashing, PoW | `argon2` |
| Groth16/BLS12-381 | ZK proofs | `arkworks` (`ark-groth16`, `ark-bls12-381`) |
| Poseidon | In-circuit hashing | `ark-crypto-primitives` + custom params |
| Pedersen | Commitments | `ark-crypto-primitives` |
| FROST | Threshold signatures | `frost-ed25519` (ZcashFoundation) |
| VOPRF | Blind token issuance | `voprf` (Ristretto255) |
| MLS | Group encryption | `openmls` |
| QUIC | Transport | `quinn` |
| TLS 1.3 | Session security | `rustls` |
| CBOR | Wire serialization | `ciborium` |
| SQLite | Local database | `rusqlite` |
| JSON-RPC | IPC | `jsonrpc-core` |
| Async runtime | Concurrency | `tokio` |
| Reed-Solomon | Erasure coding | `reed-solomon-erasure` |
| Serde | Serialization | `serde` + `serde_json` |
| Logging | Structured logs | `tracing` + `tracing-subscriber` |
| Config | Configuration file | `toml` |

### 4.2 UI (TypeScript/React)

| **Dependency** | **Purpose** |
|---|---|
| Tauri v2 | Desktop app framework |
| React 18+ | UI framework |
| Vite | Build tool |
| Tailwind CSS | Styling (utility-first) |
| React Router | Navigation |
| Zustand | State management |
| Framer Motion | Animations (spring, layout) |
| Lucide React | Icon system |

### 4.3 Build Tools

| **Tool** | **Purpose** |
|---|---|
| `cargo` | Rust workspace build |
| `cargo-nextest` | Fast parallel test runner |
| `cargo-audit` | Security vulnerability scanning |
| `cargo-deny` | License + dependency policy |
| `clippy` | Rust linting |
| `rustfmt` | Rust formatting |
| `pnpm` | Node package manager (UI) |
| `eslint` + `prettier` | TypeScript linting + formatting |
| `ts-rs` | Rust → TypeScript type generation |

---

## 5. GitHub Milestones & Issues

Each build phase from Section 24 of the v5.5 spec maps to a GitHub Milestone. Within each milestone, issues are created for discrete implementation tasks. The dependency graph from the spec is enforced by milestone ordering.

### Milestone 1: `ochra-crypto` (Phase 1)
*Dependencies: none*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #1 | Ed25519 key generation, signing, verification | Round-trip test, RFC 8032 test vectors pass |
| #2 | ChaCha20-Poly1305 AEAD encrypt/decrypt | RFC 8439 test vectors pass |
| #3 | BLAKE3 with all 36 domain-separated context strings | All strings from Section 2.3 registered, `hash`/`derive_key`/`keyed_hash` modes implemented, Section 35.2 vectors pass |
| #4 | X25519 key exchange | RFC 7748 Section 6.1 test vector passes |
| #5 | Argon2id (m=256MB, t=3, p=4) for PIK derivation | Correct output for known test input |
| #6 | Poseidon hash on BLS12-381 scalar field | Section 35.3 test vectors pass against `neptune` reference |
| #7 | Groth16/BLS12-381 prove + verify infrastructure | arkworks integration, proof size = 192 bytes, verify < 2ms |
| #8 | Pedersen commitments on BLS12-381 | Homomorphic property test |
| #9 | ECIES encrypt/decrypt (Section 2.5) | Section 35.7 round-trip vector passes |
| #10 | VOPRF (Ristretto255, RFC 9497) blind/evaluate/finalize | RFC 9497 test vectors pass |
| #11 | FROST Ed25519 DKG + ROAST wrapper | 3-of-5 threshold signing round-trip |
| #12 | `ochra-testvec` binary generating `test_vectors.json` | All Section 35 vectors generated and self-verified |

### Milestone 2: `ochra-trusted-setup` (Phase 2)
*Dependencies: M1*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #13 | Dev trusted setup ceremony script | Powers of Tau Phase 1 downloaded, Phase 2 per-circuit params generated |
| #14 | Minting circuit (Section 31.1, ~45k constraints) | Circuit compiles, prove + verify round-trips |
| #15 | zk-PoR circuit (Section 31.2, ~150-300k constraints) | Poseidon in-circuit, Merkle tree verification |
| #16 | Refund circuit (Section 31.3, ~50-60k constraints) | Nullifier derivation + commitment tree |
| #17 | Content key verification circuit (Section 31.4, ~20k constraints) | ECIES correctness proof |

### Milestone 3: `ochra-transport` (Phase 3)
*Dependencies: M1*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #18 | QUIC/TLS 1.3 connection manager | `quinn` + `rustls`, ALPN `"ochra/5"`, bidirectional streams |
| #19 | Sphinx 8192-byte packet construction + processing | Section 4.10 pseudocode implemented, 3-hop round-trip test |
| #20 | Wire protocol envelope (ProtocolMessage) | CBOR encode/decode, version field, msg_id, timestamp |
| #21 | All message payload structs (Section 26.4) | Every struct from Section 26.4 serializable/deserializable |
| #22 | CBOR key maps for all structs (Section 26.5) | Deterministic encoding verified across serialize/deserialize |
| #23 | Protocol version handshake (CapabilityExchange) | Two peers negotiate, `VERSION_MISMATCH` on incompatible |

### Milestone 4: `ochra-dht` (Phase 4)
*Dependencies: M1, M3*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #24 | Kademlia routing table (K=20, α=3) | Bucket management, FindNode, Section 4.8 parameters |
| #25 | BEP 44 mutable/immutable records | Put/Get with signatures, sequence numbers |
| #26 | Multi-record chunking (Section 28.3) | Records > 1000 bytes split and reassembled |
| #27 | Network bootstrap from seed nodes | 3-node local network bootstraps successfully |
| #28 | NullifierSet Bloom filter replication | Bloom filter sync between 2 nodes |

### Milestone 5: `ochra-invite` (Phase 5)
*Dependencies: M3, M4*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #29 | Anonymous rendezvous protocol | Full Introduction → Rendezvous flow between 2 peers via 3rd |
| #30 | Contact exchange tokens (Section 6.7) | Token generate → share → redeem round-trip |
| #31 | Invite link creation + parsing | `ochra://invite` deep link, TTL enforcement, single/unlimited use |

### Milestone 6: `ochra-storage` (Phase 6)
*Dependencies: M1, M3, M4*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #32 | 4 MB chunk splitting + Merkle tree | Content → chunks → Merkle root matches content_hash |
| #33 | Reed-Solomon (k=4, n=8) | Encode → drop 4 shards → recover |
| #34 | ABR lifecycle + LFU-DA eviction | Store chunks, evict LFU under pressure |
| #35 | Earning levels (Low/Med/High/Custom) | Allocation limits enforced per Section 13.1 HIS |
| #36 | Service receipt generation (Section 14.7) | Receipt created on chunk serve, signed correctly |
| #37 | Chunk retrieval protocol (Section 14.8) | Requester fetches chunk via circuit, blind receipt verified |

### Milestone 7: `ochra-onion` (Phase 7)
*Dependencies: M3, M4*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #38 | 3-hop circuit construction | Circuit built through 3 relay nodes, data transmitted end-to-end |
| #39 | Relay selection (Section 4.9) | AS diversity, bandwidth weighting, exclusion rules |
| #40 | Circuit rotation (10 min lifetime) | Automatic rotation with traffic migration |
| #41 | Simplified cover traffic (single-tier Poisson) | Background packets at configured rate |
| #42 | NAT traversal | Hole punching via relay, fallback to relayed connection |

### Milestone 8: `ochra-mls` (Phase 8)
*Dependencies: M3, M4, M7*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #43 | MLS group lifecycle (RFC 9420) | Create group, add member, remove member, key update |
| #44 | Sender-anonymous Sphinx routing for MLS | MLS messages routed through Sphinx circuits |
| #45 | Double Ratchet for group keys | Forward secrecy verified across member changes |
| #46 | Subgroup (Channel) management | Create/delete, grant/revoke access, message isolation |

### Milestone 9: `ochra-posrv` (Phase 9)
*Dependencies: M4*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #47 | PoSrv scoring formula (Section 9.1) | Score computed from GBs, uptime, zk-PoR, trust position |
| #48 | SybilGuard trust graph (Section 9.2) | Graph construction, trust_weight in SQLite |

### Milestone 10: `ochra-frost` (Phase 10)
*Dependencies: M1, M3, M7*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #49 | FROST DKG ceremony (Section 12.6) | Full 3-round DKG between 5 participants |
| #50 | ROAST wrapper for async liveness | Signing completes under simulated network delays |
| #51 | Quorum membership + churn dampening (Section 12.2) | Membership selection based on PoSrv, rotation limits |
| #52 | Proactive secret resharing (Section 12.8) | Key resharing without changing group public key |

### Milestone 11: `ochra-oracle` (Phases 11-12)
*Dependencies: M7, M10*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #53 | Hardcoded Oracle stub (v1) | Returns constant TWAP, `dev_set_oracle_rate` override |
| #54 | TWAP calculation formula | Moving average computed correctly from price vector |
| #55 | Denomination formula (Section 11.9) | Seed denomination computed from TWAP + infrastructure metrics |
| #56 | Circuit Breaker + Emergency Pause (Section 5.3) | Triggers on staleness, pauses minting, auto-recovers |

### Milestone 12: `ochra-mint` (Phase 13)
*Dependencies: M1, M2, M6, M9, M10*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #57 | VOPRF blind token issuance flow | Client blinds → quorum evaluates → client unblinds |
| #58 | Groth16 minting proof (Section 31.1) | Proof generated from receipt Merkle tree, quorum verifies |
| #59 | CR throttling | Minting rate adjusts with Collateral Ratio |

### Milestone 13: `ochra-nullifier` (Phase 14)
*Dependencies: M1, M4, M10*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #60 | NullifierSet Bloom filter (Section 12.4) | 20 hash functions, ~3.4 MB for 1M nullifiers |
| #61 | Nullifier gossip protocol (Section 12.5) | Gossip propagation verified across 3+ nodes |
| #62 | Refund commitment tree with epoch pruning | Tree operations, pruning at epoch boundary |

### Milestone 14: `ochra-vys` (Phase 15)
*Dependencies: M10, M12, M13*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #63 | VYS reward accumulator (Section 11.8) | Rewards accumulate based on PoSrv contributions |
| #64 | FROST-signed EpochState | Quorum signs epoch state, verifiable by all peers |
| #65 | Pull-based VYS claims | Claim rewards on demand, optional ZK proof |

### Milestone 15: `ochra-pow` (Phase 16)
*Dependencies: M1, M2, M6, M9*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #66 | Argon2id-PoW for publishing | Difficulty target met before content accepted |
| #67 | zk-PoR full circuit (Section 31.2) | Proof generated for MIN_CHUNKS=10, VRF beacon used |

### Milestone 16: `ochra-spend` (Phase 17)
*Dependencies: M2, M6, M12, M13*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #68 | Micro transactions (< 5 Seeds) | Instant settlement, 0.1% fee deducted |
| #69 | Macro transactions (≥ 5 Seeds) with escrow | Threshold escrow, 60s Creator timeout, auto-refund |
| #70 | Blind receipt tokens | Receipt generation, storage, anonymous re-download |
| #71 | P2P transfer note encryption (Section 11.3) | Encrypted note delivered with transfer |

### Milestone 17: `ochra-revenue` (Phase 18)
*Dependencies: M4, M8*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #72 | Revenue split enforcement | 10/70/20 default, immutable on creation |
| #73 | 30-day split change proposal | Proposal → countdown → apply or cancel |

### Milestone 18: `ochra-guardian` (Phases 19-20)
*Dependencies: M5, M7, M10*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #74 | Guardian DKG ceremony (Section 12.6) | 2-of-3 guardians nominated, shares distributed |
| #75 | Dead drop heartbeats | Heartbeat published/detected, health status tracked |
| #76 | 48-hour Dual-Path Cancellation | Recovery initiated → veto within window → cancelled |
| #77 | Guardian replacement | Replace one guardian, reshare without data loss |

### Milestone 19: `ochra-db` (Phase 21)
*Dependencies: M1*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #78 | SQLite schema — all 18 tables (Section 27) | Schema creates cleanly, all indices, FTS5 for search |
| #79 | Migration framework | Versioned migrations, upgrade path |
| #80 | Typed query layer | All common queries with compile-time checked SQL |

### Milestone 20: `ochra-daemon` (Phase 22)
*Dependencies: M19, all command implementations*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #81 | JSON-RPC server over Unix socket | Listens, accepts connections, dispatches commands |
| #82 | Identity commands (Section 21.1) | All 15 commands implemented and tested |
| #83 | Network/Space commands (Section 21.2) | All 27 commands implemented and tested |
| #84 | Economy commands (Section 21.3) | All 13 commands implemented and tested |
| #85 | File IO commands (Section 21.4) | All 14 commands implemented and tested |
| #86 | Whisper commands (Section 21.5) | All 14 commands implemented and tested |
| #87 | Diagnostics commands (Section 21.6) | All 7 commands implemented and tested |
| #88 | Error code mapping (Section 29) | All error codes returned correctly |
| #89 | Timeout/retry logic (Section 30) | Retries with backoff for transient failures |

### Milestone 21: `ochra-types` (Phase 23)
*Dependencies: M20*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #90 | `ts-rs` integration for Rust → TypeScript | All Section 22 structs generate correct TS types |
| #91 | `ochra.ts` generated and validated | TypeScript types match JSON-RPC request/response shapes |

### Milestone 22: `ochra-events` (Phase 24)
*Dependencies: M20*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #92 | Event emission system (Section 23) | All event types emitted by daemon with correct payloads |
| #93 | Event subscription (subscribe/unsubscribe) | Multiple subscriptions with filters, backpressure at 1000 |
| #94 | Daemon lifecycle (Section 32) | Clean startup, graceful shutdown, crash recovery |

### Milestone 23: UI — Design System (Phase 25)
*Dependencies: M21*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #95 | Tailwind config + design tokens | Color palette, typography scale, spacing |
| #96 | Core components: Button, Card, Badge, Modal, Slider, Toast | All components documented, accessible |
| #97 | Desktop sidebar layout (Shell) | Sidebar with nav tabs: Home, Seeds, Earn, You |
| #98 | Light/Dark/System theme support | Theme toggle persists, follows system |
| #99 | Session lock screen | 15-min inactivity, biometric + password re-auth |
| #100 | Spring animations + 60fps transitions | Framer Motion integration, smooth page transitions |

### Milestone 24: UI — Setup & Navigation (Phase 26)
*Dependencies: M23*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #101 | 5-step Setup Assistant | Full wizard: Welcome → Seeds → Earn → Recovery → Ready |
| #102 | Deep link parsing | `ochra://invite`, `ochra://connect`, `ochra://whisper` |
| #103 | Configuration file initialization | Section 33 config written on first launch |

### Milestone 25: UI — Home & Space Builder (Phase 27)
*Dependencies: M23, M24*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #104 | Home screen with Space cards | Role badges, unread dots, pin-to-top, search at 8+ |
| #105 | 4-step Space creation wizard | Name → Style → Invite → Summary |
| #106 | WYSIWYG Easy Mode editor | 5 templates, drag-and-drop, accent colors |
| #107 | Advanced Mode JSON editor | Raw LayoutManifest, full split sliders |

### Milestone 26: UI — Layout Renderer (Phase 28)
*Dependencies: M25*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #108 | Sandboxed LayoutRenderer | Renders all 5 templates: Shop, Community, Feed, Gallery, Library |
| #109 | Content search (FTS5 integration) | `search_catalog` connected to search bar |
| #110 | Free content, access badges, versioning | "Free" badge, "Yours"/"Access"/"Expired", successor banner |

### Milestone 27: UI — Seeds, Contacts, Purchases (Phase 29)
*Dependencies: M25, M26*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #111 | Seeds screen | Balance display, Send/Receive, transaction history |
| #112 | Contacts screen | Contact list, profile, add (QR + share), remove |
| #113 | Contact Card (frosted glass QR) | Token generation, Share Sheet, regenerate |
| #114 | P2P transfer flow | Contact → amount → note → confirm |
| #115 | Purchase library | History, access badges, re-download, refund |
| #116 | Download management | Progress bar, pause/resume, chunk-level |

### Milestone 28: UI — Host Experience (Phase 29.5)
*Dependencies: M27*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #117 | Host Dashboard | Stats cards, activity feed, earnings detail |
| #118 | People screen | Role management, promote/demote, invite |
| #119 | Moderation (Review Queue) | Keep/Remove, pseudonymous reporters, badges |
| #120 | Space Settings | Name, invite perms, publish policy, split proposal, ownership transfer |
| #121 | Invite Links management | Active list, revoke, expiry display |

### Milestone 29: UI — Whisper (Phase 29.7)
*Dependencies: M27*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #122 | Whisper hub | Active sessions, new Whisper flow, username resolution |
| #123 | Conversation view | Bubbles, typing, read receipts, Seed transfer, identity reveal |
| #124 | Username management | Setup, change, remove under You tab |
| #125 | Handle resolution + deprecation | Inline validation, successor redirect |

### Milestone 30: UI — Recovery (Phase 30)
*Dependencies: M27*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #126 | Recovery Contact setup flow | Contact selector, threshold display |
| #127 | Veto Recovery alert | Full-screen alert, "Cancel Recovery" button, countdown |
| #128 | Guardian health display | Days-since-heartbeat, replace flow |

### Milestone 31: UI — Checkout (Phase 31)
*Dependencies: M27, M28*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #129 | Checkout bottom-sheet | Pricing tiers, "Get" for free, escrow progress |
| #130 | Transaction feedback | Micro: instant ✓, Macro: progress ring, failure: ✕ + retry |
| #131 | Biometric prompts | System biometric or double-click confirm |

### Milestone 32: Platform Delivery (Phase 32)
*Dependencies: all*

| **Issue** | **Description** | **Acceptance Criteria** |
|---|---|---|
| #132 | Tauri macOS build (arm64 + x86_64) | DMG installer, code signing |
| #133 | Tauri Linux build (x86_64) | AppImage or .deb |
| #134 | Accessibility audit | VoiceOver support, WCAG AA contrast |
| #135 | Final integration test suite | Full-loop test passes on both platforms |

---

## 6. Coding Standards

### 6.1 Rust

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

- **Formatting:** `rustfmt` with default settings, enforced in CI.
- **Linting:** `clippy` at `warn` level with the following denies:

```toml
# Cargo.toml [workspace.lints.clippy]
unwrap_used = "deny"          # Use ? or expect() with context
panic = "deny"                 # No panics in library code
todo = "warn"                  # Track incomplete work
dbg_macro = "deny"             # No debug prints in committed code
```

- **Error handling:** All errors use `thiserror` for library crates, `anyhow` for the daemon binary. IPC errors must map to Section 29 error codes.
- **Unsafe:** Prohibited without explicit `// SAFETY:` comment and review.
- **Documentation:** All public types and functions must have `///` doc comments. Crate-level `//!` docs required.
- **Naming:**
  - Crate names: `ochra-{name}` (kebab-case)
  - Module names: `snake_case`
  - Struct/enum names: `PascalCase`
  - Function names: `snake_case`
  - Constants: `SCREAMING_SNAKE_CASE`
  - Protocol-internal names match the v5.5 spec exactly (e.g., `ServiceReceipt`, not `ChunkReceipt`)
- **Testing:** Every public function must have at least one unit test. Integration tests go in `tests/integration/`.

### 6.2 TypeScript/React

- **Formatting:** Prettier with 2-space indent, single quotes, trailing commas.
- **Linting:** ESLint with `typescript-eslint` strict config.
- **Components:** Functional components only. No class components.
- **State:** Zustand for global state. React hooks for local state. No prop drilling beyond 2 levels.
- **Styling:** Tailwind utility classes only. No inline styles. No CSS modules.
- **Types:** Strict mode. No `any`. Generated types from `ochra-types` are the source of truth.
- **Naming:**
  - Components: `PascalCase.tsx`
  - Hooks: `use{Name}.ts`
  - Utils: `camelCase.ts`
  - Types: `PascalCase` (matching Rust originals)
  - User-facing strings: Always use `terminology.ts` mapping, never protocol-internal names

### 6.3 Git Conventions

- **Branching:** `main` (protected), feature branches `feat/{milestone}-{description}`, bugfix branches `fix/{description}`
- **Commits:** Conventional Commits format: `feat(ochra-crypto): implement Ed25519 key generation`
  - Prefixes: `feat`, `fix`, `refactor`, `test`, `docs`, `ci`, `chore`
  - Scope: crate name or `ui`
- **PRs:** Require at least the CI checks to pass. Squash merge to `main`.
- **Tags:** Semantic versioning `v1.0.0-alpha.{n}` for milestone completions.

---

## 7. Testing Strategy

### 7.1 Unit Tests

Every crate has unit tests co-located with source code (`#[cfg(test)]` modules). Run with:

```bash
cargo nextest run --workspace
```

**Coverage target:** 80% line coverage for `ochra-crypto`, `ochra-transport`, `ochra-dht`. 60% for other crates.

### 7.2 Test Vectors (Section 35)

The `ochra-testvec` binary generates `test_vectors.json` containing all Section 35 vectors. CI runs:

```bash
cargo run --bin ochra-testvec -- --verify
```

This validates BLAKE3, Poseidon, Node ID derivation, Receipt ID derivation, Hybrid Session Secret, ECIES round-trip, Double Ratchet KDF chain, and Bloom filter hash derivation against the spec.

### 7.3 Integration Tests

Located in `tests/integration/`. These spin up multiple daemon instances and test cross-crate interactions:

- **`full_loop.rs`**: Identity creation → Space creation → content publish → content purchase → Seed earning → balance verification
- **`whisper_e2e.rs`**: Two nodes establish Whisper session → exchange messages → Seed transfer → identity reveal → close
- **`recovery_flow.rs`**: Nominate guardians → simulate password loss → initiate recovery → guardian shares → veto test → successful recovery
- **`network_bootstrap.rs`**: 3 nodes bootstrap, form DHT, exchange data
- **`revenue_split.rs`**: Create Space → publish → purchase → verify split distribution
- **`circuit_rotation.rs`**: Verify Sphinx circuits rotate every 10 minutes without data loss

### 7.4 UI Tests

- **Component tests:** Vitest + React Testing Library for component rendering
- **E2E tests:** Playwright for full UI flows against a running daemon

### 7.5 Local Test Network

`scripts/seed-network.sh` spins up a 3-node local network:

```bash
# Starts 3 daemon instances on ports 9001, 9002, 9003
# Each with its own SQLite database and PIK
# Pre-bootstrapped to discover each other
./scripts/seed-network.sh
```

---

## 8. CI/CD Configuration

### 8.1 Main CI (`.github/workflows/ci.yml`)

Triggered on every push and PR to `main`.

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  rust-checks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Format check
        run: cargo fmt --all -- --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Test vectors
        run: cargo run --bin ochra-testvec -- --verify
      - name: Unit tests
        run: cargo nextest run --workspace
      - name: Integration tests
        run: cargo nextest run --workspace --profile integration

  ui-checks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'pnpm'
          cache-dependency-path: ui/pnpm-lock.yaml
      - name: Install dependencies
        run: cd ui && pnpm install
      - name: Type check
        run: cd ui && pnpm tsc --noEmit
      - name: Lint
        run: cd ui && pnpm eslint src/
      - name: Format check
        run: cd ui && pnpm prettier --check src/
      - name: Unit tests
        run: cd ui && pnpm test

  build:
    needs: [rust-checks, ui-checks]
    strategy:
      matrix:
        os: [macos-latest, ubuntu-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: pnpm/action-setup@v2
      - name: Build daemon
        run: cargo build --release -p ochra-daemon
      - name: Build UI
        run: cd ui && pnpm install && pnpm tauri build
```

### 8.2 Security Audit (`.github/workflows/security-audit.yml`)

Weekly + on dependency changes:

```yaml
name: Security Audit
on:
  schedule:
    - cron: '0 0 * * 1'  # Weekly Monday
  push:
    paths: ['**/Cargo.lock']

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v1
      - name: cargo-deny
        run: cargo deny check
```

### 8.3 Release (`.github/workflows/release.yml`)

Triggered by version tags:

```yaml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  build-release:
    strategy:
      matrix:
        include:
          - os: macos-latest
            target: aarch64-apple-darwin
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - uses: pnpm/action-setup@v2
      - name: Build
        run: cd ui && pnpm install && pnpm tauri build --target ${{ matrix.target }}
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: ochra-${{ matrix.target }}
          path: ui/src-tauri/target/${{ matrix.target }}/release/bundle/
```

---

## 9. CLAUDE.md (Claude Code Project Instructions)

Place this file at the repository root. Claude Code reads it automatically.

```markdown
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
cargo nextest run --workspace

# Run test vector verification
cargo run --bin ochra-testvec -- --verify

# Build UI
cd ui && pnpm install && pnpm dev        # dev server
cd ui && pnpm tauri build                 # production build

# Generate TypeScript types from Rust
./scripts/generate-types.sh

# Start local 3-node test network
./scripts/seed-network.sh
```

## Key Files
- `docs/Ochra_v5_5_Unified_Technical_Specification.md` — Protocol spec (source of truth)
- `docs/Ochra_v5_5_Human_Interface_Specification.md` — UI spec (source of truth)
- `crates/ochra-types/src/` — Shared data structures (Section 22 of spec)
- `crates/ochra-daemon/src/commands/` — IPC command handlers (Section 21 of spec)
- `ui/src/types/ochra.ts` — Auto-generated TypeScript types

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
```

---

## 10. Build Order Execution Guide

This section provides Claude Code with the recommended order and approach for implementing each phase.

### 10.1 Phase 1 Priority: Start Here

Phase 1 (`ochra-crypto`) has no dependencies and unlocks nearly every other phase. **Build this first, completely, including all test vectors.** The test vector binary (`ochra-testvec`) is the very first deliverable — it becomes the ground truth for all subsequent cryptographic work.

**Recommended sub-order within Phase 1:**

1. BLAKE3 with all 36 context strings (everything depends on this)
2. Ed25519 (identity, signatures)
3. X25519 (key agreement)
4. ChaCha20-Poly1305 (encryption)
5. Argon2id (password hashing)
6. ECIES (encrypt/decrypt helper combining X25519 + ChaCha20)
7. Poseidon (ZK-friendly hash)
8. Groth16/BLS12-381 infrastructure (prove/verify API)
9. Pedersen commitments
10. VOPRF (blind tokens)
11. FROST + ROAST (threshold signatures)
12. `ochra-testvec` binary

### 10.2 Parallel Tracks After Phase 1

Once Phase 1 completes, three independent tracks can proceed in parallel:

**Track A (Network):** Phase 3 (transport) → Phase 4 (DHT) → Phase 7 (onion) → Phase 8 (MLS)

**Track B (Economics):** Phase 2 (trusted setup) → Phase 9 (PoSrv) → Phase 13 (minting)

**Track C (Storage):** Phase 6 (ABR storage) → Phase 16 (PoW + zk-PoR)

All three tracks converge at Phase 22 (daemon RPC) which requires most prior phases.

### 10.3 The "Skeleton First" Approach

For each crate, Claude Code should:

1. **Scaffold:** Create `Cargo.toml` with dependencies, `src/lib.rs` with module declarations, public API stubs
2. **Types first:** Implement all data structures for that crate (matching Section 22 exactly)
3. **Happy path:** Implement the core logic for the success case
4. **Error paths:** Add error handling with Section 29 error codes
5. **Tests:** Unit tests + integration tests
6. **Documentation:** Doc comments on all public items

---

## 11. IPC Contract

The daemon exposes exactly the commands listed in Section 21 of the v5.5 spec. The UI communicates through Tauri's IPC bridge, which forwards to the daemon's JSON-RPC Unix socket.

### 11.1 IPC Architecture

```
┌─────────────┐     Tauri IPC      ┌──────────────────┐    Unix Socket    ┌──────────────┐
│  React UI   │ ←──────────────→   │  Tauri Backend   │ ←──────────────→  │ ochra-daemon │
│  (TS/React) │    (invoke/event)  │  (ipc_bridge.rs) │    (JSON-RPC)     │   (Rust)     │
└─────────────┘                    └──────────────────┘                   └──────────────┘
```

- **Tauri Backend** maintains a single persistent connection to the daemon Unix socket
- **Commands** are 1:1 mapped: `invoke("create_group", {...})` → JSON-RPC `create_group` → daemon handler
- **Events** are proxied: daemon pushes JSON-RPC notifications → Tauri backend → React event bus

### 11.2 Error Contract

All IPC errors return a structured error:

```typescript
interface OchraError {
  code: number;      // Section 29 error code
  message: string;   // Human-readable
  data?: unknown;    // Optional structured context
}
```

The UI maps error codes to user-facing messages. The daemon never returns user-facing strings — only codes.

---

## 12. Database Schema Summary

The daemon uses a single SQLite database with 18 tables (Section 27 of v5.5 spec). Key tables:

| **Table** | **Purpose** |
|---|---|
| `pik` | Local identity (encrypted PIK, Argon2id salt) |
| `contacts` | Known contacts (PIK hash, profile key, display name) |
| `groups` | Joined Spaces (group_id, name, template, role, settings) |
| `group_members` | Per-Space member list (PIK hash, role, joined_at) |
| `content` | Published/cached content manifests |
| `purchases` | Purchase history (content_hash, tier, receipt_secret) |
| `blind_receipts` | Blind receipt tokens for re-download |
| `chunks` | ABR chunk storage metadata (chunk_id, shard_index, path) |
| `service_receipts` | Earned service receipts pending minting |
| `nullifiers` | Local nullifier Bloom filter state |
| `wallet` | Seed token balance (blinded tokens, unblinded tokens) |
| `vys` | VYS accounting (accumulated rewards, last claim epoch) |
| `guardians` | Recovery Contact configuration |
| `invites` | Generated invite links (hash, uses, TTL, creator_flag) |
| `whisper_sessions` | Active Whisper session metadata (no message storage) |
| `handles` | Username registration (handle, signing key) |
| `downloads` | Active download progress tracking |
| `kv` | General key-value store (settings, cache, config) |

**Critical invariant:** Whisper messages are NEVER stored in the database. The `whisper_sessions` table tracks only session metadata (session_id, counterparty, state). Messages exist only in memory during an active session.

---

## 13. Security Invariants

These are non-negotiable protocol invariants that must never be violated in any implementation:

1. **PIK encrypted at rest.** The PIK private key is always encrypted with ChaCha20-Poly1305 derived from Argon2id(password, salt). It is never stored in plaintext.

2. **No plaintext on the wire.** Every network connection uses QUIC/TLS 1.3. Plaintext fallback is prohibited.

3. **Contact-Space isolation.** Contact profiles never reveal Space membership. Space member lists never highlight contacts. This is a hard architectural invariant — violation is a build-breaking defect.

4. **Revenue splits are immutable at creation.** Once a Space is created with a split, peers reject manifest updates attempting to change the split without a valid 30-day timelock proposal.

5. **Whisper messages have zero persistence.** Messages are never written to disk, never logged, and never cached. When a session ends, all message state is zeroized from memory.

6. **Nullifiers are deterministic.** A Seed token's nullifier is derived deterministically from the unblinded token. Double-spend is prevented by the NullifierSet Bloom filter.

7. **Receipt secrets never leave the device unencrypted.** Blind receipt tokens are stored locally, encrypted at rest. Re-download uses ZK proof — the receipt secret is never transmitted.

8. **No unregistered BLAKE3 context strings.** All 36 context strings are registered in Section 2.3. New strings require spec update.

9. **FROST threshold ≥ t-of-n.** No operation that requires FROST signing can complete without `t` honest participants. `t` and `n` are set at DKG time and cannot be changed without resharing.

10. **Epoch boundaries are atomic.** All epoch-boundary operations (minting, VYS distribution, nullifier pruning, relay key rotation) execute atomically. A node that crashes mid-epoch must replay from the last consistent state.

---

## 14. Performance Targets (Soft Goals for v1)

| **Metric** | **Target** | **Measurement** |
|---|---|---|
| PIK generation (Argon2id) | < 3s on M1 Mac | Benchmark in `ochra-crypto` |
| Groth16 proving (minting, ~45k constraints) | < 5s desktop | Benchmark in `ochra-mint` |
| Groth16 verification | < 2ms | Benchmark in `ochra-crypto` |
| Sphinx packet processing (per hop) | < 1ms | Benchmark in `ochra-onion` |
| IPC command latency (simple queries) | < 10ms | Benchmark in `ochra-daemon` |
| UI time-to-interactive | < 2s on cold start | Measured in Lighthouse |
| SQLite query (indexed lookups) | < 5ms | Benchmark in `ochra-db` |

---

## 15. Risk Register

| **Risk** | **Impact** | **Mitigation** |
|---|---|---|
| Groth16 circuit compilation takes too long | Blocks Phases 13-17 | Start Phase 2 immediately after Phase 1. Dev ceremony params allow iteration. |
| `openmls` crate compatibility issues | Blocks Phase 8 (MLS) | Fallback: `mls-rs` crate. Both implement RFC 9420. |
| FROST crate maturity | Blocks Phase 10 | ZcashFoundation's `frost-ed25519` is production-used. Fallback: manual implementation from RFC 9591. |
| Tauri v2 desktop stability | Blocks UI phases | Tauri v2 is stable. Fallback: Electron (heavier but proven). |
| Test network coordination | Blocks integration testing | `seed-network.sh` script automates 3-node local setup. CI runs integration tests. |
| ZK proof size/time on constrained circuits | Performance misses | Soft targets for v1. Optimize in v1.1. |