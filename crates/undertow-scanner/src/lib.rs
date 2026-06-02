//! `undertow-scanner` — chain access and head tracking. Connects to a node,
//! follows new blocks, and (in later changes) recomputes borrower health and
//! surfaces liquidatable positions.

pub mod listener;
pub mod provider;

pub use listener::BlockListener;
pub use provider::{AlloyClient, ChainClient};
