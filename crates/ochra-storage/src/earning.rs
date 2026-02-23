//! Earning levels for ABR storage allocation.
//!
//! Nodes participating in ABR storage contribute disk space to the network
//! and are rewarded based on their earning level. Each level specifies
//! a target storage allocation.

use serde::{Deserialize, Serialize};

/// Bytes per gigabyte.
const BYTES_PER_GB: u64 = 1_073_741_824;

/// Storage earning level.
///
/// Determines how much disk space a node allocates for ABR storage.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EarningLevel {
    /// Low allocation: 5 GB.
    Low,
    /// Medium allocation: 25 GB.
    Medium,
    /// High allocation: 100 GB.
    High,
    /// Custom allocation in bytes.
    Custom(u64),
}

/// Get the storage allocation in bytes for a given earning level.
///
/// # Arguments
///
/// * `level` - The earning level.
///
/// # Returns
///
/// The number of bytes allocated for ABR storage.
pub fn get_allocation_bytes(level: &EarningLevel) -> u64 {
    match level {
        EarningLevel::Low => 5 * BYTES_PER_GB,
        EarningLevel::Medium => 25 * BYTES_PER_GB,
        EarningLevel::High => 100 * BYTES_PER_GB,
        EarningLevel::Custom(bytes) => *bytes,
    }
}

/// Get the storage allocation in gigabytes for a given earning level.
///
/// For custom levels, returns the allocation rounded down to whole gigabytes.
pub fn get_allocation_gb(level: &EarningLevel) -> u64 {
    get_allocation_bytes(level) / BYTES_PER_GB
}

/// Get a human-readable name for the earning level.
pub fn level_name(level: &EarningLevel) -> &'static str {
    match level {
        EarningLevel::Low => "Low (5 GB)",
        EarningLevel::Medium => "Medium (25 GB)",
        EarningLevel::High => "High (100 GB)",
        EarningLevel::Custom(_) => "Custom",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_allocation() {
        let bytes = get_allocation_bytes(&EarningLevel::Low);
        assert_eq!(bytes, 5 * BYTES_PER_GB);
        assert_eq!(get_allocation_gb(&EarningLevel::Low), 5);
    }

    #[test]
    fn test_medium_allocation() {
        let bytes = get_allocation_bytes(&EarningLevel::Medium);
        assert_eq!(bytes, 25 * BYTES_PER_GB);
        assert_eq!(get_allocation_gb(&EarningLevel::Medium), 25);
    }

    #[test]
    fn test_high_allocation() {
        let bytes = get_allocation_bytes(&EarningLevel::High);
        assert_eq!(bytes, 100 * BYTES_PER_GB);
        assert_eq!(get_allocation_gb(&EarningLevel::High), 100);
    }

    #[test]
    fn test_custom_allocation() {
        let custom = EarningLevel::Custom(50 * BYTES_PER_GB);
        assert_eq!(get_allocation_bytes(&custom), 50 * BYTES_PER_GB);
        assert_eq!(get_allocation_gb(&custom), 50);
    }

    #[test]
    fn test_custom_allocation_sub_gb() {
        let custom = EarningLevel::Custom(500_000_000);
        assert_eq!(get_allocation_bytes(&custom), 500_000_000);
        assert_eq!(get_allocation_gb(&custom), 0); // Less than 1 GB.
    }

    #[test]
    fn test_level_name() {
        assert_eq!(level_name(&EarningLevel::Low), "Low (5 GB)");
        assert_eq!(level_name(&EarningLevel::Medium), "Medium (25 GB)");
        assert_eq!(level_name(&EarningLevel::High), "High (100 GB)");
        assert_eq!(level_name(&EarningLevel::Custom(42)), "Custom");
    }

    #[test]
    fn test_earning_level_serde_roundtrip() {
        let levels = [
            EarningLevel::Low,
            EarningLevel::Medium,
            EarningLevel::High,
            EarningLevel::Custom(12345),
        ];
        for level in &levels {
            let json = serde_json::to_string(level).expect("serialize");
            let restored: EarningLevel = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(&restored, level);
        }
    }

    #[test]
    fn test_ordering_low_medium_high() {
        let low = get_allocation_bytes(&EarningLevel::Low);
        let medium = get_allocation_bytes(&EarningLevel::Medium);
        let high = get_allocation_bytes(&EarningLevel::High);
        assert!(low < medium);
        assert!(medium < high);
    }
}
