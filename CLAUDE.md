# CLAUDE.md — EmuNet v4.2 Build Guide

> **This file is the authoritative instruction set for Claude Code to construct EmuNet v4.2.**
> The full protocol specification lives in `EmuNet_v4_2_Protocol_Specification.docx` at the project root. Read that document in full before beginning any phase. This README distills the spec into actionable engineering directives.

---

## What Is EmuNet?

EmuNet is an **invite-only, fully decentralized, end-to-end encrypted peer-to-peer network** for censorship-resistant content distribution with a native economic layer. It operates as a darknet overlay on the public internet — no centralized databases, no DNS, no payment gateways.

Users earn **Seeds** (a fractional algorithmic stablecoin pegged to $1 USD) by contributing storage and bandwidth. Content is purchased atomically with Seeds. Privacy is enforced via zero-knowledge proofs, onion routing, and cryptographic subgroups.

**v4.2 is cross-platform: macOS | Windows | Linux | Android | iOS.**

---

## Project Structure

```
emunet/
├── CLAUDE.md                          # ← You are here
├── EmuNet_v4_2_Protocol_Specification.docx
│
├── daemon/                            # Rust core daemon (Cargo workspace)
│   ├── Cargo.toml
│   ├── crates/
│   │   ├── emunet-crypto/             # Phase 1: Ed25519, ChaCha20, BLAKE3, Groth16/BN254, Pedersen
│   │   ├── emunet-trusted-setup/      # Phase 2: Zcash Powers of Tau + Hermez embedding
│   │   ├── emunet-transport/          # Phase 3: QUIC/TLS 1.3 + Sphinx (Kuhn randomized padding)
│   │   ├── emunet-dht/                # Phase 4: Kademlia + BEP 44 mutable records
│   │   ├── emunet-invite/             # Phase 5: Base58 invite parsing, Ephemeral Open Invites
│   │   ├── emunet-storage/            # Phase 6: ABR chunking, Reed-Solomon, LFU-DA, mobile profiling
│   │   ├── emunet-onion/              # Phase 7: 3-hop onion routing + LAMP/alpha-mixing + cover traffic
│   │   ├── emunet-mls/                # Phase 8: Sender-Anonymous MLS RFC 9420 ratchet trees
│   │   ├── emunet-posrv/              # Phase 9: PoSrv telemetry + SybilGuard trust graphs
│   │   ├── emunet-frost/              # Phase 10: Validator Quorum election + FROST keygen
│   │   ├── emunet-oracle/             # Phase 11-12: MPC TLSNotary, TWAP, Hybrid Oracle, Circuit Breaker
│   │   ├── emunet-voprf/              # Phase 13: zk-VOPRF minting + fractional reserve collateral checks
│   │   ├── emunet-nullifier/          # Phase 14: Deterministic NullifierSet on DHT
│   │   ├── emunet-vys/                # Phase 15: VYS accounting + dynamic Collateral Ratio (CR) loops
│   │   ├── emunet-pow/                # Phase 16: Argon2id anti-spam + Proofs of Retrievability (PoR)
│   │   ├── emunet-spend/              # Phase 17: Risk-adjusted micro/macro spend logic + P2P transfers
│   │   ├── emunet-revenue/            # Phase 18: Revenue split DHT timelocks
│   │   ├── emunet-guardian/           # Phase 19-20: FROST DKG Guardian recovery + Dual-Path Cancel
│   │   ├── emunet-db/                 # Phase 21: SQLite wallet buffer + ABR state
│   │   ├── emunet-rpc/                # Phase 22: JSON-RPC daemon server
│   │   ├── emunet-types/              # Phase 23: Shared types, TS binding generation
│   │   └── emunet-events/             # Phase 24: Event streaming + OTA P2P upgrade engine
│   └── src/
│       └── main.rs                    # Daemon entrypoint
│
├── ui/                                # React + Tailwind frontend (responsive desktop + mobile)
│   ├── package.json
│   ├── tsconfig.json
│   ├── src/
│   │   ├── ipc/                       # Phase 23: Generated TypeScript bindings for JSON-RPC
│   │   ├── design-system/             # Phase 25: Responsive tokens, spring animations, acrylic blurs
│   │   ├── screens/
│   │   │   ├── SetupAssistant/        # Phase 26: Tutorial flow, Earning Power slider, Smart Night Mode
│   │   │   ├── EnclaveBuilder/        # Phase 27: Dual-mode group creation (WYSIWYG + JSON editor)
│   │   │   ├── Environments/          # Phase 28: Native layout rendering from JSON LayoutManifests
│   │   │   ├── Wallet/                # Phase 29: Progressive wallet, Contact Card, P2P transfers
│   │   │   └── Guardian/              # Phase 30: FROST DKG "Legacy Contacts" UI, veto recovery
│   │   ├── components/
│   │   │   ├── CheckoutModal.tsx       # Phase 31: Apple/Google Pay-style biometric confirm
│   │   │   ├── MacroTxSpinner.tsx      # Phase 31: Determinate ring → green checkmark + haptic
│   │   │   ├── ContactCard.tsx         # Frosted glass card with QR code + OS Share Sheet
│   │   │   ├── EarningPowerSlider.tsx  # Magnetic snap: Light (mobile default) / Balanced / Aggressive
│   │   │   ├── SmartNightToggle.tsx    # Sleep schedule visualization
│   │   │   ├── GroupSidebar.tsx        # Desktop: translucent sidebar
│   │   │   ├── BottomNavBar.tsx        # Mobile: standard bottom navigation tabs
│   │   │   ├── EnclaveWYSIWYG.tsx      # Easy Mode: drag-and-drop template builder
│   │   │   ├── ManifestJSONEditor.tsx  # Advanced Mode: raw LayoutManifest editor
│   │   │   ├── LayoutRenderer.tsx      # Sandboxed LayoutManifest → native component mapper
│   │   │   ├── StorefrontGrid.tsx      # Pre-compiled layout primitive
│   │   │   ├── ForumThread.tsx         # Pre-compiled layout primitive
│   │   │   ├── NewsFeed.tsx            # Pre-compiled layout primitive
│   │   │   ├── UpdatePrompt.tsx        # OTA: "EmuNet v5.0 is ready. Restart to apply."
│   │   │   ├── PoSrvScoreGauge.tsx     # Advanced mode: 0-100 score
│   │   │   ├── DualAssetYieldGraph.tsx # Advanced mode: Seeds vs VYS
│   │   │   └── StorageEvictionLog.tsx  # Advanced mode: ABR activity
│   │   └── App.tsx
│   └── tailwind.config.ts
│
├── packaging/                         # Phase 32: Cross-platform native packaging
│   ├── tauri/                         # macOS, Windows, Linux
│   ├── electron/                      # macOS, Windows, Linux (fallback)
│   └── mobile/                        # React Native / Kotlin Multiplatform / SwiftUI bindings
│       ├── android/
│       └── ios/
│
└── tests/
    ├── integration/
    └── e2e/
```

