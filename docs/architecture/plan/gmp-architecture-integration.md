# GMP Architecture Integration Design

**Status:** Draft
**Date:** 2026-01-28
**Purpose:** Map out exactly how LZ GMP replaces trusted-gmp signatures in our existing architecture.

---

## Current System Summary

Today, cross-chain approval works like this:

```text
Trusted-GMP (off-chain) validates → Signs intent_id → Contract checks signature → Releases funds
```

The trusted-gmp service holds private keys (Ed25519 + ECDSA) and generates approval signatures. Contracts on each chain verify these signatures before releasing funds.

**Key contracts:**

- **MVM Hub**: `fa_intent_outflow.move`, `fa_intent_inflow.move`, `intent_escrow.move`
- **SVM Connected**: `intent_escrow` program (escrow with Ed25519 signature verification)
- **EVM Connected**: `IntentInflowEscrow.sol` (escrow with ECDSA signature verification)

**Signature as approval:**

- MVM: Ed25519 signature over BCS-encoded `intent_id`
- SVM: Ed25519 signature over raw 32-byte `intent_id`
- EVM: ECDSA signature over `keccak256(abi.encodePacked(intentId))`

---

## GMP Replacement: What Changes

With GMP, the approval mechanism changes from **"trusted-gmp signs intent_id"** to **"on-chain contract receives GMP message confirming the cross-chain action"**.

```text
Before: Trusted-GMP signs → Contract verifies signature
After:  Source contract sends GMP message → Destination contract receives and acts
```

### What Moves On-Chain

| Currently in Trusted-GMP | Moves to | How |
|--------------------------|----------|-----|
| Inflow: validate escrow matches intent | Connected chain escrow contract | Contract validates requirements received via GMP before allowing escrow creation |
| Inflow: approve escrow release after hub fulfillment | Hub intent contract | Hub sends GMP message to connected chain on fulfillment → escrow auto-releases |
| Outflow: validate connected chain transfer | Connected chain validation contract | New contract validates solver's transfer and sends GMP confirmation to hub |
| Outflow: approve hub intent release | Hub intent contract | Hub receives GMP fulfillment proof → auto-releases locked tokens |
| Signature generation (Ed25519/ECDSA) | Eliminated | GMP message authentication replaces signatures |

### What Stays Off-Chain

| Component | Stays because |
|-----------|--------------|
| Coordinator event monitoring (hub only) | UX only, not security-critical. Hub has full state via GMP messages. |
| Coordinator negotiation API | Application logic, not security-critical |
| Coordinator event caching | Convenience, not security-critical |
| Trusted-GMP (local/CI only) | Relays GMP messages via native GMP endpoints |

---

## Message Flow Diagrams

### Outflow: Hub → Connected Chain

**Current flow (trusted-gmp signs):**

```text
1. Hub: Requester creates outflow intent (locks tokens)
         → emits OracleLimitOrderEvent
2. Solver: Sees intent via coordinator
3. Connected: Solver does arbitrary transfer to requester
              (ERC20 transfer / SPL transfer / FA transfer)
              Includes intent_id in tx metadata
4. Solver: Calls POST /validate-outflow-fulfillment on trusted-gmp
5. Trusted-GMP: Queries tx, validates (recipient, amount, token, solver)
6. Trusted-GMP: Signs intent_id → returns signature
7. Hub: Solver calls fulfill_outflow_intent(signature)
        → hub verifies signature, releases locked tokens to solver
```

**GMP flow (all environments):**

```text
1. Hub: Requester creates outflow intent (locks tokens)
        → contract calls lzSend() with IntentRequirements message
        → message contains: intent_id, recipient, amount, token, authorized_solver

2. Connected: Validation contract receives IntentRequirements via lzReceive()
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
              d. Contract calls lzSend() with FulfillmentProof message

4. Hub: Intent contract receives FulfillmentProof via lzReceive()
        → validates intent_id exists and is active
        → releases locked tokens to solver
        → deletes intent
```

**Key differences:**

