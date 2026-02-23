//! Cover traffic generation using Poisson timing.
//!
//! Cover traffic generates dummy Sphinx packets that are indistinguishable
//! from real packets to an external observer. This is critical for resisting
//! traffic analysis attacks.
//!
//! ## Design (v1: Simplified Single-Tier Poisson)
//!
//! Cover packets:
//! - Are the same fixed size as real packets (8192 bytes)
//! - Use the same encryption layers as real packets
//! - Are generated at randomized intervals drawn from an exponential
//!   distribution (Poisson process)
//! - Are dropped at the final hop (the exit node recognizes them as cover)
//!
//! ## Cover Token
//!
//! A 32-byte cover token is embedded at the start of the payload in cover
//! packets. The exit node checks for this token to identify and silently
//! drop cover traffic.

use std::time::Duration;

use ochra_crypto::blake3;
use tracing::debug;

use crate::{Result, SPHINX_PACKET_SIZE};

/// Default mean interval between cover packets in milliseconds.
pub const DEFAULT_COVER_INTERVAL_MS: u64 = 500;

/// Minimum interval between cover packets in milliseconds.
pub const MIN_COVER_INTERVAL_MS: u64 = 100;

/// Maximum interval between cover packets in milliseconds.
pub const MAX_COVER_INTERVAL_MS: u64 = 5000;

/// Configuration for cover traffic generation.
#[derive(Clone, Debug)]
pub struct CoverTrafficConfig {
    /// Mean interval between cover packets in milliseconds.
    /// Cover packets are sent at exponentially distributed intervals
    /// with this mean (Poisson process).
    pub mean_interval_ms: u64,
    /// Whether cover traffic generation is enabled.
    pub enabled: bool,
}

impl Default for CoverTrafficConfig {
    fn default() -> Self {
        Self {
            mean_interval_ms: DEFAULT_COVER_INTERVAL_MS,
            enabled: true,
        }
    }
}

impl CoverTrafficConfig {
    /// Create a new cover traffic config with the given mean interval.
    ///
    /// The interval is clamped to `[MIN_COVER_INTERVAL_MS, MAX_COVER_INTERVAL_MS]`.
    pub fn new(mean_interval_ms: u64) -> Self {
        Self {
            mean_interval_ms: mean_interval_ms
                .clamp(MIN_COVER_INTERVAL_MS, MAX_COVER_INTERVAL_MS),
            enabled: true,
        }
    }

    /// Create a disabled cover traffic config.
    pub fn disabled() -> Self {
        Self {
            mean_interval_ms: DEFAULT_COVER_INTERVAL_MS,
            enabled: false,
        }
    }
}

/// Generates dummy Sphinx packets at a configured Poisson rate.
///
/// The generator produces fixed-size packets filled with random-looking data
/// that is indistinguishable from real Sphinx traffic to external observers.
pub struct CoverTrafficGenerator {
    /// Configuration for timing and enablement.
    config: CoverTrafficConfig,
    /// Shared secret with the exit node, used for cover token derivation.
    exit_shared_secret: [u8; 32],
}

impl CoverTrafficGenerator {
    /// Create a new cover traffic generator.
    ///
    /// # Arguments
    ///
    /// * `config` - Timing configuration
    /// * `exit_shared_secret` - Shared secret with the exit relay for token derivation
    pub fn new(config: CoverTrafficConfig, exit_shared_secret: [u8; 32]) -> Self {
        Self {
            config,
            exit_shared_secret,
        }
    }

    /// Return whether cover traffic generation is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Compute the next delay before sending a cover packet.
    ///
    /// Draws from an exponential distribution (Poisson inter-arrival time)
    /// using the configured mean interval.
    pub fn next_delay(&self) -> Duration {
        let u: f64 = rand::Rng::gen_range(&mut rand::thread_rng(), f64::EPSILON..(1.0 - f64::EPSILON));
        let delay_ms = next_cover_delay_ms(self.config.mean_interval_ms, u);
        Duration::from_millis(delay_ms)
    }