---

## Technology Stack

| Layer | Technology | Notes |
|---|---|---|
| **Daemon** | Rust (latest stable) | Cargo workspace with one crate per phase |
| **Cryptography** | `ed25519-dalek`, `chacha20poly1305`, `blake3`, `ark-groth16`, `ark-bn254`, `curve25519-dalek` (Ristretto255) | No algorithm substitutions. No downgrade paths. |
| **Networking** | `quinn` (QUIC), custom Sphinx impl | All sockets QUIC/TLS 1.3. Plaintext fallback is **prohibited**. |
| **DHT** | Kademlia + BEP 44 | Mutable signed records for NullifierSet, TWAP, manifests, UpgradeManifests |
| **ZK Proofs** | Groth16 over BN254 (~35k constraints) | Zcash Powers of Tau Phase 1 + Hermez Phase 2 trusted setup |
| **Threshold Sigs** | FROST (RFC 9591) | Top 100 PoSrv nodes form Validator Quorum; also used for DKG Guardian recovery |
| **MLS** | RFC 9420 (Sender-Anonymous) | O(log N) subgroup key management; all payloads route through Sphinx circuits |
| **Local DB** | SQLite | Wallet buffer, ABR chunk state, receipts |
| **IPC** | JSON-RPC over Unix socket / named pipe | Strict typed interface; fiat amounts are `u64` micro-cents |
| **Frontend** | React 18 + Tailwind CSS + Framer Motion / React Spring | Progressive Disclosure. Desktop sidebar + mobile bottom nav. |
| **Desktop Packaging** | Tauri (preferred) or Electron | macOS, Windows, Linux |
| **Mobile Packaging** | React Native / Kotlin Multiplatform / SwiftUI | Android, iOS — native compilation |
| **Mobile ABR** | Android WorkManager + iOS BGTaskScheduler | Heavy ABR restricted to unmetered Wi-Fi + charger |
| **Entropy** | OS CSPRNG only | User-space entropy fallback is **strictly prohibited** |

---

## Cryptographic Primitive Manifest

**Every primitive is mandatory. No substitutions. No optional cipher negotiation. This eliminates downgrade attacks by design.**

| Category | Algorithm | Purpose |
|---|---|---|
| Asymmetric Signatures | Ed25519 (RFC 8032) | Platform Identity Keys (PIK), receipt signing |
| Zero-Knowledge Proofs | Groth16 over BN254 | Dynamic denomination proofs for Seed minting |
| Commitment Schemes | Pedersen Commitments | Value blinding for $1 token minting |
| MPC TLS Oracles | DECO / TLSNotary | Privacy-preserving TWAP price discovery (split client key: Prover + Verifier) |
| Threshold Signatures | FROST (RFC 9591) | Quorum evaluation for zk-VOPRF minting, Oracle signing, **and DKG Guardian recovery** |
| Symmetric Encryption | ChaCha20-Poly1305 (RFC 8439) | Group payload keys, Guardian packets, mailboxes |
| Forward Secrecy | Double Ratchet Algorithm | Ephemeral key ratcheting for Group Keys |
| Group Key Agreement | MLS (RFC 9420) | Scalable ratchet trees for tiered subgroup access (**Sender-Anonymous**: routed via Sphinx) |
| Hash / PRF | BLAKE3 | Merkle trees, HMAC, Fiat-Shamir heuristics |
| Key Agreement | X25519 (RFC 7748) | Session keys, mailbox routing, onion circuits |
| Identity Recovery | **FROST-based DKG** | Replaces Shamir SSS. PIK never reassembled in single memory space. |
| Anonymous Token Minting | Ristretto255 VOPRF (RFC 9497) | Blind token issuance (mathematical unlinkability) |
| Double-Spend Defense | Deterministic Nullifiers | Spend-receipts tied to unblinded tokens |
| Anti-Spam | Argon2id Proof-of-Work | Friction before publishing (prevents ABR poisoning) |
| Storage Integrity | Proofs of Retrievability (PoR) | Cryptographic audits defeating "lazy seeder" / bandwidth faking attacks |
| Sybil Defense | PoSrv + SybilGuard Social Trust Graphs | Bandwidth/uptime + trust topology mapping to identify network cuts |
| Transport | QUIC (RFC 9000) + TLS 1.3 | All sockets. No plaintext fallback. |
| Datagram Obfuscation | Sphinx Packet Format | Fixed-size packets for 3-hop onion routing. **Kuhn et al. randomized padding**. |
| Latency Minimization | LAMP / Alpha-Mixing | Dynamic packet batching based on entropy levels. Up to 7.5x latency reduction. |

