//! Borrower discovery. Backfilling borrowers from protocol events means
//! scanning a large block range, which RPCs cap per `eth_getLogs` call — so
//! the range is split into chunks. The actual log query is behind a trait; the
//! chunk planner and dedup are pure and unit-tested.

use alloy_primitives::Address;
use async_trait::async_trait;
use std::collections::BTreeSet;

/// Split `[from, to]` (inclusive) into chunks of at most `chunk` blocks.
pub fn plan_chunks(from: u64, to: u64, chunk: u64) -> Vec<(u64, u64)> {
    let mut ranges = Vec::new();
    if to < from || chunk == 0 {
        return ranges;
    }
    let mut start = from;
    loop {
        let end = start.saturating_add(chunk - 1).min(to);
        ranges.push((start, end));
        if end >= to {
            break;
        }
        start = end + 1;
    }
    ranges
}

#[async_trait]
pub trait BorrowerSource: Send + Sync {
    /// Borrowers seen in events over `[from, to]`.
    async fn borrowers_in_range(&self, from: u64, to: u64) -> anyhow::Result<Vec<Address>>;
}

/// Backfill borrowers across `[from, to]`, chunking the range and deduplicating.
pub async fn discover<S: BorrowerSource>(
    source: &S,
    from: u64,
    to: u64,
    chunk: u64,
) -> anyhow::Result<Vec<Address>> {
    let mut seen = BTreeSet::new();
    for (start, end) in plan_chunks(from, to, chunk) {
        for borrower in source.borrowers_in_range(start, end).await? {
            seen.insert(borrower);
        }
    }
    Ok(seen.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn chunks_exact_multiple() {
        assert_eq!(plan_chunks(0, 9, 5), vec![(0, 4), (5, 9)]);
    }
    #[test]
    fn chunks_with_remainder() {
        assert_eq!(plan_chunks(0, 11, 5), vec![(0, 4), (5, 9), (10, 11)]);
    }
    #[test]
    fn chunks_single_range() {
        assert_eq!(plan_chunks(100, 103, 50), vec![(100, 103)]);
    }
    #[test]
    fn chunks_empty_when_inverted_or_zero() {
        assert!(plan_chunks(10, 5, 5).is_empty());
        assert!(plan_chunks(0, 10, 0).is_empty());
    }

    struct MockSource {
        ranges_seen: Mutex<Vec<(u64, u64)>>,
    }
    #[async_trait]
    impl BorrowerSource for MockSource {
        async fn borrowers_in_range(&self, from: u64, to: u64) -> anyhow::Result<Vec<Address>> {
            self.ranges_seen.lock().expect("lock").push((from, to));
            // same borrower appears in adjacent chunks -> must dedup
            Ok(vec![Address::with_last_byte(1), Address::with_last_byte((to % 256) as u8)])
        }
    }

    #[tokio::test]
    async fn discover_chunks_and_dedups() {
        let src = MockSource {
            ranges_seen: Mutex::new(Vec::new()),
        };
        let found = discover(&src, 0, 9, 5).await.expect("discover");
        // two chunks queried
        assert_eq!(src.ranges_seen.lock().expect("lock").len(), 2);
        // Address(1) deduped despite appearing in both chunks
        assert!(found.contains(&Address::with_last_byte(1)));
        assert_eq!(found.iter().filter(|a| **a == Address::with_last_byte(1)).count(), 1);
    }
}
