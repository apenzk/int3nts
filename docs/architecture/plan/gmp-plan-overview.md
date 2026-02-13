# GMP Integration Proposal

**Status:** In Progress (Phase 4 In Progress)
**Date:** 2026-01-22
**Summary:** Add Generic Message Passing (GMP) for cross-chain messaging using Integrated GMP as the relay for all environments. Validation moves on-chain; cross-chain messaging replaces approval signatures. Coordinator and Integrated GMP are assumed (already in place).

> **🔷 Design Standard: LZ v2 Compatibility**
>
> Contracts follow **LZ v2 conventions** (function naming: `lzSend()`/`lzReceive()`, OApp patterns, wire format) as a design reference. This ensures future LZ integration is a configuration change (swap endpoint address), not a code change. All current environments use the **Integrated GMP relay**.

---

## Executive Summary

### Current System (Given)

- **Coordinator** – event monitoring, REST API, negotiation (no keys, cannot authorize releases)
- **Integrated GMP** – off-chain signer; validates and signs (Ed25519/ECDSA) to authorize escrow releases. Used in local/CI; can steal funds if compromised.

### Proposed Addition: GMP

Add **on-chain validation + GMP messaging** for cross-chain communication:

- Validation logic moves into smart contracts on each chain
- GMP handles cross-chain message delivery via Integrated GMP relay (LZ v2-compatible interfaces)
- Contracts authenticate via GMP message verification instead of approval signatures
- Coordinator unchanged (still no keys, UX only)

### Key Benefits

| Benefit | Impact |
|---------|--------|
| **Security** | Validation logic is transparent on-chain; no off-chain signing |
| **Operational simplicity** | Coordinator + Integrated GMP relay (no signer logic) |
| **LZ-ready** | LZ v2-compatible interfaces; future LZ integration is config-only |
| **Transparency** | On-chain validation, auditable contracts |

### Key Trade-offs

| Trade-off | Current (Integrated GMP signs) | With GMP |
|-----------|-----------------------------|----------|
| **Gas costs** | Low (signatures cheap) | Higher (on-chain validation + relay gas) |
| **Contract complexity** | Low | Medium (validation logic on-chain) |
| **Infrastructure** | Coordinator + Integrated GMP signer | Coordinator + Integrated GMP relay |
| **Flexibility** | Easy to update validation logic | Requires contract redeployment |

---

## Implementation Roadmap

See execution phase documents for detailed implementation plan:

- [Phase 1: Research & Design](gmp-plan-execution-phase1.md) ✅ **COMPLETE** - Interfaces, message schemas, wire format spec
- [Phase 2: Complete GMP Implementation (MVM, SVM, EVM) & Architecture Alignment](gmp-plan-execution-phase2.md) ✅ **COMPLETE** - GMP for all three chains, native relay, package restructuring, naming alignment (19 commits)
- [Phase 3: Coordinator Readiness Tracking](gmp-plan-execution-phase3.md) ✅ **COMPLETE** - Readiness tracking for outflow intents (commit f46eb3d)
- [Phase 4: Integration & Documentation](gmp-plan-execution-phase4.md) 🔄 **IN PROGRESS** - Commits 1-4 done (stripped API, frontend/solver cleanup, testnet deploy). Remaining: update existing docs, final cleanup

**Total Timeline:** ~1.5 weeks (testnet only)

**Assumed starting point:** Coordinator and Integrated GMP already exist. This plan adds GMP messaging using the Integrated GMP relay (with LZ v2-compatible interfaces).

---

## Starting Point: Coordinator + Integrated GMP (Given)

Current architecture (no change in this plan):

```text
┌─────────────────────────────────┐  ┌─────────────────────────────────┐
│      COORDINATOR SERVICE        │  │      INTEGRATED GMP SERVICE        │
│  Event monitoring, REST API,   │  │  Signs approvals (Ed25519/ECDSA)│
│  negotiation. NO KEYS.         │  │  Used in local/CI. HAS KEYS.    │
│  🟢 Cannot steal funds          │  │  🔴 Can steal funds if compromised│
└─────────────────────────────────┘  └─────────────────────────────────┘
```

**After GMP (this plan):** On-chain contracts validate and send GMP messages. Integrated GMP becomes a relay (no signing, no off-chain validation). Coordinator unchanged.

| Environment | Who authorizes releases | Trust |
|-------------|-------------------------|--------|
| **Before GMP** | Integrated GMP off-chain signer | 🔴 Our service signs approvals |
| **After GMP (all environments)** | On-chain contracts via GMP messages, Integrated GMP relay delivers | 🟡 Our relay delivers, but validation is on-chain |

