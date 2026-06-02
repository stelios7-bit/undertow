//! ERC-20 metadata cache. Decimals and symbols never change, so they're
//! fetched once per token and cached for the life of the process. The fetch is
//! behind a trait so the cache can be tested without RPC; the on-chain
//! implementation lands with the protocol adapters.

use alloy_primitives::Address;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenMeta {
    pub decimals: u8,
    pub symbol: String,
}

#[async_trait]
pub trait TokenMetaSource: Send + Sync {
    async fn fetch(&self, token: Address) -> anyhow::Result<TokenMeta>;
}

/// Caches token metadata, fetching each token from the source at most once.
pub struct TokenMetaCache<S: TokenMetaSource> {
    source: S,
    cache: Mutex<HashMap<Address, TokenMeta>>,
}

impl<S: TokenMetaSource> TokenMetaCache<S> {
    pub fn new(source: S) -> Self {
        Self {
            source,
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, token: Address) -> anyhow::Result<TokenMeta> {
        if let Some(meta) = self.cache.lock().expect("cache lock").get(&token) {
            return Ok(meta.clone());
        }
        let meta = self.source.fetch(token).await?;
        self.cache
            .lock()
            .expect("cache lock")
            .insert(token, meta.clone());
        Ok(meta)
    }

    pub fn len(&self) -> usize {
        self.cache.lock().expect("cache lock").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSource {
        calls: AtomicUsize,
    }
    #[async_trait]
    impl TokenMetaSource for CountingSource {
        async fn fetch(&self, _token: Address) -> anyhow::Result<TokenMeta> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(TokenMeta {
                decimals: 18,
                symbol: "TKN".into(),
            })
        }
    }

    #[tokio::test]
    async fn caches_after_first_fetch() {
        let source = CountingSource {
            calls: AtomicUsize::new(0),
        };
        let cache = TokenMetaCache::new(source);
        let token = Address::with_last_byte(1);

        let a = cache.get(token).await.expect("get");
        let b = cache.get(token).await.expect("get");
        assert_eq!(a, b);
        assert_eq!(cache.len(), 1);
        // second get is a cache hit — source called exactly once
        assert_eq!(cache.source.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn distinct_tokens_cached_separately() {
        let cache = TokenMetaCache::new(CountingSource {
            calls: AtomicUsize::new(0),
        });
        let _ = cache.get(Address::with_last_byte(1)).await.expect("get");
        let _ = cache.get(Address::with_last_byte(2)).await.expect("get");
        assert_eq!(cache.len(), 2);
    }
}
