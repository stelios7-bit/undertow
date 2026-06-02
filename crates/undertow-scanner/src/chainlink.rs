//! Chainlink-backed `PriceOracle`. Reads `latestRoundData` from a per-token
//! aggregator and reports the price plus its on-chain update timestamp (so the
//! fallback layer can detect staleness).

use crate::oracle::{PriceOracle, PricePoint};
use alloy::providers::Provider;
use alloy::sol;
use alloy_primitives::Address;
use async_trait::async_trait;
use std::collections::HashMap;

sol! {
    #[sol(rpc)]
    interface IAggregatorV3 {
        function latestRoundData() external view returns (
            uint80 roundId,
            int256 answer,
            uint256 startedAt,
            uint256 updatedAt,
            uint80 answeredInRound
        );
        function decimals() external view returns (uint8);
    }
}

/// Maps each token to its Chainlink USD aggregator.
pub struct ChainlinkOracle<P> {
    provider: P,
    feeds: HashMap<Address, Address>,
}

impl<P: Provider + Clone> ChainlinkOracle<P> {
    pub fn new(provider: P, feeds: HashMap<Address, Address>) -> Self {
        Self { provider, feeds }
    }
}

#[async_trait]
impl<P: Provider + Clone + Send + Sync> PriceOracle for ChainlinkOracle<P> {
    async fn price_usd(&self, token: Address) -> anyhow::Result<PricePoint> {
        let feed = *self
            .feeds
            .get(&token)
            .ok_or_else(|| anyhow::anyhow!("no Chainlink feed for {token}"))?;

        let agg = IAggregatorV3::new(feed, self.provider.clone());
        let decimals = agg.decimals().call().await?._0;
        let round = agg.latestRoundData().call().await?;

        // String-parse keeps the conversion overflow-safe across int widths.
        let answer: f64 = round
            .answer
            .to_string()
            .parse()
            .map_err(|_| anyhow::anyhow!("bad Chainlink answer"))?;
        let usd = answer / 10f64.powi(i32::from(decimals));
        let updated_at_secs: u64 = round.updatedAt.to_string().parse().unwrap_or(0);

        Ok(PricePoint {
            usd,
            updated_at_secs,
        })
    }
}