---

## Architectural Changes

This plan moves validation logic on-chain and uses GMP for cross-chain message delivery. The table below shows how each current approval-task (today done by Integrated GMP) maps to the GMP design.

### Approval-Flow Task Migration

| # | Task | How Architectural Redesign Addresses It |
| --- | ------ | --------------------------------------- |
| 1 | **Event Monitoring** | Coordinator for UX; contracts handle critical path via GMP messages |
| 2 | **Outflow: Validate Tx Success** | Validation contract enforces success on-chain; only successful calls send GMP message |
| 3 | **Outflow: Validate Intent ID** | Validation contract receives intent_id via GMP; validates as function parameter |
| 4 | **Outflow: Validate Recipient** | Validation contract enforces recipient match on-chain; requires structured function call |
| 5 | **Outflow: Validate Amount** | Validation contract enforces amount match on-chain; validates transfer within contract |
| 6 | **Outflow: Validate Token** | Validation contract enforces token match on-chain; validates via function parameters |
| 7 | **Outflow: Validate Solver** | Hub sends solver's connected-chain address via GMP; validation contract checks msg.sender |
| 8 | **Inflow: Validate Escrow Match** | Hub sends intent requirements via GMP; escrow contract validates on creation |
| 9 | **Inflow: Validate Chain IDs** | Hub sends requirements to specific chain; GMP source chain verification built-in |
| 10 | **Inflow: Validate Reserved Solver** | Hub queries registry, sends authorized solver via GMP; escrow validates on-chain |
| 11 | **Sign Intent ID (MVM)** | Replaced by GMP message authentication; contracts verify message source via GMP |
| 12 | **Sign Intent ID (EVM)** | Replaced by GMP message authentication; contracts verify message source via GMP |
| 13 | **Sign Intent ID (SVM)** | Replaced by GMP message authentication; contracts verify message source via GMP |
| 14 | **Cache & Serve Events** | Coordinator service (no keys, no validation) for UX |
| 15 | **Negotiation Routing** | Coordinator includes negotiation API (application logic, not security-critical) |

**Key Finding:** Moving validation on-chain and using GMP for delivery makes all 15 approval-flow tasks feasible (on-chain validation + GMP messaging).

---

## Proposed Architecture

### Core Principle

Replace **"our off-chain signer (Integrated GMP) validates and signs"** with **"on-chain validation + GMP messages"** in production.

### How It Works

#### Current (Integrated GMP signs)

```text
Integrated GMP (off-chain) validates → Signs approval → Contract checks signature
```

#### After GMP (production)

```text
On-chain contract validates → Sends GMP message → Receiving contract accepts via GMP
```

### GMP Message Delivery

**Key Insight:** Contracts must be explicitly called to initiate messages — contracts call `lzSend()` as part of their logic. The Integrated GMP relay watches for `MessageSent` events and delivers messages to destination chains by calling `lzReceive()` / `deliver_message()`.

---

## Detailed Flow Changes

### Inflow (Connected Chain → Hub)

#### Current Flow

1. Hub: Requester creates intent (wants assets on hub), emits event
2. Connected Chain: Requester creates escrow (offers their assets, reserved for solver) → emits `EscrowCreated` event
3. **Integrated GMP**: Observes escrow event, validates (amount, token, reservation match intent)
4. Hub: Solver fulfills intent (provides desired assets to requester)
5. **Integrated GMP**: Observes fulfillment event, validates, generates signature
6. Connected Chain: Solver submits Integrated GMP signature → escrow releases requester's offering to solver

#### GMP Flow

1. Hub: Requester creates intent (wants assets on hub), **sends GMP message to connected chain** with escrow requirements
2. Connected Chain: **Receives escrow requirements via GMP** (idempotent: if requirements already exist for intent_id + step number, ignore duplicate message), stores them in validation contract
3. Connected Chain: Requester creates escrow (offers their assets, reserved for solver) → **contract validates requirements exist and match escrow details** (reverts if requirements don't exist or don't match) → **sends GMP message to hub** (escrow confirmed)
4. Hub: **Receives escrow confirmation via GMP**, allows fulfillment
5. Hub: Solver fulfills intent (provides desired assets to requester) → **sends GMP message to connected chain** (fulfillment confirmed)
6. Connected Chain: **Receives fulfillment confirmation via GMP** → escrow automatically releases requester's offering to solver