---

## Key Changes in v4.2 (vs v4.1)

### 1. Android & iOS as First-Class Platforms
The platform target list expands from macOS/Windows/Linux to include **Android and iOS**. This affects packaging (Phase 32), ABR behavior (Phase 6), the UI architecture (Phase 25), and the Earning Power defaults.

### 2. Mobile-Specific ABR Profiling
The daemon interfaces directly with **Android's WorkManager** and **iOS BGTaskScheduler**. Heavy ABR background replication is strictly restricted to when the device is on an **unmetered Wi-Fi connection** and/or **connected to a charger**. The Earning Power slider defaults to **Light on mobile**, Aggressive on desktop. Smart Night Mode references Android's Doze mode APIs and BatteryManager.

### 3. Decentralized Protocol Upgrades (OTA Hard Forks)
Entirely new system for network-wide upgrades without centralized download servers:
- **Signed Upgrade Manifests:** Core development multisig broadcasts a time-locked `UpgradeManifest` to the DHT containing the version string, BLAKE3 hash of new binaries, and an `ActivationEpoch` timestamp (e.g., 14 days in the future).
- **P2P Binary Distribution:** New client is seeded into the ABR system. Daemons passively download update chunks in the background via 3-hop Sphinx circuits.
- **Hard Fork Boundary:** At `ActivationEpoch`, upgraded nodes rotate their Sphinx transport magic bytes, partitioning un-updated legacy nodes from the new validator quorum and DHT.
- **Apple-like UI:** User sees: *"EmuNet Version 5.0 is ready. Restart to apply."*

### 4. The Enclave Builder (Dual-Mode Group Creation)
Phase 27 is rearchitected into a full website-builder experience:
- **Easy Mode (WYSIWYG):** Squarespace/Apple Keynote-like visual builder. Users select templates (*The Storefront*, *The Forum*, *The News Feed*), drag-and-drop structural elements, pick system-safe accent colors. Auto-defaults to an **80/20 revenue split** for creators.
- **Advanced Mode (JSON Manifest Editor):** Raw `LayoutManifest` JSON editor, granular DHT timelock settings, precise fractional Revenue Split sliders.
- New template: **The News Feed** (added alongside Storefront and Forum).

### 5. Responsive Navigation Architecture
Desktop uses a sidebar. **Mobile uses standard bottom navigation tabs** with properly sized touch-targets (44px minimum).

### 6. Phase 24 Expanded
Event streaming now also includes the **OTA Consensus Upgrade engine** (P2P Binary Distribution).

### 7. New IPC Commands
- `preview_layout_manifest(config: Bytes) -> Result<RenderableLayout>` — WYSIWYG preview for Enclave Builder
- `check_protocol_updates() -> Result<UpdateStatus>` — Check for OTA upgrades
- `apply_protocol_update() -> Result<()>` — Apply downloaded OTA upgrade
- IPC Section 5 renamed from "Diagnostics, Settings & Moderation" to **"Diagnostics, Settings & Updates"**

---

## 32-Phase Build Order

Each phase maps to a crate or UI module. Phases are sequential with noted parallelization opportunities.

### Tier 1 — Cryptography & Baseline Network (Phases 1–6)

**Phase 1: `emunet-crypto`** — Core Cryptographic Primitives
- Implement Ed25519 key generation and signing (PIK creation).
- Implement ChaCha20-Poly1305 authenticated encryption.
- Implement BLAKE3 hashing (Merkle trees, HMAC, content addressing).
- Implement Groth16 prover/verifier over BN254 curve.
- Implement Pedersen Commitments for value blinding.
- Implement X25519 key agreement.
- All entropy sourced from OS CSPRNG. Panic on fallback attempts.

**Phase 2: `emunet-trusted-setup`** — Universal Trusted Setup Embedding
- Embed the **Zcash Perpetual Powers of Tau** (Phase 1) parameters.
- Embed the **Hermez** (Phase 2) universal setup transcript.
- Build verification that embedded parameters match known checksums.
- Strictly monitor host OS CSPRNG during the blinding phase — no fallback vulnerabilities.

