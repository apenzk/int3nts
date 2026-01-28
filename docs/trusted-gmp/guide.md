# Trusted GMP – Usage Guide

This guide covers how to configure operator keys, understand cross-chain validation, and debug the trusted-gmp service.

## Configuration

File: `trusted-gmp/config/trusted-gmp.toml` (relative to project root)

- **[trusted_gmp]**: `private_key` (base64, 32‑byte), `public_key` (base64, 32‑byte), polling/timeout

### Keys

- Use `cargo run --bin generate_keys` to print base64 keys
- Copy into `trusted-gmp.toml` (both keys must correspond)

**Security Warning**: The configuration file contains sensitive private keys. Protect this file with appropriate file system permissions and never commit it to version control.

## Cross‑Chain Validation Flow

The trusted-gmp service validates cross-chain conditions and generates approval signatures:

1) Hub: Requester creates regular (non‑oracle) intent
2) Connected: Requester creates escrow (non‑revocable), includes approver public key, links `intent_id`
3) Hub: Solver fulfills the intent
4) Trusted-gmp: observes fulfillment + escrow, generates approval (signature over BCS(u64=1))
5) Script: submits `complete_escrow_from_fa` on connected chain with approval

### Validation Logic

The trusted-gmp service validates:

- Shared `intent_id` across chains links hub intents to escrows on connected chains
- Trusted-gmp validates `chain_id` matches between intent `offered_chain_id` and escrow `chain_id`
- Each `EscrowEvent` includes a `chain_type` field (Mvm, Evm, Svm) set by the trusted-gmp service based on which monitor discovered the event

## Debugging

- Useful commands:
  - `curl -s http://127.0.0.1:3334/public-key`
  - `curl -s http://127.0.0.1:3334/approvals | jq`
