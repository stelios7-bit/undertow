//! Opportunity batching. Each cycle drains the highest-profit opportunities
//! from the queue up to a per-cycle cap, so the executor works the best
//! liquidations first and bounds how many it submits at once.

use undertow_core::{Opportunity, OpportunityQueue};

#[derive(Debug, Clone, Copy)]
pub struct BatchConfig {
    pub max_per_cycle: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self { max_per_cycle: 4 }
    }
}

/// Pop up to `max` best opportunities, highest profit first.
pub fn take_batch(queue: &mut OpportunityQueue, max: usize) -> Vec<Opportunity> {
    let mut batch = Vec::new();
    while batch.len() < max {
        match queue.pop_best() {
            Some(opp) => batch.push(opp),
            None => break,
        }
    }
    batch
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;
    use undertow_core::Position;

    fn opp(profit: f64) -> Opportunity {
        Opportunity {
            position: Position {
                protocol: "t".into(),
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
    fn takes_best_first_up_to_cap() {
        let mut q = OpportunityQueue::new();
        for p in [10.0, 100.0, 50.0, 5.0] {
            q.push(opp(p));
        }
        let batch = take_batch(&mut q, 2);
        let profits: Vec<f64> = batch.iter().map(|o| o.net_profit_usd).collect();
        assert_eq!(profits, vec![100.0, 50.0]);
        assert_eq!(q.len(), 2); // remainder stays
    }

    #[test]
    fn cap_larger_than_queue_drains_all() {
        let mut q = OpportunityQueue::new();
        q.push(opp(1.0));
        q.push(opp(2.0));
        assert_eq!(take_batch(&mut q, 10).len(), 2);
        assert!(q.is_empty());
    }
}
