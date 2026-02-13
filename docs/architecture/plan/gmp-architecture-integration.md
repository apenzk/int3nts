# GMP Architecture Integration Design

**Status:** Complete
**Date:** 2026-01-28
**Purpose:** Document how GMP messaging works across the system — message flows, contract integration points, relay design, and environment configuration.

---

## Message Flow Diagrams

### Outflow: Hub → Connected Chain

```text
1. Hub: Requester creates outflow intent (locks tokens)
        → contract calls gmpSend() with IntentRequirements message
        → message contains: intent_id, recipient, amount, token, authorized_solver

2. Connected: Validation contract receives IntentRequirements via gmpReceive()
              → stores requirements in state (keyed by intent_id)

3. Connected: Authorized solver calls validationContract.fulfillIntent(intent_id, token, amount)
              Within this single solver-initiated transaction:
              a. Token transfer executes:
                 → EVM: solver calls approve(validationContract, exactAmount) beforehand,
                   fulfillIntent() executes transferFrom
                 → SVM: solver signs the transaction, program executes CPI transfer
                   using solver's signer authority
              b. Contract validates: amount, token, solver match stored requirements
              c. Contract forwards tokens to requester address
              d. Contract calls gmpSend() with FulfillmentProof message

4. Hub: Intent contract receives FulfillmentProof via gmpReceive()
        → validates intent_id exists and is active
        → releases locked tokens to solver
        → deletes intent
```

### Inflow: Connected Chain → Hub

```text
1. Hub: Requester creates inflow intent
        → contract calls gmpSend() with IntentRequirements message
        → message contains: intent_id, required_amount, required_token, authorized_solver

2. Connected: Escrow contract receives IntentRequirements via gmpReceive()
              → stores requirements in state (keyed by intent_id)

3. Connected: Requester creates escrow
              → contract validates requirements exist for this intent_id
              → contract validates escrow params match requirements
              → reverts if no requirements or mismatch
              → escrow created, tokens locked
              → contract calls gmpSend() with EscrowConfirmation message

4. Hub: Intent contract receives EscrowConfirmation via gmpReceive()
        → marks intent as escrow-confirmed (enables fulfillment)

5. Hub: Solver calls fulfill_inflow_intent()
        → provides desired tokens to requester
        → contract calls gmpSend() with FulfillmentProof message

6. Connected: Escrow contract receives FulfillmentProof via gmpReceive()
              → automatically releases escrowed tokens to reserved_solver
```

### Message Handling

These apply to all `gmpReceive()` handlers in both flows:

- **Idempotency**: Each message carries intent_id + step number. If state is already updated for that step, the duplicate is ignored.
- **Ordering**: Step numbers enforce ordering — step N can only be processed if step N-1 is complete.
- **Failure/timeout**: Existing expiry mechanisms handle incomplete flows. Intent/escrow expires, requester cancels and recovers funds.

---

## Integration Points: Contracts

### MVM Hub Contracts

**`fa_intent_outflow.move`**:

- `create_outflow_intent()`: After creating intent, calls `gmpSend()` with `IntentRequirements`
- `receive_fulfillment_proof()`: Called by `gmpReceive()`, releases locked tokens to solver

**`fa_intent_inflow.move`**:

- `create_inflow_intent()`: After creating intent, calls `gmpSend()` with `IntentRequirements`
- `receive_escrow_confirmation()`: Called by `gmpReceive()`, marks intent as escrow-confirmed
- `fulfill_inflow_intent()`: Gated on escrow confirmation; after fulfillment, calls `gmpSend()` with `FulfillmentProof`

**`intent_inflow_escrow.move`** (MVM as connected chain):

- `receive_intent_requirements()`: Called by `gmpReceive()`, stores requirements
- `create_escrow()`: Validates against stored requirements before allowing creation
- `receive_fulfillment_proof()`: Called by `gmpReceive()`, auto-releases escrow

**`gmp/intent_gmp.move`** - GMP endpoint:

- `gmp_send()`: Encode and send message via integrated GMP endpoint
- `deliver_message()`: Entry point called by relay, dispatches to handlers
- Remote GMP endpoint verification

### SVM Connected Chain

**`intent_escrow` program**:

- `gmp_receive` instruction for requirements and fulfillment proof
- On-chain validation in `create_escrow`

**`outflow-validator` program**:

- `gmp_receive`: Stores intent requirements from hub
- `fulfill_intent`: Solver calls this; validates, transfers, sends GMP proof

**`integrated-gmp-endpoint` program**:

- `send`: Emits `MessageSent` event
- `deliver_message`: Relay delivers messages

### EVM Connected Chain

