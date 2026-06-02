//! Uniswap V3 TWAP oracle — the fallback price source when Chainlink is stale
//! or unavailable. Reads the pool's `observe` accumulator over a window and
//! converts the average tick into a price. A TWAP is computed fresh each call,
//! so it never reads as stale to the fallback layer.

use crate::oracle::{PriceOracle, PricePoint};
use alloy::providers::Provider;
use alloy::sol;
use alloy_primitives::Address;
use async_trait::async_trait;
use std::collections::HashMap;

sol! {
    #[sol(rpc)]
    interface IUniswapV3Pool {
        function observe(uint32[] secondsAgos)
            external view
            returns (int56[] tickCumulatives, uint160[] secondsPerLiquidityCumulativeX128);
    }
}

/// Mean tick over the window: (cumulativeNow − cumulativeAgo) / elapsed.
pub fn average_tick(tick_cum_now: i64, tick_cum_ago: i64, elapsed: u32) -> i32 {
    if elapsed == 0 {
        return 0;
    }
    ((tick_cum_now - tick_cum_ago) / i64::from(elapsed)) as i32
}

/// Raw price ratio token1/token0 for a tick: 1.0001^tick.
pub fn tick_to_price_ratio(tick: i32) -> f64 {
    1.0001f64.powi(tick)
}

/// Per-token pool wiring for the TWAP read.
pub struct PoolConfig {
    pub pool: Address,
    pub seconds_ago: u32,
    pub token_decimals: u8,
    pub quote_decimals: u8,
    /// Whether the priced token is token0 of the pool.
    pub token_is_token0: bool,
}

pub struct UniswapV3Twap<P> {
    provider: P,
    pools: HashMap<Address, PoolConfig>,
}

impl<P: Provider + Clone> UniswapV3Twap<P> {
    pub fn new(provider: P, pools: HashMap<Address, PoolConfig>) -> Self {
        Self { provider, pools }
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[async_trait]
impl<P: Provider + Clone + Send + Sync> PriceOracle for UniswapV3Twap<P> {
    async fn price_usd(&self, token: Address) -> anyhow::Result<PricePoint> {
        let cfg = self
            .pools
            .get(&token)
            .ok_or_else(|| anyhow::anyhow!("no Uniswap V3 pool for {token}"))?;

        let pool = IUniswapV3Pool::new(cfg.pool, self.provider.clone());
        let ret = pool.observe(vec![cfg.seconds_ago, 0]).call().await?;
        let cums = ret.tickCumulatives;
        if cums.len() < 2 {
            anyhow::bail!("observe returned too few points");
        }

        let ago: i64 = cums[0].to_string().parse()?;
        let now: i64 = cums[1].to_string().parse()?;
        let avg = average_tick(now, ago, cfg.seconds_ago);

        let ratio = tick_to_price_ratio(avg);
        let decimals_adj =
            10f64.powi(i32::from(cfg.token_decimals) - i32::from(cfg.quote_decimals));
        let usd = if cfg.token_is_token0 {
            ratio * decimals_adj
        } else {
            (1.0 / ratio) * decimals_adj
        };

        Ok(PricePoint {
            usd,
            updated_at_secs: unix_now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_tick_math() {
        // 100_000 tick-seconds accrued over 1000s -> avg tick 100
        assert_eq!(average_tick(200_000, 100_000, 1000), 100);
        assert_eq!(average_tick(100, 100, 10), 0);
        assert_eq!(average_tick(100, 0, 0), 0); // guard
    }

    #[test]
    fn ratio_is_monotonic_and_anchored() {
        assert!((tick_to_price_ratio(0) - 1.0).abs() < 1e-12);
        assert!(tick_to_price_ratio(100) > tick_to_price_ratio(0));
        assert!(tick_to_price_ratio(-100) < tick_to_price_ratio(0));
    }
}
