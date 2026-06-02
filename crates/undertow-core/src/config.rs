//! Configuration loaded from a TOML profile (see `config/*.toml`).

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub chain: ChainConfig,
    pub strategy: StrategyConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChainConfig {
    pub name: String,
    pub chain_id: u64,
    pub ws_url: String,
    pub http_url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StrategyConfig {
    /// Minimum net profit (USD) below which opportunities are dropped.
    pub min_profit_usd: f64,
    /// Skip submission when the network gas price exceeds this ceiling.
    pub max_gas_gwei: f64,
}

impl Config {
    pub fn from_toml(s: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_profile() {
        let toml = r#"
            [chain]
            name = "bnb"
            chain_id = 56
            ws_url = "wss://example/ws"
            http_url = "https://example/http"

            [strategy]
            min_profit_usd = 25.0
            max_gas_gwei = 5.0
        "#;
        let cfg = Config::from_toml(toml).expect("valid config");
        assert_eq!(cfg.chain.chain_id, 56);
        assert_eq!(cfg.chain.name, "bnb");
        assert!((cfg.strategy.min_profit_usd - 25.0).abs() < 1e-9);
    }
}
