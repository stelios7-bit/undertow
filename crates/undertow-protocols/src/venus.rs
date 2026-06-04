//! Venus adapter (Compound-style money market on BNB Chain).
//!
//! Health on a Compound-fork is aggregated across every market the borrower
//! entered: effective collateral is the sum of each market's USD value scaled
//! by its collateral factor, and the health factor is that weighted collateral
//! over total debt. The arithmetic helpers are pure and unit-tested; the
//! adapter wires them to live `getAccountSnapshot` reads.

use crate::util::{build_position, token_amount_to_f64};
use alloy::providers::Provider;
use alloy::sol;
use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use std::collections::HashMap;
use undertow_core::{LendingProtocol, Position};
use undertow_scanner::PriceOracle;

/// Compound/Venus fixed-point scale (1e18).
const MANTISSA: f64 = 1e18;

sol! {
    #[sol(rpc)]
    interface IComptroller {
        function getAssetsIn(address account) external view returns (address[] memory);
        function getAllMarkets() external view returns (address[] memory);
        function markets(address vToken) external view returns (bool isListed, uint256 collateralFactorMantissa);
    }
}

sol! {
    #[sol(rpc)]
    interface IVToken {
        function getAccountSnapshot(address account) external view returns (
            uint256 err,
            uint256 vTokenBalance,
            uint256 borrowBalance,
            uint256 exchangeRateMantissa
        );
        function underlying() external view returns (address);
    }
}

/// A `1e18`-scaled mantissa (collateral factor, etc.) as a float.
pub fn mantissa_to_f64(m: U256) -> f64 {
    m.to_string().parse::<f64>().unwrap_or(0.0) / MANTISSA
}

/// Underlying base-unit balance behind a vToken position:
/// `vTokenBalance × exchangeRate / 1e18`.
pub fn vtoken_underlying(vtoken_balance: U256, exchange_rate_mantissa: U256) -> f64 {
    let balance: f64 = vtoken_balance.to_string().parse().unwrap_or(0.0);
    let rate: f64 = exchange_rate_mantissa.to_string().parse().unwrap_or(0.0);
    (balance * rate) / MANTISSA
}

/// Collateral-factor-weighted collateral across markets: Σ(usd × cf).
pub fn weighted_collateral(markets: &[(f64, f64)]) -> f64 {
    markets.iter().map(|(usd, cf)| usd * cf).sum()
}

pub fn total_debt(debts: &[f64]) -> f64 {
    debts.iter().sum()
}

/// Venus adapter over a provider, a price oracle, and per-underlying decimals.
pub struct VenusAdapter<P, O> {
    provider: P,
    oracle: O,
    comptroller: Address,
    underlying_decimals: HashMap<Address, u8>,
    close_factor: f64,
    liquidation_bonus: f64,
}

impl<P: Provider + Clone, O: PriceOracle> VenusAdapter<P, O> {
    pub fn new(
        provider: P,
        oracle: O,
        comptroller: Address,
        underlying_decimals: HashMap<Address, u8>,
    ) -> Self {
        Self {
            provider,
            oracle,
            comptroller,
            underlying_decimals,
            close_factor: 0.5,
            liquidation_bonus: 0.10,
        }
    }

    fn decimals_of(&self, token: Address) -> u8 {
        *self.underlying_decimals.get(&token).unwrap_or(&18)
    }
}

#[async_trait]
impl<P: Provider + Clone + Send + Sync, O: PriceOracle> LendingProtocol for VenusAdapter<P, O> {
    fn name(&self) -> &str {
        "venus"
    }

    fn close_factor(&self) -> f64 {
        self.close_factor
    }

    fn liquidation_bonus(&self) -> f64 {
        self.liquidation_bonus
    }

    async fn fetch_position(&self, borrower: Address) -> anyhow::Result<Option<Position>> {
        let comptroller = IComptroller::new(self.comptroller, self.provider.clone());
        let assets = comptroller.getAssetsIn(borrower).call().await?._0;
        if assets.is_empty() {
            return Ok(None);
        }

        let mut collateral_markets: Vec<(f64, f64)> = Vec::new();
        let mut debts: Vec<f64> = Vec::new();

        for vtoken in assets {
            let v = IVToken::new(vtoken, self.provider.clone());
            let snap = v.getAccountSnapshot(borrower).call().await?;
            let underlying = v.underlying().call().await?._0;
            let decimals = self.decimals_of(underlying);
            let price = self.oracle.price_usd(underlying).await?.usd;

            let collateral_base = vtoken_underlying(snap.vTokenBalance, snap.exchangeRateMantissa);
            let collateral_usd = (collateral_base / 10f64.powi(i32::from(decimals))) * price;

            let cf = comptroller
                .markets(vtoken)
                .call()
                .await
                .map(|m| mantissa_to_f64(m.collateralFactorMantissa))
                .unwrap_or(0.0);
            collateral_markets.push((collateral_usd, cf));

            let debt_usd = token_amount_to_f64(snap.borrowBalance, decimals) * price;
            debts.push(debt_usd);
        }

        // Fold the per-market collateral factors into the collateral side and
        // leave the threshold at 1.0 so health_factor = weighted / debt.
        let collateral_usd = weighted_collateral(&collateral_markets);
        let debt_usd = total_debt(&debts);

        Ok(Some(build_position(
            "venus",
            borrower,
            Address::ZERO,
            Address::ZERO,
            collateral_usd,
            debt_usd,
            1.0,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mantissa_conversion() {
        // 0.8 collateral factor encoded as 8e17
        let m = U256::from(8u64) * U256::from(10u64).pow(U256::from(17u64));
        assert!((mantissa_to_f64(m) - 0.8).abs() < 1e-9);
    }

    #[test]
    fn vtoken_underlying_math() {
        // balance 100 vTokens (raw) at exchangeRate 2e18 -> 200 base units
        let balance = U256::from(100u64);
        let rate = U256::from(2u64) * U256::from(10u64).pow(U256::from(18u64));
        assert!((vtoken_underlying(balance, rate) - 200.0).abs() < 1e-6);
    }

    #[test]
    fn weighted_collateral_and_debt() {
        let markets = [(1000.0, 0.8), (500.0, 0.6)];
        assert!((weighted_collateral(&markets) - 1100.0).abs() < 1e-9);
        assert!((total_debt(&[300.0, 200.0]) - 500.0).abs() < 1e-9);
    }

    #[test]
    fn weighted_collateral_drives_liquidatable() {
        // weighted collateral 1100, debt 1200 -> hf < 1
        let p = build_position(
            "venus",
            Address::ZERO,
            Address::ZERO,
            Address::ZERO,
            weighted_collateral(&[(1000.0, 0.8), (500.0, 0.6)]),
            1200.0,
            1.0,
        );
        assert!(p.is_liquidatable());
    }
}
