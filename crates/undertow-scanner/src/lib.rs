//! `undertow-scanner` — chain access and head tracking. Connects to a node,
//! follows new blocks, and (in later changes) recomputes borrower health and
//! surfaces liquidatable positions.

pub mod chainlink;
pub mod listener;
pub mod oracle;
pub mod provider;
pub mod scanner;
pub mod token_meta;

pub use chainlink::ChainlinkOracle;
pub use listener::BlockListener;
pub use oracle::{FallbackOracle, PriceOracle, PricePoint};
pub use provider::{AlloyClient, ChainClient};
pub use scanner::{HealthScanner, TierCounts};
pub use token_meta::{TokenMeta, TokenMetaCache, TokenMetaSource};
