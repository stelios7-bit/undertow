//! Transaction building. Encodes the liquidation call to the on-chain
//! liquidator contract and assembles the EIP-1559 `TransactionRequest` (nonce,
//! fees, gas) ready to simulate and sign.

use crate::gas::Fees;
use alloy::network::TransactionBuilder;
use alloy::rpc::types::TransactionRequest;
use alloy::sol;
use alloy::sol_types::SolCall;
use alloy_primitives::{Address, Bytes, U256};

sol! {
    function executeLiquidation(
        address protocol,
        address borrower,
        address collateral,
        address debt,
        uint256 repayAmount,
        address flashSource
    );
}

/// Parameters for one liquidation call.
#[derive(Debug, Clone)]
pub struct LiquidationCall {
    pub protocol: Address,
    pub borrower: Address,
    pub collateral: Address,
    pub debt: Address,
    pub repay_amount: U256,
    pub flash_source: Address,
}

/// ABI-encode the liquidation call into the liquidator's calldata.
pub fn encode_liquidation(call: &LiquidationCall) -> Bytes {
    executeLiquidationCall {
        protocol: call.protocol,
        borrower: call.borrower,
        collateral: call.collateral,
        debt: call.debt,
        repayAmount: call.repay_amount,
        flashSource: call.flash_source,
    }
    .abi_encode()
    .into()
}

/// Assemble the EIP-1559 transaction request to the liquidator contract.
pub fn build_tx(
    liquidator: Address,
    from: Address,
    calldata: Bytes,
    nonce: u64,
    fees: Fees,
    gas_limit: u64,
) -> TransactionRequest {
    TransactionRequest::default()
        .with_from(from)
        .with_to(liquidator)
        .with_input(calldata)
        .with_nonce(nonce)
        .with_max_fee_per_gas(fees.max_fee_per_gas_wei)
        .with_max_priority_fee_per_gas(fees.max_priority_fee_per_gas_wei)
        .with_gas_limit(gas_limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::TxKind;

    fn sample_call() -> LiquidationCall {
        LiquidationCall {
            protocol: Address::with_last_byte(1),
            borrower: Address::with_last_byte(2),
            collateral: Address::with_last_byte(3),
            debt: Address::with_last_byte(4),
            repay_amount: U256::from(1000u64),
            flash_source: Address::with_last_byte(5),
        }
    }

    #[test]
    fn calldata_has_selector_and_args() {
        let data = encode_liquidation(&sample_call());
        // 4-byte selector + 6 ABI words
        assert_eq!(data.len(), 4 + 6 * 32);
        assert_eq!(&data[0..4], executeLiquidationCall::SELECTOR.as_slice());
    }

    #[test]
    fn builds_request_fields() {
        let liquidator = Address::with_last_byte(0xAA);
        let from = Address::with_last_byte(0xBB);
        let fees = Fees {
            max_fee_per_gas_wei: 11_000_000_000,
            max_priority_fee_per_gas_wei: 1_000_000_000,
        };
        let data = encode_liquidation(&sample_call());
        let tx = build_tx(liquidator, from, data.clone(), 7, fees, 500_000);

        assert_eq!(tx.to, Some(TxKind::Call(liquidator)));
        assert_eq!(tx.from, Some(from));
        assert_eq!(tx.nonce, Some(7));
        assert_eq!(tx.max_fee_per_gas, Some(11_000_000_000));
        assert_eq!(tx.gas, Some(500_000));
        assert_eq!(tx.input.input(), Some(&data));
    }
}
