//! Price oracle abstraction. The scanner values positions in USD through a
//! `PriceOracle`; `FallbackOracle` tries a primary source and drops to a
//! secondary when the primary errors or returns a stale price.

use alloy_primitives::Address;
use async_trait::async_trait;

/// A USD price with the timestamp it was last updated on-chain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PricePoint {
    pub usd: f64,
    pub updated_at_secs: u64,
}

impl PricePoint {
    /// Older than `max_age_secs` relative to `now_secs`.
    pub fn is_stale(&self, now_secs: u64, max_age_secs: u64) -> bool {
        now_secs.saturating_sub(self.updated_at_secs) > max_age_secs
    }
}

#[async_trait]
pub trait PriceOracle: Send + Sync {
    async fn price_usd(&self, token: Address) -> anyhow::Result<PricePoint>;
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Primary-with-fallback oracle: returns the primary price when it's fresh,
/// otherwise the secondary's.
pub struct FallbackOracle<A: PriceOracle, B: PriceOracle> {
    primary: A,
    secondary: B,
    max_age_secs: u64,
    now: Box<dyn Fn() -> u64 + Send + Sync>,
}

impl<A: PriceOracle, B: PriceOracle> FallbackOracle<A, B> {
    pub fn new(primary: A, secondary: B, max_age_secs: u64) -> Self {
        Self {
            primary,
            secondary,
            max_age_secs,
            now: Box::new(unix_now),
        }
    }

    /// Inject a fixed clock — used by tests for deterministic staleness.
    pub fn with_clock(
        primary: A,
        secondary: B,
        max_age_secs: u64,
        now: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            primary,
            secondary,
            max_age_secs,
            now: Box::new(now),
        }
    }
}

#[async_trait]
impl<A: PriceOracle, B: PriceOracle> PriceOracle for FallbackOracle<A, B> {
    async fn price_usd(&self, token: Address) -> anyhow::Result<PricePoint> {
        match self.primary.price_usd(token).await {
            Ok(p) if !p.is_stale((self.now)(), self.max_age_secs) => Ok(p),
            _ => self.secondary.price_usd(token).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed(PricePoint);
    #[async_trait]
    impl PriceOracle for Fixed {
        async fn price_usd(&self, _t: Address) -> anyhow::Result<PricePoint> {
            Ok(self.0)
        }
    }
    struct Failing;
    #[async_trait]
    impl PriceOracle for Failing {
        async fn price_usd(&self, _t: Address) -> anyhow::Result<PricePoint> {
            anyhow::bail!("primary down")
        }
    }

    fn pp(usd: f64, at: u64) -> PricePoint {
        PricePoint {
            usd,
            updated_at_secs: at,
        }
    }

    #[tokio::test]
    async fn fresh_primary_wins() {
        let o = FallbackOracle::with_clock(Fixed(pp(100.0, 1000)), Fixed(pp(200.0, 1000)), 60, || 1030);
        assert_eq!(o.price_usd(Address::ZERO).await.unwrap().usd, 100.0);
    }

    #[tokio::test]
    async fn stale_primary_falls_back() {
        let o = FallbackOracle::with_clock(Fixed(pp(100.0, 1000)), Fixed(pp(200.0, 1090)), 60, || 1100);
        assert_eq!(o.price_usd(Address::ZERO).await.unwrap().usd, 200.0);
    }

    #[tokio::test]
    async fn failing_primary_falls_back() {
        let o = FallbackOracle::with_clock(Failing, Fixed(pp(200.0, 1000)), 60, || 1000);
        assert_eq!(o.price_usd(Address::ZERO).await.unwrap().usd, 200.0);
    }

    #[test]
    fn staleness() {
        let p = pp(1.0, 1000);
        assert!(!p.is_stale(1050, 60));
        assert!(p.is_stale(1100, 60));
    }
}
