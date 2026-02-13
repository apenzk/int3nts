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
// Create an escrow and deposit funds atomically (expiry is contract-defined)
// reservedSolver: Required solver address that will receive funds (must not be address(0))
function createEscrow(uint256 intentId, address token, uint256 amount, address reservedSolver) external

// Claim funds (after FulfillmentProof received via GMP)
// Funds always go to reservedSolver address
function claim(uint256 intentId, bytes memory signature) external

// Cancel escrow and reclaim funds (requester only, after expiry)
function cancel(uint256 intentId) external

// Get escrow data
function getEscrow(uint256 intentId) external view returns (address, address, uint256, bool, uint256, address)
```

### Events

- `EscrowInitialized(uint256 indexed intentId, address indexed escrow, address indexed requester, address token, address reservedSolver)`
- `DepositMade(uint256 indexed intentId, address indexed requester, uint256 amount, uint256 total)` - `requester` is the requester who created the escrow
- `EscrowClaimed(uint256 indexed intentId, address indexed recipient, uint256 amount)`
- `EscrowCancelled(uint256 indexed intentId, address indexed requester, uint256 amount)`

## Quick Start

See the [component README](../../intent-frameworks/evm/README.md) for quick start commands.

## Security Considerations

- GMP message verification: Only messages from authorized GMP endpoints accepted
- Intent ID binding: Requirements keyed by intent_id prevent cross-escrow attacks
- Reentrancy protection: Uses OpenZeppelin's SafeERC20
- Access control: Only requester can cancel (after expiry)
- Solver reservation: Required at creation, prevents unauthorized recipients
- On-chain validation: All requirement matching happens on-chain

## Testing

```bash
npx hardhat test
```

Tests cover escrow initialization, deposits, claiming, cancellation, expiry enforcement, GMP message handling, and error cases.

Test accounts: Hardhat provides 20 accounts (10000 ETH each). Account 0 is deployer/approver, Account 1 is requester, Account 2 is solver. Private keys are deterministic from mnemonic: `test test test test test test test test test test test junk`
