//! `undertow-executor` — turns a scored opportunity into a signed, simulated,
//! submitted transaction: EIP-1559 fee pricing, nonce allocation, calldata
//! building, the `eth_call` simulation gate, submission, and flash-loan source
//! routing.

pub mod builder;
pub mod gas;
pub mod nonce;
pub mod simulation;

pub use builder::{build_tx, encode_liquidation, LiquidationCall};
pub use gas::{FeeConfig, Fees};
pub use nonce::NonceManager;
pub use simulation::{passes_gate, EthCallSimulator, Simulator};
