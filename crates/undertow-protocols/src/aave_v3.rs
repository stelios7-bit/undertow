//! Aave V3 adapter.
//!
//! Aave does the aggregation on-chain: `getUserAccountData` returns the
//! borrower's total collateral and debt already valued in the protocol's base
//! currency (USD, 8 decimals) plus the blended liquidation threshold. So the
//! adapter is a thin mapping into the core `Position` — no per-market walk or
//! external oracle needed.

use crate::util::build_position;
use alloy::providers::Provider;
use alloy::sol;
use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use undertow_core::{LendingProtocol, Position};

/// Aave reports collateral/debt in USD with 8 decimals.
const AAVE_BASE: f64 = 1e8;
/// Basis-point denominator for thresholds/ltv.
const BPS: f64 = 1e4;

sol! {
    #[sol(rpc)]
    interface IPool {
        function getUserAccountData(address user) external view returns (
            uint256 totalCollateralBase,
            uint256 totalDebtBase,
            uint256 availableBorrowsBase,
            uint256 currentLiquidationThreshold,
            uint256 ltv,
            uint256 healthFactor
        );
        function getReservesList() external view returns (address[] memory);
    }
}

sol! {
    #[sol(rpc)]
    interface IPoolDataProvider {
        function getReserveConfigurationData(address asset) external view returns (
            uint256 decimals,
            uint256 ltv,
            uint256 liquidationThreshold,
            uint256 liquidationBonus,
            uint256 reserveFactor,
            bool usageAsCollateralEnabled,
            bool borrowingEnabled,
            bool stableBorrowRateEnabled,
            bool isActive,
            bool isFrozen
        );
    }
}

/// USD value from Aave's 8-decimal base currency.
pub fn base_to_usd(raw: U256) -> f64 {
    raw.to_string().parse::<f64>().unwrap_or(0.0) / AAVE_BASE
}

/// Basis-point figure (e.g. `8000`) to a ratio (`0.8`).
pub fn bps_to_ratio(bps: U256) -> f64 {
    bps.to_string().parse::<f64>().unwrap_or(0.0) / BPS
}

pub struct AaveV3Adapter<P> {
    provider: P,
    pool: Address,
    data_provider: Address,
    close_factor: f64,
    liquidation_bonus: f64,
}

impl<P: Provider + Clone> AaveV3Adapter<P> {
    pub fn new(provider: P, pool: Address, data_provider: Address) -> Self {
        Self {
            provider,
            pool,
            data_provider,
            close_factor: 0.5,
            liquidation_bonus: 0.05,
        }
    }

    /// All reserve (asset) addresses listed by the pool.
    pub async fn reserves(&self) -> anyhow::Result<Vec<Address>> {
        let pool = IPool::new(self.pool, self.provider.clone());
        Ok(pool.getReservesList().call().await?._0)
    }

    /// `(decimals, liquidation_threshold_ratio, liquidation_bonus_ratio)` for a reserve.
    pub async fn reserve_config(&self, asset: Address) -> anyhow::Result<(u8, f64, f64)> {
        let dp = IPoolDataProvider::new(self.data_provider, self.provider.clone());
        let c = dp.getReserveConfigurationData(asset).call().await?;
        let decimals: u8 = c.decimals.to_string().parse().unwrap_or(18);
        // liquidationBonus is encoded as 1e4-based with 10000 = no bonus
        let bonus = (bps_to_ratio(c.liquidationBonus) - 1.0).max(0.0);
        Ok((decimals, bps_to_ratio(c.liquidationThreshold), bonus))
    }
}

#[async_trait]
impl<P: Provider + Clone + Send + Sync> LendingProtocol for AaveV3Adapter<P> {
    fn name(&self) -> &str {
        "aave-v3"
    }

    fn close_factor(&self) -> f64 {
        self.close_factor
    }

    fn liquidation_bonus(&self) -> f64 {
        self.liquidation_bonus
    }

    async fn fetch_position(&self, borrower: Address) -> anyhow::Result<Option<Position>> {
        let pool = IPool::new(self.pool, self.provider.clone());
        let d = pool.getUserAccountData(borrower).call().await?;

        // No debt -> nothing to liquidate.
        if d.totalDebtBase.is_zero() {
            return Ok(None);
        }

        Ok(Some(build_position(
            "aave-v3",
            borrower,
            Address::ZERO,
            Address::ZERO,
            base_to_usd(d.totalCollateralBase),
            base_to_usd(d.totalDebtBase),
            bps_to_ratio(d.currentLiquidationThreshold),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_currency_conversion() {
        // 1500 USD in 8-decimal base
        let raw = U256::from(1500u64) * U256::from(10u64).pow(U256::from(8u64));
        assert!((base_to_usd(raw) - 1500.0).abs() < 1e-6);
    }

    #[test]
    fn bps_conversion() {
        assert!((bps_to_ratio(U256::from(8000u64)) - 0.8).abs() < 1e-9);
        assert!((bps_to_ratio(U256::from(10_000u64)) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn maps_to_liquidatable_position() {
        // collateral 1000 @ 0.8 threshold, debt 850 -> hf 0.94 < 1
        let p = build_position(
            "aave-v3",
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            1000.0,
            850.0,
            0.8,
        );
        assert!(p.is_liquidatable());
    }
}
