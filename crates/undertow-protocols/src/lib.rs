//! `undertow-protocols` — lending-protocol adapters. Each adapter implements
//! the core `LendingProtocol` trait so the scanner and executor stay protocol
//! agnostic. This crate also carries shared valuation helpers and Multicall3
//! batching used by every adapter.

pub mod aave_v3;
pub mod multicall;
pub mod util;
pub mod venus;

pub use aave_v3::AaveV3Adapter;
pub use multicall::{aggregate3, call3, chunk_calls, Call3, CallResult, MULTICALL3};
pub use util::{build_position, token_amount_to_f64, usd_value};
pub use venus::VenusAdapter;
