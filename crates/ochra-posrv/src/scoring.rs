//! PoSrv scoring formula.
//!
//! Computes the composite Proof-of-Service score for a relay node.
//! The score is a weighted average of four components:
//!
//! ```text
//! score = gbs_served_normalized * 0.4
//!       + uptime_fraction * 0.3
//!       + zkpor_pass_rate * 0.2
//!       + trust_weight * 0.1
//! ```
//!
//! GB served is normalized to [0, 1] using a sigmoid function:
//! `1 / (1 + exp(-gbs / 100))`.

use serde::{Deserialize, Serialize};

use crate::{PoSrvError, Result};

/// Weight for GBs served component.
pub const W_GBS_SERVED: f64 = 0.4;

/// Weight for uptime component.
pub const W_UPTIME: f64 = 0.3;

/// Weight for zk-PoR pass rate component.
pub const W_ZK_POR: f64 = 0.2;

/// Weight for SybilGuard trust weight component.
pub const W_TRUST: f64 = 0.1;

/// Sigmoid divisor for GBs served normalization.
pub const SIGMOID_DIVISOR: f64 = 100.0;

/// Minimum PoSrv score required for quorum eligibility.
pub const QUORUM_THRESHOLD: f64 = 0.60;

/// Input metrics for PoSrv score calculation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoSrvInput {
    /// Gigabytes served during the scoring period.
    pub gbs_served: f64,
    /// Uptime fraction in [0.0, 1.0].
    pub uptime_fraction: f64,
    /// zk-PoR challenge pass rate in [0.0, 1.0].
    pub zkpor_pass_rate: f64,
    /// SybilGuard trust weight in [0.0, 1.0].
    pub trust_weight: f64,
}

/// Breakdown of the PoSrv composite score.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PoSrvBreakdown {
    /// Normalized GBs served score [0.0, 1.0] (after sigmoid).
    pub gbs_served_normalized: f64,
    /// Uptime fraction [0.0, 1.0].
    pub uptime_score: f64,
    /// zk-PoR pass rate [0.0, 1.0].
    pub zkpor_score: f64,
    /// Trust weight [0.0, 1.0].
    pub trust_score: f64,
    /// Final composite score [0.0, 1.0].
    pub composite: f64,
    /// Whether this node meets the quorum eligibility threshold.
    pub quorum_eligible: bool,
}

/// Compute the PoSrv composite score from input metrics.
///
/// # Arguments
///
/// * `input` - The raw input metrics.
///
/// # Returns
///
/// The composite PoSrv score as an `f64` in [0.0, 1.0].
pub fn compute_posrv(input: &PoSrvInput) -> Result<f64> {
    let breakdown = compute_posrv_breakdown(input)?;
    Ok(breakdown.composite)
}

/// Compute the full PoSrv score breakdown.
///
/// # Arguments
///
/// * `input` - The raw input metrics.
///
/// # Returns
///
/// A [`PoSrvBreakdown`] with individual component scores and the composite.
pub fn compute_posrv_breakdown(input: &PoSrvInput) -> Result<PoSrvBreakdown> {
    // Validate input ranges.
    validate_fraction("uptime_fraction", input.uptime_fraction)?;
    validate_fraction("zkpor_pass_rate", input.zkpor_pass_rate)?;
    validate_fraction("trust_weight", input.trust_weight)?;

    if input.gbs_served < 0.0 {
        return Err(PoSrvError::OutOfRange {
            name: "gbs_served",
            value: input.gbs_served,
        });
    }

    // Normalize GBs served using sigmoid: 1 / (1 + exp(-gbs / 100)).
    let gbs_served_normalized = sigmoid(input.gbs_served / SIGMOID_DIVISOR);

    // Weighted sum.
    let composite = W_GBS_SERVED * gbs_served_normalized
        + W_UPTIME * input.uptime_fraction
        + W_ZK_POR * input.zkpor_pass_rate
        + W_TRUST * input.trust_weight;

    let quorum_eligible = composite >= QUORUM_THRESHOLD;

    Ok(PoSrvBreakdown {
        gbs_served_normalized,
        uptime_score: input.uptime_fraction,
        zkpor_score: input.zkpor_pass_rate,
        trust_score: input.trust_weight,
        composite,
        quorum_eligible,
    })
}

