# EVM E2E Tests

Tests mixed-chain intent framework: intents on Move VM Hub with escrows on two independent EVM chains.

Two Hardhat instances run simultaneously (port 2000/chain 2 and port 3000/chain 3), validating multi-EVM routing end-to-end.

## Quick Start

```bash
# Inflow tests (Connected EVM Chain → Hub, both instances)
./testing-infra/ci-e2e/e2e-tests-evm/run-tests-inflow.sh

# Outflow tests (Hub → Connected EVM Chain, both instances)
./testing-infra/ci-e2e/e2e-tests-evm/run-tests-outflow.sh
```

> **Note**: These E2E tests only run on Linux (for CI). They do not work on macOS because Docker images for the test chains are not available for macOS.

## What's Tested

1. **Coordinator-Based Negotiation**: Draft submission, solver polling, and signature retrieval
2. **Intent Creation**: Creates intent on Move VM Hub with solver signature from coordinator
3. **GMP Message Delivery**: IntentRequirements delivered to connected chain via GMP relay
4. **Escrow Creation**: Creates escrow on EVM chain with locked tokens, validated against GMP requirements
5. **Intent Fulfillment**: Solver fulfills intent on Hub, FulfillmentProof sent via GMP
6. **Escrow Auto-Release**: Escrow auto-released on EVM chain upon FulfillmentProof receipt
7. **Multi-Chain Routing**: Tests run against both EVM instances sequentially, validating the solver and relay route to the correct chain by chain ID
