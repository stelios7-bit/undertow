//! Simulation gate. Before signing and broadcasting, every liquidation is
//! dry-run with `eth_call`. A revert here (lost the race, bad estimate, stale
//! state) costs nothing, so the gate drops it before any gas is spent.

use alloy::providers::Provider;
use alloy::rpc::types::TransactionRequest;
use async_trait::async_trait;

#[async_trait]
pub trait Simulator: Send + Sync {
    /// Whether the transaction would succeed if submitted now.
    async fn would_succeed(&self, tx: &TransactionRequest) -> anyhow::Result<bool>;
}

/// Production simulator backed by `eth_call`.
pub struct EthCallSimulator<P> {
    provider: P,
}

impl<P: Provider> EthCallSimulator<P> {
    pub fn new(provider: P) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync> Simulator for EthCallSimulator<P> {
    async fn would_succeed(&self, tx: &TransactionRequest) -> anyhow::Result<bool> {
        // A revert surfaces as an Err — treat it as "do not submit".
        Ok(self.provider.call(tx).await.is_ok())
    }
}

/// Gate an opportunity through the simulator: only `Ok(true)` passes.
pub async fn passes_gate<S: Simulator>(
    sim: &S,
    tx: &TransactionRequest,
) -> anyhow::Result<bool> {
    sim.would_succeed(tx).await
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSim(bool);
    #[async_trait]
    impl Simulator for MockSim {
        async fn would_succeed(&self, _tx: &TransactionRequest) -> anyhow::Result<bool> {
            Ok(self.0)
        }
    }

    #[tokio::test]
    async fn passing_simulation_clears_gate() {
        let tx = TransactionRequest::default();
        assert!(passes_gate(&MockSim(true), &tx).await.unwrap());
    }

    #[tokio::test]
    async fn reverting_simulation_blocks_gate() {
        let tx = TransactionRequest::default();
        assert!(!passes_gate(&MockSim(false), &tx).await.unwrap());
    }
}
