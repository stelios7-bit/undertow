//! Tracks chain head advancement. `poll` returns the block numbers that have
//! appeared since the previous call so the scanner can react to each one.

use crate::provider::ChainClient;
use std::sync::Arc;

pub struct BlockListener<C: ChainClient> {
    client: Arc<C>,
    last_seen: u64,
}

impl<C: ChainClient> BlockListener<C> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            last_seen: 0,
        }
    }

    /// New block numbers since the last poll. The first poll returns only the
    /// current head (no historical backfill); later polls return the gap.
    pub async fn poll(&mut self) -> anyhow::Result<Vec<u64>> {
        let latest = self.client.block_number().await?;
        if latest <= self.last_seen {
            return Ok(Vec::new());
        }
        let start = if self.last_seen == 0 {
            latest
        } else {
            self.last_seen + 1
        };
        self.last_seen = latest;
        Ok((start..=latest).collect())
    }

    pub fn last_seen(&self) -> u64 {
        self.last_seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct MockClient {
        bn: Mutex<u64>,
    }
    impl MockClient {
        fn new(start: u64) -> Self {
            Self {
                bn: Mutex::new(start),
            }
        }
        fn set(&self, n: u64) {
            *self.bn.lock().expect("lock") = n;
        }
    }
    #[async_trait]
    impl ChainClient for MockClient {
        async fn block_number(&self) -> anyhow::Result<u64> {
            Ok(*self.bn.lock().expect("lock"))
        }
        async fn chain_id(&self) -> anyhow::Result<u64> {
            Ok(31337)
        }
    }

    #[tokio::test]
    async fn first_poll_returns_only_head() {
        let client = Arc::new(MockClient::new(100));
        let mut listener = BlockListener::new(client);
        assert_eq!(listener.poll().await.expect("poll"), vec![100]);
        assert_eq!(listener.last_seen(), 100);
    }

    #[tokio::test]
    async fn no_advance_returns_empty() {
        let client = Arc::new(MockClient::new(100));
        let mut listener = BlockListener::new(client);
        let _ = listener.poll().await.expect("poll");
        assert!(listener.poll().await.expect("poll").is_empty());
    }

    #[tokio::test]
    async fn returns_the_gap_on_advance() {
        let client = Arc::new(MockClient::new(100));
        let mut listener = BlockListener::new(client.clone());
        let _ = listener.poll().await.expect("poll");
        client.set(103);
        assert_eq!(listener.poll().await.expect("poll"), vec![101, 102, 103]);
    }
}