**Phase 3: `emunet-transport`** — Transport Layer
- QUIC (via `quinn`) with TLS 1.3 for all sockets.
- UDP multiplexing.
- Sphinx packet formatting: fixed-size, indistinguishable datagrams.
- **Kuhn et al. randomized padding:** Sphinx headers must use randomized padding arrays, NOT legacy zero-byte padding. Prevents path-length inference by the final mix-node.
- **Hard rule:** Protocol explicitly denies plaintext fallback.

**Phase 4: `emunet-dht`** — Kademlia DHT
- BEP 44 mutable signed records.
- Routing table management.
- Used for: NullifierSet, TWAP broadcasts, ChunkAvailRecords, GroupManifests, LayoutManifests, **UpgradeManifests**.
- Ephemeral Open Invite TTL enforcement: DHT natively drops invite signatures past the 30-day epoch boundary.

**Phase 5: `emunet-invite`** — Genesis Bootstrapping
- Base58 decoding of `emunet://invite...` deep links.
- Ed25519 signature verification of invite payloads.
- Time-lock enforcement (TTL expiry).
- Use-count enforcement.
- Cascading revocation support.
- **Ephemeral Open Invites:** `uses: None` (unlimited) but `ttl_days` hard-capped at 30. DHT drops signature at epoch boundary.

**Phase 6: `emunet-storage`** — Local Storage Engine
- ABR (Automatic Background Replication) chunk splitting.
- Reed-Solomon erasure coding.
- **LFU-DA eviction algorithm:**
  - Query DHT `ChunkAvailRecord` for `hll_replica_est` (HyperLogLog global chunk density).
  - `Weight = (fetch_count / (now - last_accessed)) * (1 / hll_replica_est)`
  - Evict lowest-weight chunks until storage < 90% quota.
- **Earning Power allocation:** Storage expands lazily based on selected power level (Light / Balanced / Aggressive).
- **Mobile-Specific Profiling (Android/iOS):**
  - Interface with Android's WorkManager and iOS BGTaskScheduler.
  - Strictly restrict heavy ABR background replication to **unmetered Wi-Fi** and/or **charger connected**.
  - Monitor OS states (Android Doze mode, BatteryManager, macOS sleep state APIs).

### Tier 2 — Access Control & Consensus (Phases 7–15)

**Phase 7: `emunet-onion`** — Datagram Onion Routing
- 3-hop UDP/QUIC circuit construction using Sphinx packets.
- **Hard rule:** Direct peer-to-peer IP connections for chunk retrieval are strictly prohibited.
- **LAMP/Alpha-Mixing integration:** Relay nodes dynamically calculate real-time entropy of incoming traffic. Batch and flush packets when sufficient anonymity sets are reached. Target: up to 7.5x latency reduction while preserving Sender-Receiver Unlinkability (SR-L).
- **Last Hop Attack defense:** Cover traffic to random destination peers across independent 3-hop circuits (NOT self-addressed cascades). Continuous global cover traffic indistinguishable from payload data.
- Statistical chaff: pad headers uniformly, issue lightweight decoy requests (KB-scale).

**Phase 8: `emunet-mls`** — Sender-Anonymous MLS Ratchet Trees for Subgroups
- Implement RFC 9420 Messaging Layer Security.
- O(log N) add/evict operations — single broadcast payload.
- Zero-knowledge visibility: members without subgroup access cannot decrypt content manifest.
- **Sender-Anonymous routing:** All MLS group management payloads route through Sphinx onion circuit to prevent DS metadata leakage.
- Continuous Double Ratchet forward secrecy on Group Keys.

**Phase 9: `emunet-posrv`** — Proof-of-Service Telemetry & SybilGuard
- Scoring logic based on GBs served and uptime.
- **SybilGuard Social Trust Graph integration:** Map trust topologies to identify network cuts. Trust graph cuts feed into normalized PoSrv Score.
- Normalized PoSrv Score drives VYS calculation and Quorum election eligibility.

**Phase 10: `emunet-frost`** — Validator Quorum & FROST
- Top 100 PoSrv nodes form the Validator Quorum.
- FROST (RFC 9591) distributed key generation.
- Threshold signing for zk-VOPRF minting, Oracle data, **and DKG Guardian recovery**.

**Phase 11: `emunet-oracle` (part 1)** — MPC TLSNotary Integration
- DECO/TLSNotary multi-party computation.
- Redundant polling: concurrent MPC TLS sessions against 5 exchange APIs (Binance, Coinbase, etc.) over Tor.
- **Split client key architecture:** Prover key + Verifier key; Prover cannot forge TWAP without Verifier consent.
- ZKP attestation of HTTP JSON payloads.

**Phase 12: `emunet-oracle` (part 2)** — TWAP, Hybrid Oracle & Circuit Breaker
- TWAP calculation. FROST-signed broadcast to GroupManifest.
- **Hybrid Oracle Model:** If TWAP timestamp > 12 hours stale → Circuit Breaker triggers.
  - Economy does NOT freeze. Falls back to hybrid reserve pricing.
  - Spending continues. Collateral-ratio-adjusted minting continues safely.
  - 0.1% ad valorem fee keeps distributing to VYS holders.

