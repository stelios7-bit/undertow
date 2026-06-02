//! `undertow-core` — shared domain model for the Undertow liquidation bot:
//! configuration, position types, the profit model, the lending-protocol
//! trait, and the opportunity queue. Everything downstream (scanner,
//! protocol adapters, executor) is written against these types.

pub mod config;
pub mod profit;
pub mod protocol;
pub mod queue;
pub mod types;

pub use config::{ChainConfig, Config, StrategyConfig};
pub use profit::{ProfitEstimate, ProfitInputs};
pub use protocol::LendingProtocol;
pub use queue::{Opportunity, OpportunityQueue};
pub use types::{Health, Position};
