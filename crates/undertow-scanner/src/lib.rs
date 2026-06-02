//! `undertow-scanner` — chain access and head tracking. Connects to a node,
//! follows new blocks, and (in later changes) recomputes borrower health and
//! surfaces liquidatable positions.

pub mod listener;
pub mod provider;
pub mod scanner;
pub mod token_meta;

pub use listener::BlockListener;
pub use provider::{AlloyClient, ChainClient};
pub use scanner::{HealthScanner, TierCounts};
pub use token_meta::{TokenMeta, TokenMetaCache, TokenMetaSource};