**Phase 13: `emunet-voprf`** — zk-VOPRF Minting Pipeline
- Ristretto255 VOPRF (RFC 9497) blind token issuance.
- Client-side flow:
  1. Calculate denomination using TWAP Oracle.
  2. Create Pedersen commitment.
  3. Generate Groth16 proof (~35k constraints, <2.5s on mobile ARM).
  4. FROST Quorum verifies proof (<1ms), signs blinded payload.
  5. Client unblinds threshold shares → buffers $1 Seed.
- **Fractional Reserve collateral checks:** Minting validates against current dynamic Collateral Ratio (CR).

**Phase 14: `emunet-nullifier`** — Deterministic NullifierSet
- Cryptographic spend-receipts tied to unblinded tokens. Stored and queried via DHT.

**Phase 15: `emunet-vys`** — Validator Yield Shares & Collateral Ratio
- VYS = 1:1 mapping of normalized PoSrv Score (including SybilGuard trust graph cuts).
- Non-transferable, tied to PIK.
- 0.1% ad valorem fee distributes pro-rata to VYS holders each epoch, adjusting dynamically based on CR.
- **Decay:** 5% daily if offline. Hard-slash to 0 after 7 consecutive offline days.
- **Dynamic CR adjustment loops:** Shift Collateral Ratio based on confidence metrics, redemption pressure, oracle health.

### Tier 3 — Application Logic & Daemon Binding (Phases 16–24)

**Phase 16: `emunet-pow`** — Argon2id Anti-Spam & Proofs of Retrievability
- Argon2id proof-of-work before publishing. Prevents ABR storage poisoning.
- **PoR:** Validators issue random cryptographic challenges to passive nodes proving physical possession of block data.

**Phase 17: `emunet-spend`** — Risk-Adjusted Spend Logic
- **Micro (< $5):** O(1) local latency. Publisher verifies Groth16 proof locally (<1ms), releases key, broadcasts Nullifier async.
- **Macro (> $5):** Synchronous DHT NullifierSet check. 2–3 second penalty. Eliminates double-spend.
- **P2P Transfers:** `send_funds` RPC for direct peer-to-peer value transfer.

**Phase 18: `emunet-revenue`** — Revenue Split Timelocks
- 30-day cryptographic timelock for any split change:
  ```rust
  struct RevenueSplitChangeProposal {
      group_id: [u8; 32],
      sequence: u32,
      proposed_owner_pct: u8,
      proposed_pub_pct: u8,
      proposed_abr_pct: u8,
      effective_at: u64,    // MUST be >= now + 30 days
      broadcast_at: u64,
      owner_sig: [u8; 64],  // Ed25519(owner_PIK, all fields above)
  }
  ```

**Phase 19: `emunet-guardian` (part 1)** — FROST-based DKG Recovery
- **Replaces Shamir's Secret Sharing entirely.** PIK never exists on a single disk or in memory.
- Recovery signatures are distributed, blinded computations.

**Phase 20: `emunet-guardian` (part 2)** — Dual-Path Cancellation
- 48-hour recovery timelock with veto capability. Conflict resolution for competing DKG recovery attempts.

**Phase 21: `emunet-db`** — SQLite Local Database
- Wallet buffer, ABR chunk state, purchase history, earning power settings.

**Phase 22: `emunet-rpc`** — JSON-RPC Daemon Server
- Unix socket (macOS/Linux) / named pipe (Windows). Strict typed interface. All fiat amounts `u64` micro-cents.

**Phase 23: `emunet-types`** — TypeScript Binding Generation
- Auto-generate TypeScript types from Rust structs. Strict 1:1 mapping. Output to `ui/src/ipc/`.

**Phase 24: `emunet-events`** — Event Streaming & OTA Upgrade Engine
- Real-time event stream for UI progress bars (downloads, sync, minting).
- **Decentralized Protocol Upgrades (OTA Hard Forks):**
  - **Signed UpgradeManifest:** Core development multisig broadcasts to DHT. Contains version string, BLAKE3 hash of new binaries, and `ActivationEpoch` timestamp (e.g., 14 days in the future).
  - **P2P Binary Distribution:** New client seeded into ABR system. Daemons passively download update chunks via anonymous 3-hop Sphinx circuits.
  - **Hard Fork Boundary:** At `ActivationEpoch`, upgraded nodes rotate Sphinx transport magic bytes, partitioning legacy nodes from the new validator quorum and DHT.
  - **UI:** Non-intrusive prompt: *"EmuNet Version 5.0 is ready. Restart to apply."*

### Tier 4 — UI, Interface & Polish (Phases 25–32)

**Design Philosophy: Progressive Disclosure.** Default mode is consumer-friendly (Apple/Google-like). Advanced Mode toggle exposes full telemetry. **Responsive:** Desktop sidebar → Mobile bottom navigation tabs (44px min touch-targets).

**Phase 25: Foundation & Responsive Design System**
- Global Tailwind configuration prioritizing native-app feel.
- **Responsive architecture:** Sidebar for desktop. Standard Bottom Navigation Bar for Android/iOS with 44px minimum touch-targets.
- Physics-based spring animations (Framer Motion / React Spring) for all state changes.
- 60fps hardware-accelerated transitions. All crypto on background threads — zero UI blocking.
- Design tokens: SF Pro / Inter typography, 16px border radii, monochromatic base palette, semantic blues/greens, acrylic blurs.

