# SVM Intent Framework

Escrow and validation programs for cross-chain intents on Solana, using GMP for cross-chain message authentication.

## Overview

### Inflow Escrow (`intent_inflow_escrow`)

Secure escrow program for inflow intents:

- Requesters deposit SPL tokens into escrows tied to intent IDs
- Escrow creation is validated against IntentRequirements delivered via GMP
- Escrow auto-releases to reserved solver when FulfillmentProof arrives via GMP
- Requesters can cancel and reclaim funds after expiry

### Outflow Validator (`intent-outflow-validator`)

Validation program for outflow intents:

- Receives IntentRequirements from hub via GMP
- Solver calls `fulfill_intent` -- program validates parameters, pulls tokens, transfers to requester
- Sends FulfillmentProof back to hub via GMP

## Architecture

GMP messages handle cross-chain authentication. Programs validate requirements on-chain.

Inflow flow:

1. Hub sends IntentRequirements via GMP to connected chain
2. Requester creates escrow -- program validates against stored requirements
3. Escrow sends EscrowConfirmation back to hub via GMP
4. Solver fulfills on hub -- FulfillmentProof sent via GMP to connected chain
5. Escrow auto-releases to reserved solver

Outflow flow:

1. Hub sends IntentRequirements via GMP to connected chain
2. Solver calls validation program -- validates, transfers tokens, sends FulfillmentProof via GMP
3. Hub receives proof -- releases locked tokens to solver

## Program Interface

### Instructions

```rust
// Initialize program with GMP config
fn initialize(ctx: Context<Initialize>, approver: Pubkey) -> Result<()>

// Receive GMP message (IntentRequirements or FulfillmentProof)
fn gmp_receive(src_chain_id: u32, remote_gmp_endpoint_addr: [u8; 32], payload: Vec<u8>)

// Create escrow and deposit tokens atomically
// Validates against stored IntentRequirements
fn create_escrow(ctx: Context<CreateEscrow>, intent_id: [u8; 32], amount: u64) -> Result<()>

// Claim funds (after FulfillmentProof received via GMP, no signature required)
fn claim(ctx: Context<Claim>, intent_id: [u8; 32]) -> Result<()>

// Cancel escrow and reclaim funds (requester only, after expiry)
fn cancel(ctx: Context<Cancel>, intent_id: [u8; 32]) -> Result<()>
```

### Events

- `EscrowInitialized` - Emitted when escrow is created with funds
- `EscrowClaimed` - Emitted when solver claims funds
- `EscrowCancelled` - Emitted when requester cancels after expiry

### Errors

- `EscrowAlreadyClaimed` - Escrow has already been claimed
- `EscrowDoesNotExist` - Intent ID doesn't match escrow
- `NoDeposit` - No funds in escrow
- `UnauthorizedRequester` - Caller is not the requester
- `EscrowExpired` - Cannot claim after expiry
- `EscrowNotExpiredYet` - Cannot cancel before expiry
- `RequirementsNotFound` - No IntentRequirements stored for this intent_id
- `AmountMismatch` - Escrow amount doesn't match requirements

## Quick Start

See the [component README](../../intent-frameworks/svm/README.md) for quick start commands.

## Security Considerations

- GMP message verification: Only messages from authorized GMP endpoints accepted
- Remote endpoint verification: Source chain and address validated against stored config
- Intent ID binding: Requirements keyed by intent_id prevent cross-escrow attacks
- PDA authority: Escrow vault is controlled by escrow PDA
- Access control: Only requester can cancel (after expiry)
- Solver reservation: Required at creation, prevents unauthorized recipients
- On-chain validation: All requirement matching happens on-chain

## Testing

```bash
# Build and run tests (handles dependencies and keypair setup)
./scripts/test.sh
```

Tests cover escrow initialization, deposits, claiming, cancellation, expiry enforcement, GMP message handling, and error cases.

See [intent-frameworks/svm/README.md](../../intent-frameworks/svm/README.md) for toolchain constraints and workarounds.