- Solver no longer does arbitrary transfer; must call validation contract
- Solver actively initiates the token transfer (EVM: approve exact amount + transferFrom; SVM: signer authority, no approval needed)
- No off-chain signature needed; GMP message IS the proof
- Hub release is automatic on GMP message receipt

### Inflow: Connected Chain → Hub

**Current flow (trusted-gmp signs):**

```text
1. Hub: Requester creates inflow intent
        → emits LimitOrderEvent
2. Connected: Requester creates escrow (locks tokens, reserved for solver)
              → emits EscrowInitialized
3. Hub: Solver calls fulfill_inflow_intent()
        → provides desired tokens to requester on hub
        → emits LimitOrderFulfillmentEvent
4. Trusted-GMP: Monitors hub fulfillment event
5. Trusted-GMP: Validates escrow matches intent (amount, token, solver, chain)
6. Trusted-GMP: Signs intent_id → caches signature
7. Connected: Solver calls escrow.claim(signature)
              → escrow verifies signature, releases to reserved_solver
```

**GMP flow (all environments):**

```text
1. Hub: Requester creates inflow intent
        → contract calls lzSend() with IntentRequirements message
        → message contains: intent_id, required_amount, required_token, authorized_solver

2. Connected: Escrow contract receives IntentRequirements via lzReceive()
              → stores requirements in state (keyed by intent_id)

3. Connected: Requester creates escrow
              → contract validates requirements exist for this intent_id
              → contract validates escrow params match requirements
              → reverts if no requirements or mismatch
              → escrow created, tokens locked
              → contract calls lzSend() with EscrowConfirmation message

4. Hub: Intent contract receives EscrowConfirmation via lzReceive()
        → marks intent as escrow-confirmed (enables fulfillment)

5. Hub: Solver calls fulfill_inflow_intent()
        → provides desired tokens to requester
        → contract calls lzSend() with FulfillmentProof message

6. Connected: Escrow contract receives FulfillmentProof via lzReceive()
              → automatically releases escrowed tokens to reserved_solver
```

**Key differences:**

- Escrow creation now validated on-chain (requirements received via GMP)
- Hub fulfillment gated on escrow confirmation (prevents solver fulfilling without escrow)
- Escrow release is automatic on GMP fulfillment proof receipt
- No off-chain signature needed

### Message Handling

These apply to all `lzReceive()` handlers in both flows:

- **Idempotency**: Each message carries intent_id + step number. If state is already updated for that step, the duplicate is ignored.
- **Ordering**: Step numbers enforce ordering — step N can only be processed if step N-1 is complete.
- **Failure/timeout**: Existing expiry mechanisms handle incomplete flows. Intent/escrow expires, requester cancels and recovers funds.

---

## Integration Points: Existing Contracts

### MVM Hub Contracts

**`fa_intent_outflow.move`** - Needs GMP hooks:

- `create_outflow_intent()`: After creating intent, call `lzSend()` with `IntentRequirements`
- New: `receive_fulfillment_proof()`: Called by `lzReceive()`, releases locked tokens to solver
- `fulfill_outflow_intent()`: Remove signature verification; release now handled by `receive_fulfillment_proof()`
- `ApproverConfig`: Replace approver public key with GMP endpoint address

**`fa_intent_inflow.move`** - Needs GMP hooks:

- `create_inflow_intent()`: After creating intent, call `lzSend()` with `IntentRequirements`
- New: `receive_escrow_confirmation()`: Called by `lzReceive()`, marks intent as escrow-confirmed
- `fulfill_inflow_intent()`: Gate on escrow confirmation before allowing fulfillment; after fulfillment, call `lzSend()` with `FulfillmentProof`

**`intent_inflow_escrow.move`** (MVM as connected chain) - Needs GMP hooks:

- New: `receive_intent_requirements()`: Called by `lzReceive()`, stores requirements
- `create_escrow()`: Validate against stored requirements before allowing creation
- New: `receive_fulfillment_proof()`: Called by `lzReceive()`, auto-releases escrow
- `complete_escrow()`: Remove signature verification; release now handled by `receive_fulfillment_proof()`