**Phase 26: The Setup Assistant & "Magic" Onboarding**
- Paginated "Welcome Experience":
  - **Step 1: Your Digital Wallet.** "Think of your Wallet like digital cash..."
  - **Step 2: Growing Seeds.** "You earn 'Seeds' simply by leaving EmuNet running..."
  - **Step 3: Choose Your Earning Power.** Fluid slider:
    - Light: Barely noticeable, minimal earning. **(Default for Mobile.)**
    - Balanced: Smart background usage.
    - Aggressive: Maximizes earning potential. **(Desktop heavy.)**
  - **Smart Night Mode:** "Only earn Seeds when my device is plugged in and asleep." Daemon uses Android Doze mode APIs, BatteryManager, macOS sleep state APIs to silently wake, run ABR PoR checks, and suspend.
- Deep-link parsing: `emunet://` instant authentication.
- Android WorkManager + iOS BGTaskScheduler integration for binding to unmetered Wi-Fi + charging states.

**Phase 27: The Enclave Builder (Group Creation & UI Templating)**
- **Dual-mode creation flow:**
  - **Easy Mode (WYSIWYG):** Visual website-builder (Squarespace/Apple Keynote-like). Users select base templates (*The Storefront*, *The Forum*, *The News Feed*), drag-and-drop structural elements, pick system-safe accent colors. Auto-defaults to **80/20 revenue split** for creators.
  - **Advanced Mode (JSON Manifest Editor):** Raw `LayoutManifest` JSON editor, granular DHT timelock settings, precise fractional Revenue Split sliders.

**Phase 28: Adaptive Environments (Native Layout Rendering)**
- Sandboxed UI interpreter: map Group Owner's `LayoutManifest` to pre-compiled native UI components.
- Built-in primitives: `<StorefrontGrid />`, `<ForumThread />`, `<NewsFeed />`.
- **Hard rule:** No external CSS, WebFonts, or web-views. All layout primitives ship compiled. Prevents CSS-based side-channel leaks, IP tracking, browser fingerprinting.

**Phase 29: Progressive Wallet & Effortless Connections**
- **Default mode:** Single unified fiat balance. "Add Funds", "Withdraw", "Send" buttons (Apple/Google Pay aesthetic).
- **Advanced mode:** Separate Seeds vs VYS, TWAP Oracle pegs, collateral ratios, Groth16 proof logs.
- **Contact Card:** Frosted glass card with dynamic QR code. In-person: scan via Android/iOS camera → "Add [Name]?" modal. Digital: native OS Share Sheet (Android Intent, iMessage, Mail) sends `emunet://contact?pik=[Hash]`.
- **P2P Transfers:** Tap "Send" → select contact → type fiat amount → "Double-Click to Confirm" or device biometrics (Android Fingerprint, FaceID, Windows Hello).

**Phase 30: Trusted Contacts (FROST Guardian Setup)**
- FROST DKG as intuitive "Legacy Contacts" interface. Select friends, assign as Guardians.
- High-contrast critical-alert UI for 48-hour "Veto Recovery" window.

**Phase 31: Seamless Checkout (Micro/Macro Transaction Modals)**
- Smooth bottom-sheet modal. Biometric prompt (Android Fingerprint/FaceID/Windows Hello).
- **Macro (> $5):** Determinate progress ring → green checkmark + localized haptic "ping".
- **Micro (< $5):** Instant confirmation.

**Phase 32: Delivery, Optimization & Native Packaging**
- **Desktop:** Tauri/Electron native packaging + binary signing for macOS/Windows/Linux.
- **Mobile:** React Native (or Kotlin Multiplatform/SwiftUI bindings) for native Android/iOS compilation.
- **Battery profiling:** Ensure Rust daemon respects mobile doze states and unmetered connection requirements.
- Final cross-platform security audit.

---

## Progressive UI Mode Reference

Both modes render the same data — the difference is exposure level. Mobile uses bottom navigation tabs instead of the desktop sidebar.

| Application View | Default Mode (Simple / Apple-esque) | Advanced Mode (Technical / Power User) |
|---|---|---|
| **The Group Hub** | Clean navigation layout. Large colorful group icons. Unread badges. | Group Root Hashes, MLS epoch IDs, active peer connection counts. |
| **Enclave Builder** | WYSIWYG visual builder. Select themes (Storefront, Forum, News Feed), pick colors, launch. Auto 80/20 revenue split. | Raw JSON Manifest Editor, Revenue Split sliders, DHT timelock parameters, manual TTL settings. |
| **Wallet & Economy** | Single unified balance ("$45.00"). "Add Funds", "Withdraw", "Send". No "Seeds" jargon. | Separate Seeds vs VYS. TWAP Oracle pegs, collateral ratios, Groth16 proof logs. |
| **Earning Settings** | "Earning Power" scale + "Smart Night Mode" toggle. | LFU-DA eviction logs, GB quotas, PoSrv scores, Sphinx latency, DHT routing health. |

---

## Complete IPC Command Reference

The Rust daemon exposes this JSON-RPC API. All fiat amounts are `u64` micro-cents (1 USD = 100_000_000 micro-cents).