/// Rank a set of nodes by their PoSrv composite scores (descending).
///
/// Returns indices sorted by composite score, highest first.
pub fn rank_nodes(scores: &[PoSrvBreakdown]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..scores.len()).collect();
    indices.sort_by(|&a, &b| {
        scores[b]
            .composite
            .partial_cmp(&scores[a].composite)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    indices
}

/// Standard sigmoid function: 1 / (1 + exp(-x)).
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Validate that a value is in [0.0, 1.0].
fn validate_fraction(name: &'static str, value: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&value) {
        return Err(PoSrvError::OutOfRange { name, value });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weights_sum_to_one() {
        let total = W_GBS_SERVED + W_UPTIME + W_ZK_POR + W_TRUST;
        assert!((total - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sigmoid_at_zero() {
        let s = sigmoid(0.0);
        assert!((s - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_sigmoid_monotonic() {
        let s1 = sigmoid(-1.0);
        let s2 = sigmoid(0.0);
        let s3 = sigmoid(1.0);
        assert!(s1 < s2);
        assert!(s2 < s3);
    }

    #[test]
    fn test_sigmoid_bounds() {
        let low = sigmoid(-10.0);
        let high = sigmoid(10.0);
        assert!(low > 0.0);
        assert!(high < 1.0);
        assert!(low < 0.001);
        assert!(high > 0.999);
    }

    #[test]
    fn test_perfect_score() {
        let input = PoSrvInput {
            gbs_served: 10000.0, // Very high: sigmoid(100) ~ 1.0
            uptime_fraction: 1.0,
            zkpor_pass_rate: 1.0,
            trust_weight: 1.0,
        };
        let breakdown = compute_posrv_breakdown(&input).expect("compute");
        // Sigmoid never reaches exactly 1.0, but composite should be close.
        assert!(breakdown.composite > 0.99);
        assert!(breakdown.quorum_eligible);
    }

    #[test]
    fn test_zero_gbs_served() {
        let input = PoSrvInput {
            gbs_served: 0.0,
            uptime_fraction: 1.0,
            zkpor_pass_rate: 1.0,
            trust_weight: 1.0,
        };
        let breakdown = compute_posrv_breakdown(&input).expect("compute");
        // sigmoid(0) = 0.5, so gbs_served_normalized = 0.5
        assert!((breakdown.gbs_served_normalized - 0.5).abs() < f64::EPSILON);
        // composite = 0.4*0.5 + 0.3*1 + 0.2*1 + 0.1*1 = 0.2 + 0.3 + 0.2 + 0.1 = 0.8
        assert!((breakdown.composite - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_all_zero_metrics() {
        let input = PoSrvInput {
            gbs_served: 0.0,
            uptime_fraction: 0.0,
            zkpor_pass_rate: 0.0,
            trust_weight: 0.0,
        };
        let breakdown = compute_posrv_breakdown(&input).expect("compute");
        // sigmoid(0) = 0.5, so composite = 0.4*0.5 = 0.2
        assert!((breakdown.composite - 0.2).abs() < 0.001);
        assert!(!breakdown.quorum_eligible);
    }

    #[test]
    fn test_quorum_threshold() {
        let input = PoSrvInput {
            gbs_served: 200.0, // sigmoid(2) ~ 0.88
            uptime_fraction: 0.8,
            zkpor_pass_rate: 0.9,
            trust_weight: 0.7,
        };
        let breakdown = compute_posrv_breakdown(&input).expect("compute");
        assert!(breakdown.composite > QUORUM_THRESHOLD);
        assert!(breakdown.quorum_eligible);
    }

    #[test]
    fn test_invalid_uptime_rejected() {
        let input = PoSrvInput {
            gbs_served: 100.0,
            uptime_fraction: 1.5,
            zkpor_pass_rate: 0.5,
            trust_weight: 0.5,
        };
        assert!(compute_posrv_breakdown(&input).is_err());
    }

    #[test]
    fn test_invalid_zkpor_rejected() {
        let input = PoSrvInput {
            gbs_served: 100.0,
            uptime_fraction: 0.5,
            zkpor_pass_rate: -0.1,
            trust_weight: 0.5,
        };
        assert!(compute_posrv_breakdown(&input).is_err());
    }

    #[test]
    fn test_invalid_trust_rejected() {
        let input = PoSrvInput {
            gbs_served: 100.0,
            uptime_fraction: 0.5,
            zkpor_pass_rate: 0.5,
            trust_weight: 2.0,
        };
        assert!(compute_posrv_breakdown(&input).is_err());
    }

    #[test]
    fn test_negative_gbs_rejected() {
        let input = PoSrvInput {
            gbs_served: -10.0,
            uptime_fraction: 0.5,
            zkpor_pass_rate: 0.5,
            trust_weight: 0.5,
        };
        assert!(compute_posrv_breakdown(&input).is_err());
    }

    #[test]
    fn test_compute_posrv_matches_breakdown() {
        let input = PoSrvInput {
            gbs_served: 150.0,
            uptime_fraction: 0.75,
            zkpor_pass_rate: 0.85,
            trust_weight: 0.6,
        };
        let score = compute_posrv(&input).expect("compute");
        let breakdown = compute_posrv_breakdown(&input).expect("breakdown");
        assert!((score - breakdown.composite).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rank_nodes() {
        let scores = vec![
            PoSrvBreakdown {
                gbs_served_normalized: 0.5,
                uptime_score: 0.5,
                zkpor_score: 0.5,
                trust_score: 0.5,
                composite: 0.5,
                quorum_eligible: false,
            },
            PoSrvBreakdown {
                gbs_served_normalized: 1.0,
                uptime_score: 1.0,
                zkpor_score: 1.0,
                trust_score: 1.0,
                composite: 1.0,
                quorum_eligible: true,
            },
            PoSrvBreakdown {
                gbs_served_normalized: 0.3,
                uptime_score: 0.3,
                zkpor_score: 0.3,
                trust_score: 0.3,
                composite: 0.3,
                quorum_eligible: false,
            },
        ];
        let ranked = rank_nodes(&scores);
        assert_eq!(ranked, vec![1, 0, 2]);
    }
}
