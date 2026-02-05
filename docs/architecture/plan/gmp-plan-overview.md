# GMP Integration Proposal

**Status:** In Progress (Phase 2 Started)
**Date:** 2026-01-22
**Summary:** Add Generic Message Passing (GMP) for cross-chain messaging. Production can use either Trusted GMP (our relay) or LZ. Validation moves on-chain; cross-chain messaging replaces approval signatures. Coordinator and Trusted GMP are assumed (already in place).

> **🔷 GMP Protocol: LZ v2**
>
> This proposal uses **LZ v2** as the GMP protocol. LZ provides the best cross-chain coverage (Movement/Aptos, EVM, Solana), mature integration, and flexible executor network. See [GMP Protocol Comparison](#gmp-protocol-comparison) for full analysis.

---

## Executive Summary

### Current System (Given)

- **Coordinator** – event monitoring, REST API, negotiation (no keys, cannot authorize releases)
- **Trusted GMP** – off-chain signer; validates and signs (Ed25519/ECDSA) to authorize escrow releases. Used in local/CI; can steal funds if compromised.

### Proposed Addition: GMP

Add **on-chain validation + GMP messaging** for cross-chain communication:

- Validation logic moves into smart contracts on each chain
- GMP handles cross-chain message delivery (either Trusted GMP relay or LZ v2)
- Contracts authenticate via GMP message verification instead of approval signatures
- Coordinator unchanged (still no keys, UX only); Trusted GMP can be used in production or local/CI

### Key Benefits

| Benefit | Impact |
|---------|--------|
| **Flexible production options** | Production can use Trusted GMP relay or LZ; choice of trust model |
| **Censorship resistance** | Permissionless GMP networks vs. our Trusted GMP signer |
| **Decentralization** | Trust GMP validator networks instead of our service |
| **Security** | Validation logic is transparent on-chain |
| **Operational simplicity** | Coordinator only (no signer to run in production) |

### Key Trade-offs

| Trade-off | Current (Trusted GMP signs) | With GMP |
|-----------|-----------------------------|----------|
| **Gas costs** | Low (signatures cheap) | Higher (GMP fees + on-chain validation) |
| **Contract complexity** | Low | Medium (validation logic on-chain) |
| **Infrastructure** | Coordinator + Trusted GMP | Coordinator + Trusted GMP relay or LZ |
| **Flexibility** | Easy to update validation logic | Requires contract redeployment |

---

## Implementation Roadmap

See execution phase documents for detailed implementation plan:

- [Phase 1: Research & Design](gmp-plan-execution-phase1.md) ✅ **COMPLETE** - Interfaces, message schemas, wire format spec
- [Phase 2: SVM + MVM Core](gmp-plan-execution-phase2.md) ✅ **COMPLETE** - Build both chains together for real cross-chain testing
- [Phase 3: EVM Expansion](gmp-plan-execution-phase3.md) (1-2 days) - Add EVM connected chain support
- [Phase 4: Coordinator GMP Integration](gmp-plan-execution-phase4.md) (1 day) - Add GMP message tracking to coordinator
- [Phase 5: Integration & Documentation](gmp-plan-execution-phase5.md) (1-2 days) - Frontend, solver SDK, fee analysis, final cleanup

**Total Timeline:** ~1.5 weeks (testnet only)

**Assumed starting point:** Coordinator and Trusted GMP already exist. This plan adds GMP messaging (using either Trusted GMP relay or LZ).

---

## Starting Point: Coordinator + Trusted GMP (Given)

Current architecture (no change in this plan):

```text
┌─────────────────────────────────┐  ┌─────────────────────────────────┐
│      COORDINATOR SERVICE        │  │      TRUSTED GMP SERVICE        │
│  Event monitoring, REST API,   │  │  Signs approvals (Ed25519/ECDSA)│
│  negotiation. NO KEYS.         │  │  Used in local/CI. HAS KEYS.    │
│  🟢 Cannot steal funds          │  │  🔴 Can steal funds if compromised│
└─────────────────────────────────┘  └─────────────────────────────────┘
```

**After GMP (this plan):** Production uses LZ for cross-chain approval; Trusted GMP remains for local/CI only. Coordinator unchanged.

| Environment | Who authorizes releases | Trust |
|-------------|-------------------------|--------|
| **Local/CI (today & after)** | Trusted GMP (our signer) | 🔴 Our service |
| **Production (after GMP)** | LZ DVNs | 🟡 GMP network |

---

## Architectural Changes

This plan moves validation logic on-chain and uses GMP for cross-chain message delivery. The table below shows how each current approval-task (today done by Trusted GMP) maps to the GMP design.

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

Replace **"our off-chain signer (Trusted GMP) validates and signs"** with **"on-chain validation + GMP messages"** in production.

### How It Works

#### Current (Trusted GMP signs)

```text
Trusted GMP (off-chain) validates → Signs approval → Contract checks signature
```

#### After GMP (production)

```text
On-chain contract validates → Sends GMP message → Receiving contract accepts via GMP
```

### GMP Auto-Execution Support

| GMP | Auto-Delivery to Destination | Auto-Initiation from Events | Who Triggers Source Call |
|-----|------------------------------|----------------------------|--------------------------|
| **LZ** | ✅ YES (Executors) | ❌ NO | Solver / User / Relayer |
| **Wormhole** | ⚠️ PARTIAL (if configured) | ❌ NO | Solver / User / Relayer |
| **Axelar** | ✅ YES (Gas Service) | ❌ NO | Solver / User / Relayer |
| **CCIP** | ✅ YES (built-in) | ❌ NO | Solver / User / Relayer |

**Key Insight:** GMPs provide auto-delivery to destination chains, but contracts must be explicitly called to initiate messages. This is not a problem - contracts call `lzSend()` / `gateway.callContract()` / etc. as part of their logic.

---

## Detailed Flow Changes

### Inflow (Connected Chain → Hub)

#### Current Flow

1. Hub: Requester creates intent (wants assets on hub), emits event
2. Connected Chain: Requester creates escrow (offers their assets, reserved for solver) → emits `EscrowCreated` event
3. **Trusted GMP**: Observes escrow event, validates (amount, token, reservation match intent)
4. Hub: Solver fulfills intent (provides desired assets to requester)
5. **Trusted GMP**: Observes fulfillment event, validates, generates signature
6. Connected Chain: Solver submits Trusted GMP signature → escrow releases requester's offering to solver

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
| **Connected Chain Escrow** | Validates via Trusted GMP signature | Validate requirements on-chain during creation | **MODIFY** - Move validation logic on-chain |
| **Connected Chain Escrow** | Validates via Trusted GMP signature | Add GMP send on escrow creation | **NEW** - Add outbound message |
| **Connected Chain Escrow** | Requires signature for release | Add GMP receive handler for fulfillment proof | **NEW** - Add inbound handler |
| **Connected Chain Escrow** | Uses `ed25519::verify_signature` | Use GMP message verification | **REPLACE** - Different auth mechanism |
| **Trusted GMP (production)** | Observes, validates, signs | **Not used in production** | Production uses GMP only; Trusted GMP stays for local/CI |

### Outflow (Hub → Connected Chain)

#### Current Flow

1. Hub: Intent created (locks funds), emits event with requirements (recipient, amount, token, connected_chain_id)
2. **Trusted GMP**: Observes intent event, caches it
3. Connected Chain: Solver submits **arbitrary transaction** (ERC20 transfer, SPL transfer, etc.)
4. **Trusted GMP**: Queries transaction by hash, parses arguments/logs, validates (recipient, amount, token, solver address)
5. **Trusted GMP**: Queries hub solver registry for solver's connected-chain address
6. **Trusted GMP**: Signs intent_id if valid
7. Hub: Solver submits Trusted GMP signature → intent releases escrow

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
| **Trusted GMP (production)** | Parses txs, validates, queries registry, signs | **Not used in production** | Production uses GMP; Trusted GMP for local/CI only |

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
| **Private Key Management** | Secure storage of Trusted GMP keys | 🔴 **CRITICAL** - Key compromise = total system breach |
| **Transaction Parsing** | Extract data from arbitrary transactions | 🔴 **HIGH** - Parsing bugs can validate wrong data |
| **Cross-Chain State Queries** | Query solver registry on hub | 🟡 **MEDIUM** - Must remain synced |

### What Remains (Given)

**Coordinator** (unchanged by this plan): event monitoring, caching, REST API, negotiation. No keys. **Trusted GMP**: can be used for production relay or local/CI.

| Function | Coordinator (given) | Trusted GMP (given) | After GMP (production) |
|----------|---------------------|---------------------|-------------------------|
| **Event monitoring / API** | ✅ YES | — | Coordinator only |
| **Who authorizes releases** | — | Trusted GMP relay | Trusted GMP relay OR LZ |
| **Production relay options** | — | ✅ YES | Trusted GMP relay OR LZ |

### Updated Infrastructure Complexity (After GMP)

| Aspect | Current (Trusted GMP signs) | After GMP (production) | Impact |
|--------|-----------------------------|------------------------|--------|
| **Infrastructure YOU run** | Coordinator + Trusted GMP | Coordinator + (Trusted GMP relay OR LZ) | **Flexible relay choice** |
| **Infrastructure SOMEONE runs** | Just you (signer) | You (Trusted GMP relay) OR GMP protocol operators | **Choice of trust model** |
| **Relay criticality in prod** | 🔴 **CRITICAL** | 🟡 **DEPENDS** (your relay or decentralized) | **Flexibility** |
| **Security requirements** | 🔴 **MAXIMUM** (signer holds keys) | 🟢 **MINIMAL** (coordinator read-only) | **Massive improvement** |
| **Censorship power** | 🔴 **HIGH** (can refuse to sign) | 🟢 **NONE** (permissionless GMP) | **Eliminated** |

---

## Security Implications

### Trust Model Comparison

| Aspect | Current (Trusted GMP signs) | After GMP (production) |
|--------|-----------------------------|-------------------------|
| **Authority source** | Our Trusted GMP private key | GMP protocol (LZ DVNs, relayers) |
| **Validation location** | Off-chain (Trusted GMP) | On-chain (smart contracts) |
| **Censorship resistance** | ❌ Our signer can refuse | ✅ Permissionless GMP networks |
| **Liveness dependency** | Our signer must be online | GMP network (highly redundant) |
| **Security assumption** | Trust our signer + validation logic | Trust GMP + on-chain validation |
| **Key compromise impact** | 🔴 Total breach (our keys) | 🟢 No our keys in prod; worst case API DoS |
| **Validation bug impact** | 🔴 Wrong signatures, funds lost | 🟡 Contract bug isolated, auditable |
| **Transparency** | ❌ Off-chain logic | ✅ On-chain logic, transparent |

### Attack Surface Reduction

**Current attack vectors (Trusted GMP as signer):**

- Trusted GMP private key theft
- Trusted GMP service compromise
- Validation logic bugs in off-chain code
- Transaction parsing vulnerabilities
- Cross-chain state desynchronization

**New attack vectors:**

- GMP protocol vulnerability (mitigated by established protocols)
- On-chain validation logic bugs (mitigated by audits, formal verification)

**Eliminated in production (with GMP):**

- ✅ No our signer key to steal in production
- ✅ No off-chain validation logic bugs (on-chain only)
- ✅ No transaction parsing vulnerabilities (contracts enforce structure)
- ✅ No key management for production signer

---

## Cost Analysis

### Current System Costs (Trusted GMP as signer)

| Component | Cost Type | Estimate |
|-----------|-----------|----------|
| **Coordinator + Trusted GMP** | Fixed monthly | $X/month (servers, monitoring) |
| **Development/Maintenance** | Ongoing | Y hours/month |
| **Security Audits** | Periodic | High (signer is critical) |
| **Key Management** | Operational | Security overhead |

### GMP System Costs

| Component | Cost Type | Estimate |
|-----------|-----------|----------|
| **GMP Fees** | Per-message variable | $0.10-$5 per message (varies by GMP and chains) |
| **Gas Costs** | Per-transaction | Higher than current (on-chain validation) |
| **Coordinator Infrastructure** | Fixed monthly | $X/month (reduced from current) |
| **Development/Maintenance** | Ongoing | Reduced (simpler service) |
| **Security Audits** | Periodic | Medium (contracts auditable, no keys) |

**Cost Shift:** From **fixed operational costs** to **variable per-use costs**.

---

## GMP Protocol Comparison

### Evaluation Criteria

| Criterion | LZ | Axelar | Wormhole | CCIP |
|-----------|-----------|--------|----------|------|
| **Movement Support** | ✅ Aptos (similar) | ✅ Aptos | ✅ Aptos | ⚠️ Limited |
| **Solana Support** | ✅ YES | ✅ YES | ✅ YES (native) | ⚠️ Expanding |
| **EVM Support** | ✅ Excellent | ✅ Excellent | ✅ Excellent | ✅ Excellent |
| **Auto-Execution** | ✅ Executors | ✅ Gas Service | ⚠️ Partial | ✅ Built-in |
| **Message Fees** | Low-Medium | Medium | Low-Medium | Medium-High |
| **Trust Model** | Oracle+Relayer | Validator Set | Guardian Network | Chainlink DON |
| **Maturity** | High | High | High | High |

### Decision: LZ v2

**FINAL SELECTION:** LZ v2 has been selected as the GMP protocol for this implementation.

**Rationale:**

- Best cross-chain coverage (MVM, EVM, SVM)
- Mature Aptos integration (similar to Movement)
- Flexible executor network
- Competitive fees
- Strong ecosystem support

**Note:** This decision is final. All implementation phases will use LZ v2.

---

## Trusted GMP Mode: Local/CI GMP Alternative

### The Concept

**The Trusted GMP Service** acts as a message relay for testing. It is used for:

- **Local development** - No need for testnet GMP infrastructure
- **CI testing** - Fast, deterministic message delivery
- **Debugging** - Easier to trace and debug message flows
- **Testing environments** - Full control over message delivery

### How It Works

| Environment | GMP Provider | Flow |
|-------------|--------------|------|
| **Production** | Real GMP (LZ) | Contract → `lzSend()` → [Real GMP DVNs + Executors] → `lzReceive()` → Destination |
| **Local/CI/Testing** | Trusted GMP Service | Contract → `lzSend()` → [Mock endpoint emits event] → [Trusted GMP watches] → [Calls `lzReceive()`] → Destination |

### Key Principle

**Contracts remain identical across all environments** - they use the same GMP interfaces. Only the underlying endpoint implementation differs:

- **Production**: Real LZ endpoint
- **Local/CI/Testing**: Mock endpoint + Trusted GMP service

### Trusted GMP Role (Given)

| Aspect | Today (Trusted GMP signs) | After GMP – Local/CI | After GMP – Production |
|--------|---------------------------|------------------------|------------------------|
| **Watches events** | Intent/escrow events | `MessageSent` (mock endpoints) | N/A – not used |
| **Validates logic** | 15+ checks off-chain | ❌ None (contracts validate) | N/A |
| **Action taken** | Generates signatures | Calls `lzReceive()` (mock) | N/A – LZ delivers |
| **Private keys** | ✅ YES (signing) | ✅ YES (operator wallet per chain) | N/A |
| **Can steal funds** | 🔴 YES | 🔴 YES (same risk in CI) | 🟡 LZ DVNs |

### Implementation

**Coordinator and Trusted GMP already exist as separate services:**

1. **Coordinator Service** - UX functions (event monitoring, API, negotiation) - NO KEYS, CANNOT STEAL FUNDS
2. **Trusted GMP Service** - message relay for local/CI testing - REQUIRES FUNDED OPERATOR WALLET on each chain (private key in config, pays gas to call `lzReceive()`), CAN FORGE MESSAGES, CAN STEAL FUNDS

**Contracts use configurable GMP endpoint address:**

- **Local/CI**: Mock endpoint → Trusted GMP service relays messages
- **Production**: Real LZ endpoint → LZ handles delivery

> ⚠️ **Production only.** In production, contracts use GMP (no our signer). Trusted GMP remains for local/CI; current signature-based flow is deprecated for production.

### Benefits

- **Production-ready contracts** - same code path in all environments
- **Fast local/CI tests** - message delivery in ~500ms vs 1-30s with real GMP
- **Deterministic tests** - no flaky tests from network delays
- **Cost efficient** - no testnet gas fees for every CI run
- **Easier debugging** - full control over message delivery timing
- **Local development** - developers can test without testnet GMP setup

### Environment Configuration

| Environment | Movement | Solana | EVM |
| ----------- | -------- | ------ | --- |
| **Local/CI** | Mock + Trusted GMP | Mock + Trusted GMP | Mock + Trusted GMP |
| **Testnet** | Mock + Trusted GMP (LZ not yet available) | Real LZ (devnet) | Real LZ (Base Sepolia) |
| **Mainnet** | Real LZ | Real LZ | Real LZ |

> **Note:** LZ does not yet support Movement testnet. Until LZ testnet support is available, Movement testnet uses mock endpoints + Trusted GMP (same as local/CI). Mainnet can use Trusted GMP relay or LZ.

---

## Open Questions

1. **Gas Fee Economics:** Are GMP fees + on-chain validation costs acceptable for users?
2. **Solver Adoption:** Will solvers adapt to calling validation contracts instead of arbitrary transactions?
3. **Coordinator Role:** The coordinator handles negotiation/discovery - security doesn't depend on it, but UX does.
4. **Failure Modes:** How to handle GMP message delivery failures or delays? ✅ **RESOLVED** - On-chain expiry handles stuck intents, idempotency handles duplicate messages
5. **State Synchronization:** How to ensure intent requirements are always in sync across chains?
6. **Trusted GMP Mode:** Use Trusted GMP for local/CI (mock GMP) so production contracts stay identical. ✅ **ASSUMED** – already in place.

---

## Conclusion

### Summary

Adding GMP so production does not use our signer is **FEASIBLE** but requires:

- **On-chain validation** – move approval logic from Trusted GMP into contracts
- **GMP integration** – add cross-chain messaging to all contracts
- **New validation contracts** – deploy on all connected chains
- **Coordinator** – unchanged (already in place; no keys, no validation)
- **~1.5 week timeline** – testnet only

### Key Benefits

- ✅ **Production: no our signer** – no single key we operate
- ✅ **Decentralize trust** – leverage GMP validator networks
- ✅ **Transparency** – validation logic on-chain
- ✅ **Security** – no our private keys in production
- ✅ **Censorship resistance** – permissionless GMP execution

### Key Trade-offs

- ⚠️ **Higher gas costs** - GMP fees + on-chain validation
- ⚠️ **Contract complexity** - moderate increase (~50-100 lines per contract)
- ⚠️ **Development time** - ~1.5 weeks for testnet (with AI assistance)
- ⚠️ **New dependencies** - rely on GMP protocol security
- ⚠️ **Solver UX changes** - must use validation contracts

---

**End of Proposal**

> See execution phase documents for detailed implementation plan.