### 1. Identity, Contacts & Guardians

```
init_pik(password: String) -> Result<PikMeta>
get_my_pik() -> Result<Hash>
export_revocation_certificate() -> Result<String>
nominate_guardian(contact_pik: Hash, share: String) -> Result<()>
initiate_recovery(guardian_shares: Vec<String>) -> Result<TimelockStatus>
veto_recovery(auth_payload: Bytes) -> Result<()>
add_contact(pik: Hash, alias: String) -> Result<()>
get_contacts() -> Result<Vec<Contact>>
```

### 2. Network, Enclaves & Subgroups

```
join_group(invite_uri: String) -> Result<GroupId>
generate_invite(group_id: GroupId, uses: Option<u32>, ttl_days: u8) -> Result<String>
revoke_invite(invite_hash: Hash) -> Result<()>
get_group_members(group_id: GroupId) -> Result<Vec<PeerProfile>>
create_subgroup(group_id: GroupId, name: String) -> Result<SubgroupId>
get_subgroup_members(subgroup_id: SubgroupId) -> Result<Vec<PeerProfile>>
mls_grant_subgroup_access(subgroup_id: SubgroupId, target_piks: Vec<Hash>) -> Result<()>
mls_revoke_subgroup_access(subgroup_id: SubgroupId, target_piks: Vec<Hash>) -> Result<()>
preview_layout_manifest(config: Bytes) -> Result<RenderableLayout>
update_group_layout_manifest(group_id: GroupId, layout_type: String, config: Bytes) -> Result<()>
get_onion_circuit_health() -> Result<CircuitMetrics>
```

### 3. Economy, Stablecoin & Oracles

```
get_oracle_twap() -> Result<{ usd_peg: u64, is_circuit_breaker_active: bool, stale_hours: f32 }>
get_wallet_balance() -> Result<{ stable_seeds: u64, yield_shares: u64, yield_decay_rate: f32 }>
get_purchase_history() -> Result<Vec<PurchaseRecord>>
send_funds(recipient_pik: Hash, amount_usd: u64) -> Result<TxHash>
force_flush_receipts(groth16_proof: Bytes) -> Result<FlushStats>
init_tls_notary_share(target_api: String) -> Result<MpcSession>
propose_revenue_split(group_id: GroupId, new_split: RevenueSplit) -> Result<TimelockStatus>
get_earnings_breakdown(group_id: GroupId) -> Result<EarningsReport>
issue_refund(buyer_pik: Hash, content_hash: Hash) -> Result<()>
```

### 4. File IO, ABR & Publishing

```
get_store_catalog(group_id: GroupId) -> Result<Vec<ContentManifest>>
publish_file(path: String, target_id: TargetId, price_usd: u64) -> Result<ContentHash>
set_content_price(content_hash: Hash, new_price_usd: u64) -> Result<()>
download_file(content_hash: Hash, destination: String) -> Result<Stream<Progress>>
pause_download(content_hash: Hash) -> Result<()>
get_abr_telemetry() -> Result<{ used_bytes, evictions_24h, posrv_score }>
update_earning_settings(power_level: String, smart_night_mode: bool) -> Result<()>
pin_content(content_hash: Hash) -> Result<()>
```

### 5. Diagnostics, Settings & Updates

```
check_protocol_updates() -> Result<UpdateStatus>
apply_protocol_update() -> Result<()>
get_daemon_logs(level: String) -> Result<Vec<LogEntry>>
export_diagnostics() -> Result<String>
set_theme_settings(mode: ThemeMode, accent_color: String) -> Result<()>
report_content(content_hash: Hash, reason: String) -> Result<()>
owner_tombstone_content(content_hash: Hash) -> Result<()>
```

---

## IPC Changes from v4.1

| Change | Detail |
|---|---|
| **Added** `preview_layout_manifest()` | Returns `RenderableLayout` for WYSIWYG Enclave Builder preview |
| **Added** `check_protocol_updates()` | Checks DHT for signed `UpgradeManifest` — returns `UpdateStatus` |
| **Added** `apply_protocol_update()` | Applies downloaded OTA upgrade, triggers Sphinx magic byte rotation at ActivationEpoch |
| **Renamed** IPC Section 5 | From "Diagnostics, Settings & Moderation" → **"Diagnostics, Settings & Updates"** |

---

## The Role Triad

| Role | Function | Incentive |
|---|---|---|
| **Group Owner** | Cryptographic genesis entity. Controls invite tree root, dictates upload policy, **sets the UI Layout Manifest** via Enclave Builder. | Earns passive % of all economic velocity in enclave. |
| **Publisher** | Injects files into Group Merkle root. Sets fiat price. | Primary share of downstream sales. |
| **Passive Node** | Dedicates local storage lazily to opaque network data. | Earns newly minted $1 Seeds via PoS-S when serving requested chunks. |

---

## Hard Rules & Invariants

These are non-negotiable protocol constraints. Violating any of them is a build-breaking defect.

