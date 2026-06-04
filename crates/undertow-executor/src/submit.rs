//! Transaction submission. On Ethereum, liquidations go through a private relay
//! (Flashbots) to avoid being front-run out of the public mempool; on L2s a
//! private RPC endpoint serves the same role. The route is selected per chain;
//! private endpoints must be encrypted transports.

use alloy::providers::Provider;
use alloy::rpc::types::TransactionRequest;
use alloy_primitives::TxHash;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum SubmitRoute {
    /// Broadcast through the node's public mempool.
    Public,
    /// Submit to a private relay / RPC endpoint.
    Private { url: String },
}

/// Private endpoints must use an encrypted transport — a plaintext relay URL
/// would leak the bundle.
pub fn validate_private_endpoint(url: &str) -> anyhow::Result<()> {
    if url.starts_with("https://") || url.starts_with("wss://") {
        Ok(())
    } else {
        anyhow::bail!("private RPC must be https:// or wss://, got: {url}")
    }
}

#[async_trait]
pub trait Submitter: Send + Sync {
    async fn submit(&self, tx: TransactionRequest) -> anyhow::Result<TxHash>;
}

/// Submitter that signs + broadcasts through a wallet-backed provider.
pub struct ProviderSubmitter<P> {
    provider: P,
}

impl<P: Provider> ProviderSubmitter<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync> Submitter for ProviderSubmitter<P> {
    async fn submit(&self, tx: TransactionRequest) -> anyhow::Result<TxHash> {
        let pending = self.provider.send_transaction(tx).await?;
        Ok(*pending.tx_hash())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_encrypted_endpoints() {
        assert!(validate_private_endpoint("https://relay.example/?auth=k").is_ok());
        assert!(validate_private_endpoint("wss://relay.example").is_ok());
    }

    #[test]
    fn rejects_plaintext_endpoints() {
        assert!(validate_private_endpoint("http://relay.example").is_err());
        assert!(validate_private_endpoint("relay.example").is_err());
    }

    struct MockSubmitter(TxHash);
    #[async_trait]
    impl Submitter for MockSubmitter {
        async fn submit(&self, _tx: TransactionRequest) -> anyhow::Result<TxHash> {
            Ok(self.0)
        }
    }

    #[tokio::test]
    async fn submitter_returns_hash() {
        let h = TxHash::with_last_byte(7);
        let s = MockSubmitter(h);
        assert_eq!(s.submit(TransactionRequest::default()).await.unwrap(), h);
    }
}
