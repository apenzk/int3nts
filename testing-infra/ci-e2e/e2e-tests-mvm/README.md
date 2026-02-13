# Move VM E2E Tests

Tests Move VM-only cross-chain intent framework: intents on Hub and escrows on Chain 2 (connected).

## Quick Start

```bash
# Inflow tests (Connected Chain → Hub)
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-inflow.sh

# Outflow tests (Hub → Connected Chain)
./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-outflow.sh
```

> **Note**: These E2E tests only run on Linux (for CI). They do not work on macOS because Docker images for the test chains are not available for macOS.

## What's Tested

1. **Coordinator-Based Negotiation**: Draft submission, solver polling, and signature retrieval
2. **Intent Creation**: Creates intent on Hub with solver signature from coordinator
3. **GMP Message Delivery**: IntentRequirements delivered to connected chain via GMP relay
4. **Escrow Creation**: Creates escrow on Chain 2 with locked tokens, validated against GMP requirements
5. **Intent Fulfillment**: Solver fulfills intent on Hub, FulfillmentProof sent via GMP
6. **Escrow Auto-Release**: Escrow auto-released on Chain 2 upon FulfillmentProof receipt

## Integration Tests

The `coordinator-rust-integration-tests/` directory contains Rust integration tests for the coordinator (connectivity, deployment, event polling). These are automatically run by `run-tests-inflow.sh`.
