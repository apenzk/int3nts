# Integrated GMP Service

A integrated GMP relay service that watches for `MessageSent` events on source chains and delivers cross-chain messages by calling `deliver_message` on destination chains.

Integrated-gmp is a pure relay — invisible to clients. The coordinator is the single API surface for frontends and solvers.

## Architecture

The integrated-gmp service is infrastructure only:

1. Watches mock GMP endpoint contracts for `MessageSent` events on all chains
2. Picks up pending messages and delivers them to destination chain GMP endpoints
3. Destination contracts process the delivered messages (e.g., release escrows, confirm fulfillments)

In production, this relay can be replaced by an external GMP provider's endpoint infrastructure.

### Components

- **Integrated GMP Relay**: Watches `MessageSent` events and calls `deliver_message` on destination chains
- **Event Monitor**: Listens for GMP events on hub and connected chains (MVM, EVM, SVM)
- **Chain Clients**: MVM, EVM, and SVM clients for reading events and submitting transactions

## Project Structure

```text
integrated-gmp/
├── config/              # Configuration files (contains operator wallet keys)
├── src/
│   ├── monitor/         # Event monitoring (hub and connected chains)
│   ├── integrated_gmp_relay/ # Core relay logic
│   ├── evm_client.rs    # EVM chain client
│   ├── mvm_client.rs    # MVM chain client
│   ├── svm_client.rs    # SVM chain client
│   ├── config.rs        # Configuration loading
│   ├── crypto/          # Cryptographic operations
│   ├── validator/       # Cross-chain validation logic
│   └── bin/             # Utility binaries (generate_keys, get_approver_eth_address)
└── Cargo.toml
```

## Security Requirements

**CRITICAL**: This service has operator wallet keys and can deliver arbitrary messages. Ensure proper key management and access controls for production use.

## Quick Start

See the [component README](../../integrated-gmp/README.md) for quick start commands.

## Dependencies

Uses pinned `aptos-core` version for stable Rust compatibility: `aptos-framework-v1.37.0` (SHA: `a10a3c02f16a2114ad065db6b4a525f0382e96a6`)