    /// Generate a dummy Sphinx-sized packet (8192 bytes).
    ///
    /// The packet payload begins with the cover token derived from the
    /// exit shared secret, followed by pseudo-random padding. The entire
    /// packet is indistinguishable from real traffic at the network level.
    pub fn generate_packet(&self) -> Result<Vec<u8>> {
        let cover_token = derive_cover_token(&self.exit_shared_secret);

        let mut packet = vec![0u8; SPHINX_PACKET_SIZE];

        // Fill with pseudo-random data.
        let pad_key = blake3::derive_key("Ochra v1 cover-pad", &cover_token);
        let mut pad = vec![0u8; SPHINX_PACKET_SIZE];
        blake3::hash_xof(&pad_key, &mut pad);
        packet.copy_from_slice(&pad);

        // Place the cover token at a known offset in the payload section.
        // Header occupies the first part; we place the token after a fixed offset.
        let token_offset = 512; // After header area
        if packet.len() >= token_offset + 32 {
            packet[token_offset..token_offset + 32].copy_from_slice(&cover_token);
        }

        debug!("Generated cover traffic packet");
        Ok(packet)
    }

    /// Return the cover token for this generator's exit secret.
    pub fn cover_token(&self) -> [u8; 32] {
        derive_cover_token(&self.exit_shared_secret)
    }

    /// Update the configuration (e.g., change interval).
    pub fn set_config(&mut self, config: CoverTrafficConfig) {
        self.config = config;
    }

    /// Update the exit shared secret (e.g., after circuit rotation).
    pub fn set_exit_secret(&mut self, secret: [u8; 32]) {
        self.exit_shared_secret = secret;
    }
}

/// Derive the cover traffic token from a shared secret.
///
/// The cover token is placed in cover packet payloads so that the exit node
/// can identify and drop them without forwarding.
pub fn derive_cover_token(shared_secret: &[u8; 32]) -> [u8; 32] {
    blake3::derive_key("Ochra v1 cover-traffic-token", shared_secret)
}

/// Check if a decrypted payload is cover traffic by looking for the
/// cover token at the expected offset.
///
/// # Arguments
///
/// * `payload` - The decrypted payload to check
/// * `cover_token` - The expected cover token for this circuit
/// * `token_offset` - Offset within the payload where the token is expected
pub fn is_cover_traffic(payload: &[u8], cover_token: &[u8; 32], token_offset: usize) -> bool {
    if payload.len() < token_offset + 32 {
        return false;
    }
    payload[token_offset..token_offset + 32] == cover_token[..]
}

