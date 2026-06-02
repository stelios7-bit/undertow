//! Profit model for a candidate liquidation.
//!
//! A liquidation is worth executing only if the value of the seized
//! collateral (including the liquidation bonus) exceeds everything it costs to
//! capture: the debt repaid, gas, the flash-loan fee, and expected DEX
//! slippage on swapping the collateral back to the debt asset.

/// All USD-denominated inputs to a profit estimate.
#[derive(Debug, Clone)]
pub struct ProfitInputs {
    /// Collateral seized, including the liquidation bonus.
    pub seized_collateral_usd: f64,
    /// Debt repaid to the protocol.
    pub repaid_debt_usd: f64,
    /// Gas cost of the liquidation transaction.
    pub gas_cost_usd: f64,
    /// Flash-loan premium.
    pub flash_fee_usd: f64,
    /// Expected slippage swapping seized collateral back to the debt token.
    pub swap_slippage_usd: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfitEstimate {
    pub net_usd: f64,
}

impl ProfitInputs {
    pub fn estimate(&self) -> ProfitEstimate {
        let net = self.seized_collateral_usd
            - self.repaid_debt_usd
            - self.gas_cost_usd
            - self.flash_fee_usd
            - self.swap_slippage_usd;
        ProfitEstimate { net_usd: net }
    }
}

impl ProfitEstimate {
    /// Whether the net profit clears the configured minimum.
    pub fn is_profitable(&self, min_profit_usd: f64) -> bool {
        self.net_usd >= min_profit_usd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profitable_liquidation() {
        let inputs = ProfitInputs {
            seized_collateral_usd: 1080.0, // 1000 debt + 8% bonus
            repaid_debt_usd: 1000.0,
            gas_cost_usd: 5.0,
            flash_fee_usd: 0.5,
            swap_slippage_usd: 3.0,
        };
        let est = inputs.estimate();
        assert!((est.net_usd - 71.5).abs() < 1e-9);
        assert!(est.is_profitable(50.0));
        assert!(!est.is_profitable(100.0));
    }

    #[test]
    fn unprofitable_when_costs_exceed_bonus() {
        let inputs = ProfitInputs {
            seized_collateral_usd: 1010.0,
            repaid_debt_usd: 1000.0,
            gas_cost_usd: 20.0,
            flash_fee_usd: 0.5,
            swap_slippage_usd: 5.0,
        };
        assert!(!inputs.estimate().is_profitable(0.0));
    }
}
