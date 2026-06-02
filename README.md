# Undertow

> Multi-chain, flash-loan-backed liquidation bot — written in Rust.

Undertow monitors under-collateralized positions across DeFi lending protocols
and executes profitable liquidations using flash loans — zero upfront capital,
zero position risk. If a liquidation is unprofitable at execution time, the whole
transaction reverts atomically; the only cost is a failed simulation's gas.

## How it works

1. **Listen** — WebSocket listener receives new blocks and log events.
2. **Decode** — protocol adapters normalize events into a shared `Position`.
3. **Price** — price engine reads USD prices (Chainlink, with TWAP fallback).
4. **Scan** — health scanner flags positions with health factor below `1.0`.
5. **Estimate** — profit calculator simulates the full liquidation end to end.
6. **Route** — flash-loan router picks the cheapest source.
7. **Build** — transaction builder encodes, dry-runs via `eth_call`, signs, submits.
8. **Execute** — on-chain liquidator atomically borrows → liquidates → swaps →
   repays the flash loan → forwards profit. Any failure reverts everything.

## Stack

- Rust (async, `tokio`) workspace of focused crates
- Solidity liquidator contracts (Foundry)
- Flash loans (Aave V3 / Balancer / Uniswap V3), Chainlink pricing
- Prometheus + Grafana observability, Docker deploy

## Status

In active development, PR-driven. See the issues and milestones for scope.

## License

MIT
