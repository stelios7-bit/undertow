//! Health scanning. Given the current set of tracked positions, classify them
//! into Hot/Warm/Cold tiers and surface the ones that are liquidatable now.

use undertow_core::{Health, Position};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TierCounts {
    pub hot: usize,
    pub warm: usize,
    pub cold: usize,
}

pub struct HealthScanner;

impl HealthScanner {
    /// Positions that are liquidatable right now (health factor < 1.0).
    pub fn liquidatable(positions: &[Position]) -> Vec<&Position> {
        positions.iter().filter(|p| p.is_liquidatable()).collect()
    }

    /// Count positions per health tier — feeds scan-rate metrics.
    pub fn tier_counts(positions: &[Position]) -> TierCounts {
        let mut counts = TierCounts::default();
        for p in positions {
            match p.health() {
                Health::Hot => counts.hot += 1,
                Health::Warm => counts.warm += 1,
                Health::Cold => counts.cold += 1,
            }
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;

    fn pos(collateral: f64, debt: f64) -> Position {
        Position {
            protocol: "test".into(),
            borrower: Address::ZERO,
            collateral_token: Address::ZERO,
            debt_token: Address::ZERO,
            collateral_usd: collateral,
            debt_usd: debt,
            liquidation_threshold: 0.8,
        }
    }

    #[test]
    fn finds_liquidatable_and_counts_tiers() {
        let positions = vec![
            pos(1000.0, 500.0),  // hf 1.6 -> cold
            pos(1000.0, 900.0),  // hf 0.888 -> hot
            pos(1000.0, 770.0),  // hf 1.038 -> warm
            pos(1000.0, 1000.0), // hf 0.8 -> hot
        ];
        let liq = HealthScanner::liquidatable(&positions);
        assert_eq!(liq.len(), 2);

        let counts = HealthScanner::tier_counts(&positions);
        assert_eq!(
            counts,
            TierCounts {
                hot: 2,
                warm: 1,
                cold: 1
            }
        );
    }

    #[test]
    fn empty_is_all_zero() {
        assert_eq!(HealthScanner::tier_counts(&[]), TierCounts::default());
        assert!(HealthScanner::liquidatable(&[]).is_empty());
    }
}