**`IntentInflowEscrow.sol`** - Inflow escrow with GMP validation
**`OutflowValidator.sol`** - Outflow validation contract
**`NativeGmpEndpoint.sol`** - Integrated GMP endpoint

---

## Integrated-GMP Relay Design

The integrated GMP relay handles message delivery in all environments. It watches for `MessageSent` events on integrated GMP endpoints and delivers messages to destination chains.

### How It Works

```text
┌──────────┐     ┌──────────────────────┐     ┌──────────┐
│  MVM Hub │     │   Integrated-GMP     │     │   SVM    │
│  (GMP    │────>│   Relay              │────>│  (GMP    │
│ endpoint)│     │                      │     │ endpoint)│
│          │<────│  Watches MessageSent │<────│          │
└──────────┘     │  Calls deliver_msg   │     └──────────┘
                 └──────────────────────┘
```

1. Contracts call `gmpSend()` on integrated GMP endpoint
2. Integrated GMP endpoint emits `MessageSent` event
3. Integrated-GMP polls for `MessageSent` events on all chains
4. Integrated-GMP calls `deliver_message()` on destination chain's integrated GMP endpoint
5. Integrated GMP endpoint calls `gmpReceive()` on destination contract
6. Destination contract processes message normally

### Relay Characteristics

- **Watches**: `MessageSent` events on integrated GMP endpoints (MVM, SVM, EVM)
- **Delivers**: Calls `deliver_message()` / `gmpReceive()` on destination
- **Needs**: Funded operator wallet per chain (pays gas for delivery)
- **Config**: Chain RPCs, GMP endpoint addresses, operator keys
- **Polling**: Configurable interval (default 500ms for fast CI)
- **Validation**: None — contracts validate on-chain. Relay is a pure message transport.

### Contracts Stay Identical

Contracts use the same GMP interface in all environments:

```text
gmp_send(endpoint, dst_chain_id, destination, payload);
```

---

## Environment Matrix

| Environment | MVM Hub | SVM Connected | EVM Connected | GMP Delivery |
|-------------|---------|---------------|---------------|--------------|
| **Local/CI** | Integrated GMP endpoint | Integrated GMP endpoint | Integrated GMP endpoint | Integrated GMP relay |
| **Testnet** | Integrated GMP endpoint | Integrated GMP endpoint | Integrated GMP endpoint | Integrated GMP relay |
| **Mainnet** | Integrated GMP endpoint | Integrated GMP endpoint | Integrated GMP endpoint | Integrated GMP relay |

> **Note:** All environments use integrated GMP. Contracts follow GMP conventions so that future external GMP provider integration is a configuration change (swap endpoint address).

---

## What Triggers gmpSend()?

Contracts call `gmpSend()` as part of their normal execution. Every message is triggered by a user transaction (requester or solver).

### Who Triggers Each Message

| Message | Direction | Triggered By | When |
|---------|-----------|--------------|------|
| `IntentRequirements` | Hub → Connected | Hub contract | On intent creation (`create_outflow_intent()` / `create_inflow_intent()`) |
| `EscrowConfirmation` | Connected → Hub | Connected escrow contract | On escrow creation (`create_escrow()`) |
| `FulfillmentProof` (outflow) | Connected → Hub | Connected validation contract | On solver fulfillment (`fulfill_intent()`) |
| `FulfillmentProof` (inflow) | Hub → Connected | Hub contract | On solver fulfillment (`fulfill_inflow_intent()`) |

### Gas Costs

The caller of the transaction pays gas for `gmpSend()`. This means:

- **Requester pays** for `IntentRequirements` (part of intent creation tx)
- **Requester pays** for `EscrowConfirmation` (part of escrow creation tx)
- **Solver pays** for `FulfillmentProof` (part of fulfillment tx)

With integrated GMP, there are no third-party GMP fees. The relay operator pays gas for delivery on the destination chain.

---

## Architecture Overview

```text
ALL ENVIRONMENTS:
┌──────────────────┐      Integrated GMP Relay       ┌──────────────────┐
│    MVM Hub        │ ◄─── deliver_message ────► │  SVM/EVM         │
│  Intent contracts │                             │  Escrow/Validator │
│  + GMP endpoint   │                             │  + GMP endpoint   │
└──────────────────┘                             └──────────────────┘
        │
        │ Coordinator (read-only)
        │ Reads hub state only
        │ Event monitoring, UX
```

**Off-chain components:**

- Coordinator (event monitoring, negotiation, UX, readiness tracking)
  - Monitors IntentRequirementsReceived events on connected chains
  - Provides `ready_on_connected_chain` flag via API
  - Does NOT track full GMP message lifecycle (MessageSent/MessageDelivered)
- On-chain contracts (same code, different GMP endpoint config)
- Frontend / solver bots query coordinator API for readiness status instead of polling connected chains directly
