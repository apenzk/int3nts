# Trusted GMP Service

A service that validates cross-chain fulfillment conditions and provides approval signatures for cross-chain operations.

The trusted-gmp service supports two cross-chain flows:

**Outflow (hub → connected chain):**

1. Validates fulfillment transactions on connected chains (MVM, EVM, and SVM)
2. Validates that transfer conditions match intent requirements
3. Generates approval signatures for intent fulfillment on hub chain

**Inflow (connected chain → hub):**

1. Monitors escrow events on connected chains (MVM, EVM, and SVM)
2. Monitors fulfillment events on the hub chain (when solver fulfills)
3. Validates that fulfillment matches escrow conditions
4. Generates approval signatures for escrow release on connected chain

## Architecture

### Components

- **Cross-chain Validator**: Validates fulfillment conditions on hub and connected chains (MVM, EVM, and SVM)
- **Approval Service**: Provides approval signatures by signing the `intent_id` (Ed25519 for MVM and SVM, ECDSA for EVM)
- **Event Monitor**: Listens for intent and escrow events on hub and connected chains (MVM, EVM, and SVM)

## Project Structure

```text
trusted-gmp/
├── config/          # Configuration files (contains private keys)
├── src/
│   ├── monitor/     # Event monitoring (hub and connected chains)
│   ├── validator/   # Cross-chain validation logic
│   ├── crypto/      # Cryptographic operations and key management
│   ├── api/         # REST API server
│   └── bin/         # Utility binaries (generate_keys, get_approver_eth_address)
└── Cargo.toml
```

## SVM Outflow Validation

SVM outflow fulfillment transactions must include a strict memo + transfer pattern so the trusted-gmp service can tie the connected-chain transfer to the hub `intent_id`:

- The first instruction is an SPL memo with `intent_id=0x...` (32-byte hex).
- The transaction contains exactly one SPL `transferChecked` instruction.
- The transfer authority is a signer and must match the solver address for the intent.
- The transfer destination must match the intent's connected-chain recipient.
- The transfer amount and mint must match the intent's desired amount and token metadata.

This strict pattern prevents forged memos from being accepted without a matching SPL token transfer.

## Quick Start

See the [component README](../../trusted-gmp/README.md) for quick start commands.

## API Endpoints

- `GET /health` - Health check
- `GET /public-key` - Get trusted-gmp public key
- `GET /approvals` - Get cached approval signatures
- `POST /approval` - Create approval signature
- `POST /validate-outflow-fulfillment` - Validate connected chain transaction for outflow intent
- `POST /validate-inflow-escrow` - Validate escrow for inflow intent

For detailed API documentation, see [api.md](api.md). For usage guide, see [guide.md](guide.md).

## Dependencies

Uses pinned `aptos-core` version for stable Rust compatibility: `aptos-framework-v1.37.0` (SHA: `a10a3c02f16a2114ad065db6b4a525f0382e96a6`)
