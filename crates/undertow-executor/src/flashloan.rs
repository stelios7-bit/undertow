//! Flash-loan source routing. Every liquidation borrows the debt asset, repays
//! the loan in the same transaction, and keeps the spread. Sources charge
//! different premiums, so the router picks the cheapest one that can serve the
//! borrow. Premium math and selection are pure and unit-tested; the encoders
//! produce each source's flash-loan calldata.

use alloy::sol;
use alloy::sol_types::SolCall;
use alloy_primitives::{Address, Bytes, U256};

const BPS_DENOMINATOR: f64 = 10_000.0;

/// Available flash-loan sources, cheapest premium first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashSource {
    /// Balancer vault — 0% premium.
    Balancer,
    /// Aave V3 — 0.05% premium.
    AaveV3,
    /// Uniswap V3 pool flash — pool fee (representative 0.30%).
    UniswapV3,
}

impl FlashSource {
    pub fn premium_bps(self) -> u32 {
        match self {
            FlashSource::Balancer => 0,
            FlashSource::AaveV3 => 5,
            FlashSource::UniswapV3 => 30,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            FlashSource::Balancer => "balancer",
            FlashSource::AaveV3 => "aave-v3",
            FlashSource::UniswapV3 => "uniswap-v3",
        }
    }
}

/// Premium charged on a borrow of `amount_usd`.
pub fn flash_fee_usd(amount_usd: f64, source: FlashSource) -> f64 {
    amount_usd * f64::from(source.premium_bps()) / BPS_DENOMINATOR
}

/// Cheapest source among those available.
pub fn cheapest(sources: &[FlashSource]) -> Option<FlashSource> {
    sources.iter().copied().min_by_key(|s| s.premium_bps())
}

sol! {
    function flashLoanSimple(
        address receiverAddress,
        address asset,
        uint256 amount,
        bytes params,
        uint16 referralCode
    );
}

sol! {
    function flashLoan(
        address recipient,
        address[] tokens,
        uint256[] amounts,
        bytes userData
    );
}

/// Encode an Aave V3 `flashLoanSimple` call.
pub fn encode_aave_flash(receiver: Address, asset: Address, amount: U256, params: Bytes) -> Bytes {
    flashLoanSimpleCall {
        receiverAddress: receiver,
        asset,
        amount,
        params,
        referralCode: 0,
    }
    .abi_encode()
    .into()
}

/// Encode a Balancer vault `flashLoan` call.
pub fn encode_balancer_flash(
    recipient: Address,
    tokens: Vec<Address>,
    amounts: Vec<U256>,
    user_data: Bytes,
) -> Bytes {
    flashLoanCall {
        recipient,
        tokens,
        amounts,
        userData: user_data,
    }
    .abi_encode()
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premiums() {
        assert_eq!(FlashSource::Balancer.premium_bps(), 0);
        assert_eq!(FlashSource::AaveV3.premium_bps(), 5);
        assert_eq!(FlashSource::UniswapV3.premium_bps(), 30);
    }

    #[test]
    fn fee_math() {
        assert!((flash_fee_usd(1000.0, FlashSource::AaveV3) - 0.5).abs() < 1e-9);
        assert_eq!(flash_fee_usd(1000.0, FlashSource::Balancer), 0.0);
    }

    #[test]
    fn router_picks_cheapest() {
        let avail = [FlashSource::UniswapV3, FlashSource::AaveV3, FlashSource::Balancer];
        assert_eq!(cheapest(&avail), Some(FlashSource::Balancer));
        assert_eq!(
            cheapest(&[FlashSource::UniswapV3, FlashSource::AaveV3]),
            Some(FlashSource::AaveV3)
        );
        assert_eq!(cheapest(&[]), None);
    }

    #[test]
    fn encoders_emit_correct_selectors() {
        let aave = encode_aave_flash(Address::ZERO, Address::ZERO, U256::from(1u64), Bytes::new());
        assert_eq!(&aave[0..4], flashLoanSimpleCall::SELECTOR.as_slice());

        let bal = encode_balancer_flash(Address::ZERO, vec![Address::ZERO], vec![U256::from(1u64)], Bytes::new());
        assert_eq!(&bal[0..4], flashLoanCall::SELECTOR.as_slice());
    }
}
