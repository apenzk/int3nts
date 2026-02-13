# Solver Guide: GMP Integration

This guide explains how solvers interact with the GMP-based cross-chain intent system.

## Overview

Solvers no longer interact with the integrated-gmp service directly. Instead:

- **Coordinator** is the single API for event polling and negotiation
- **On-chain contracts** handle all validation via GMP messages
- **GMP relay** operates invisibly in the background

## Inflow Flow (Solver Perspective)

Tokens locked on connected chain, solver fulfills on hub.

### Steps

1. **Poll coordinator** for pending inflow intents (`GET /events`)
2. **Wait for escrow confirmation** -- poll hub chain `is_escrow_confirmed(intent_id)`
   - The requester creates an escrow on the connected chain
   - GMP delivers EscrowConfirmation to hub automatically
   - Until confirmed, the solver cannot fulfill
3. **Fulfill on hub** -- call `fulfill_inflow_intent(intent_id)` on hub chain
   - Hub sends FulfillmentProof via GMP to connected chain
4. **Wait for auto-release** -- the connected chain automatically releases escrowed funds to the solver when FulfillmentProof arrives via GMP
   - No manual claim needed; solver monitors for release event

## Outflow Flow (Solver Perspective)

Tokens locked on hub, solver fulfills on connected chain.

### Steps

1. **Poll coordinator** for pending outflow intents (`GET /events`)
2. **Check readiness** -- use coordinator's `ready_on_connected_chain` flag
   - IntentRequirements must arrive on connected chain before solver can act
   - Alternatively, poll connected chain `has_outflow_requirements(intent_id)` directly
3. **Fulfill on connected chain** -- call the validation contract:
   - MVM: `fulfill_intent(solver, intent_id, token_metadata)`
   - EVM: `fulfillIntent(intentId, token)`
   - SVM: `fulfill_intent` instruction
   - The contract validates against stored IntentRequirements, transfers tokens to requester, and sends FulfillmentProof via GMP
4. **Wait for proof delivery** -- poll hub `is_fulfillment_proof_received(intent_id)`
5. **Claim on hub** -- call `fulfill_outflow_intent(intent)` to claim locked tokens

## Token Approval (Outflow, One-Time Setup)

For outflow fulfillment, the solver must approve the validation contract to spend tokens:

- **EVM**: Call `token.approve(validatorContractAddr, MAX_UINT256)` once per token
- **SVM**: Create associated token account and delegate authority
- **MVM**: No explicit approval needed (Move's resource model)

This is a one-time operation per token per chain.

## Timing and Polling

GMP message delivery is not instantaneous. Solvers must account for delivery latency:

| Event | Typical Latency | Polling Interval |
| ----- | --------------- | ---------------- |
| IntentRequirements delivery | 2-10 seconds | 2 seconds |
| EscrowConfirmation delivery | 2-10 seconds | 2 seconds |
| FulfillmentProof delivery | 2-10 seconds | 2 seconds |

### Expiry Handling

- Always check `expiry_time` before attempting fulfillment
- If GMP delivery hasn't occurred and expiry is approaching, skip the intent
- On-chain expiry mechanisms handle stuck intents (no manual intervention needed)

### Idempotency

- GMP messages are idempotent -- duplicate deliveries are safely ignored
- Contracts return `E_ALREADY_DELIVERED` for duplicate messages
- Solvers can retry without risk of double-fulfillment

## Configuration

Solvers only need coordinator configuration:

```toml
[coordinator]
base_url = "http://localhost:3333"
```

No integrated-gmp URL is needed. The solver never calls the relay directly.

## Coordinator API Endpoints Used

| Endpoint | Purpose |
| -------- | ------- |
| `GET /events` | Poll for new intents, escrows, fulfillments |
| `GET /events?intent_id=0x...` | Get events for specific intent |
| `POST /negotiate` | Submit negotiation offers |

The `ready_on_connected_chain` flag in event responses indicates whether GMP has delivered IntentRequirements to the connected chain.