#### Changes Required

| Component | Current | New | Change Type |
|-----------|---------|-----|-------------|
| **Hub Intent Contract** | Emits event only | Add GMP send with intent details | **MODIFY** - Add outbound message |
| **Hub Intent Contract** | N/A | Add GMP receive handler for escrow confirmations | **NEW** - Add inbound handler |
| **Hub Intent Contract** | Emits fulfillment event | Add GMP send on fulfillment | **MODIFY** - Add outbound message |
| **Connected Chain Escrow** | Just locks funds + emits event | Add GMP receive handler for intent requirements | **NEW** - Add inbound handler |
| **Connected Chain Escrow** | Validates via Integrated GMP signature | Validate requirements on-chain during creation | **MODIFY** - Move validation logic on-chain |
| **Connected Chain Escrow** | Validates via Integrated GMP signature | Add GMP send on escrow creation | **NEW** - Add outbound message |
| **Connected Chain Escrow** | Requires signature for release | Add GMP receive handler for fulfillment proof | **NEW** - Add inbound handler |
| **Connected Chain Escrow** | Uses `ed25519::verify_signature` | Use GMP message verification | **REPLACE** - Different auth mechanism |
| **Integrated GMP** | Observes, validates, signs | Relay only (watches `MessageSent`, calls `deliver_message()`) | **REPLACE** - Signer becomes relay |

### Outflow (Hub → Connected Chain)

#### Current Flow

1. Hub: Intent created (locks funds), emits event with requirements (recipient, amount, token, connected_chain_id)
2. **Integrated GMP**: Observes intent event, caches it
3. Connected Chain: Solver submits **arbitrary transaction** (ERC20 transfer, SPL transfer, etc.)
4. **Integrated GMP**: Queries transaction by hash, parses arguments/logs, validates (recipient, amount, token, solver address)
5. **Integrated GMP**: Queries hub solver registry for solver's connected-chain address
6. **Integrated GMP**: Signs intent_id if valid
7. Hub: Solver submits Integrated GMP signature → intent releases escrow

#### GMP Flow

1. Hub: Intent created (locks funds) → **sends GMP message to connected chain** with requirements (recipient, amount, token, authorized solver address)
2. Connected Chain: **Validation contract receives requirements via GMP** (idempotent: if requirements already exist for intent_id + step number, ignore duplicate message), stores them (maps `intent_id/step → {requirements, authorizedSolver}`)
3. Connected Chain: **Authorized solver approves validation contract** to spend tokens (one-time, with large amount like MAX_UINT256)
4. Connected Chain: **Authorized solver calls validation contract function** (e.g., `fulfillIntent(intent_id, token, amount)`)
5. Validation Contract: **Pulls tokens via `transferFrom(authorizedSolver, contract, amount)`** (requires approval)
6. Validation Contract: **Validates** (amount, token match stored requirements, solver matches authorized solver)
7. Validation Contract: **Forwards tokens to user wallet**
8. Validation Contract: **Sends GMP message to hub** (calls `lzSend()`)
9. Hub: **Receives fulfillment proof via GMP** → releases escrow to solver

**Note:** Steps 3-8 happen atomically in one transaction (after the one-time approval in step 3). The contract pulls tokens from the authorized solver's wallet, validates, forwards, and sends GMP - all in the same transaction.

**Idempotency & Failure Handling:**

- Each `intent_id` can have multiple sequenced GMP messages (e.g., `intent_id/step1`, `intent_id/step2`, etc.) that can go in both directions (hub → connected chain, connected chain → hub)
- Messages are sequenced with step numbers, so out-of-order delivery is not a concern (step1 must come before step2)
- Duplicate GMP messages (same step) are ignored (idempotent: if requirements already exist for `intent_id + step number`, ignore the duplicate)
- Messages don't have timeouts - if a message never arrives, the intent/escrow will expire on-chain (existing expiry mechanism)
- No retry logic needed - on-chain expiry handles stuck intents

#### Changes Required