/// Compute the next cover traffic delay in milliseconds.
///
/// Uses an exponential distribution: given a uniform random value in `[0, 1)`,
/// returns `-mean * ln(1 - u)`, clamped to valid bounds.
///
/// # Arguments
///
/// * `mean_ms` - The mean interval in milliseconds
/// * `uniform_random` - A uniform random value in `[0.0, 1.0)`
pub fn next_cover_delay_ms(mean_ms: u64, uniform_random: f64) -> u64 {
    let u = uniform_random.clamp(f64::EPSILON, 1.0 - f64::EPSILON);
    let delay = -(mean_ms as f64) * (1.0 - u).ln();
    let clamped = delay.clamp(MIN_COVER_INTERVAL_MS as f64, MAX_COVER_INTERVAL_MS as f64);
    clamped as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_cover_token_deterministic() {
        let secret = [0xAAu8; 32];
        let token1 = derive_cover_token(&secret);
        let token2 = derive_cover_token(&secret);
        assert_eq!(token1, token2);
    }

    #[test]
    fn test_different_secrets_different_tokens() {
        let token1 = derive_cover_token(&[0x01u8; 32]);
        let token2 = derive_cover_token(&[0x02u8; 32]);
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_is_cover_traffic() {
        let secret = [0xBBu8; 32];
        let token = derive_cover_token(&secret);

        let mut payload = vec![0u8; 600];
        payload[512..544].copy_from_slice(&token);

        assert!(is_cover_traffic(&payload, &token, 512));
    }

    #[test]
    fn test_is_not_cover_traffic() {
        let token = derive_cover_token(&[0xCCu8; 32]);
        let payload = vec![0u8; 600];
        assert!(!is_cover_traffic(&payload, &token, 512));
    }

    #[test]
    fn test_short_payload_not_cover() {
        let token = derive_cover_token(&[0xDDu8; 32]);
        let payload = vec![0u8; 16];
        assert!(!is_cover_traffic(&payload, &token, 512));
    }

    #[test]
    fn test_cover_config_default() {
        let config = CoverTrafficConfig::default();
        assert_eq!(config.mean_interval_ms, DEFAULT_COVER_INTERVAL_MS);
        assert!(config.enabled);
    }

    #[test]
    fn test_cover_config_clamped() {
        let config = CoverTrafficConfig::new(10);
        assert_eq!(config.mean_interval_ms, MIN_COVER_INTERVAL_MS);

        let config = CoverTrafficConfig::new(100_000);
        assert_eq!(config.mean_interval_ms, MAX_COVER_INTERVAL_MS);
    }

    #[test]
    fn test_cover_config_disabled() {
        let config = CoverTrafficConfig::disabled();
        assert!(!config.enabled);
    }

    #[test]
    fn test_next_cover_delay_positive() {
        let delay = next_cover_delay_ms(500, 0.5);
        assert!(delay >= MIN_COVER_INTERVAL_MS);
        assert!(delay <= MAX_COVER_INTERVAL_MS);
    }

    #[test]
    fn test_next_cover_delay_extreme_values() {
        let delay_low = next_cover_delay_ms(500, 0.01);
        assert!(delay_low >= MIN_COVER_INTERVAL_MS);

        let delay_high = next_cover_delay_ms(500, 0.99);
        assert!(delay_high <= MAX_COVER_INTERVAL_MS);
    }

    #[test]
    fn test_generator_create() {
        let config = CoverTrafficConfig::default();
        let gen = CoverTrafficGenerator::new(config, [0xAAu8; 32]);
        assert!(gen.is_enabled());
    }

    #[test]
    fn test_generator_generate_packet() {
        let config = CoverTrafficConfig::default();
        let gen = CoverTrafficGenerator::new(config, [0xAAu8; 32]);
        let packet = gen.generate_packet().expect("generate packet");
        assert_eq!(packet.len(), SPHINX_PACKET_SIZE);
    }

    #[test]
    fn test_generator_cover_token() {
        let secret = [0xBBu8; 32];
        let config = CoverTrafficConfig::default();
        let gen = CoverTrafficGenerator::new(config, secret);
        assert_eq!(gen.cover_token(), derive_cover_token(&secret));
    }

    #[test]
    fn test_generator_next_delay() {
        let config = CoverTrafficConfig::new(500);
        let gen = CoverTrafficGenerator::new(config, [0u8; 32]);
        let delay = gen.next_delay();
        assert!(delay.as_millis() >= u128::from(MIN_COVER_INTERVAL_MS));
        assert!(delay.as_millis() <= u128::from(MAX_COVER_INTERVAL_MS));
    }

    #[test]
    fn test_generator_update_config() {
        let config = CoverTrafficConfig::default();
        let mut gen = CoverTrafficGenerator::new(config, [0u8; 32]);
        assert!(gen.is_enabled());

        gen.set_config(CoverTrafficConfig::disabled());
        assert!(!gen.is_enabled());
    }

    #[test]
    fn test_generator_update_secret() {
        let config = CoverTrafficConfig::default();
        let mut gen = CoverTrafficGenerator::new(config, [0x01u8; 32]);
        let token1 = gen.cover_token();

        gen.set_exit_secret([0x02u8; 32]);
        let token2 = gen.cover_token();

        assert_ne!(token1, token2);
    }
}
