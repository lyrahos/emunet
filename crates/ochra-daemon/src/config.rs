//! Configuration file management (Section 33).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Complete daemon configuration (Section 33).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Network settings.
    #[serde(default)]
    pub network: NetworkConfig,
    /// Storage settings.
    #[serde(default)]
    pub storage: StorageConfig,
    /// Identity settings.
    #[serde(default)]
    pub identity: IdentityConfig,
    /// Privacy settings.
    #[serde(default)]
    pub privacy: PrivacyConfig,
    /// Advanced settings.
    #[serde(default)]
    pub advanced: AdvancedConfig,
}

/// Network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// 0 = OS-assigned ephemeral port.
    #[serde(default)]
    pub listen_port: u16,
    /// Bootstrap seed nodes.
    #[serde(default = "default_bootstrap_nodes")]
    pub bootstrap_nodes: Vec<String>,
    /// Maximum concurrent QUIC connections.
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// Participate as a relay for others.
    #[serde(default = "default_true")]
    pub relay_enabled: bool,
}

/// Storage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory. Empty = platform default.
    #[serde(default)]
    pub data_dir: String,
    /// Earning level: "low" | "medium" | "high" | "custom".
    #[serde(default = "default_earning_level")]
    pub earning_level: String,
    /// Custom allocation in GB (used when earning_level = "custom").
    #[serde(default = "default_custom_allocation")]
    pub custom_allocation_gb: u32,
    /// Earn While I Sleep (2-8 AM).
    #[serde(default = "default_true")]
    pub smart_night_mode: bool,
    /// Chunk storage path. Empty = $data_dir/chunks/.
    #[serde(default)]
    pub chunk_storage_path: String,
}

/// Identity configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Session timeout in minutes.
    #[serde(default = "default_session_timeout")]
    pub session_timeout_minutes: u32,
    /// Biometric authentication enabled.
    #[serde(default)]
    pub biometric_enabled: bool,
}

/// Privacy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    /// Cover traffic enabled. Strongly recommended.
    #[serde(default = "default_true")]
    pub cover_traffic_enabled: bool,
    /// Enforce >= 2 countries per circuit.
    #[serde(default = "default_true")]
    pub relay_country_diversity: bool,
}

/// Advanced configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedConfig {
    /// Show fiat equivalents, CR, TWAP in UI.
    #[serde(default)]
    pub advanced_mode: bool,
    /// Log level: "debug" | "info" | "warn" | "error".
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Log file path. Empty = stderr.
    #[serde(default)]
    pub log_file: String,
}

// Default value functions

fn default_bootstrap_nodes() -> Vec<String> {
    vec![
        "198.51.100.1:4433".to_string(),
        "198.51.100.2:4433".to_string(),
    ]
}

fn default_max_connections() -> u32 {
    256
}

fn default_true() -> bool {
    true
}

fn default_earning_level() -> String {
    "medium".to_string()
}

fn default_custom_allocation() -> u32 {
    25
}

fn default_session_timeout() -> u32 {
    15
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: 0,
            bootstrap_nodes: default_bootstrap_nodes(),
            max_connections: default_max_connections(),
            relay_enabled: true,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: String::new(),
            earning_level: default_earning_level(),
            custom_allocation_gb: default_custom_allocation(),
            smart_night_mode: true,
            chunk_storage_path: String::new(),
        }
    }
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            session_timeout_minutes: default_session_timeout(),
            biometric_enabled: false,
        }
    }
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            cover_traffic_enabled: true,
            relay_country_diversity: true,
        }
    }
}

impl Default for AdvancedConfig {
    fn default() -> Self {
        Self {
            advanced_mode: false,
            log_level: default_log_level(),
            log_file: String::new(),
        }
    }
}

impl DaemonConfig {
    /// Load configuration from the default config file location.
    ///
    /// Falls back to defaults if file does not exist.
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: DaemonConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Get the data directory path.
    pub fn data_dir(&self) -> PathBuf {
        if self.storage.data_dir.is_empty() {
            Self::default_data_dir()
        } else {
            PathBuf::from(&self.storage.data_dir)
        }
    }

    /// Get the config file path.
    fn config_path() -> PathBuf {
        // Check env var override first
        if let Ok(dir) = std::env::var("OCHRA_DATA_DIR") {
            return PathBuf::from(dir).join("config.toml");
        }
        Self::default_data_dir().join("config.toml")
    }

    /// Platform-specific default data directory.
    fn default_data_dir() -> PathBuf {
        if let Ok(dir) = std::env::var("OCHRA_DATA_DIR") {
            return PathBuf::from(dir);
        }
        #[cfg(target_os = "macos")]
        {
            dirs_fallback("Library/Application Support/Ochra")
        }
        #[cfg(target_os = "linux")]
        {
            dirs_fallback(".ochra")
        }
        #[cfg(target_os = "windows")]
        {
            dirs_fallback("Ochra")
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            dirs_fallback(".ochra")
        }
    }
}

/// Fallback home directory resolution.
fn dirs_fallback(subpath: &str) -> PathBuf {
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join(subpath))
        .unwrap_or_else(|_| PathBuf::from("/tmp/ochra"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DaemonConfig::default();
        assert_eq!(config.network.listen_port, 0);
        assert!(config.network.relay_enabled);
        assert_eq!(config.storage.earning_level, "medium");
        assert_eq!(config.identity.session_timeout_minutes, 15);
        assert!(config.privacy.cover_traffic_enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = DaemonConfig::default();
        let toml_str = toml::to_string(&config).expect("serialize");
        let _parsed: DaemonConfig = toml::from_str(&toml_str).expect("parse");
    }
}
