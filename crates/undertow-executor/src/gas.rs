//! EIP-1559 fee pricing. The executor turns the chain's current base fee into
//! a `maxFeePerGas` / `maxPriorityFeePerGas` pair, and refuses to submit when
//! the base fee blows past the configured ceiling (a failed liquidation only
//! costs gas, but a runaway gas market can still bleed the hot wallet).

const GWEI: f64 = 1e9;

#[derive(Debug, Clone, Copy)]
pub struct FeeConfig {
    /// Hard ceiling on base fee (gwei) above which we don't submit.
    pub max_base_fee_gwei: f64,
    /// Tip paid to the proposer (gwei).
    pub priority_fee_gwei: f64,
    /// Headroom multiplier applied to base fee for `maxFeePerGas`.
    pub base_fee_multiplier: f64,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            max_base_fee_gwei: 10.0,
            priority_fee_gwei: 1.0,
            base_fee_multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fees {
    pub max_fee_per_gas_wei: u128,
    pub max_priority_fee_per_gas_wei: u128,
}

pub fn gwei_to_wei(gwei: f64) -> u128 {
    (gwei * GWEI).max(0.0) as u128
}

impl FeeConfig {
    /// True when `base_fee_wei` exceeds the configured ceiling.
    pub fn exceeds_ceiling(&self, base_fee_wei: u128) -> bool {
        base_fee_wei > gwei_to_wei(self.max_base_fee_gwei)
    }

    /// EIP-1559 fees for the current base fee:
    /// `maxFee = base × multiplier + priority`, `maxPriority = priority`.
    pub fn fees_for(&self, base_fee_wei: u128) -> Fees {
        let priority = gwei_to_wei(self.priority_fee_gwei);
        let bumped = (base_fee_wei as f64 * self.base_fee_multiplier) as u128;
        Fees {
            max_fee_per_gas_wei: bumped.saturating_add(priority),
            max_priority_fee_per_gas_wei: priority,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gwei_conversion() {
        assert_eq!(gwei_to_wei(1.0), 1_000_000_000);
        assert_eq!(gwei_to_wei(0.0), 0);
    }

    #[test]
    fn fees_apply_multiplier_and_tip() {
        let cfg = FeeConfig {
            max_base_fee_gwei: 100.0,
            priority_fee_gwei: 1.0,
            base_fee_multiplier: 2.0,
        };
        let base = gwei_to_wei(5.0); // 5 gwei
        let fees = cfg.fees_for(base);
        // 5*2 + 1 = 11 gwei max fee, 1 gwei priority
        assert_eq!(fees.max_fee_per_gas_wei, gwei_to_wei(11.0));
        assert_eq!(fees.max_priority_fee_per_gas_wei, gwei_to_wei(1.0));
    }

    #[test]
    fn ceiling_check() {
        let cfg = FeeConfig {
            max_base_fee_gwei: 10.0,
            ..FeeConfig::default()
        };
        assert!(!cfg.exceeds_ceiling(gwei_to_wei(9.0)));
        assert!(cfg.exceeds_ceiling(gwei_to_wei(11.0)));
    }
}
