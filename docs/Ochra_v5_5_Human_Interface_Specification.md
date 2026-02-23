# Ochra v5.5

## Human Interface Specification

*Consumer-facing design for a private, peer-to-peer content network*

| **Property** | **Value** |
|---|---|
| Version | 5.5 |
| Status | Build-ready |
| Companion Document | Ochra v5.5 Unified Technical Specification |
| Design Philosophy | Progressive Disclosure. Apple-simple in Default Mode; full technical depth in Advanced Mode. |
| Platforms | macOS · Windows · Linux · Android · iOS |

### Document Scope

This document specifies everything the user sees, touches, and reads: navigation, visual language, copy, flows, progressive disclosure, and accessibility. All cryptographic protocols, network mechanics, data structures, IPC commands, Whisper messaging protocol, username system, and wire formats are defined in the companion **Ochra v5.5 Unified Technical Specification**. IPC command names, Rust struct field names, and DHT record formats retain protocol-internal names for backward compatibility. The UI layer performs the mapping.

### The Grandmother Test

Every term, label, and screen must be understandable by a first-time user within 5 seconds. If it fails this test, it gets renamed to a plain-English equivalent. Technical depth is never removed — it is layered behind Advanced Mode.

---

## 1. User-Facing Terminology

v5.5 maintains a systematic split between user-facing language (Default Mode) and protocol-internal names (code, IPC, Advanced Mode, Unified Technical Specification). The following table is the authoritative mapping:

| **User-Facing (Default Mode)** | **Protocol-Internal** | **Notes** |
|---|---|---|
| Space | Group / Enclave | All UI surfaces use "Space" |
| Host | Group Owner | "Host" in UI, "owner" in code/IPC |
| Creator | Publisher | "Creator" in UI, "publisher" in code/IPC |
| Moderator | Moderator | Same in both |
| Seeds (with seed icon) | Seeds / stable_seeds | Balance shown as "45 Seeds", never "$45.00" |
| Recovery Contacts | Guardians | FROST DKG Guardians presented as "Recovery Contacts" |
| Earning Level | Earning Power | Slider labels: 🌱 Low / 🌿 Medium / 🌳 High / ⚙️ Custom |
| Earn While I Sleep | Smart Night Mode | Descriptive toggle label |
| Creator Share | Revenue Split | Simplified display in Default Mode |
| Home | Group Hub | Bottom nav / sidebar label |
| Seeds (tab) | Wallet | Bottom nav tab for balance and transfers |
| Earn | Earning Settings | Bottom nav tab for ABR configuration |
| You | Settings | Personal settings tab |
| Shop / Community / Feed / Gallery / Library | Storefront / Forum / News Feed / Gallery / Library | Space Builder template names |
| Space Builder | Enclave Builder | Creation flow name |
| Dashboard | (none) | Host-only Space management view |
| People | (none) | Host-only member/Creator management |
| Space Settings | (none) | Host-only configuration screen |
| Invite Links | (none) | Host invite management view |
| Oracle Rate | TWAP Oracle peg | Advanced Mode only |
| Network Health | Collateral Ratio | Advanced Mode only |
| Network Fee | Ad valorem fee (0.1%) | Advanced Mode only |
| Whisper | ephemeral_msg | Ephemeral private messaging. All UI surfaces use "Whisper." |
| Username / @username | Handle | Optional globally unique identifier for messaging and Seed transfers. Displayed with @ prefix. |
| Channels | Subgroups | Tiered content access within a Space (Advanced/Host feature) |
| Free | price_seeds = 0 | Content with zero-Seed pricing tier |
| Update Available | OTAUpdateAvailable | Protocol upgrade notification |

---

## 2. Setup Assistant (Tutorial Flow)

When a user opens the application for the first time, they are greeted by a paginated Welcome Experience with five steps.

### Step 1 — Welcome to Ochra

- A single text field: "What should we call you?" (display name).
- Password creation with strength indicator.
- Optional biometric enrollment (FaceID / Fingerprint / Windows Hello).
- The daemon generates the PIK silently in the background.
- Copy at the bottom: "Everything stays on your device. Private by design."
- "Next" button.

### Step 2 — Meet Seeds

- A gentle animation: a seed sprouts, grows into a plant, a number ticks upward beside it.
- Copy: "Seeds are rewards you earn just by being part of Ochra. Use them to discover music, videos, art — anything creators share. The more you help the network, the more you grow."
- No action required — this is a passive education screen.
- "Next" button.

### Step 3 — Choose How You Earn

- A fluid slider with magnetic snap points and a plant-growth visual metaphor:
  - 🌱 **Low** — "Barely noticeable." *(Mobile default.)*
  - 🌿 **Medium** — "Smart background usage." *(Desktop default.)*
  - 🌳 **High** — "Maximizes earning potential."
  - ⚙️ **Custom** — "You decide." Opens a precise GB allocation field with floor (500 MB) and ceiling (80% free space) enforcement.
- Below the slider, the **Earn While I Sleep** toggle.
- Slider copy: "Share a little of your spare storage and bandwidth to earn Seeds in the background. It's automatic — you won't even notice."
- Toggle copy: "Ochra quietly earns Seeds overnight while your device charges."

**Bandwidth Disclosure (Desktop Only):** Below the toggle, a subtle informational line: "Ochra uses some bandwidth to keep you private — about 30 GB/month in the background, more while active." This surfaces the cover traffic cost (Section 3.5 of the Unified Technical Specification: ~29 GB/month Idle, ~127 GB/month Active). On mobile, this disclosure is omitted since mobile cover traffic is heavily constrained and negligible.

- "Next" button.

### Step 4 — Protect Your Account

- Copy: "Pick a few people you trust. If you ever lose your password, they can help you get back in."
- A contact selector showing the user's Ochra contacts (if any exist from a pre-existing invite chain).
- Threshold display: "2 of 3 needed" (or similar based on the number of contacts selected).
- A large friendly "Set Up Now" button alongside a subtle "I'll do this later" link.
- This step is skippable. If skipped, the app will periodically remind users who have not configured Recovery Contacts via a non-intrusive card on the Home screen.

### Step 5 — You're Ready

- A summary card showing the user's display name, chosen Earning Level, and Recovery Contact status.
- Copy: "You're all set. Join a Space to discover content, or create your own."
- A large "Get Started" button that navigates to the Home screen.

**Technical:** The wizard calls `init_pik(password)` in Step 1, `enroll_biometric()` if opted in, `update_earning_settings(power_level, smart_night_mode)` in Step 3, and `nominate_guardian()` calls in Step 4. Deep-link parsing for `ochra://invite`, `ochra://connect`, and `ochra://whisper` is active from first launch.

---

## 3. Home Screen

The Home screen is the primary landing view of the app. It shows all Spaces the user belongs to and provides the entry point for creating new ones.

### 3.1 Layout

**Mobile:** A vertical scrollable list of Space cards. Each card is a rounded rectangle showing the Space icon (or emoji), Space name, and a subtle unread indicator (a small dot) when new content or activity has occurred since the user last opened that Space.

**Desktop:** A sidebar on the left displays a compact vertical list of Space icons (similar to Discord/Slack). Clicking a Space icon loads it in the main content area. The sidebar shows a small unread dot on Spaces with new activity.

### 3.2 Space Cards (Mobile)

Each Space card shows:

- **Space icon** — Photo or emoji, left-aligned.
- **Space name** — Bold, primary text.
- **Role badge** — A subtle pill next to the Space name: "Host" (gold) if the user owns the Space. "Creator" (blue) if the user is a Creator. No badge for regular members (clean default).
- **Last activity line** — Light secondary text: "3 new items" or "Updated 2 hours ago" or "No activity yet."
- **Unread dot** — A small accent-colored dot on the right edge when there is unseen content or activity.

Tapping a card navigates into the Space.

### 3.3 Sections & Sorting

Spaces are displayed in a single flat list sorted by most recent activity (Spaces with new content float to the top). There is no forced grouping by role.

**Pinning:** Users can long-press (mobile) or right-click (desktop) a Space card to "Pin to Top." Pinned Spaces always appear above unpinned Spaces, in the order they were pinned. A small pin icon appears next to pinned Space names.

**Search:** When the user has 8+ Spaces, a search bar appears at the top of the Home screen. It filters by Space name as the user types.

### 3.4 Empty State

A new user with no Spaces sees a warm illustration and two options:

- Copy: "Welcome to Ochra. Join a Space to start discovering content, or create your own."
- **"Create a Space"** — Large primary button. Opens the Space Builder wizard (Section 4).
- **"I have an invite"** — Subtle secondary link. Opens the system camera for QR scanning or prompts paste from clipboard. Parsed invite links fire `join_group(invite_uri)`. On success, the Space appears in the Home screen list.

### 3.5 Create a Space Button

On mobile: a floating "+" button in the bottom-right corner (above the tab bar). On desktop: a "+" icon at the top of the sidebar. Both open the Space Builder wizard.

### 3.6 Home Screen (Advanced Mode)

Reveals: Group Root Hashes, MLS epoch IDs, active peer connection counts per Space, last-synced timestamps, and PoSrv score.

**Technical:** The Home screen is populated by `get_my_groups()`. Unread state is tracked locally by comparing the latest `ActivityEvent` timestamp against the user's last-opened timestamp for each Space.

---

## 4. Space Builder (Creation & Editing)

The Space Builder is how users create new Spaces and edit existing ones. It is accessed via a "Create a Space" button on the Home screen or an "Edit Space" action from within an existing Space.

### 4.1 Space Creation Wizard

A 4-step guided wizard opens as a full-screen modal (mobile) or centered dialog (desktop) when a user taps "Create a Space."

#### Step 1 — Name Your Space

- A single text field with a large placeholder: "What's your Space called?"
- Optional: tap to add a Space icon (photo picker or emoji selector).
- Optional: tap to add a background image.
- Copy below the field: "You can always change this later."
- "Next" button.

#### Step 2 — Choose a Style

