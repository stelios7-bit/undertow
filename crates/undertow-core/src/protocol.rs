//! The single trait every lending-protocol adapter implements.
//!
//! Adding a protocol (Venus, Aave V3, Compound, ...) means implementing
//! `LendingProtocol` — the scanner, profit model, and executor are written
//! against this trait and never need to know which protocol they're driving.

use crate::types::Position;
use alloy_primitives::Address;
use async_trait::async_trait;

#[async_trait]
pub trait LendingProtocol: Send + Sync {
    /// Human-readable adapter name, e.g. "venus" or "aave-v3".
    fn name(&self) -> &str;

    /// Fetch the current position for a borrower, or `None` if they have none.
    async fn fetch_position(&self, borrower: Address) -> anyhow::Result<Option<Position>>;

    /// Max fraction of debt repayable in a single liquidation (e.g. 0.5).
    fn close_factor(&self) -> f64;

    /// Liquidation bonus / incentive (e.g. 0.08 = 8%).
    fn liquidation_bonus(&self) -> f64;
}
