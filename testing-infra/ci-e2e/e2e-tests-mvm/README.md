# Move VM E2E Tests

Tests Move VM-only cross-chain intent framework: intents on Hub with escrows on two independent MVM connected chains.

Two Docker-based Aptos localnet instances run simultaneously (port 2000/chain 2 and port 3000/chain 3), validating multi-MVM routing end-to-end.

## Quick Start

```bash
# Inflow tests (Connected MVM Chain → Hub, both instances)
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-inflow.sh

# Outflow tests (Hub → Connected MVM Chain, both instances)
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-outflow.sh
```

> **Note**: These E2E tests only run on Linux (for CI). They do not work on macOS because Docker images for the test chains are not available for macOS.

## What's Tested

1. **Coordinator-Based Negotiation**: Draft submission, solver polling, and signature retrieval
2. **Intent Creation**: Creates intent on Hub with solver signature from coordinator
3. **GMP Message Delivery**: IntentRequirements delivered to connected chain via GMP relay
4. **Escrow Creation**: Creates escrow on MVM chain with locked tokens, validated against GMP requirements
5. **Intent Fulfillment**: Solver fulfills intent on Hub, FulfillmentProof sent via GMP
6. **Escrow Auto-Release**: Escrow auto-released on MVM chain upon FulfillmentProof receipt
7. **Multi-Chain Routing**: Tests run against both MVM instances sequentially, validating the solver and relay route to the correct chain by chain ID

## Integration Tests

The `coordinator-rust-integration-tests/` directory contains Rust integration tests for the coordinator (connectivity, deployment, event polling). These are automatically run by `run-tests-inflow.sh`.
