# EVM E2E Tests

Tests mixed-chain intent framework: intents on Move VM Hub and escrows on EVM Chain 3.

## Quick Start

```bash
# Inflow tests (Connected EVM Chain → Hub)
./testing-infra/ci-e2e/e2e-tests-evm/run-tests-inflow.sh

# Outflow tests (Hub → Connected EVM Chain)
./testing-infra/ci-e2e/e2e-tests-evm/run-tests-outflow.sh
```

> **Note**: These E2E tests only run on Linux (for CI). They do not work on macOS because Docker images for the test chains are not available for macOS.

## What's Tested

1. **Coordinator-Based Negotiation**: Draft submission, solver polling, and signature retrieval
2. **Intent Creation**: Creates intent on Move VM Hub with solver signature from coordinator
3. **GMP Message Delivery**: IntentRequirements delivered to connected chain via GMP relay
4. **Escrow Creation**: Creates escrow on EVM Chain 3 with locked tokens, validated against GMP requirements
5. **Intent Fulfillment**: Solver fulfills intent on Hub, FulfillmentProof sent via GMP
6. **Escrow Auto-Release**: Escrow auto-released on EVM Chain 3 upon FulfillmentProof receipt