Large visual cards for each template. Each card shows a realistic mini-preview of what the Space will look like when populated:

- **Shop** — Grid of items with prices. "Sell music, videos, art, or anything digital."
- **Community** — Threaded discussion view. "A place for your people to talk."
- **Feed** — Chronological timeline. "Share updates, news, and announcements."
- **Gallery** — Visual mosaic grid. "Showcase photos, artwork, and portfolios."
- **Library** — Clean list with cover art. "Distribute books, guides, and documents."

Tapping a card selects it with a subtle spring animation and checkmark. A "Start from scratch" link at the bottom opens the Advanced Mode JSON Manifest Editor directly.

"Next" button.

#### Step 3 — Invite Creators

- Copy: "Creators are the people who'll add content to your Space. You can always add more later."
- The user's contact list (from `get_contacts()`) with a simple toggle next to each name: "Make Creator."
- Toggling a contact on queues a `grant_publisher_role` call to execute after Space creation.
- If the user has no contacts yet: "No contacts yet. You can invite Creators after your Space is live."
- A secondary link: "Just me for now" — for Hosts who are the sole Creator (common for solo artists). This skips the step; the Host retains default Creator authority.
- "Next" button.

#### Step 4 — You're the Host

A summary card with a warm illustration:

- Space name and icon at top.
- Template style chosen.
- Number of Creators invited (or "Just you").
- Creator Share line: "Creators keep 70% of every sale. You earn 10% as the Host."
- Copy below: "You're in charge. You can change your Space's look, manage who creates, and keep things running smoothly."
- A large "Create Space" button.
- A subtle "Adjust Creator Share" link opens the Easy Mode revenue slider. Most users will never tap this.

**Revenue Split Default:** 10% Host / 70% Creator / 20% Network. The "Network" share (abr_pct) is invisible in Default Mode — the UI shows only the Host and Creator shares. In Default Mode, the display reads: "Creators keep 70%. You earn 10%. The rest supports the network." Advanced Mode reveals the full owner_pct / pub_pct / abr_pct breakdown.

**Technical:** The wizard calls `create_group()` which atomically creates the group on the DHT, applies the selected LayoutManifest template, sets the default revenue split (owner_pct=10, pub_pct=70, abr_pct=20), and queues any `grant_publisher_role` calls. The wizard state is held locally until "Create Space" is tapped — no network calls until final confirmation.

### 4.2 Easy Mode (WYSIWYG Visual Builder)

For editing existing Spaces or making further customizations after creation:

- Users drag-and-drop structural elements (sections, grids, text blocks, hero banners).
- System-safe accent color picker (no custom hex in Easy Mode — a curated palette of 12 colors).
- Template switcher to change the Space's base layout.
- Creator Share displayed as: "Creators keep 70% of every sale. You earn 10% as the Host."

### 4.3 Advanced Mode (JSON Manifest Editor)

- Raw LayoutManifest JSON editor with syntax highlighting.
- Granular DHT timelock settings for revenue split changes.
- Precise fractional Creator Share sliders exposing the full owner_pct / pub_pct / abr_pct breakdown.
- Full accent color hex picker.

### 4.4 Live Preview

The Space Builder includes a live preview so Hosts see exactly what members will see before publishing.

**Desktop:** The screen splits — left panel for builder controls, right panel for live-rendered preview. Changes reflect instantly.

**Mobile:** A floating "Preview" button in the bottom-right corner. Tapping it slides the builder down and shows a full-screen preview with a "Back to editing" bar at the top.

The preview uses the same LayoutRenderer and pre-compiled components as the live Space view — not a separate rendering path. If the Space has published content, the preview shows real content. If new, it shows placeholder cards with generic titles in the chosen accent color.

Preview calls `preview_layout_manifest` locally. No DHT write occurs until the Host taps "Publish Changes" or "Create Space."

### 4.5 LayoutManifest Schema (v2)

```json
{
    "version": 2,
    "template": "shop" | "community" | "feed" | "gallery" | "library" | "custom",
    "theme": {
        "accent_color": "#HEX",
        "background": "light" | "dark" | "system"
    },
    "sections": [
        {
            "type": "hero" | "grid" | "list" | "feed" | "text" | "mosaic" | "library_list",
            "title": "string",
            "filter_tags": ["string"],
            "sort": "newest" | "price_asc" | "price_desc" | "popular" | "title_az",
            "columns": 1 | 2 | 3 | 4
        }
    ],
    "navigation": {
        "show_search": true | false,
        "show_categories": true | false,
        "pinned_tags": ["string"]
    }
}
```

The LayoutRenderer maps these keys to a fixed allowlist of pre-compiled components: StorefrontGrid, ForumThread, NewsFeed, HeroBanner, TextBlock, GalleryMosaic, LibraryList. Arbitrary component injection is a security violation. The `template` field uses protocol-internal names in the JSON schema; the UI presents them as Shop, Community, Feed, Gallery, and Library.

### 4.6 LayoutManifest Propagation

When a Host publishes changes, the signed manifest is published to the DHT via `update_group_layout_manifest`. Online Space members receive the update via `LayoutManifestUpdated` event. Offline members fetch the latest manifest when they next open the Space.

### 4.7 Privacy-Preserving Execution

External CSS, WebFonts, and web-views are strictly forbidden. The binary ships with natively compiled, sandboxed layout primitives. This prevents CSS-based side-channel leaks, IP tracking, and browser fingerprinting.

---

## 5. Space Settings (Host-Only)

When a user navigates into a Space they own, the Space header includes a gear icon that opens Space Settings. This is the centralized configuration hub for the Space — separate from the visual Space Builder (which handles layout and appearance) and the People screen (which handles individual members).

Space Settings is invisible to non-Host members.

### 5.1 Default Mode

A clean settings list, grouped into logical sections with standard iOS/Android settings-style rows.

#### General

- **Space Name** — Tap to edit. Shows current name with a pencil icon. Changes propagate via `update_group_profile`.
- **Space Icon** — Tap to change. Opens photo picker or emoji selector.
- **Description** — Tap to edit. A short text field (max 200 characters). "Describe your Space for people who find it." This is stored in the GroupProfile on the DHT and shown in Space Info (Section 9).

#### Who Can Join

- **Invite Permissions** — A segmented control with two options:
  - **"Anyone in the Space"** — All members can generate and share invite links (default).
  - **"Only me"** — Only the Host can generate invite links. Members see a disabled "Invite" button with tooltip: "The host manages invites for this Space."
- Copy below: "Controls who can invite new people to your Space."

#### Who Can Publish

- **Publishing** — A segmented control with two options:
  - **"Creators only"** — Only members explicitly granted Creator status can publish content (default). This is the standard model.
  - **"Everyone"** — All members can publish without needing Creator status. When enabled, every member who joins is automatically granted Creator privileges. The People screen still shows Creator badges, but the "Make Creator" action is hidden since it's unnecessary.
- Copy below: "Choose whether anyone can add content or only people you pick as Creators."
- Note: Reverting from "Everyone" to "Creators only" does not auto-revoke existing Creator grants. Members who received auto-Creator status retain it until explicitly revoked.

Changes to invite permissions and publishing policy fire `update_group_settings`.

#### Creator Share

- **Current Split** — Display: "Creators keep 70%. You earn 10%."
- **"Propose Change"** — Opens the Revenue Split Change flow (Section 5.3).
- If a pending proposal exists, a countdown card appears: "Split change pending — takes effect in [X days]." with a "Cancel" option.

#### Channels (Advanced Mode Only)

- **Manage Channels** — Opens the Channels management screen (Section 5.4). Only visible when Advanced Mode is enabled.

#### Danger Zone

- **Transfer Ownership** — "Hand this Space to someone else." Tapping opens the Ownership Transfer flow (Section 5.2).

### 5.2 Ownership Transfer Flow

Accessible from Space Settings → Transfer Ownership.

1. **Select New Host** — The member list (from `get_group_members`) appears. Only current members can be selected. The Host taps a member row to select them.
2. **Confirmation** — A serious-toned confirmation screen: "[Name] will become the Host of [Space Name]. You'll become a regular member. This takes 7 days to complete, and you can cancel anytime during that period."
3. **"Start Transfer"** button. Fires `transfer_group_ownership`. The UI shows a countdown card: "Transferring to [Name] — 6 days, 23 hours left. Cancel transfer."
4. **Pending State** — During the 7-day timelock, a banner appears at the top of the Space (visible only to the Host): "Ownership transfer in progress. [X days left.] Cancel." The incoming new Host sees: "You've been chosen as the new Host. Transfer completes in [X days]."
5. **Completion** — After 7 days with no cancellation, the transfer completes. The old Host's Dashboard and People access is removed. The new Host gains all Host-level access. Both parties receive a notification.
6. **Cancellation** — The Host can tap "Cancel" at any point during the 7-day window. Fires `veto_ownership_transfer`. The transfer is silently canceled. The incoming new Host receives: "The ownership transfer was canceled."

### 5.3 Revenue Split Change Flow

Revenue split changes require a 30-day timelock to prevent bait-and-switch scenarios. This flow is accessed from Space Settings → Creator Share → "Propose Change."

1. **Adjust Sliders** — Three linked sliders for Host / Creator / Network shares, constrained to sum to 100%. Default Mode shows only Host and Creator sliders; the Network share adjusts automatically to fill the remainder. Advanced Mode shows all three.
2. **Preview** — "New split: Creators keep [X]%. You earn [Y]%." with comparison to current split.
3. **"Propose Change"** button — Confirmation: "This change takes 30 days to go into effect. All members will be notified." On confirm, fires `propose_revenue_split`.
4. **Pending State** — A countdown card appears in Space Settings and on the Dashboard: "Creator Share change — takes effect in [X days]. Cancel."
5. **Member Notification** — All members see a notification: "The Host proposed a Creator Share change for [Space Name]. New split takes effect [date]." The Space Info screen (Section 9) shows both current and pending splits during the timelock period.
6. **Cancellation** — The Host can cancel at any point during the 30-day window.
7. **Completion** — After 30 days, the new split takes effect silently. No further user action required.

