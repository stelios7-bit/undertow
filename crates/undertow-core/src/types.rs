//! Shared domain types: borrower positions and their health classification.

use alloy_primitives::Address;

/// Coarse health bucket used to prioritise scanning work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    /// Health factor below 1.0 — liquidatable now.
    Hot,
    /// Close to the threshold; watch closely.
    Warm,
    /// Comfortably collateralised.
    Cold,
}

impl Health {
    /// Bucket a health factor. `< 1.0` is Hot, `< 1.05` is Warm, else Cold.
    pub fn classify(health_factor: f64) -> Self {
        if health_factor < 1.0 {
            Health::Hot
        } else if health_factor < 1.05 {
            Health::Warm
        } else {
            Health::Cold
        }
    }
}

/// A borrower's position on a lending protocol, valued in USD.
#[derive(Debug, Clone)]
pub struct Position {
    pub protocol: String,
    pub borrower: Address,
    pub collateral_token: Address,
    pub debt_token: Address,
    pub collateral_usd: f64,
    pub debt_usd: f64,
    /// Weighted liquidation threshold for the collateral (e.g. 0.8 = 80%).
    pub liquidation_threshold: f64,
}

impl Position {
    /// Health factor = (collateral_usd × liquidation_threshold) / debt_usd.
    /// A position with no debt is infinitely healthy.
    pub fn health_factor(&self) -> f64 {
        if self.debt_usd <= 0.0 {
            return f64::INFINITY;
        }
        (self.collateral_usd * self.liquidation_threshold) / self.debt_usd
    }

    pub fn health(&self) -> Health {
        Health::classify(self.health_factor())
    }

    pub fn is_liquidatable(&self) -> bool {
        self.health_factor() < 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(collateral: f64, debt: f64, threshold: f64) -> Position {
        Position {
            protocol: "test".into(),
            borrower: Address::ZERO,
            collateral_token: Address::ZERO,
            debt_token: Address::ZERO,
            collateral_usd: collateral,
            debt_usd: debt,
            liquidation_threshold: threshold,
        }
    }

    #[test]
    fn healthy_position_is_not_liquidatable() {
        let p = pos(1000.0, 500.0, 0.8); // hf = 1.6
        assert!(p.health_factor() > 1.0);
        assert!(!p.is_liquidatable());
        assert_eq!(p.health(), Health::Cold);
    }

    #[test]
    fn underwater_position_is_liquidatable() {
        let p = pos(1000.0, 900.0, 0.8); // hf = 0.888
        assert!(p.is_liquidatable());
        assert_eq!(p.health(), Health::Hot);
    }

    #[test]
    fn no_debt_is_infinitely_healthy() {
        let p = pos(1000.0, 0.0, 0.8);
        assert!(p.health_factor().is_infinite());
        assert!(!p.is_liquidatable());
    }

    #[test]
    fn classify_buckets() {
        assert_eq!(Health::classify(0.9), Health::Hot);
        assert_eq!(Health::classify(1.02), Health::Warm);
        assert_eq!(Health::classify(2.0), Health::Cold);
    }
}
