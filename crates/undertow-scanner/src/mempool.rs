//! Mempool race detection. A competing bot's pending liquidation reveals its
//! target borrower in the transaction calldata. `RaceDetector` scans pending
//! calldata for any borrower we're tracking so the executor can back off
//! instead of burning gas on a race it's likely to lose.
//!
//! The live pending-tx subscription is a thin wrapper around this pure check.

use alloy_primitives::Address;
use std::collections::HashSet;

pub struct RaceDetector {
    tracked: HashSet<Address>,
}

impl RaceDetector {
    pub fn new(tracked: impl IntoIterator<Item = Address>) -> Self {
        Self {
            tracked: tracked.into_iter().collect(),
        }
    }

    pub fn track(&mut self, borrower: Address) {
        self.tracked.insert(borrower);
    }

    pub fn len(&self) -> usize {
        self.tracked.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tracked.is_empty()
    }

    /// Scan calldata word-by-word (ABI encodes addresses as 32-byte,
    /// left-padded words) and return the first tracked borrower referenced.
    pub fn competing_borrower(&self, calldata: &[u8]) -> Option<Address> {
        for word in calldata.chunks_exact(32) {
            // an address sits in the low 20 bytes of the word
            let addr = Address::from_slice(&word[12..32]);
            if self.tracked.contains(&addr) {
                return Some(addr);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ABI-encode an address as a 32-byte left-padded word.
    fn word(addr: Address) -> [u8; 32] {
        let mut w = [0u8; 32];
        w[12..32].copy_from_slice(addr.as_slice());
        w
    }

    #[test]
    fn detects_tracked_borrower_in_calldata() {
        let target = Address::with_last_byte(0xAB);
        let detector = RaceDetector::new([target]);

        // selector (4 bytes) + two args, second is the target borrower
        let mut calldata = vec![0x12, 0x34, 0x56, 0x78];
        calldata.extend_from_slice(&word(Address::with_last_byte(0x01)));
        calldata.extend_from_slice(&word(target));

        // align to 32-byte words after the selector for the scan
        let aligned = &calldata[4..];
        assert_eq!(detector.competing_borrower(aligned), Some(target));
    }

    #[test]
    fn no_match_returns_none() {
        let detector = RaceDetector::new([Address::with_last_byte(0xAB)]);
        let calldata = word(Address::with_last_byte(0x01));
        assert_eq!(detector.competing_borrower(&calldata), None);
    }

    #[test]
    fn track_adds_borrowers() {
        let mut detector = RaceDetector::new([]);
        assert!(detector.is_empty());
        detector.track(Address::with_last_byte(1));
        assert_eq!(detector.len(), 1);
    }
}