**Advanced Mode:** Shows the full `RevenueSplitChangeProposal` details including sequence number, exact percentages, and broadcast timestamp.

### 5.4 Channels (Host Advanced Feature)

Channels allow a Host to create tiered content access within a Space — for example, a "Premium" channel with exclusive content only accessible to specific members. Channels appear in Space Settings only when Advanced Mode is enabled, keeping Default Mode simple.

- **Create Channel** — Name the channel. Fires `create_subgroup`.
- **Manage Members** — Select which members have access. Fires `mls_grant_subgroup_access` / `mls_revoke_subgroup_access`.
- **Channel List** — Shows all channels with member counts. From `get_subgroup_members`.

When publishing content (Section 11), Creators with access to Channels can choose to publish to a specific Channel instead of the main Space catalog. Members without Channel access cannot see or discover Channel-exclusive content.

**Limits:** Maximum 100 channels per Space. Maximum 2 nesting levels.

### 5.5 Space Settings (Advanced Mode)

Reveals: Group Root Hash, MLS epoch ID, DHT manifest version, raw GroupSettings struct, invite policy flags, publishing policy flags, and pending `RevenueSplitChangeProposal` details.

---

## 6. Host Dashboard

When a user navigates into a Space they own, the Space header gains a "Dashboard" icon (a simple bar chart glyph) that opens the Host Dashboard. This is a per-Space view — Hosts who run multiple Spaces see a separate Dashboard for each.

The Dashboard is invisible to non-Host members. It surfaces only for the Space's Host.

### 6.1 Default Mode

A clean, glanceable summary with Apple Health-style cards. No charts or tables — just numbers and labels. Data from `get_space_stats`.

#### Top Summary Row

Three cards in a horizontal scroll (mobile) or side-by-side row (desktop):

- **Seeds Earned** — Large number with seed icon. Label: "Your earnings from this Space." Subtitle: "This week" with a small trend arrow (up/down/flat from `earnings_trend`). Tapping opens the Earnings Detail view (Section 6.3).
- **Members** — Large number. Label: "People in your Space." Shows total Creators and Moderators as a subtitle: "X creators · Y moderators." Tapping opens the People screen (Section 7).
- **Content** — Large number. Label: "Items available." Tapping opens the Space's content catalog.

#### Recent Activity Feed

Below the summary cards, a reverse-chronological activity feed in plain language:

- "[Name] joined your Space" — with timestamp.
- "[Name] published [Content Title]" — with timestamp.
- "[Content Title] was purchased" — no buyer name (privacy). Shows Seed amount earned.
- "[Name] was made a Creator" — for Host-initiated role changes.
- "You earned [X] Seeds this epoch" — appears once per epoch as a summary line.
- "Space settings were updated" — for SettingsChanged events.

Each row is tappable for detail where relevant (e.g., tapping a published item opens its catalog entry).

The feed is populated by `get_space_activity(group_id, limit, offset)` with pagination.

#### Review Badge

When the Space has pending content reports, a red badge appears on the Dashboard icon and a card appears at the top of the activity feed: "[X] items need your attention." Tapping opens the Review Queue (Section 8).

#### Quick Actions Row

Below the activity feed, a horizontal strip of action buttons:

- **Invite People** — Opens the invite flow (Section 7.3).
- **Add Content** — Opens the publish flow (if the Host is also a Creator).
- **Edit Space** — Opens the Space Builder (Section 4).

### 6.2 Advanced Mode

Reveals: per-Creator revenue breakdown (from `get_earnings_breakdown`), epoch-by-epoch revenue charts, Creator Share audit log (showing any pending `RevenueSplitChangeProposal` timelocks and their countdown), DHT manifest version history, member churn rate (joins vs. leaves per epoch), content publish/tombstone timeline, raw per-content sales counts, and per-content earnings (from `ContentEarning` structs).

### 6.3 Earnings Detail View

Accessed by tapping the "Seeds Earned" summary card.

- A chronological list of earnings events.
- Each row: epoch date, total Seeds earned, breakdown tooltip ("Host share: X Seeds from Y sales").
- Per-content earnings: title, all-time earnings, this-epoch earnings, purchase count.
- Time range picker at top: "This Week" / "This Month" / "All Time."
- Default Mode: clean list only. Advanced Mode: adds a line chart overlay and full EarningsReport breakdown (owner_share, creator_share, abr_share).

---

## 7. People (Creator & Member Management)

Accessible via the Host Dashboard or by tapping a "People" icon in the Space header when the user is the Host. This screen is the central hub for managing who's in the Space and what they can do.

### 7.1 Member List (Default Mode)

A clean scrollable list of all Space members (from `get_group_members`).

Each row shows:

- Avatar (or initial letter circle).
- Display name.
- Role badge (pill-shaped, color-coded):
  - **Host** (you) — gold badge. Always pinned to the top of the list.
  - **Creator** — blue badge.
  - **Moderator** — green badge.
  - **Member** — no badge (clean, default state).
- Joined-since date in relative format ("3 days ago").

### 7.2 Member Actions

Tapping any member row opens a slide-up profile sheet.

#### For a Member (no role):

- Display name, avatar, joined date.
- **"Make Creator"** button (prominent, blue). Confirmation: "[Name] will be able to add content to your Space." → "Confirm" / "Cancel." On confirm, `grant_publisher_role` fires. Badge animates from blank to blue "Creator."
- **"Make Moderator"** option (subtle, below Creator). Confirmation: "[Name] will be able to review reports and remove members." → "Confirm" / "Cancel." On confirm, `grant_moderator_role` fires. Badge animates to green.
- **"Remove from Space"** at the bottom (red text). Confirmation: "[Name] will lose access to this Space. This can't be undone." → "Remove" / "Cancel." Fires `kick_member`.

Note: If the Space's publishing policy is set to "Everyone" (Section 5.1), the "Make Creator" button is hidden since all members can publish.

#### For a Creator:

- Everything above, plus:
- **"Remove as Creator"** (red text, not a red button). Confirmation: "[Name] won't be able to add new content, but their existing content stays." → "Remove" / "Cancel." Fires `revoke_publisher_role`. Badge animates from blue to blank.

#### For a Moderator:

- Everything above, plus:
- **"Remove as Moderator"** (red text). Fires `revoke_moderator_role`. Badge animates from green to blank.

#### For the Host's own row:

- Display name, avatar, "Host since [date]." No action buttons (you can't demote yourself).

### 7.3 Inviting New People

A floating "+" button (bottom-right on mobile, top-right on desktop) opens the invite flow.

If the Space's invite permission is set to "Only me" (Section 5.1), this button is only visible to the Host. Other members see a grayed-out invite icon with a tooltip: "The host manages invites for this Space."

The invite flow presents two options:

- **"Invite as Member"** — Generates a standard invite link via `generate_invite`.
- **"Invite as Creator"** — Generates an invite link with `creator_flag = true`. When the invitee joins, the Host's daemon automatically fires `grant_publisher_role`.

After selecting the invite type, an Invite Options screen appears:

- **Uses** — A segmented control: "Single use" / "Unlimited." Default: Single use.
- **Expires** — A picker: "1 day" / "7 days" / "30 days." Default: 7 days. These map to the `ttl_days` parameter on `generate_invite`. Maximum 30-day TTL.
- **"Create Link"** button.

After the link is generated, the standard Share Sheet appears: QR code display, copy link, or OS share (Android Intent, iMessage, Mail, etc.). Invite links use the anonymous rendezvous protocol — no PIK hashes or IP addresses in the link.

### 7.4 Invite Links (Host-Only)

Accessible via a "Invite Links" row in Space Settings or a subtle "Manage" link next to the invite button on the People screen.

A list of all currently active invite links for this Space (from `get_active_invites`). Each row shows:

- **Type** — "Member" or "Creator" badge.
- **Uses** — "Single use" or "Unlimited." If limited and partially used: "3 of 10 used."
- **Expires** — Relative time: "Expires in 5 days" or "Expired."
- **Status** — Active (green dot) or Expired (gray).

Tapping a row opens a detail sheet with:

- A "Copy Link" button to re-share.
- A **"Revoke"** button (red text). Confirmation: "This link will stop working immediately. People who already joined aren't affected." → "Revoke" / "Cancel." Fires `revoke_invite`.

Expired invites are shown at the bottom in a collapsed "Expired" section for reference, automatically cleared after 7 days.

### 7.5 People Screen (Advanced Mode)

Reveals: PIK hashes, MLS leaf indices, subgroup memberships, per-Creator publishing stats (items published, total revenue generated), raw grant/revoke timestamps, and Moderator action counts.

---

## 8. Content Moderation

A lightweight moderation system that stays invisible until needed. No moderation UI appears until the first content report arrives.

### 8.1 Reporting Content (Member Side)

Any member can report content via a "..." menu on any content item → "Report." A simple selection appears:

- "It's spam"
- "It's offensive or harmful"
- "It's broken or misleading"
- "Other" (optional short text field, max 200 characters)

This fires `report_content`. The report is delivered to the Host via the Space's Sphinx-routed event channel. The member sees: "Thanks. The host has been notified."

Disclosure: "The host of this Space will see your report." Reports use pseudonymous identifiers — the Host and Moderators see a hashed pseudonym for the reporter (per-content, per-epoch), not the reporter's real name or identity. This prevents cross-report correlation while still allowing duplicate detection.

### 8.2 Review Queue (Host / Moderator Side)

When pending reports exist, a red badge appears on the Dashboard icon. Inside the Dashboard, a card appears: "[X] items need your attention."

Tapping opens the Review Queue — a list of reported content from `get_content_reports`, newest first. Each item shows:

- Content title and Creator name.
- Thumbnail (if applicable).
- Report reason and pseudonymous reporter tag (displayed as a short hash, e.g., "Reporter #a3f2").
- Report timestamp.
- Two large buttons: **"Keep"** (dismiss the report) and **"Remove"** (tombstones the content).

**"Remove"** confirmation: "This will hide [Title] from your Space. Existing buyers keep access." On confirm, fires `owner_tombstone_content`.

**"Keep"** dismisses the report silently. The reporter is not notified (no retaliation vector). Fires `dismiss_content_report`.

If the same content is reported by multiple members, reports are grouped under one entry with a count: "Reported by 3 people."

### 8.3 Moderator Role

For Spaces with many members, Hosts may need help. Moderators are granted via the People screen (Section 7.2).

**Moderators can:** View the Review Queue, Keep or Remove reported content, and kick members.

**Moderators cannot:** Grant/revoke Creator status, change the Creator Share, edit the Space layout, transfer ownership, or grant/revoke other Moderators.

Moderator actions (tombstone, kick) are countersigned by the Moderator's PIK and logged in the Space's DHT manifest for Host auditability.

### 8.4 Moderation (Advanced Mode)

Reveals: full moderation audit log (who took what action, when), per-Moderator action counts, and the ability to configure auto-tombstone thresholds (e.g., "auto-remove if 5+ reports within 1 epoch").

---

## 9. Space Info (Member View)

Every Space has a Space Info view accessible by tapping the Space name or an info icon in the Space header. This is the member-facing information screen — distinct from the Host-only Dashboard and Space Settings.

### 9.1 Default Mode

A clean card-style layout:

- **Space Icon & Name** — Large, centered.
- **Description** — The Space description set by the Host (if any). If no description is set, this line is omitted.
- **Host** — "Hosted by [Name]." Tapping the Host name opens their contact profile.
- **Members** — "[X] members." Tapping opens a read-only member list (names and roles only — no management actions).
- **Creators** — "[X] creators." Shown only if the Space has creators beyond the Host.
- **Creator Share** — "Creators keep 70% of every sale." This gives prospective Creators transparency into the revenue split before publishing.
  - If a revenue split change is pending: an additional line appears: "Changing to [X]% on [date]." This ensures members are informed about upcoming changes.
- **Created** — "Created [relative date]."

#### Actions

Below the info card:

- **Notifications** — A row with a toggle to mute/unmute notifications for this Space. Fires `set_group_notification_settings`. Advanced Mode reveals granular controls: notify on purchases, joins, and reports independently, plus a "Mute until" date picker.
- **Invite a Friend** — Generates a member invite link for this Space via `generate_invite` and opens the Share Sheet. This button respects the Space's invite permissions: if set to "Only me," this row is hidden for non-Host members.
- **Leave Space** — Red text at the very bottom. See Section 9.2.

### 9.2 Leave Space

The "Leave Space" action is placed at the bottom of the Space Info screen, styled as red text (not a button — it's an infrequent, consequential action).

Tapping opens a confirmation screen:

- Copy: "Are you sure you want to leave [Space Name]?"
- Detail: "You'll lose access to this Space. Content you've purchased stays yours, but you'll need a new invite to rejoin."
- **"Leave"** (red button) / **"Cancel."**

On confirm, fires `leave_group`. The user is returned to the Home screen. The Space is removed from their Space list.

For Hosts: The "Leave Space" action is not available. Instead, the row reads: "You're the Host. Transfer ownership first to leave." Tapping opens the Ownership Transfer flow (Section 5.2).

### 9.3 Space Info (Advanced Mode)

Reveals: Group Root Hash, MLS epoch, creator count, DHT manifest version, full revenue split breakdown (owner_pct / pub_pct / abr_pct), pending `RevenueSplitChangeProposal` details, and Space creation timestamp (epoch-level precision).

---

## 10. Seeds Screen (Send & Receive)

The wallet is a closed-loop, earn-and-spend system. There are no fiat on-ramps or off-ramps. Users acquire Seeds exclusively by contributing infrastructure to the network.

### 10.1 Default Mode

- **Balance Display:** A large, centered balance with a custom seed glyph: e.g., "[seed-icon] 45" with "Seeds" written below in lighter text. Clean, Apple Health-card aesthetic with a subtle gradient. Below the balance: "Earned by helping the network." No dollar signs, no fiat equivalents, no currency symbols. Balance fetched from `get_wallet_balance`.
- **Send Button:** Tap Send → select contact → enter amount in Seeds → optional note (encrypted end-to-end, max 200 characters) → Double-Click to Confirm or biometric.
- **Receive Button:** Opens the Contact Card (Section 12.3) for the sender to scan or share.
- **Whisper Icon:** A small chat-bubble icon in the top-right of the Seeds screen header, next to Send/Receive. Tapping opens the Whisper hub for ephemeral private messaging (Section 10.3).
- **Transaction History:** A chronological list of all sends, receives, purchases, earnings, and refunds. Each entry shows Seed amount with seed icon, counterparty (display name or "Anonymous" for purchases), timestamp, and type icon (earned / sent / received / purchased / refunded). Whisper-originated Seed transfers appear with a Whisper icon variant.

### 10.2 Advanced Mode

Reveals: separate Seeds vs VYS balances, real-time Oracle Rate (the TWAP peg, from `get_oracle_twap`), current Network Health score (Collateral Ratio with trend, from `get_collateral_ratio`), Groth16 proof generation logs, total circulating supply estimate (from `get_circulating_supply`), per-epoch VYS distribution amounts, and a "Claim VYS Rewards" button (fires `claim_vys_rewards`). Also shows the 0.1% Network Fee disclosure and a "Flush Receipts" button for immediate minting of buffered service receipts (fires `force_flush_receipts` — useful when the user needs Seeds before the next epoch boundary).

### 10.3 Whisper Hub

The Whisper hub is the central screen for ephemeral messaging. Accessed via the Whisper icon on the Seeds screen header.

#### Active Sessions

A list of active Whisper sessions (from `get_active_whispers`). Each row shows:

- **Counterparty** — Display name (if revealed), @username (if revealed), or "Anonymous" (default).
- **Last message preview** — First line of the most recent message, truncated.
- **Timestamp** — Relative time of last message.
- **Unread badge** — Unread message count.
- **State indicator** — A subtle icon for sessions in "background_grace" state (a small clock icon).

Tapping a session opens the conversation view (Section 10.4).

#### Empty State

If no active Whisper sessions: "No conversations yet. Whisper someone to start a private, disappearing chat."

#### New Whisper

A "+" button opens the New Whisper flow:

1. **Choose Recipient** — Two paths:
   - **By Username** — A text field with "@" prefix and live availability/resolution via `resolve_handle`. Shows the resolved handle status (Active / Deprecated with successor link). If deprecated, shows "This username has moved to @[successor]. Start a Whisper with them instead?" with a tap-to-redirect.
   - **From Contacts** — The user's contact list. Tapping a contact starts a Whisper session directly (no username required).
2. Fires `start_whisper` with either `WhisperTarget::Handle(username)` or `WhisperTarget::Contact(pik_hash)`.

#### Ephemeral Notice

A persistent subtle banner at the top of the Whisper hub: "Messages disappear when you close them. Nothing is saved."

### 10.4 Whisper Conversation View

A full-screen chat view for an active Whisper session.

#### Message Bubbles

- Outgoing messages: right-aligned, accent-colored.
- Incoming messages: left-aligned, neutral background.
- System messages (identity reveal, Seed transfer, session start/end): centered, dimmed text.
- Max 500 characters per message. Text-only — no images, files, or audio.

#### Input Bar

- Text input with emoji keyboard toggle.
- Send button. Fires `send_whisper`.
- Typing indicators: the input bar shows "Typing..." when the counterparty is composing. Fires `send_typing_indicator` on keystroke (debounced). Received via `WhisperReceived` events with `msg_type: "Typing"`.
- Read receipts: Messages show a subtle "Read" indicator when the counterparty acknowledges. Fires `send_read_ack` on message display.

#### Inline Seed Transfer

A "[seed-icon]" button next to the text input. Tapping opens a compact transfer sheet within the conversation: enter amount → optional note → biometric/Double-Click to Confirm. The transfer appears as a system message bubble in the conversation. Fires `send_whisper_seeds`.

#### Identity Controls

A header bar at the top of the conversation shows the counterparty's identity:

- **Anonymous** (default) — "Anonymous Whisper" with a subtle mask icon.
- **Revealed** — Display name and/or @username, with a verified checkmark if identity is cryptographically proven.

A "Reveal yourself" option in the conversation menu (accessible via "..." in the header). Fires `reveal_identity`. After revealing, the counterparty sees the user's display name and/or username. This action is irreversible within the session.

#### Relay-Cost Indicator

When the sender has exceeded the free message tier (20 messages, or 100 for verified contacts), a subtle indicator appears near the input bar: "Helping the network..." with a small spinner during relay work. This surfaces the anti-spam relay-cost mechanism without technical jargon. The indicator appears only during active relay packet forwarding.

Advanced Mode reveals the full throttle status: current tier, receipts required per message, global hourly count and surcharge (from `get_whisper_throttle_status`).

#### Session Lifecycle

- **Close** — "End Conversation" in the "..." menu. Fires `close_whisper`. Confirmation: "This conversation will disappear. Messages can't be recovered." All message data is zeroized.
- **Block** — "Block" in the "..." menu. Fires `block_whisper`. Confirmation: "Block this person? The conversation will end and they won't be able to Whisper you again from this session." Session torn down and counterparty's session key blacklisted.
- **Background Grace** — When the app backgrounds, a subtle banner appears on return: "Session paused — [X]s remaining" with a countdown of the grace period (120s mobile, 5min desktop). If the grace period expires, the session is torn down and a "Session ended" notice appears.
- **Offline Detection** — If the counterparty goes offline, a banner: "[Name/Anonymous] is offline. This conversation has ended."

### 10.5 Username Management

Accessible from the **"You"** tab → **"Username"** row.

#### Setup Flow

1. **Choose a Username** — Text field with "@" prefix. Live availability checking via `check_handle_availability` as the user types (debounced 300ms). Constraints shown: 3–20 characters, lowercase letters, numbers, and underscores only.
2. **Confirm** — "Register @[username]?" with a brief note: "Your username expires if you're offline for more than 7 days. It renews automatically when the app runs."
3. Fires `register_handle`. Shows a brief progress indicator (Argon2id PoW computation may take a few seconds).
4. **Success** — "You're @[username]." The username appears in the You tab and is available for Whisper.

#### Change Flow

1. User taps their current username → "Change Username."
2. Same text field with live availability checking.
3. Confirmation: "Change from @[old] to @[new]? People who search for your old name will be redirected."
4. Fires `change_handle`. This atomically registers the new name and deprecates the old one with a successor redirect.

#### Remove Flow

1. User taps their current username → "Remove Username."
2. Confirmation: "Remove @[username]? People won't be able to find you by name anymore."
3. Fires `deprecate_handle(successor_handle: None)`.

---

## 11. Content Browsing, Purchasing & Downloads

### 11.1 Content Catalog

Each Space renders its content catalog using the template-specific LayoutRenderer (Section 4.5). Content is loaded from `get_store_catalog(group_id)`. Content items display:

- **Title** — Primary text.
- **Creator name** — Secondary text.
- **Price tag** — In Seeds with seed icon. For free content: "Free" in green text (no seed icon). If multiple pricing tiers exist, the lowest price is shown: "From 2 Seeds."
- **Thumbnail** — If the content type supports it.
- **Access badge** — If the user already owns or has access: green "Yours" badge (permanent) or "Access — X days left" (rental).

**Search:** A search bar at the top of the catalog (when `show_search` is enabled in the LayoutManifest). Fires `search_catalog(group_id, query, tags)` against the local FTS5 index. Results ranked by relevance, recency, and purchase count. Tag filters appear as tappable pills when `show_categories` is enabled.

**Content Versioning:** When a content item has a `successor_hash` (i.e., a newer version exists), a banner appears on the original item: "A newer version is available." Tapping navigates to the successor. Free updates (successor with 0-Seed pricing tier) are handled seamlessly.

**Content Detail View:** Tapping a content item opens a detail view showing title, description, Creator name, tags, pricing tiers, file size, published date, and access status (from `get_access_status`). If the user has purchased the content, download and re-download actions are available.

**Content Pinning (Advanced Mode):** In the content detail view, an Advanced Mode option "Pin this content" keeps the content's ABR chunks on the user's device, preventing LFU-DA eviction. Fires `pin_content`. Pinned content shows a pin icon. "Unpin" reverses the action via `unpin_content`. Pinned content is capped at 50% of ABR allocation and is evicted last under disk pressure — but is not exempt.

### 11.1.1 Publishing Content (Creator View)

Creators see an "Add Content" button in Spaces where they have publishing permission. Tapping opens the publish flow:

1. **Select File** — File picker for the content to publish. Max 50 GB.
2. **Details** — Title (required), description (optional), up to 5 tags.
3. **Pricing** — Up to 4 pricing tiers. Each tier specifies: type (permanent or rental), price in Seeds (0 for free), and rental duration (if rental). At least one tier is required.
   - Example: "Free" + "Yours Forever — 5 Seeds" + "7-Day Access — 2 Seeds."
4. **Channel** — If the Space has Channels (Section 5.4), the Creator can select which Channel to publish to. Default: main Space catalog.
5. **Publish** — Fires `publish_file`. A brief progress indicator shows during Argon2id proof-of-work computation and chunk upload. The content appears in the catalog once the ContentManifest is broadcast to the Space's MLS group.

**Pricing Changes:** After publishing, Creators can update pricing tiers via the content detail view → "Edit Pricing." Fires `set_content_pricing`. This does not affect existing purchases.

**Force Macro (Advanced Mode):** A toggle for Creators to force escrow-based settlement even for content priced below 5 Seeds. Visible only in Advanced Mode.

### 11.2 Checkout

Tapping a price tag slides up a bottom-sheet modal. If the content has multiple pricing tiers, the modal displays all options:

- "7-Day Access — 2 Seeds"
- "Yours Forever — 5 Seeds"
- "Free" — For tiers with `price_seeds = 0`. No payment flow required — the user taps "Get" and the content key is delivered directly. No blind receipt token, no escrow.

Content already owned permanently: green "Yours" badge and "Download" button. Active access: "Access — X days left" with optional "Keep Forever" button.

Tapping a purchase option fires `purchase_content(content_hash, tier_index)`. The daemon returns a `DownloadProgress` stream that the UI renders as the download progress bar.

### 11.3 P2P Transfers

Tap Send → select contact → type amount in Seeds → optional encrypted note (max 200 characters) → Double-Click to Confirm or biometric. Fires `send_funds`. The recipient receives the transferred amount minus the 0.1% Network Fee. The fee is invisible in Default Mode.

Seeds can also be sent inline within a Whisper conversation (Section 10.4).

### 11.4 Transaction Feedback

- **Authorization:** Double-Click to Confirm or system biometrics (FaceID, Fingerprint, Windows Hello).
- **Free content:** Instant green checkmark — no authorization required.
- **Micro (< 5 Seeds):** Instant green checkmark + haptic ping.
- **Macro (≥ 5 Seeds):** Determinate progress ring ("Confirming...") for approximately 2 seconds (escrow verification), then green checkmark + haptic ping. If the Creator fails to deliver the content key within 60 seconds, auto-refund occurs and the user sees: "Something went wrong. Your Seeds have been returned."
- **Failure:** Red X icon + specific error message (mapped from Section 29 error codes in the Unified Technical Specification). "Try Again" button always shown.

### 11.5 Download Management

After purchase, content downloads begin automatically. The UI provides:

- **Download Progress** — A progress bar showing `downloaded_bytes / total_bytes` and chunk-level progress (`chunks_complete / chunks_total`). State indicators: "Downloading," "Paused," "Verifying," "Complete," or "Failed."
- **Pause/Resume** — A pause button on active downloads. Fires `pause_download` / `download_file` to resume. Downloads can survive app restart (chunks already downloaded are retained).
- **Re-download** — Previously purchased content can be re-downloaded from the catalog or purchase history. Tapping "Download" fires `redownload_content`. The daemon uses the Blind Receipt Token system. The user does not need to understand the underlying zero-knowledge mechanism.

### 11.6 Purchase Library

Accessible from the **"You"** tab → **"My Purchases"** row.

A chronological list of all purchased content (from `get_purchase_history`). Each row shows:

- Content title and Space name.
- Access status badge: "Yours" (permanent, green) or "Access — X days left" (rental, with countdown).
- Download button for re-download.
- "Keep Forever" upgrade button for rental content that can be permanently purchased.

**Advanced Mode:** Shows receipt details (from `get_purchase_receipts`) including receipt IDs, last republish epoch, and expiry timestamps.

**Access Expiry:** Content approaching expiry (within 24 hours) shows an orange warning: "Expires soon." Expired rental content is grayed out with "Expired" badge.

### 11.7 Anonymous Refunds

Users can request an anonymous refund for any purchase within 30 days.

- In the purchase library or content detail view: "Request Refund" link.
- Confirmation: "Request a refund? The Creator won't know who you are."
- Fires `request_anonymous_refund`. The UI shows a status indicator: "Refund requested" → "Refund approved" (Seeds returned) or "Refund rejected."
- The underlying Groth16 zero-knowledge proof mechanism is invisible to the user.

---

## 12. Contacts

Contacts are the people you've connected with on Ochra. The Contacts system is deliberately isolated from Spaces — a contact's profile never reveals which Spaces they belong to, and a Space's member list never highlights which members are your contacts. This separation ensures that if a device is compromised, the attacker cannot reconstruct a social graph linking identities across Spaces.

### 12.1 Contacts Screen

Accessible via the **"You"** tab → **"Contacts"** row. This is the central hub for managing your connections.

#### Contact List

A clean alphabetical list of all contacts (from `get_contacts()`). Each row shows:

- **Avatar** — Photo or initial letter circle.
- **Display name** — Primary text.
- **Online indicator** — A subtle green dot if the contact's daemon is currently reachable (optional, can be disabled in settings for additional privacy).

A search bar appears at the top when the user has 8+ contacts, filtering by name as the user types.

#### Empty State

A new user with no contacts sees:

- Copy: "Add people you know to send Seeds, share invites, and stay connected."
- **"Add a Contact"** — Large primary button. Opens the Add Contact flow (Section 12.3).

### 12.2 Contact Profile

Tapping a contact row opens a slide-up profile sheet:

- **Avatar and display name** — Large, centered.
- **Added** — "Connected [relative date]."
- **Send Seeds** — Button. Opens the Send flow pre-filled with this contact.
- **Whisper** — Button. Opens a Whisper session directly with this contact (no username required — uses contact's profile introduction points).
- **Remove Contact** — Red text at the bottom.

The profile intentionally shows **no Space membership information, no shared Spaces, and no activity history**. This is a privacy-by-design decision: contacts and Spaces are separate trust domains.

#### Remove Contact

Tapping "Remove Contact" opens a confirmation:

- Copy: "Remove [Name] from your contacts? You won't be able to send them Seeds directly. You can always add them again later."
- **"Remove"** (red button) / **"Cancel."**

On confirm, fires `remove_contact`. The daemon triggers a profile key rotation: a new profile key is generated, the profile blob is re-encrypted, and the new key is distributed to remaining contacts. The removed contact can no longer decrypt the user's profile.

### 12.3 Adding Contacts

The "+" button on the Contacts screen (or "Add a Contact" in the empty state) opens the Add Contact flow with two paths:

#### Path 1 — Share Your Contact Card

The Contact Card is a frosted glass UI card displaying a dynamic QR code generated by `generate_contact_token`. Below the QR code:

- **"Share"** button — Triggers the native OS Share Sheet (Android Intent, iMessage, Mail). The shared link is a short random string: `ochra://connect?token=[Base58]`.
- **Token expiry** — Subtle text below: "This link works once and expires in [X] hours."
- **Regenerate** — A subtle "New Link" text button that invalidates the current token and generates a fresh one.

#### Path 2 — Scan or Paste

- **"Scan QR Code"** — Opens the device camera.
- **"Paste Invite"** — For links received via messaging apps. A text field for pasting an `ochra://connect?token=...` link.

After scanning or pasting, the user sees: "Add [Name] to your contacts?" with an **"Add"** button. On confirm, fires `add_contact`.

### 12.4 Contact Token Properties (Invisible to Users)

- Tokens are ephemeral one-time-use. After claimed, the DHT entry is consumed.
- No persistent identity is exposed on any clearnet channel — no PIK hash, no display name, no IP address.
- Tamper-evident: if an attacker intercepts first, the intended recipient gets a miss and the sender regenerates.

### 12.5 Contacts (Advanced Mode)

Reveals: PIK hashes, profile key fingerprints, last-seen epoch, and contact exchange timestamps.

---

## 13. Earn Screen

The Earn tab is the user's window into their infrastructure contribution and earning configuration.

### 13.1 Default Mode

- **Earning Level Slider** — Same fluid slider as the Setup wizard (Section 2, Step 3), with snap points for 🌱 Low, 🌿 Medium, 🌳 High, and ⚙️ Custom. The Custom option opens a GB allocation field.
  - **Allocations:** Low: 5 GB desktop / 1 GB mobile. Medium: 25 GB / 5 GB. High: 100 GB / 15 GB. Custom: user-defined (floor 500 MB, ceiling 80% free space).
- **Earn While I Sleep Toggle** — Controls 2-8 AM smart wake for ABR checks.
- **Current Earnings Summary** — A card showing Seeds earned today and this epoch.
- **Storage Used** — A visual indicator: "Using X GB of Y GB." A gentle progress bar.

### 13.2 Advanced Mode

Reveals: LFU-DA eviction logs, GB quotas per-level, PoSrv score and component breakdown, Sphinx relay latency stats, DHT health metrics, zk-PoR submission status and proving time, ABR telemetry (from `get_abr_telemetry`), cover traffic statistics (from `get_cover_traffic_stats`), and manual zk-PoR submission button (fires `submit_zk_por_proof`).

### 13.3 Disk Pressure Alert

When free disk space falls below 20%, a non-dismissible banner appears on the Earn screen: "Low disk space. Ochra is reducing stored data to free up room." The daemon automatically triggers LFU-DA eviction. The banner clears when space recovers above 25%.

---

## 14. Navigation & Progressive UI Modes

Desktop uses a sidebar. Mobile uses standard bottom navigation tabs with 44px minimum touch-targets.

### 14.1 Global Navigation

| **Tab** | **Default Mode** | **Advanced Mode** |
|---|---|---|
| **Home** | Space cards with icons, names, role badges, activity lines, unread dots. Search bar at 8+ Spaces. Pinned Spaces at top. | Group Root Hashes, MLS epoch IDs, active peer connection counts, PoSrv score. |
| **Seeds** | Balance in Seeds with seed icon. Send + Receive buttons. Whisper icon (opens Whisper hub). Transaction history. | Seeds vs VYS. Oracle Rate. Network Health (CR + trend). Proof logs. Circulating supply. Claim VYS rewards. |
| **Earn** | Earning Level slider (🌱/🌿/🌳/⚙️). Earn While I Sleep toggle. Earnings summary. Storage used. | LFU-DA logs. GB quotas. PoSrv scores. Sphinx latency. DHT health. zk-PoR status. Cover traffic stats. |
| **You** | Theme, notifications, display name, username (@handle), password change, Recovery Contacts, Contacts, My Purchases, Lock, About. | Daemon logs. Export diagnostics. Protocol version. Handle signing key fingerprint. Circuit health. Configuration file editor. |

### 14.1.1 You Tab Detail

The You tab is the personal settings hub. Rows in order:

- **Display Name** — Tap to edit. Fires `update_display_name`.
- **Username** — Shows current @handle or "Set up a username." Links to username management (Section 10.5). Fires `get_my_handle` to display current status.
- **Theme** — Light / Dark / System toggle. Fires `set_theme_settings`.
- **Password** — "Change Password." Fires `change_password`.
- **Biometrics** — Toggle for FaceID / Fingerprint / Windows Hello. Fires `enroll_biometric`.
- **Recovery Contacts** — Shows current guardian count and health. Fires `get_guardian_health`. Links to Recovery Contact setup (Phase 30).
- **Contacts** — Links to Contacts screen (Section 12).
- **My Purchases** — Links to Purchase Library (Section 11.6).
- **Notifications** — Global notification preferences.
- **Lock** — "Lock Ochra." Fires `lock_session`.
- **Export Data** — "Download your data." Fires `export_user_data`.
- **About** — App version, protocol version (from `get_network_stats`), and check for updates.

**Advanced Mode additions:** Daemon logs (from `get_daemon_logs`), Export diagnostics (fires `export_diagnostics`), PIK hash (from `get_my_pik`), export revocation certificate (fires `export_revocation_certificate`), Onion circuit health (from `get_onion_circuit_health`), configuration file path.

### 14.2 Inside a Space (Member View)

| **View** | **Default Mode** | **Advanced Mode** |
|---|---|---|
| **Catalog** | Content rendered by template (Shop grid, Community threads, etc.). Prices in Seeds. Search bar. "Free" badges. Access status badges. | Content hashes, publication timestamps, Creator PIKs, successor chains. |
| **Space Info** | Space name, icon, description, Host name, member count, Creator Share (with pending changes), creation date. Notification toggle. Invite a Friend. Leave Space. | Group Root Hash, MLS epoch, creator count, DHT manifest version, full revenue split, pending split proposal details. |

### 14.3 Inside a Space (Host View)

Everything in the member view, plus:

| **View** | **Default Mode** | **Advanced Mode** |
|---|---|---|
| **Dashboard** | Seeds Earned / Members / Content summary cards. Activity feed with pagination. Quick actions. Review badge. | Per-Creator revenue, epoch charts, Creator Share audit log with timelock countdowns, churn rate, moderation audit log, per-content earnings. |
| **People** | Member list with role badges. Make Creator / Make Moderator / Remove. Invite shortcuts. Invite Links management. | PIK hashes, MLS leaves, subgroup memberships, per-Creator publish stats, grant/revoke timestamps. |
| **Space Settings** | Name, icon, description editing. Invite permissions. Publishing policy. Creator Share with proposal flow. Ownership transfer. | Group Root Hash, raw GroupSettings, DHT manifest version, Channel (subgroup) management, pending split proposal struct. |
| **Edit Space** | WYSIWYG builder with live preview. | JSON Manifest Editor. Full Creator Share sliders. DHT timelocks. |

### 14.4 Full-Screen Modals

The following flows open as full-screen modals (mobile) or centered dialogs (desktop):

- Space Creation Wizard (Section 4.1)
- Space Builder / Editor (Section 4.2-4.3)
- Checkout (Section 11.2)
- Recovery Contact setup (Section 2, Step 4)
- Ownership Transfer (Section 5.2)
- Revenue Split Change (Section 5.3)
- Whisper Conversation (Section 10.4)

---

## 15. System Behaviors

### 15.1 Notifications

**Mobile Push:** Privacy-preserving push via FCM (Android) and APNs (iOS). The push payload contains only a wake signal — no content. On wake, the app fetches actual notification data via the Sphinx circuit. On Android, UnifiedPush-compatible endpoints are supported as an opt-in alternative to avoid Google dependency.

**Notification Events:**

- New content in a Space (configurable per-Space).
- Incoming P2P transfer.
- Recovery Contact recovery request.
- Update available (with mandatory/optional distinction).
- Recovery Contact health alert (days since heartbeat).
- Approaching session timeout.
- Access expiring soon (24 hours before expiry).
- Refund received.
- Escrow timeout (auto-refund).
- Content reported (Host/Moderator only).
- Ownership transfer initiated/completed/canceled (Host and new Host only).
- Invite link expiring soon (Host only, 24 hours before expiry).
- Revenue split change proposed (all members).
- New Whisper session request (push wake signal — no sender info in payload).
- Whisper Seed transfer received (amount shown only after app unlock).
- Missed Whisper attempt (dead drop ping — "Someone tried to reach you," no sender info).
- Username expiring soon (24 hours before HandleDescriptor TTL expiry).
- Disk pressure alert (free space <20%).
- Minting complete (Seeds minted this epoch).

**Desktop:** Native OS notification center integration.

**Event-to-Notification Mapping:**

The UI subscribes to daemon events via `subscribe_events` and maps them to user-facing notifications:

| **Daemon Event** | **UI Notification** | **Visibility** |
|---|---|---|
| `MemberJoined` | "[Name] joined your Space" | Host/Moderator |
| `MemberLeft` | "[Name] left your Space" | Host |
| `ContentPublished` | "New content in [Space]" | Configurable per-Space |
| `ContentPurchased` | "[Title] was purchased — [X] Seeds earned" | Host/Creator |
| `ContentTombstoned` | "[Title] was removed" | Creator (own content) |
| `ContentReported` | "New report in [Space]" | Host/Moderator |
| `CreatorGranted` | "You're now a Creator in [Space]" | Target member |
| `CreatorRevoked` | "Creator access removed in [Space]" | Target member |
| `ModeratorGranted` | "You're now a Moderator in [Space]" | Target member |
| `ModeratorRevoked` | "Moderator access removed in [Space]" | Target member |
| `SettingsChanged` | "Settings updated in [Space]" | All members |
| `OwnershipTransferPending` | "Ownership transfer started in [Space]" | Host + new Host |
| `OwnershipTransferCompleted` | "You're the new Host of [Space]" | New Host |
| `OwnershipTransferCanceled` | "Ownership transfer canceled" | New Host |
| `EpochEarningsSummary` | "You earned [X] Seeds today" | All earners |
| `FundsReceived` | "[Name] sent you [X] Seeds" | Recipient |
| `FundsSent` | "Sent [X] Seeds to [Name]" | Sender (local only) |
| `RefundReceived` | "Refund received — [X] Seeds" | Buyer |
| `EscrowTimeout` | "Something went wrong. [X] Seeds returned." | Buyer |
| `MintingComplete` | "Minted [X] Seeds this epoch" | Earner (Advanced Mode) |
| `VysRewardsClaimed` | "Claimed [X] VYS rewards" | Claimant (Advanced Mode) |
| `CollateralRatioChanged` | (Silent — updates Advanced Mode displays) | Advanced Mode |
| `AccessExpiringSoon` | "[Title] access expires in [X] hours" | Buyer |
| `InviteExpiringSoon` | "Invite link expiring soon" | Host |
| `RecoveryContactAlert` | "Recovery attempt on your account" | Account owner |
| `RecoveryContactHealthAlert` | "Recovery Contact [Name] hasn't been seen in [X] days" | Account owner |
| `OTAUpdateAvailable` | "Update available" / "Update required" | All users |
| `DiskPressureAlert` | "Low disk space" banner | Earner |
| `CircuitBreakerActivated` | "Oracle stale" amber banner | Advanced Mode |
| `CircuitBreakerDeactivated` | (Silent — clears amber banner) | Advanced Mode |
| `DaemonStarted` | (Silent — initializes UI state) | Internal |
| `DaemonShuttingDown` | (Silent — triggers graceful UI teardown) | Internal |
| `ZkPorSubmitted` | "Storage proof submitted" | Advanced Mode |
| `LayoutManifestUpdated` | Space UI refreshes automatically | All members |
| `WhisperSessionStarted` | "New Whisper" (push wake, no sender info) | Recipient |
| `WhisperReceived` | Message appears in conversation | Recipient |
| `WhisperSessionEnded` | "Session ended" notice in conversation | Both parties |
| `WhisperSeedTransferReceived` | "[X] Seeds received in Whisper" (after unlock) | Recipient |
| `WhisperIdentityRevealed` | Identity update in conversation header | Counterparty |
| `WhisperThrottleChanged` | Relay-cost indicator updates | Sender |
| `WhisperBackgroundGraceStarted` | Grace countdown banner | App foregrounded |
| `WhisperPingReceived` | "Someone tried to reach you" | Recipient |
| `HandleDeprecated` | Successor redirect in resolution UI | Whisper users |
| `HandleExpiring` | "Your username expires soon" | Handle owner |

### 15.2 Offline Mode

When the device has no internet connectivity:

- **Available:** Browse previously downloaded content, view wallet balance (cached), view contacts and Space lists, compose queued P2P transfers (executed on reconnect).
- **Unavailable:** Purchasing, re-downloading, sending transfers, joining Spaces, ABR serving, earning Seeds, Whisper messaging (all sessions require both parties online), catalog search (FTS index is local but new content won't sync).
- **Access Enforcement:** The daemon enforces access expiry locally even without connectivity.
- **Indication:** A subtle banner at the top: "Offline — some features unavailable."

### 15.3 Session Lock

After 15 minutes of inactivity (configurable), the app locks and requires re-authentication (password or biometric). During lock:

- **Whisper:** Active sessions continue receiving messages in the background. Messages display after re-authentication. Sending requires re-authentication. The grace period timer continues running.
- **Notifications:** Push wake signals are still received, but notification content is hidden behind "Unlock to view."
- **Manual Lock:** A "Lock" option in the "You" tab triggers immediate lock. Fires `lock_session`.
- **Re-authentication:** Password entry or biometric verification. Fires `authenticate` or `authenticate_biometric`.

### 15.4 Protocol Updates (OTA)

When a protocol update is available (detected via `check_protocol_updates` or `OTAUpdateAvailable` event):

- **Optional Update:** A non-intrusive banner on the Home screen: "Update available — [version]." Tapping shows a changelog summary and an "Update Now" button. The update downloads and installs via peer-to-peer binary distribution. The user can dismiss and update later.
- **Mandatory Update:** A blocking modal: "Ochra needs to update to continue working. This update is required for security." with an "Update Now" button. The user cannot dismiss. A countdown shows the activation epoch deadline. After the activation epoch, legacy nodes are partitioned from the network.
- **Update Progress:** A determinate progress bar during download and verification.

Fires `apply_protocol_update` on user confirmation.

### 15.5 Accessibility

- **Screen Readers:** Full VoiceOver (iOS/macOS) and TalkBack (Android) support.
- **Dynamic Type:** Respects system font size preferences on all platforms.
- **Contrast:** All text meets WCAG 2.1 AA contrast ratios (4.5:1 minimum for body text, 3:1 for large text).
- **Touch Targets:** 44px minimum on all interactive elements.
- **Role Badges:** Color-coded badges also include text labels, never relying on color alone.

### 15.6 Data Export

Users can export their data via `export_user_data()` which produces a JSON archive containing: contact list (including profile keys), purchase history, blind receipt secrets (for re-download), earnings history, Space membership list, Recovery Contact configuration, and handle registration data (handle signing keypair, registered username). This does not include Seed tokens, published content, or Whisper message history (which is never persisted).

### 15.7 Disk Pressure

When free disk space drops below 20%, the system triggers automatic LFU-DA eviction to 50% of the ABR allocation. The Earn screen shows a persistent alert (Section 13.3). The daemon resumes normal allocation when space recovers above 25% (5% hysteresis). Pinned content is evicted last but is not exempt.

### 15.8 Circuit Breaker (Advanced Mode Only)

When the TWAP Oracle becomes stale (6+ hours without a fresh price), the system enters Circuit Breaker mode. In Advanced Mode, a persistent amber banner appears: "Oracle stale — economic operations may be delayed." This affects minting rates and Collateral Ratio adjustments. The banner clears when the Oracle recovers (CircuitBreakerDeactivated event). Default Mode users are unaffected — the system continues operating with cached rates.

---

## 16. UI Build Phases

These phases correspond to Phases 25-32 in the Unified Technical Specification's build order and detail the UI-specific implementation.

### Phase 25 — Design System

Responsive design tokens. Tailwind config. Desktop sidebar + mobile bottom nav (44px targets). Spring animations. 60fps transitions. Accessibility foundations (VoiceOver, TalkBack, WCAG AA). Bottom nav tabs: Home, Seeds, Earn, You. Light/Dark/System theme support. Session lock screen with biometric + password entry.

### Phase 26 — Setup Assistant

Five-step Welcome Experience: identity creation (name + password + optional biometric), Meet Seeds tutorial animation, Earning Level slider with plant-growth metaphor (🌱 Low / 🌿 Medium / 🌳 High / ⚙️ Custom), Earn While I Sleep toggle, bandwidth disclosure (desktop), Protect Your Account (Recovery Contact prompt with threshold display and skip option), and You're Ready summary. Deep-link parsing for `ochra://invite` (rendezvous), `ochra://connect` (contact exchange), and `ochra://whisper` (Whisper session). Configuration file initialization (Section 33 of the Unified Technical Specification).

### Phase 27 — Space Builder & Home Screen

Home screen: Space card list with icons, names, role badges (Host gold, Creator blue), activity lines, unread dots, pin-to-top, search at 8+ Spaces, empty state. 4-step creation wizard: Name → Style → Invite Creators → Summary. WYSIWYG Easy Mode with 5 templates (Shop, Community, Feed, Gallery, Library), drag-and-drop sections, curated accent color palette, Creator Share default ("Creators keep 70%," 10/70/20 split). Advanced Mode JSON editor with full split sliders. Live preview: split-screen (desktop) / toggle (mobile).

### Phase 28 — Layout Renderer

Sandboxed LayoutRenderer. Pre-compiled primitives: StorefrontGrid, ForumThread, NewsFeed, HeroBanner, TextBlock, GalleryMosaic, LibraryList. LayoutManifest schema v2. No external CSS/WebFonts/web-views. Template-to-component mapping with fallback to "custom" raw layout. Content search integration (FTS5 local index via `search_catalog`). Free content "Free" badge rendering. Access status badges ("Yours" / "Access — X days left" / "Expired"). Content versioning successor banners.

### Phase 29 — Seeds & Connections

Seeds screen: balance display with seed icon (no fiat), Send + Receive buttons, Whisper icon entry point, transaction history with type icons (earned/sent/received/purchased/refunded), offline queuing. Contacts screen: alphabetical contact list, contact profile (no Space linkage), add contact (QR + Share Sheet + paste), remove contact with profile key rotation, search at 8+ contacts, Whisper button on contact profiles. Ephemeral contact exchange: token generation, QR code display on frosted glass Contact Card, OS Share Sheet integration. P2P transfer flow with encrypted notes (max 200 chars). Purchase library with access status badges ("Yours" / "Access — X days left" / "Expired"). Re-download button. Anonymous refund request flow. Download management with pause/resume and progress states.

### Phase 29.5 — Host Experience

Host Dashboard: SpaceStats summary cards (Seeds Earned with trend / Members with creator+moderator counts / Content), activity feed with pagination, earnings detail timeline with per-content breakdown, quick actions strip. People screen: member list with role badges (Host/Creator/Moderator/Member), one-tap promote/demote, profile sheets with action buttons, "Invite as Creator" shortcut with creator_flag. Invite Links management: active invite list, revoke, expiry display. Content moderation: Review Queue (Keep/Remove), pseudonymous reporter tags, report grouping, red badge indicator. Moderator role: grant/revoke, audit log (Advanced Mode). Space Settings: name/icon/description editing, invite permissions (Anyone/Only me), publishing policy (Creators only/Everyone with revert note), Creator Share with 30-day proposal flow and countdown, ownership transfer flow with 7-day timelock countdown, Channel management (Advanced Mode: create/manage subgroups). Space Info (member view): description, Host name, member count, Creator Share with pending change display, creation date, granular notification settings, Leave Space action, Invite a Friend.

### Phase 29.7 — Whisper

Full Whisper UI implementation:

- Whisper hub (active session list with state indicators, empty state, new Whisper flow with username resolution and contact selection).
- Conversation view (message bubbles, input bar with emoji keyboard, typing indicators, read receipts, inline Seed transfer, identity reveal controls, ephemeral notice banner, relay-cost throttle indicator with "Helping the network..." copy, background grace countdown banner, offline detection banner, block session, close session with zeroization confirmation).
- Username management under You tab (setup with live availability checking, change with atomic old→new redirect, remove flows).
- Handle resolution UI (inline validation, deprecation notices with successor redirect links).
- Notification integration for Whisper events (session request, Seed transfer, missed attempt, username expiring).
- Deep link handling for `ochra://whisper?to=username`.

### Phase 30 — Recovery Contact UI

Recovery Contact setup flow ("Protect Your Account"). Contact selector with threshold display (e.g., "2 of 3 needed"). Guardian health status display with days-since-heartbeat (from `get_guardian_health`). 48-hour Veto Recovery alert with countdown — when a recovery is initiated, the user sees a full-screen alert: "Someone is trying to recover your account. If this wasn't you, cancel now." with a "Cancel Recovery" button (fires `veto_recovery`). Recovery initiation flow for users who have lost their password: collect guardian shares and fire `initiate_recovery`. Dead drop-based health alerts. `replace_guardian` flow with new contact selection. Recovery initiation flow.

### Phase 31 — Checkout

Bottom-sheet checkout modals with pricing tier selection (amounts in Seeds). Free content "Get" button (no authorization). Atomic escrow progress indication for macro transactions (determinate ring → checkmark → auto-refund on 60s Creator timeout). Biometric prompts. Error states with mapped error codes from Section 29 of the Unified Technical Specification. Success haptics. Access expiry banners. "Keep Forever" upgrade flow. Anonymous refund request flow (user taps "Request Refund" → daemon handles ZK mechanism invisibly). Download progress with pause/resume controls.

### Phase 32 — Platform Delivery

Tauri/Electron (desktop). React Native / KMP / SwiftUI (mobile). Push notification integration (FCM/APNs/UnifiedPush). Battery profiling for ABR mobile constraints. OTA update UI (optional and mandatory flows with progress). Final security audit and accessibility audit.

---

## 17. Design Principles

Every screen, label, and interaction in this specification follows these rules:

1. **Grandmother Test.** Every term must be understandable by a first-time user within 5 seconds. "Make Creator," "Remove," "Keep," "Seeds Earned," "Free." No protocol jargon surfaces in Default Mode.

2. **Progressive Disclosure.** Default Mode shows the minimum needed to act. Advanced Mode reveals the full technical depth. The user never needs to toggle Advanced Mode to accomplish any core task.

3. **Invisible Until Needed.** The Review Queue doesn't exist visually until someone reports something. The Moderator role doesn't appear in the creation wizard. The Dashboard icon appears only for Hosts. Space Settings is Host-only. Channels appear only in Advanced Mode. Disk pressure alerts appear only when triggered. Features surface only when the user's context requires them.

4. **Single-Action Defaults.** "Make Creator" is one tap + one confirm. "Invite as Creator" is one flow instead of invite-then-promote. Free content is one tap — no authorization. The system favors fewer steps over more configurability.

5. **No Dead Ends.** The creation wizard shows "You can always change this later." The People screen always shows an invite button. The Dashboard always shows quick actions. The empty Home screen always shows how to get started. The Whisper hub shows how to start a conversation. The user is never left wondering "what now?"

6. **Privacy Preserved.** Purchase activity in the Dashboard shows Seed amounts but never buyer names. Content reports use pseudonymous reporter tags, not real identities. All Dashboard data is derived from locally cached DHT state — no new privacy-compromising queries are introduced. Contacts and Spaces are separate trust domains.

7. **Seeds Are the Language.** All prices, balances, earnings, and transaction amounts are denominated in Seeds with the seed icon. Dollar signs, fiat equivalents, and the word "stablecoin" never appear in Default Mode. The Oracle Rate, Collateral Ratio, and VYS are Advanced Mode concepts. Free content shows "Free" — not "0 Seeds."

8. **Consistent Role Language.** Host, Creator, Moderator, Member. These four words are the only role labels a user ever sees. They appear in badges, confirmations, and invitations. They never change based on context.

---

## 18. Version History

### 18.1 Changes in v4.8 (vs v4.7)

| **Change** | **Detail** |
|---|---|
| Renamed to Ochra | All references to "EmuNet" replaced with "Ochra." Deep links changed from emunet:// to ochra://. |
| Home Screen | New section specifying the Home screen: Space card layout, role badges, unread dots, pin-to-top, search at 8+ Spaces, empty state, and Advanced Mode details. |
| Contacts Screen | New section specifying the full Contacts experience: contact list under the "You" tab, contact profiles (deliberately showing no Space membership), add contact via QR/Share Sheet/paste, remove contact with profile key rotation, search at 8+ contacts. |
| Contacts-Spaces Isolation | By design, contact profiles never reveal Space membership and Space member lists never highlight contacts. This prevents social graph reconstruction if a device is compromised. |
| Space Settings | New section specifying Host-only Space configuration: name/icon/description editing, invite permissions, publishing policy, and ownership transfer entry point. |
| Ownership Transfer UI | Full ownership transfer flow: new Host selection, 7-day timelock countdown, cancellation, and completion notifications. |
| Space Info (Member View) | Member-facing information screen: Space description, Host name, member count, Creator Share transparency, creation date, notification toggle, Invite a Friend, and Leave Space action. |
| Leave Space | Leave Space confirmation flow with irreversibility warning. Hosts see "Transfer ownership first" instead. |
| Invite Link Management | Host-only Invite Links view: active invite list with type/uses/expiry, copy link, revoke action, and expired invite reference section. |
| Publishing Policy | Space Settings introduces "Who Can Publish" control: Creators only (default) or Everyone. |

### 18.2 Changes in v4.9 (vs v4.8)

| **Change** | **Detail** |
|---|---|
| Whisper Specification | Whisper messaging UI integrated (previously separate companion document). |
| Seeds Screen | Whisper icon entry point added to header bar. Whisper-originated Seed transfers shown in transaction history. |
| Contact Profile | "Whisper" button added below "Send Seeds." |
| You Tab | Username row added below Display Name. |
| Notifications | Four new events: Whisper session request, Whisper Seed transfer received, missed Whisper attempt, username expiring. |

### 18.3 Changes in v5.5 (vs v4.9)

| **Change** | **Detail** |
|---|---|
| Unified Technical Spec | Single companion document replaces separate Protocol Specification and Whisper Specification. All protocol-internal references now point to the **Ochra v5.5 Unified Technical Specification**. |
| Revenue Split Defaults | Default split changed to 10% Host / 70% Creator / 20% Network (was presented as "Creators keep 80%, Host earns 20%"). Default Mode displays "Creators keep 70%. You earn 10%. The rest supports the network." |
| Revenue Split Proposals | New 30-day timelock system for revenue split changes. Space Settings gains "Propose Change" flow with countdown, member notifications, and cancellation. Space Info shows pending split changes. |
| Free Content | Full support for 0-Seed pricing tiers. UI shows "Free" badge and "Get" button — no payment flow, no blind receipt, no escrow. |
| Content Search | In-Space catalog search via local FTS5 index. `search_catalog` integration with relevance ranking and tag filters. |
| Content Versioning | Successor banner on content items with newer versions. Free update support. |
| Download Management | Pause/resume downloads with detailed progress states (downloading, paused, verifying, complete, failed). Chunk-level progress display. |
| Custom Earning Level | Fourth earning option (⚙️ Custom) with user-defined GB allocation (floor 500 MB, ceiling 80% free space). |
| Bandwidth Disclosure | Setup wizard Step 3 (desktop only) discloses cover traffic bandwidth costs (~30 GB/month background). |
| Channels (Subgroups) | Advanced Mode feature for Hosts to create tiered content access within Spaces. Up to 100 channels, 2 nesting levels. |
| Session Lock | 15-minute inactivity lock with manual lock option in You tab. Whisper sessions continue receiving during lock. |
| Pseudonymous Reports | Content reports use hashed pseudonyms instead of reporter names. Moderators see "Reporter #a3f2" style tags. |
| Typing Indicators | Whisper conversations show typing indicators. |
| Read Receipts | Whisper messages show "Read" status when acknowledged by counterparty. |
| Background Grace UI | Whisper sessions show grace period countdown on app return. State indicators in session list. |
| Handle Change | Atomic username change (old → new with automatic redirect) added to Username management. |
| Anonymous Refunds | User-facing refund request flow added to purchase library and content detail view. |
| Granular Notifications | Per-Space notification settings expanded: purchases, joins, reports independently configurable. Mute-until date picker. |
| OTA Updates | Protocol update notification UI with optional (banner) and mandatory (blocking modal) flows. Progress display. |
| Disk Pressure | Automatic alert on Earn screen when free space <20%. Eviction notification. |
| Circuit Breaker | Advanced Mode amber banner when TWAP Oracle is stale. |
| VYS Claims | Advanced Mode button to claim VYS rewards. |
| Earn Screen | New dedicated section (Section 13) with earnings summary, storage used indicator, and Advanced Mode diagnostics. |
| Escrow Timeout | 60-second Creator timeout with auto-refund and user notification for macro transactions. |
| Build Phase Updates | All build phases updated to reflect v5.5 features: 5-step setup, revenue proposal flows, free content, download management, search, channels, session lock, OTA, pseudonymous reports, typing/read receipts, background grace UI, handle change, refund flow, disk pressure, circuit breaker. |
| Setup Wizard | Changed from 4 steps to 5 steps (added "You're Ready" summary screen). |
| Role Badge on Home | Creator role badge (blue) now shown on Space cards in addition to Host (gold). |
| BLS12-381 | All cryptographic references updated from BN254 to BLS12-381 (Advanced Mode displays). |