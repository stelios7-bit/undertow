//! `undertow-executor` — turns a scored opportunity into a signed, simulated,
//! submitted transaction: EIP-1559 fee pricing, nonce allocation, calldata
//! building, the `eth_call` simulation gate, submission, and flash-loan source
//! routing.

pub mod gas;
pub mod nonce;

pub use gas::{FeeConfig, Fees};
pub use nonce::NonceManager;