| Component | Current | New | Change Type |
|-----------|---------|-----|-------------|
| **Hub Intent Contract** | Emits event only | Add GMP send with intent requirements | **MODIFY** - Add outbound message |
| **Hub Intent Contract** | Stores solver's hub address | Query solver registry for connected-chain address, send via GMP | **MODIFY** - Add registry lookup |
| **Hub Intent Contract** | Uses `ed25519/ecdsa::verify_signature` | Add GMP receive handler for fulfillment proof | **REPLACE** - Different auth mechanism |
| **Connected Chain** | No contract required | **Deploy validation contract** | **NEW** - New contract on each connected chain |
| **Validation Contract** | N/A | Receive intent requirements via GMP | **NEW** - Inbound handler |
| **Validation Contract** | N/A | Enforce requirements on-chain (recipient, amount, token) | **NEW** - On-chain validation logic |
| **Validation Contract** | N/A | Pull tokens via `transferFrom()` (requires solver approval) | **NEW** - Transfer execution via approval pattern |
| **Validation Contract** | N/A | Validate requirements and forward tokens to user | **NEW** - On-chain validation and forwarding |
| **Validation Contract** | N/A | Send GMP message on success | **NEW** - Outbound message |
| **Solver Flow** | Submit arbitrary tx → wait → submit signature | Approve contract (one-time) → Call validation contract function (transfer + validation + GMP send in one tx) | **REPLACE** - Different interaction pattern |
| **Integrated GMP** | Parses txs, validates, queries registry, signs | Relay only (watches `MessageSent`, calls `deliver_message()`) | **REPLACE** - Signer becomes relay |

---

## Contract Deployment Requirements

### Current Contracts

| Chain Type | Contracts Deployed |
|------------|-------------------|
| Hub (MVM) | Intent contracts (reservation, registry, escrow, outflow) |
| Connected MVM | Escrow contract |
| Connected EVM | Escrow contract |
| Connected SVM | Escrow program |

### New Contracts (GMP-based)

| Chain Type | Existing Contracts | New Contracts Needed |
|------------|-------------------|----------------------|
| Hub (MVM) | Intent contracts | **GMP receiver/sender modules** integrated into existing contracts |
| Connected MVM | Escrow contract | **GMP receiver/sender integration** + **Outflow validation contract** |
| Connected EVM | Escrow contract | **GMP receiver/sender integration** + **Outflow validation contract** |
| Connected SVM | Escrow program | **GMP receiver/sender integration** + **Outflow validation program** |

**Note:** Each connected chain needs **TWO** contracts:

1. **Inflow**: Enhanced escrow contract (receives intent requirements, sends confirmations)
2. **Outflow**: New validation contract (receives intent requirements, validates solver fulfillment)

---

## Infrastructure Changes

### What Gets Eliminated

| Component | Current Role | Security Impact |
|-----------|-------------|-----------------|
| **Validation Logic** | Off-chain validation of 15+ checks | 🔴 **CRITICAL** - Bugs can cause fund loss |
| **Signature Generation** | Ed25519/ECDSA signing of approvals | 🔴 **CRITICAL** - Wrong signature = wrong outcome |
| **Private Key Management** | Secure storage of Integrated GMP keys | 🔴 **CRITICAL** - Key compromise = total system breach |
| **Transaction Parsing** | Extract data from arbitrary transactions | 🔴 **HIGH** - Parsing bugs can validate wrong data |
| **Cross-Chain State Queries** | Query solver registry on hub | 🟡 **MEDIUM** - Must remain synced |

### What Remains (Given)

**Coordinator** (unchanged by this plan): event monitoring, caching, REST API, negotiation. No keys. **Integrated GMP**: relay for all environments.

| Function | Coordinator (given) | Integrated GMP (before) | After GMP |
|----------|---------------------|---------------------|-------------------------|
| **Event monitoring / API** | ✅ YES | — | Coordinator only |
| **Who authorizes releases** | — | Off-chain signer | On-chain contracts (relay delivers messages) |
| **Message delivery** | — | N/A | Integrated GMP relay |

### Updated Infrastructure Complexity (After GMP)

| Aspect | Current (Integrated GMP signs) | After GMP | Impact |
|--------|-----------------------------|------------------------|--------|
| **Infrastructure YOU run** | Coordinator + Integrated GMP signer | Coordinator + Integrated GMP relay | **Same services, simpler role** |
| **Relay criticality** | 🔴 **CRITICAL** (signer = authority) | 🟡 **MEDIUM** (relay delivers, contracts validate) | **Reduced risk** |
| **Security requirements** | 🔴 **MAXIMUM** (signer holds approval keys) | 🟡 **MODERATE** (relay has operator wallets for gas) | **Improvement** |
| **Validation location** | Off-chain (Integrated GMP) | On-chain (contracts) | **Transparency** |
| **LZ upgrade path** | N/A | Swap endpoint address to LZ (config change only) | **Future-ready** |

