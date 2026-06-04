//! Nonce allocation for the hot wallet. Liquidations are submitted back to
//! back, so nonces are handed out locally rather than re-fetched per tx; the
//! manager is re-synced to the chain's confirmed nonce on startup and after
//! drops. Allocation is forward-only — `sync` never rewinds past nonces.

use std::sync::atomic::{AtomicU64, Ordering};

pub struct NonceManager {
    next: AtomicU64,
}

impl NonceManager {
    pub fn new(start: u64) -> Self {
        Self {
            next: AtomicU64::new(start),
        }
    }

    /// Hand out the next nonce and advance.
    pub fn allocate(&self) -> u64 {
        self.next.fetch_add(1, Ordering::SeqCst)
    }

    /// The nonce that would be handed out next (without advancing).
    pub fn peek(&self) -> u64 {
        self.next.load(Ordering::SeqCst)
    }

    /// Re-sync to the chain's confirmed nonce. Forward-only: a stale chain read
    /// can't rewind locally-allocated nonces.
    pub fn sync(&self, chain_nonce: u64) {
        let mut current = self.next.load(Ordering::SeqCst);
        while chain_nonce > current {
            match self.next.compare_exchange(
                current,
                chain_nonce,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_advances() {
        let m = NonceManager::new(5);
        assert_eq!(m.allocate(), 5);
        assert_eq!(m.allocate(), 6);
        assert_eq!(m.peek(), 7);
    }

    #[test]
    fn sync_is_forward_only() {
        let m = NonceManager::new(10);
        m.sync(8); // stale read -> ignored
        assert_eq!(m.peek(), 10);
        m.sync(12); // ahead -> jump forward
        assert_eq!(m.peek(), 12);
    }
}
