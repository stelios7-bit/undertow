//! Priority queue of liquidation opportunities, ordered by net profit.

use crate::types::Position;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// A scored, ready-to-execute liquidation.
#[derive(Debug, Clone)]
pub struct Opportunity {
    pub position: Position,
    pub net_profit_usd: f64,
}

impl PartialEq for Opportunity {
    fn eq(&self, other: &Self) -> bool {
        self.net_profit_usd == other.net_profit_usd
    }
}
impl Eq for Opportunity {}

impl PartialOrd for Opportunity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Opportunity {
    // NaN profits sort as equal rather than panicking — a NaN estimate is a
    // bug upstream, but the hot path must never crash on it.
    fn cmp(&self, other: &Self) -> Ordering {
        self.net_profit_usd
            .partial_cmp(&other.net_profit_usd)
            .unwrap_or(Ordering::Equal)
    }
}

/// Max-heap: `pop_best` always returns the most profitable opportunity.
#[derive(Default)]
pub struct OpportunityQueue {
    heap: BinaryHeap<Opportunity>,
}

impl OpportunityQueue {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, opp: Opportunity) {
        self.heap.push(opp);
    }
    pub fn pop_best(&mut self) -> Option<Opportunity> {
        self.heap.pop()
    }
    pub fn len(&self) -> usize {
        self.heap.len()
    }
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;

    fn opp(profit: f64) -> Opportunity {
        Opportunity {
            position: Position {
                protocol: "test".into(),
                borrower: Address::ZERO,
                collateral_token: Address::ZERO,
                debt_token: Address::ZERO,
                collateral_usd: 0.0,
                debt_usd: 0.0,
                liquidation_threshold: 0.8,
            },
            net_profit_usd: profit,
        }
    }

    #[test]
    fn pops_most_profitable_first() {
        let mut q = OpportunityQueue::new();
        q.push(opp(10.0));
        q.push(opp(100.0));
        q.push(opp(50.0));
        assert_eq!(q.len(), 3);
        assert_eq!(q.pop_best().unwrap().net_profit_usd, 100.0);
        assert_eq!(q.pop_best().unwrap().net_profit_usd, 50.0);
        assert_eq!(q.pop_best().unwrap().net_profit_usd, 10.0);
        assert!(q.is_empty());
    }
}