---

## Security Implications

### Trust Model Comparison

| Aspect | Current (Integrated GMP signs) | After GMP |
|--------|-----------------------------|-------------------------|
| **Authority source** | Our Integrated GMP private key (approval signer) | On-chain contracts (Integrated GMP relay delivers) |
| **Validation location** | Off-chain (Integrated GMP) | On-chain (smart contracts) |
| **Liveness dependency** | Our signer must be online | Our relay must be online (same liveness, less authority) |
| **Security assumption** | Trust our signer + validation logic | Trust on-chain validation + our relay for delivery |
| **Key compromise impact** | 🔴 Total breach (signer keys = approval authority) | 🟡 Relay keys = gas wallets only (cannot forge valid on-chain state) |
| **Validation bug impact** | 🔴 Wrong signatures, funds lost | 🟡 Contract bug isolated, auditable |
| **Transparency** | ❌ Off-chain logic | ✅ On-chain logic, transparent |

### Attack Surface Reduction

**Current attack vectors (Integrated GMP as signer):**

- Integrated GMP private key theft
- Integrated GMP service compromise
- Validation logic bugs in off-chain code
- Transaction parsing vulnerabilities
- Cross-chain state desynchronization

**New attack vectors:**

- Integrated GMP relay compromise (relay has operator wallets; can deliver forged messages to integrated GMP endpoints)
- On-chain validation logic bugs (mitigated by audits, formal verification)

**Eliminated (with GMP):**

- ✅ No off-chain validation logic (on-chain only)
- ✅ No transaction parsing vulnerabilities (contracts enforce structure)
- ✅ No approval signature generation (GMP messages replace signatures)

**Remaining risk:**

- ⚠️ Integrated GMP relay still has operator wallets and can forge messages (same trust model as before for delivery, but validation is now on-chain)

---

## Cost Analysis

### Current System Costs (Integrated GMP as signer)

| Component | Cost Type | Estimate |
|-----------|-----------|----------|
| **Coordinator + Integrated GMP** | Fixed monthly | $X/month (servers, monitoring) |
| **Development/Maintenance** | Ongoing | Y hours/month |
| **Security Audits** | Periodic | High (signer is critical) |
| **Key Management** | Operational | Security overhead |

### GMP System Costs (Integrated GMP relay)

| Component | Cost Type | Estimate |
|-----------|-----------|----------|
| **Relay Gas Costs** | Per-message | Operator wallet pays gas for `deliver_message()` on each chain |
| **On-chain Validation Gas** | Per-transaction | Higher than current (validation logic on-chain) |
| **Coordinator + Relay Infrastructure** | Fixed monthly | $X/month (same services, simpler logic) |
| **Development/Maintenance** | Ongoing | Reduced (relay is simpler than signer) |
| **Security Audits** | Periodic | Medium (contracts auditable) |

**Cost Shift:** Relay gas costs replace signature generation costs. No third-party GMP fees.

---

## GMP Design Reference: LZ v2

### Why LZ v2 as Reference Standard

Contracts follow LZ v2 conventions as a design standard. This provides:

- **Industry-standard interfaces** — `lzSend()` / `lzReceive()` naming, OApp patterns
- **Future upgrade path** — swapping to real LZ endpoints is a config change (endpoint address), not a code change
- **Proven patterns** — LZ's OApp architecture is battle-tested across Aptos, EVM, and Solana

### Current Implementation

All environments currently use the **Integrated GMP relay** with LZ v2-compatible integrated GMP endpoints. See [LZ SVM Integration Research](lz-svm-integration.md) and [LZ MVM Integration Research](lz-mvm-integration.md) for the design reference research that informed our interfaces.

**Note:** LZ integration is not part of the current phases. All phases use Integrated GMP only.

---

## Integrated GMP Relay

### The Concept

**The Integrated GMP Service** acts as the message relay for all environments. It watches for `MessageSent` events on integrated GMP endpoints and delivers messages to destination chains.

### How It Works

| Environment | GMP Provider | Flow |
|-------------|--------------|------|
| **All environments** | Integrated GMP Service | Contract → `lzSend()` → [Integrated GMP endpoint emits event] → [Integrated GMP watches] → [Calls `lzReceive()`] → Destination |

### Key Principle

**Contracts use LZ v2-compatible interfaces** (`lzSend()`/`lzReceive()`). The Integrated GMP endpoints implement these interfaces. If LZ integration is added in the future, swapping to real LZ endpoints is a configuration change only.

