# Intent System as Escrow Mechanism

⚠️ **Important**: In this escrow system, the **trusted-gmp service is an oracle** that approves escrow conditions. The trusted-gmp service signs the `intent_id` - the signature itself is the approval. If the trusted-gmp service doesn't sign, there's no approval.

## Overview

The MVM Intent Framework provides a simple escrow system through the `intent_as_escrow.move` module. This abstraction makes it easy to lock tokens and wait for trusted-gmp approval. The actual swap conditions and logic happen off-chain or on another chain - this chain just locks tokens and awaits binary yes/no from the trusted-gmp service.

**Important**: Escrows created through `intent_as_escrow` **must** specify a reserved solver address. While the underlying `fa_intent_with_oracle` intent type supports optional reservations, escrows enforce this requirement for security (preventing signature replay attacks).

## Simple Escrow API

The `intent_as_escrow.move` module provides a clean interface for escrow functionality:

```move
// 1. Create escrow (must specify solver address)
let reservation = intent_reservation::new_reservation(solver_addr);
let escrow_intent = intent_as_escrow::create_escrow(
    requester_signer,
    offered_asset,
    approver_public_key,
    expiry_time,
    intent_id, // Intent ID from hub chain (for cross-chain matching)
    reservation, // Required - escrow must specify a solver address
);

// 2. Solver takes escrow (solver signer must match reserved solver)
let (escrowed_asset, session) = intent_as_escrow::start_escrow_session(solver, escrow_intent);

// 3. Trusted GMP signs the intent_id - signature itself is the approval
let intent_id = @0x1; // Same intent_id used when creating escrow
let approver_signature = ed25519::sign_arbitrary_bytes(&approver_secret_key, bcs::to_bytes(&intent_id));

// 4. Complete escrow (solver signer must match reserved solver)
intent_as_escrow::complete_escrow(
    solver,
    session,
    solver_payment,
    approver_signature,
);
```

## API Functions

### Core Functions

- **`create_escrow()`** - Create escrow with trusted-gmp requirement (just locks tokens). **Requires** `reservation` parameter with solver address (unlike general `fa_intent_with_oracle` intents, escrows must always be reserved).
- **`start_escrow_session()`** - Start escrow for solver. Requires solver signer that matches the reserved solver address.
- **`complete_escrow()`** - Complete with trusted-gmp signature. Requires solver signer that matches the reserved solver address. The signature itself is the approval - trusted-gmp service signs the `intent_id`.
- **`revoke_escrow()`** - Revoke and return assets to requester (not available - escrows are non-revocable)

## Escrow Lifecycle

### 1. Creation

Requester locks tokens and specifies:

- Which trusted-gmp service can approve
- When escrow expires
- **Which solver address will receive funds** (required for escrows)
- (No swap parameters - actual logic is off-chain)

### 2. Solver Participation

Solver takes the escrowed assets (actual swap logic happens off-chain)

- The solver address must be specified at escrow creation
- Only the specified solver can start the session (enforced on-chain)

### 3. Trusted GMP Verification

Trusted-gmp service:

- Monitors conditions off-chain or on another chain
- Signs the `intent_id` to approve the escrow
- Provides Ed25519 signature (signature itself is the approval)

### 4. Completion

If trusted-gmp signature verifies correctly, tokens are released to solver

### 5. Fallback

If escrow expires, requester can reclaim tokens

## Architecture

The escrow system is deployed on a single MVM chain. The trusted-gmp service (oracle) monitors escrow conditions (possibly on other chains) and signs the `intent_id` to approve the escrow release. The signature itself is the approval.

## Security Features

- **Timelock**: Escrow expires automatically
- **Trusted GMP Verification**: Only authorized trusted-gmp service can approve
- **Signature Verification**: Ed25519 signatures prevent forgery
- **Solver Reservation**: All escrows must specify a solver address at creation, preventing unauthorized claims and signature replay attacks
- **Event Transparency**: All actions are auditable

## Usage Examples

### Simple Token Escrow

```move
// Requester locks TokenA and waits for approver approval
let reservation = intent_reservation::new_reservation(solver_addr);
let escrow = intent_as_escrow::create_escrow(
    requester_signer,
    token_a_asset,
    approver_public_key,
    expiry_time,
    intent_id, // Intent ID from hub chain
    reservation, // Required - must specify solver address
);
```

### Trusted GMP Approval

```move
// Trusted GMP monitors conditions and signs the intent_id:
let intent_id = @0x1; // Same intent_id used when creating escrow
let approver_signature = ed25519::sign_arbitrary_bytes(&approver_key, bcs::to_bytes(&intent_id));

// Escrow releases tokens to solver (signature itself is the approval)
intent_as_escrow::complete_escrow(solver, session, payment, approver_signature);
```