**New: `layerzero/oapp.move`** - LZ OApp base:

- `lz_send()`: Encode and send message via LZ endpoint
- `lz_receive()`: Entry point called by LZ endpoint, dispatches to handlers
- Trusted remote verification

**New: `gmp/intent_gmp.move`** - Native GMP endpoint:

- `send()`: Emits event (no real cross-chain)
- `deliver_message()`: Trusted-GMP calls this to relay messages

### SVM Connected Chain

**`intent_escrow` program** - Modify to use GMP:

- Add `lz_receive` instruction for requirements and fulfillment proof
- Add on-chain validation in `create_escrow`
- Remove signature verification in `claim`

**New: `outflow-validator` program** - For outflow validation:

- `lz_receive`: Stores intent requirements from hub
- `fulfill_intent`: Solver calls this; validates, transfers, sends GMP proof

**New: `native-gmp-endpoint` program** - Native GMP endpoint:

- `send`: Emits `MessageSent` event
- `deliver_message`: Trusted-GMP relays messages

### EVM Connected Chain

**`IntentInflowEscrow.sol`** - Modify to use GMP (same approach as SVM)

**New: `OutflowValidator.sol`** - For outflow validation
**New: `NativeGmpEndpoint.sol`** - Native GMP endpoint

### Decision: New Contracts vs Modify Existing

**Decision: Modify existing contracts to use GMP.** The signature-based approach is being fully replaced, not maintained alongside. There is no dual-mode support.

Rationale:

- Single code path — no mode flags or conditional logic
- Existing signature verification code gets removed, not preserved
- All environments (local/CI, testnet, mainnet) use the same GMP contract interface
- Local/CI uses native GMP endpoints with trusted-GMP for message relay

---

## Trusted-GMP Relay Design

In production, LZ handles message delivery. In local/CI, trusted-GMP relays messages between native GMP endpoints.

### How It Works

```text
                    Local/CI Environment
┌──────────┐     ┌──────────────────────┐     ┌──────────┐
│  MVM Hub │     │   Trusted-GMP        │     │   SVM    │
│  (local  │────>│   Relay Mode         │────>│  (local  │
│   GMP    │     │                      │     │   GMP    │
│ endpoint)│<────│  Watches MessageSent  │<────│ endpoint)│
└──────────┘     │  Calls deliver_msg   │     └──────────┘
                 └──────────────────────┘
```

1. Contracts call `lzSend()` on native GMP endpoint
2. Native GMP endpoint emits `MessageSent` event (no real cross-chain)
3. Trusted-GMP polls for `MessageSent` events on all chains
4. Trusted-GMP calls `deliver_message()` on destination chain's native GMP endpoint
5. Native GMP endpoint calls `lzReceive()` on destination contract
6. Destination contract processes message normally

### Trusted-GMP Relay Requirements

- **Watches**: `MessageSent` events on native GMP endpoints (MVM, SVM, EVM)
- **Delivers**: Calls `deliver_message()` / `lzReceive()` on destination
- **Needs**: Funded operator wallet per chain (pays gas for delivery)
- **Config**: Chain RPCs, GMP endpoint addresses, operator keys
- **Mode**: `--mode relay` flag on trusted-gmp binary
- **Polling**: Configurable interval (default 500ms for fast CI)
- **Fidelity**: Minimal. Local endpoints emit events and deliver messages only — no DVN simulation, no fee calculation

### Relay Mode vs Current Trusted-GMP

| Aspect | Current Trusted-GMP | Relay Mode |
|--------|--------------------|----|
| **Watches** | Intent/escrow events | `MessageSent` events on native GMP endpoints |
| **Validates** | 15+ off-chain checks | None (contracts validate on-chain) |
| **Action** | Signs intent_id | Calls `deliver_message()` |
| **Keys needed** | Approver private key | Operator wallet (gas payment only) |
| **Can forge** | Approval signatures | GMP messages (same risk level) |

### Contracts Stay Identical

Contracts use the same GMP interface in all environments. Only the endpoint differs:

- **Production**: LZ GMP endpoint → DVNs verify and deliver
- **Local/CI**: Native GMP endpoint → Trusted-GMP watches and relays

```text
// Same contract code in all environments:
lz_send(endpoint, dst_chain_id, destination, payload);

// GMP endpoint is configured at deployment:
// Production: 0x1a44076050125825900e736c501f859c50fE728c (LZ)
// Local/CI:   <intent_gmp_address>
```

---

## Environment Matrix

| Environment | MVM Hub | SVM Connected | EVM Connected | GMP Delivery |
|-------------|---------|---------------|---------------|--------------|
| **Local/CI** | Native GMP endpoint | Native GMP endpoint | Native GMP endpoint | Trusted-GMP relay |
| **Testnet** | LZ GMP endpoint | LZ GMP endpoint (Solana devnet) | LZ GMP endpoint (Base Sepolia) | LZ DVNs + Executors |
| **Mainnet** | LZ GMP endpoint | LZ GMP endpoint | LZ GMP endpoint | LZ DVNs + Executors |

---

## What Triggers lzSend()?

This is a critical design question. In our current system, trusted-gmp is an external service that signs. With GMP, the contracts themselves must call `lzSend()`.

### Who Triggers Each Message

| Message | Direction | Triggered By | When |
|---------|-----------|--------------|------|
| `IntentRequirements` | Hub → Connected | Hub contract | On intent creation (`create_outflow_intent()` / `create_inflow_intent()`) |
| `EscrowConfirmation` | Connected → Hub | Connected escrow contract | On escrow creation (`create_escrow()`) |
| `FulfillmentProof` (outflow) | Connected → Hub | Connected validation contract | On solver fulfillment (`fulfill_intent()`) |
| `FulfillmentProof` (inflow) | Hub → Connected | Hub contract | On solver fulfillment (`fulfill_inflow_intent()`) |

**Key insight:** Every `lzSend()` is triggered by a user transaction (requester or solver). No external service needs to initiate messages. The contract logic calls `lzSend()` as part of its normal execution.

### Gas Costs

The caller of the transaction pays gas for `lzSend()`. This means:

- **Requester pays** for `IntentRequirements` (part of intent creation tx)
- **Requester pays** for `EscrowConfirmation` (part of escrow creation tx)
- **Solver pays** for `FulfillmentProof` (part of fulfillment tx)

LZ fees are paid in the transaction as well (msg.value on EVM, lamports on SVM).

---

## Summary: Architecture with GMP

```text
PRODUCTION:
┌──────────────────┐        LZ         ┌──────────────────┐
│    MVM Hub        │ ◄─── GMP Messages ────► │  SVM/EVM         │
│  Intent contracts │                          │  Escrow/Validator │
│  + GMP endpoint   │                          │  + GMP endpoint   │
└──────────────────┘                          └──────────────────┘
        │
        │ Coordinator (read-only)
        │ Reads hub state only
        │ Event monitoring, UX

LOCAL/CI:
┌──────────────────┐      Trusted-GMP Relay      ┌──────────────────┐
│    MVM Hub        │ ◄─── deliver_message ───► │  SVM/EVM         │
│  Intent contracts │                            │  Escrow/Validator │
│  + GMP endpoint   │                            │  + GMP endpoint   │
└──────────────────┘                            └──────────────────┘
        │
        │ Coordinator (read-only)
        │ Reads hub state only
        │ Event monitoring, UX
```

**What's eliminated in production:**

- Trusted-GMP service (no signer needed)
- Approval signatures (GMP messages replace them)
- Off-chain validation logic (moved on-chain)
- Private key management (no keys in production)

**What remains in all environments:**

- Coordinator (event monitoring, negotiation, UX, readiness tracking)
  - Monitors IntentRequirementsReceived events on connected chains
  - Provides `ready_on_connected_chain` flag via API
  - Does NOT track full GMP message lifecycle (MessageSent/MessageDelivered)
- On-chain contracts (same code, different GMP endpoint config)
- Frontend / solver bots query coordinator API for readiness status instead of polling connected chains directly