### Integrated GMP Role

| Aspect | Before GMP (Integrated GMP signs) | After GMP (Integrated GMP relays) |
|--------|---------------------------|------------------------|
| **Watches events** | Intent/escrow events | `MessageSent` events on integrated GMP endpoints |
| **Validates logic** | 15+ checks off-chain | ❌ None (contracts validate on-chain) |
| **Action taken** | Generates approval signatures | Calls `deliver_message()` / `lzReceive()` on destination |
| **Private keys** | ✅ YES (approval signing keys) | ✅ YES (operator wallet per chain, gas only) |
| **Can steal funds** | 🔴 YES (signer = authority) | 🔴 YES (can forge messages to integrated GMP endpoints) |

### Implementation

**Coordinator and Integrated GMP already exist as separate services:**

1. **Coordinator Service** - UX functions (event monitoring, API, negotiation) - NO KEYS, CANNOT STEAL FUNDS
2. **Integrated GMP Service** - message relay for all environments - REQUIRES FUNDED OPERATOR WALLET on each chain (private key in config, pays gas to call `deliver_message()`), CAN FORGE MESSAGES, CAN STEAL FUNDS

> ⚠️ **Trust model:** The Integrated GMP relay has the same trust level as the old signer — it can forge messages. The improvement is that validation logic is now on-chain and transparent, but the relay is still a trusted component. Future LZ integration would remove this trust requirement.

### Benefits

- **LZ-compatible contracts** — same code path, future LZ swap is config-only
- **Fast message delivery** — ~500ms relay latency
- **Deterministic tests** — no flaky tests from external network delays
- **Cost efficient** — no third-party GMP fees
- **Full control** — easier debugging, full control over message delivery timing

### Environment Configuration

| Environment | Movement | Solana | EVM |
| ----------- | -------- | ------ | --- |
| **Local/CI** | Integrated GMP endpoint + relay | Integrated GMP endpoint + relay | Integrated GMP endpoint + relay |
| **Testnet** | Integrated GMP endpoint + relay | Integrated GMP endpoint + relay | Integrated GMP endpoint + relay |
| **Mainnet** | Integrated GMP endpoint + relay | Integrated GMP endpoint + relay | Integrated GMP endpoint + relay |

> **Note:** All environments use Integrated GMP. Contracts follow LZ v2 conventions so that future LZ integration is a configuration change (swap endpoint address).

---

## Open Questions

1. **Gas Fee Economics:** Are relay gas + on-chain validation costs acceptable for users?
2. **Solver Adoption:** Will solvers adapt to calling validation contracts instead of arbitrary transactions?
3. **Coordinator Role:** The coordinator handles negotiation/discovery - security doesn't depend on it, but UX does.
4. **Failure Modes:** How to handle GMP message delivery failures or delays? ✅ **RESOLVED** - On-chain expiry handles stuck intents, idempotency handles duplicate messages
5. **State Synchronization:** How to ensure intent requirements are always in sync across chains?
6. **Integrated GMP Mode:** Use Integrated GMP for local/CI (mock GMP) so production contracts stay identical. ✅ **ASSUMED** – already in place.

---

## Conclusion

### Summary

Adding GMP so production does not use our signer is **FEASIBLE** but requires:

- **On-chain validation** – move approval logic from Integrated GMP into contracts
- **GMP integration** – add cross-chain messaging to all contracts
- **New validation contracts** – deploy on all connected chains
- **Coordinator** – unchanged (already in place; no keys, no validation)
- **~1.5 week timeline** – testnet only

### Key Benefits

- ✅ **On-chain validation** – approval logic moved from off-chain signer to contracts
- ✅ **Transparency** – validation logic on-chain, auditable
- ✅ **Simpler relay** – Integrated GMP relay only delivers messages, no validation/signing
- ✅ **LZ-ready** – LZ v2-compatible interfaces; future LZ integration is config-only
- ✅ **Reduced key exposure** – relay has gas wallets only, not approval authority

### Key Trade-offs

- ⚠️ **Higher gas costs** - on-chain validation + relay delivery gas
- ⚠️ **Contract complexity** - moderate increase (~50-100 lines per contract)
- ⚠️ **Development time** - ~1.5 weeks for testnet (with AI assistance)
- ⚠️ **Relay still trusted** - Integrated GMP relay can forge messages (same trust as before for delivery)
- ⚠️ **Solver UX changes** - must use validation contracts

---

**End of Proposal**

> See execution phase documents for detailed implementation plan.
