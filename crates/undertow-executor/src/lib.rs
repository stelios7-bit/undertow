//! `undertow-executor` — turns a scored opportunity into a signed, simulated,
//! submitted transaction: EIP-1559 fee pricing, nonce allocation, calldata
//! building, the `eth_call` simulation gate, submission, and flash-loan source
//! routing.

pub mod batcher;
pub mod builder;
pub mod flashloan;
pub mod gas;
pub mod nonce;
pub mod simulation;
pub mod submit;

pub use batcher::{take_batch, BatchConfig};
pub use flashloan::{
    cheapest, encode_aave_flash, encode_balancer_flash, flash_fee_usd, FlashSource,
};
pub use builder::{build_tx, encode_liquidation, LiquidationCall};
pub use gas::{FeeConfig, Fees};
pub use nonce::NonceManager;
pub use simulation::{passes_gate, EthCallSimulator, Simulator};
pub use submit::{validate_private_endpoint, ProviderSubmitter, SubmitRoute, Submitter};
