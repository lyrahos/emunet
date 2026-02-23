//! Configuration & Layout structures (Section 22.9).

use serde::{Deserialize, Serialize};

use crate::ContentHash;

/// Layout configuration for a Space (Section 22.9).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct LayoutConfig {
    pub layout_type: super::space::SpaceTemplate,
    pub sections: Vec<LayoutSection>,
    /// Sandboxed CSS subset.
    pub custom_css: Option<String>,
}

/// Layout section (Section 22.9).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct LayoutSection {
    pub section_type: SectionType,
    pub title: Option<String>,
    pub max_items: Option<u32>,
    pub filter_tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SectionType {
    Hero,
    Grid,
    List,
    Featured,
    Categories,
}

/// Rendered layout for the UI (Section 22.9).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct RenderableLayout {
    pub layout_type: super::space::SpaceTemplate,
    pub rendered_sections: Vec<RenderedSection>,
    pub content_items: Vec<super::content::ContentManifest>,
}

/// Rendered section (Section 22.9).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct RenderedSection {
    pub section_type: SectionType,
    pub title: Option<String>,
    #[ts(type = "string[]")]
    pub content_hashes: Vec<ContentHash>,
}

/// Notification settings (Section 22.9).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct NotificationSettings {
    pub mute_all: bool,
    /// Unix timestamp.
    pub mute_until: Option<u64>,
    pub notify_purchases: bool,
    pub notify_joins: bool,
    pub notify_reports: bool,
}

/// Download progress (Section 22.9).
#[derive(Clone, Debug, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct DownloadProgress {
    #[ts(type = "string")]
    pub content_hash: ContentHash,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub chunks_complete: u32,
    pub chunks_total: u32,
    pub state: DownloadState,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum DownloadState {
    Downloading,
    Paused,
    Verifying,
    Complete,
    Failed,
}
