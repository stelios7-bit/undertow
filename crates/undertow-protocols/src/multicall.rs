//! Multicall3 batching. Scanning many borrowers means many `eth_call`s;
//! batching them through Multicall3 collapses each batch into one RPC round
//! trip. The call planner (chunking) is pure and unit-tested; `aggregate3`
//! issues the batched call.

use alloy::providers::Provider;
use alloy::sol;
use alloy_primitives::{Address, Bytes};

sol! {
    #[sol(rpc)]
    interface IMulticall3 {
        struct Call3 {
            address target;
            bool allowFailure;
            bytes callData;
        }
        struct Result {
            bool success;
            bytes returnData;
        }
        function aggregate3(Call3[] calls) external payable returns (Result[] returnData);
    }
}

pub use IMulticall3::{Call3, Result as CallResult};

/// Canonical Multicall3 deployment (same address on every supported chain).
pub const MULTICALL3: Address = Address::new([
    0xca, 0x11, 0xbd, 0xe0, 0x59, 0x77, 0xb3, 0x63, 0x11, 0x67, 0x02, 0x88, 0x62, 0xbe, 0x2a, 0x17,
    0x39, 0x76, 0xca, 0x11,
]);

/// Split calls into batches of at most `size` to stay under per-call gas/size
/// limits.
pub fn chunk_calls<T>(mut calls: Vec<T>, size: usize) -> Vec<Vec<T>> {
    if calls.is_empty() {
        return Vec::new();
    }
    if size == 0 {
        return vec![calls];
    }
    let mut batches = Vec::new();
    while !calls.is_empty() {
        let take = size.min(calls.len());
        batches.push(calls.drain(..take).collect());
    }
    batches
}

/// Execute one batched `aggregate3` call against the Multicall3 contract.
pub async fn aggregate3<P: Provider>(
    provider: P,
    multicall: Address,
    calls: Vec<Call3>,
) -> anyhow::Result<Vec<CallResult>> {
    let mc = IMulticall3::new(multicall, provider);
    Ok(mc.aggregate3(calls).call().await?.returnData)
}

/// Build an allow-failure `Call3` to `target` with the given calldata.
pub fn call3(target: Address, calldata: Bytes) -> Call3 {
    Call3 {
        target,
        allowFailure: true,
        callData: calldata,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_into_batches() {
        let calls: Vec<u32> = (0..5).collect();
        let batches = chunk_calls(calls, 2);
        assert_eq!(batches, vec![vec![0, 1], vec![2, 3], vec![4]]);
    }

    #[test]
    fn empty_is_no_batches() {
        let batches: Vec<Vec<u32>> = chunk_calls(Vec::new(), 3);
        assert!(batches.is_empty());
    }

    #[test]
    fn zero_size_is_single_batch() {
        assert_eq!(chunk_calls(vec![1, 2, 3], 0), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn multicall3_address_is_canonical() {
        assert_eq!(
            format!("{MULTICALL3:?}").to_lowercase(),
            "0xca11bde05977b3631167028862be2a173976ca11"
        );
    }
}
