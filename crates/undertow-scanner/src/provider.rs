//! Chain access. `ChainClient` is the minimal RPC surface the scanner needs;
//! `AlloyClient` is the production HTTP-backed implementation. Keeping it a
//! trait lets the listener and scanner be unit-tested without a live node.

use alloy::providers::{Provider, ProviderBuilder};
use async_trait::async_trait;

#[async_trait]
pub trait ChainClient: Send + Sync {
    /// Latest block number seen by the node.
    async fn block_number(&self) -> anyhow::Result<u64>;
    /// EVM chain id.
    async fn chain_id(&self) -> anyhow::Result<u64>;
}

/// HTTP-backed client over an alloy provider.
pub struct AlloyClient<P> {
    provider: P,
}

impl<P: Provider> AlloyClient<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

impl AlloyClient<()> {
    /// Connect to a node by URL (http/https/ws/wss). Uses a boxed transport so
    /// the concrete type stays simple.
    pub async fn connect(url: &str) -> anyhow::Result<AlloyClient<impl Provider>> {
        let provider = ProviderBuilder::new().on_builtin(url).await?;
        Ok(AlloyClient { provider })
    }
}

#[async_trait]
impl<P: Provider + Send + Sync> ChainClient for AlloyClient<P> {
    async fn block_number(&self) -> anyhow::Result<u64> {
        Ok(self.provider.get_block_number().await?)
    }

    async fn chain_id(&self) -> anyhow::Result<u64> {
        Ok(self.provider.get_chain_id().await?)
    }
}
