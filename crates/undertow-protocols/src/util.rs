//! Shared helpers for protocol adapters: turning raw on-chain token amounts
//! into USD-denominated floats the core `Position` model works in.

use alloy_primitives::U256;
use undertow_core::Position;

/// Convert a raw token amount (base units) to a human float using `decimals`.
/// Goes via string to stay overflow-safe for full-width U256 values.
pub fn token_amount_to_f64(raw: U256, decimals: u8) -> f64 {
    let n: f64 = raw.to_string().parse().unwrap_or(0.0);
    n / 10f64.powi(i32::from(decimals))
}

/// USD value of a raw token amount at `price_usd` per whole token.
pub fn usd_value(raw: U256, decimals: u8, price_usd: f64) -> f64 {
    token_amount_to_f64(raw, decimals) * price_usd
}

/// Assemble a core `Position` from already-valued USD figures.
#[allow(clippy::too_many_arguments)]
pub fn build_position(
    protocol: impl Into<String>,
    borrower: alloy_primitives::Address,
    collateral_token: alloy_primitives::Address,
    debt_token: alloy_primitives::Address,
    collateral_usd: f64,
    debt_usd: f64,
    liquidation_threshold: f64,
) -> Position {
    Position {
        protocol: protocol.into(),
        borrower,
        collateral_token,
        debt_token,
        collateral_usd,
        debt_usd,
        liquidation_threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;

    #[test]
    fn one_whole_token_18_decimals() {
        let raw = U256::from(10u64).pow(U256::from(18u64)); // 1e18
        assert!((token_amount_to_f64(raw, 18) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn six_decimal_token() {
        let raw = U256::from(2_500_000u64); // 2.5 with 6 decimals
        assert!((token_amount_to_f64(raw, 6) - 2.5).abs() < 1e-9);
    }

    #[test]
    fn usd_valuation() {
        let raw = U256::from(10u64).pow(U256::from(18u64));
        assert!((usd_value(raw, 18, 2000.0) - 2000.0).abs() < 1e-6);
    }

    #[test]
    fn builds_position_with_expected_health() {
        let p = build_position("venus", Address::ZERO, Address::ZERO, Address::ZERO, 1000.0, 900.0, 0.8);
        assert!(p.is_liquidatable()); // hf = 0.888
    }
}
