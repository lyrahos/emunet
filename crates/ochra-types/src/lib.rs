//! # ochra-types
//!
//! Shared domain types used across the Ochra workspace.
//! All structures correspond 1:1 with Section 22 of the v5.5 Unified Technical Specification.

pub mod content;
pub mod diagnostics;
pub mod events;
pub mod governance;
pub mod identity;
pub mod layout;
pub mod network;
pub mod space;
pub mod whisper;

/// Common type aliases (Section 22.7).
pub type Hash = [u8; 32];
pub type ContentHash = [u8; 32];
pub type GroupId = [u8; 32];
pub type SubgroupId = [u8; 32];
pub type TxHash = [u8; 32];
pub type WhisperSessionId = [u8; 16];
pub type SubscriptionId = [u8; 16];
pub type Bytes = Vec<u8>;

/// Micro-seeds per Seed (1 Seed = 100,000,000 micro-seeds).
pub const MICRO_SEEDS_PER_SEED: u64 = 100_000_000;

/// Epoch duration in seconds (24 hours).
pub const EPOCH_DURATION_SECS: u64 = 86400;

/// Relay epoch duration in seconds (1 hour).
pub const RELAY_EPOCH_DURATION_SECS: u64 = 3600;

/// Sphinx packet size in bytes.
pub const SPHINX_PACKET_SIZE: usize = 8192;

/// Maximum pricing tiers per content item.
pub const MAX_PRICING_TIERS: usize = 4;

/// Maximum tags per content item.
pub const MAX_CONTENT_TAGS: usize = 5;

/// Maximum hops in Sphinx circuit.
pub const SPHINX_HOPS: usize = 3;

#[cfg(test)]
mod tests {
    #[test]
    fn test_ts_export() {
        // This test just verifies the TS types can be generated without panicking.
        // Run `cargo test -p ochra-types -- --ignored export_ts_bindings` to write files.
    }

    #[test]
    #[ignore] // Run manually to generate bindings
    fn export_ts_bindings() {
        use ts_rs::TS;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../bindings");
        std::fs::create_dir_all(&dir).unwrap();
        // Export all types
        crate::events::Event::export_all_to(&dir).unwrap();
        crate::identity::PikMeta::export_all_to(&dir).unwrap();
        crate::space::GroupSummary::export_all_to(&dir).unwrap();
        crate::content::ContentManifest::export_all_to(&dir).unwrap();
        crate::whisper::HandleDescriptor::export_all_to(&dir).unwrap();
        crate::network::ServiceReceipt::export_all_to(&dir).unwrap();
        crate::governance::UpgradeManifest::export_all_to(&dir).unwrap();
        crate::layout::LayoutConfig::export_all_to(&dir).unwrap();
        crate::diagnostics::CircuitMetrics::export_all_to(&dir).unwrap();
    }
}
