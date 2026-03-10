# EVM Intent Framework

Escrow and validation contracts for cross-chain intents on EVM chains, using GMP for cross-chain message authentication.

## Overview

### Inflow Escrow (`IntentInflowEscrow`)

Secure escrow system for inflow intents:

- Requesters deposit ERC20 tokens into escrows tied to intent IDs
- Escrow creation is validated against IntentRequirements delivered via GMP
- Escrow auto-releases to reserved solver when FulfillmentProof arrives via GMP
- Requesters can cancel and reclaim funds after expiry

### Outflow Validator (`IntentOutflowValidator`)

Validation contract for outflow intents:

- Receives IntentRequirements from hub via GMP
- Solver calls `fulfillIntent()` -- contract validates parameters, pulls tokens, transfers to requester
- Sends FulfillmentProof back to hub via GMP

## Architecture

GMP messages handle cross-chain authentication. Contracts validate requirements on-chain.

Inflow flow:

1. Hub sends IntentRequirements via GMP to connected chain
2. Requester creates escrow -- contract validates against stored requirements
3. Escrow sends EscrowConfirmation back to hub via GMP
4. Solver fulfills on hub -- FulfillmentProof sent via GMP to connected chain
5. Escrow auto-releases to reserved solver

Outflow flow:

1. Hub sends IntentRequirements via GMP to connected chain
2. Solver calls validation contract -- validates, transfers tokens, sends FulfillmentProof via GMP
3. Hub receives proof -- releases locked tokens to solver

## Contract Interface

### Inflow Escrow Functions

```solidity
// Create an escrow and deposit funds atomically
// Validates amount, token, requester, and expiry against stored IntentRequirements
function createEscrowWithValidation(bytes32 intentId, address token, uint64 amount) external

// Cancel escrow and return funds to requester (admin only, after expiry)
function cancel(bytes32 intentId) external

// Receive FulfillmentProof from hub via GMP and auto-release escrow to solver
// Called by GMP endpoint -- not directly by users
function receiveFulfillmentProof(uint32 srcChainId, bytes32 remoteGmpEndpointAddr, bytes calldata payload) external

// Get escrow details
function getEscrow(bytes32 intentId) external view returns (StoredEscrow memory)
```

### Events

- `IntentRequirementsReceived(bytes32 indexed intentId, uint32 srcChainId, bytes32 requesterAddr, uint64 amountRequired, bytes32 tokenAddr, bytes32 solverAddr, uint64 expiry)`
- `EscrowCreated(bytes32 indexed intentId, bytes32 escrowId, address indexed requester, uint64 amount, address indexed token, bytes32 reservedSolver, uint64 expiry)`
- `EscrowConfirmationSent(bytes32 indexed intentId, bytes32 escrowId, uint64 amountEscrowed, uint32 dstChainId)`
- `FulfillmentProofReceived(bytes32 indexed intentId, uint32 srcChainId, bytes32 solverAddr, uint64 amountFulfilled, uint64 timestamp)`
- `EscrowReleased(bytes32 indexed intentId, address indexed solver, uint64 amount)`
- `EscrowCancelled(bytes32 indexed intentId, address indexed requester, uint64 amount)`

## Quick Start

See the [component README](../../intent-frameworks/evm/README.md) for quick start commands.

## Security Considerations

- GMP message verification: Only messages from authorized GMP endpoints accepted
- Intent ID binding: Requirements keyed by intent_id prevent cross-escrow attacks
- Reentrancy protection: Uses OpenZeppelin's SafeERC20
- Access control: Only admin can cancel (after expiry), funds return to original requester
- Solver reservation: Required at creation, prevents unauthorized recipients
- On-chain validation: All requirement matching happens on-chain

## Testing

```bash
npx hardhat test
```

Tests cover escrow creation, fulfillment proof handling, cancellation, expiry enforcement, GMP message handling, and error cases.

Test accounts: Hardhat provides 20 accounts (10000 ETH each). Account 0 is deployer/approver, Account 1 is requester, Account 2 is solver. Private keys are deterministic from mnemonic: `test test test test test test test test test test test junk`