1. **No plaintext fallback.** Every socket is QUIC + TLS 1.3.
2. **No direct IP connections for chunk retrieval.** All chunk traffic routes through 3-hop Sphinx circuits.
3. **No user-space entropy.** All randomness from OS CSPRNG. Panic on failure.
4. **No algorithm negotiation.** The cryptographic suite is fixed. No cipher selection handshakes.
5. **Revenue split changes require 30-day timelocks.** `effective_at >= now + 30 days`.
6. **VYS hard-slash after 7 days offline.** Balance → 0.
7. **Circuit Breaker triggers when TWAP > 12 hours stale.** Falls back to hybrid reserve pricing — economy does NOT freeze.
8. **Micro/Macro threshold is $5.00.** Below = local verification. Above = synchronous DHT check.
9. **Earning Power controls ABR allocation.** Light (mobile default) / Balanced / Aggressive.
10. **"Seeds" is hidden from end users in Default Mode.** All prices display in localized fiat. "Seeds" only in Advanced Mode.
11. **No external CSS, WebFonts, or web-views in layout rendering.** All layout primitives ship compiled.
12. **Sphinx padding must be randomized (Kuhn et al.).** Zero-byte padding is prohibited.
13. **PIK never exists in a single memory space during recovery.** FROST DKG is mandatory. Shamir SSS is prohibited.
14. **Ephemeral Open Invites hard-capped at 30-day TTL.** DHT drops the signature at epoch boundary.
15. **Mobile ABR restricted to unmetered Wi-Fi + charger.** Daemon must interface with Android WorkManager / iOS BGTaskScheduler.
16. **OTA upgrades use P2P distribution only.** No centralized download servers (GitHub, AWS, etc.).
17. **Enclave Builder Easy Mode defaults to 80/20 revenue split.** Community-standard default.

---

## Key Rust Crate Dependencies (Suggested)

```toml
# Cryptography
ed25519-dalek = "2"
x25519-dalek = "2"
curve25519-dalek = { version = "4", features = ["ristretto"] }
chacha20poly1305 = "0.10"
blake3 = "1"
ark-groth16 = "0.4"
ark-bn254 = "0.4"
ark-std = "0.4"
argon2 = "0.5"

# Networking
quinn = "0.11"
rustls = "0.23"

# Storage
rusqlite = { version = "0.31", features = ["bundled"] }
reed-solomon-erasure = "6"

# Serialization / IPC
serde = { version = "1", features = ["derive"] }
serde_json = "1"
jsonrpsee = "0.24"

# Async
tokio = { version = "1", features = ["full"] }

# MLS (evaluate available crates)
openmls = "0.6"

# FROST (used for Quorum AND Guardian DKG)
frost-ed25519 = "2"
```

---

## Testing Strategy

- **Unit tests:** Every crate has `#[cfg(test)]` modules covering all public APIs.
- **Integration tests:** `tests/integration/` covers cross-crate interactions (minting pipeline, fractional reserve, FROST DKG recovery, OTA upgrade manifest verification).
- **E2E tests:** `tests/e2e/` spins up multiple daemon instances — invite flows (including Ephemeral Open Invites), purchases, eviction, FROST DKG recovery, P2P transfers, OTA upgrade lifecycle.
- **Fuzz targets:** Cryptographic primitives, Sphinx packet parsing (randomized padding), LayoutManifest JSON parsing, UpgradeManifest validation.
- **Benchmarks:** Groth16 proving (<2.5s ARM), verification (<1ms), Sphinx routing, LAMP flush timing.
- **PoR validation tests:** Simulate lazy seeders, verify challenge-response identifies faked storage.
- **SybilGuard tests:** Simulate trust graph topologies with known Sybil regions, verify network cut detection.
- **Mobile battery tests:** Verify daemon respects doze states, unmetered connection requirements, charger detection.

---

## Notes for Claude Code

1. **Always consult the spec.** When in doubt, re-read `EmuNet_v4_2_Protocol_Specification.docx`. This CLAUDE.md is a distillation, not a replacement.
2. **One crate per phase.** Keep the workspace modular. Inter-crate dependencies minimal and explicit.
3. **Types crate is shared.** `emunet-types` is the single source of truth for all data structures shared between daemon and UI.
4. **Error handling:** `thiserror` for library crates, `anyhow` for the daemon binary. All IPC errors must be serializable JSON-RPC errors.
5. **No `unwrap()` in production code.** Panics reserved only for invariant violations (e.g., OS CSPRNG failure).
6. **Logging:** `tracing` with structured fields. `get_daemon_logs` RPC must filter by level.
7. **The UI is strictly a presentation layer.** No cryptographic logic in TypeScript. All crypto in the Rust daemon.
8. **Progressive Disclosure is architectural.** Components must support Default and Advanced modes from the component level — not an afterthought CSS toggle. Use a `mode` prop or context to gate advanced telemetry.
9. **FROST DKG is not optional.** Do not implement Shamir SSS as a fallback. The spec explicitly calls SSS a vulnerability.
10. **Layout rendering is sandboxed.** `LayoutRenderer` maps LayoutManifest JSON keys to a fixed allowlist of pre-compiled components. Arbitrary component injection is a security violation.
11. **Mobile is a first-class target.** The daemon must compile for ARM and interface with platform-specific background task APIs. ABR behavior must respect battery and connectivity constraints.
12. **OTA upgrades must be fully decentralized.** No fallback to centralized download servers. The upgrade binary distributes exclusively through the ABR/Sphinx pipeline.
