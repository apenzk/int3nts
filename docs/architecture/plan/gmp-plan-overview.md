# GMP Integration Proposal: Replacing the Trusted Verifier

**Status:** Proposal
**Date:** 2026-01-22
**Summary:** Architectural proposal to replace the trusted off-chain verifier with Generic Message Passing (GMP) protocol integration, moving validation on-chain and leveraging cross-chain messaging for authorization.

> **🔷 GMP Protocol: LayerZero v2**
>
> This proposal uses **LayerZero v2** as the GMP protocol. LayerZero provides the best cross-chain coverage (Movement/Aptos, EVM, Solana), mature integration, and flexible executor network. See [GMP Protocol Comparison](#gmp-protocol-comparison) for full analysis.

---

## Executive Summary

### Current System

The intent framework uses a **trusted off-chain verifier service** that:

- Monitors events across hub and connected chains (Movement, Base, Solana)
- Validates transaction details (amounts, recipients, tokens, solver addresses)
- Generates cryptographic signatures (Ed25519/ECDSA) to authorize escrow releases
- Provides REST API for frontend and solver coordination

### Proposed System

Replace the verifier with **on-chain validation + GMP messaging**:

- Validation logic moves into smart contracts on each chain
- GMPs (LayerZero, Axelar, Wormhole, CCIP) handle cross-chain message delivery
- Contracts authenticate via GMP message verification (not signatures)
- Coordinator service for UX (no private keys, no validation, handles negotiation/discovery)

### Key Benefits

| Benefit | Impact |
|---------|--------|
| **Eliminate trusted party** | No single verifier key that can compromise system |
| **Censorship resistance** | Permissionless GMP networks vs. single verifier service |
| **Decentralization** | Trust GMP validator networks instead of single service |
| **Security** | Validation logic is transparent on-chain |
| **Operational simplicity** | No critical infrastructure to secure and maintain |

### Key Trade-offs

| Trade-off | Current | New |
|-----------|---------|-----|
| **Gas costs** | Low (signatures cheap) | Higher (GMP fees + on-chain validation) |
| **Contract complexity** | Low | Medium (validation logic on-chain) |
| **Infrastructure burden** | High (run verifier service) | Low (coordinator only) |
| **Flexibility** | Easy to update validation logic | Requires contract redeployment |

---

## Verifier Separation (Phase 0)

**Critical first step:** Before migrating to real GMP protocols, we split the current verifier into two independent services:

### Current Verifier (Monolithic)

```
┌─────────────────────────────────────────────────────────────┐
│                    VERIFIER SERVICE                         │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │ Event       │ │ Validation  │ │ Signature           │   │
│  │ Monitoring  │ │ Logic       │ │ Generation          │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐   │
│  │ REST API    │ │ Negotiation │ │ 🔴 PRIVATE KEYS     │   │
│  └─────────────┘ └─────────────┘ └─────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### New Architecture (Split)

```text
┌─────────────────────────────────┐  ┌─────────────────────────────────┐
│      COORDINATOR SERVICE        │  │      TRUSTED GMP SERVICE        │
│  ┌─────────────┐ ┌───────────┐  │  │  ┌─────────────┐ ┌───────────┐  │
│  │ Event       │ │ REST API  │  │  │  │ GMP Event   │ │ Message   │  │
│  │ Monitoring  │ │ (read)    │  │  │  │ Monitoring  │ │ Delivery  │  │
│  └─────────────┘ └───────────┘  │  │  └─────────────┘ └───────────┘  │
│  ┌─────────────┐ ┌───────────┐  │  │  ┌─────────────────────────┐    │
│  │ Event       │ │Negotiation│  │  │  │ 🔴 OPERATOR WALLET       │    │
│  │ Caching     │ │ API       │  │  │  │    (privkey in config)   │    │
│  └─────────────┘ └───────────┘  │  │  └─────────────────────────┘    │
│                                 │  │                                 │
│  🟢 NO KEYS                     │  │  🔴 CAN STEAL FUNDS             │
│  🟢 NO VALIDATION               │  │     (same risk as verifier)     │
│  🟢 CANNOT STEAL FUNDS          │  │                                 │
│                                 │  │  (Only for local/CI testing -   │
│                                 │  │   production uses real GMP)     │
└─────────────────────────────────┘  └─────────────────────────────────┘
```

**Security model by environment:**

| Environment | Message Verification | Trust Model |
|-------------|---------------------|-------------|
| **Current (Verifier)** | Verifier signatures | 🔴 Our service can steal funds |
| **Local/CI (Trusted GMP)** | Trusted GMP relays | 🔴 Our service can steal funds (same risk) |
| **Production (Real GMP)** | LayerZero DVNs | 🟡 LayerZero DVNs can steal funds |

**We are not eliminating trust - we are moving it.** In production, we trust LayerZero's DVN network instead of our own verifier. Whether this is "better" depends on:

- LayerZero's security track record
- DVN decentralization (how many, who operates them)
- Economic incentives and slashing mechanisms
- Your threat model (internal vs external attackers)

### Why This Matters

| Aspect | Current Verifier | Coordinator | Trusted GMP (testing) | Production (real GMP) |
|--------|------------------|-------------|----------------------|----------------------|
| **Can steal funds** | 🔴 YES (us) | 🟢 NO | 🔴 YES (us) | 🟡 YES (LayerZero DVNs) |
| **Validation logic** | 🔴 Off-chain | 🟢 On-chain | 🟢 On-chain | 🟢 On-chain |
| **If compromised** | 🔴 Steal funds | 🟢 Disrupt UX | 🔴 Steal funds | 🟡 Steal funds |
| **Trust assumption** | 🔴 Our service | 🟢 None | 🔴 Our service | 🟡 LayerZero network |

### Migration Benefits

- ✅ **Coordinator has no keys** - Cannot steal funds, only affects UX
- ✅ **Clean break** - Old verifier completely removed, no legacy code
- ✅ **Cleaner architecture** - Clear separation of concerns
- ✅ **On-chain validation** - All validation logic is transparent and auditable
- ✅ **Production security** - Trust moves from us to LayerZero DVNs
- ⚠️ **Testing still requires trust** - Trusted GMP for local/CI has same risk as verifier

> **See [Phase 0: Verifier Separation](gmp-plan-execution-phase0.md) for implementation details.**

---

## Architectural Changes

This migration moves validation logic on-chain and uses GMP for cross-chain message delivery. The table below shows how each current verifier task maps to the new architecture.

### Verifier Task Migration

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

**Key Finding:** While GMPs cannot directly replace the verifier's current functions, **architectural redesign moving validation on-chain makes all 15 verifier tasks feasible** through different mechanisms (on-chain validation + GMP messaging).

---

## Proposed Architecture

### Core Principle

Replace **"trusted off-chain validation + signatures"** with **"trustless on-chain validation + GMP messages"**.

### How It Works

#### Current Model

```
Off-chain Verifier parses arbitrary txs → Validates details → Signs approval
```

#### Proposed Model

```
On-chain Contract validates locally → Emits event → GMP delivers message → Receiving contract accepts
```

### GMP Auto-Execution Support

| GMP | Auto-Delivery to Destination | Auto-Initiation from Events | Who Triggers Source Call |
|-----|------------------------------|----------------------------|--------------------------|
| **LayerZero** | ✅ YES (Executors) | ❌ NO | Solver / User / Relayer |
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
3. **Verifier**: Observes escrow event, validates (amount, token, reservation match intent)
4. Hub: Solver fulfills intent (provides desired assets to requester)
5. **Verifier**: Observes fulfillment event, validates, generates signature
6. Connected Chain: Solver submits verifier signature → escrow releases requester's offering to solver

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
| **Connected Chain Escrow** | Validates via verifier signature | Validate requirements on-chain during creation | **MODIFY** - Move validation logic on-chain |
| **Connected Chain Escrow** | Validates via verifier signature | Add GMP send on escrow creation | **NEW** - Add outbound message |
| **Connected Chain Escrow** | Requires signature for release | Add GMP receive handler for fulfillment proof | **NEW** - Add inbound handler |
| **Connected Chain Escrow** | Uses `ed25519::verify_signature` | Use GMP message verification | **REPLACE** - Different auth mechanism |
| **Verifier Service** | Observes, validates, signs | **ELIMINATED** | **DELETE** |

### Outflow (Hub → Connected Chain)

#### Current Flow

1. Hub: Intent created (locks funds), emits event with requirements (recipient, amount, token, connected_chain_id)
2. **Verifier**: Observes intent event, caches it
3. Connected Chain: Solver submits **arbitrary transaction** (ERC20 transfer, SPL transfer, etc.)
4. **Verifier**: Queries transaction by hash, parses arguments/logs, validates (recipient, amount, token, solver address)
5. **Verifier**: Queries hub solver registry for solver's connected-chain address
6. **Verifier**: Signs intent_id if valid
7. Hub: Solver submits verifier signature → intent releases escrow

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
| **Verifier Service** | Parses txs, validates, queries registry, signs | **ELIMINATED** | **DELETE** |

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
| **Private Key Management** | Secure storage of verifier keys | 🔴 **CRITICAL** - Key compromise = total system breach |
| **Transaction Parsing** | Extract data from arbitrary transactions | 🔴 **HIGH** - Parsing bugs can validate wrong data |
| **Cross-Chain State Queries** | Query solver registry on hub | 🟡 **MEDIUM** - Must remain synced |

### What Remains (Transformed)

The verifier service transforms from **"Trusted Authority"** to **"Coordinator"**:

| Function | Current "Verifier" | New "Coordinator" | Change |
|----------|-------------------|---------------|--------|
| **Event monitoring** | ✅ YES | ✅ YES | Same functionality |
| **Event caching** | ✅ YES | ✅ YES | Same functionality |
| **REST API** | ✅ YES | ✅ YES | Same functionality |
| **Transaction parsing** | ✅ Complex parsing | ❌ NO | **Eliminated** |
| **Validation logic** | ✅ 15+ validation checks | ❌ NO | **Eliminated** |
| **Signature generation** | ✅ Ed25519/ECDSA signing | ❌ NO | **Eliminated** |
| **Private keys** | ✅ Critical security | ❌ NO KEYS | **Eliminated** |
| **Trust level** | 🔴 **CRITICAL** (can steal funds) | 🟢 **LOW** (read-only) | **Major improvement** |
| **If compromised** | 🔴 Funds can be stolen | 🟢 Worst case: API DoS | **Major improvement** |
| **If it goes down** | 🔴 System broken | 🟡 Security works, UX degraded (no negotiation) | **Major improvement** |

### Updated Infrastructure Complexity

| Aspect | Current | New | Impact |
|--------|---------|-----|--------|
| **Infrastructure YOU run** | Verifier service (Rust, monitoring, RPC, DB, keys, API) | Coordinator (monitoring, DB, API only) | **Major reduction** |
| **Infrastructure SOMEONE runs** | Just you | **GMP protocol operators** (already exists) | **Shifts to decentralized network** |
| **Service criticality** | 🔴 **CRITICAL** (system doesn't work without it) | 🟡 **REQUIRED FOR UX** (security works without it) | **Much better** |
| **Security requirements** | 🔴 **MAXIMUM** (holds keys, can steal funds) | 🟢 **MINIMAL** (read-only, no keys) | **Massive improvement** |
| **Operational burden** | 🔴 **HIGH** (key management, uptime critical) | 🟡 **MEDIUM** (uptime nice-to-have) | **Improved** |
| **Censorship power** | 🔴 **HIGH** (can refuse to sign valid txs) | 🟢 **NONE** (users can query chain directly) | **Eliminated** |

---

## Security Implications

### Trust Model Comparison

| Aspect | Current (Verifier) | New (GMP) |
|--------|-------------------|-----------|
| **Authority source** | Single verifier private key | GMP protocol (oracle network, validators, relayers) |
| **Validation location** | Off-chain (verifier service) | On-chain (smart contracts) |
| **Censorship resistance** | ❌ Verifier can refuse to sign | ✅ Permissionless GMP networks |
| **Liveness dependency** | Single verifier service must be online | GMP network must be operational (highly redundant) |
| **Security assumption** | Trust verifier key security + validation logic correctness | Trust GMP security model + on-chain validation correctness |
| **Key compromise impact** | 🔴 Total system breach - attacker can steal all escrowed funds | 🟢 No keys in coordinator - worst case is API DoS |
| **Validation bug impact** | 🔴 Wrong signatures issued, funds lost | 🟡 Contract bug affects only that contract (isolated, auditable) |
| **Transparency** | ❌ Off-chain logic, opaque validation | ✅ On-chain logic, fully transparent |

### Attack Surface Reduction

**Current attack vectors:**

- Verifier private key theft
- Verifier service compromise
- Validation logic bugs in off-chain code
- Transaction parsing vulnerabilities
- Cross-chain state desynchronization

**New attack vectors:**

- GMP protocol vulnerability (mitigated by established protocols)
- On-chain validation logic bugs (mitigated by audits, formal verification)

**Eliminated attack vectors:**

- ✅ No verifier private key to steal
- ✅ No off-chain validation logic bugs
- ✅ No transaction parsing vulnerabilities
- ✅ No key management security concerns

---

## Implementation Roadmap

See execution phase documents for detailed implementation plan:

- [Phase 0: Verifier Separation](gmp-plan-execution-phase0.md) (3-4 days) - **NEW** - Separate verifier into Coordinator + Trusted GMP
- [Phase 1: Research & Design](gmp-plan-execution-phase1.md) (2-3 days)
- [Phase 2: SVM Prototype](gmp-plan-execution-phase2.md) (3-4 days)
- [Phase 3: Multi-Chain Expansion](gmp-plan-execution-phase3.md) (5-7 days)
- [Phase 4: Coordinator Service](gmp-plan-execution-phase4.md) (2 days) - **UPDATED** - Now focuses on GMP integration (coordinator already extracted in Phase 0)
- [Phase 5: Integration & Documentation](gmp-plan-execution-phase5.md) (2-3 days)

**Total Timeline:** 2-3 weeks (testnet only)

---

## Cost Analysis

### Current System Costs

| Component | Cost Type | Estimate |
|-----------|-----------|----------|
| **Verifier Infrastructure** | Fixed monthly | $X/month (servers, monitoring) |
| **Development/Maintenance** | Ongoing | Y hours/month |
| **Security Audits** | Periodic | High (critical component) |
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

| Criterion | LayerZero | Axelar | Wormhole | CCIP |
|-----------|-----------|--------|----------|------|
| **Movement Support** | ✅ Aptos (similar) | ✅ Aptos | ✅ Aptos | ⚠️ Limited |
| **Solana Support** | ✅ YES | ✅ YES | ✅ YES (native) | ⚠️ Expanding |
| **EVM Support** | ✅ Excellent | ✅ Excellent | ✅ Excellent | ✅ Excellent |
| **Auto-Execution** | ✅ Executors | ✅ Gas Service | ⚠️ Partial | ✅ Built-in |
| **Message Fees** | Low-Medium | Medium | Low-Medium | Medium-High |
| **Trust Model** | Oracle+Relayer | Validator Set | Guardian Network | Chainlink DON |
| **Maturity** | High | High | High | High |

### Decision: LayerZero v2

**FINAL SELECTION:** LayerZero v2 has been selected as the GMP protocol for this implementation.

**Rationale:**
- Best cross-chain coverage (MVM, EVM, SVM)
- Mature Aptos integration (similar to Movement)
- Flexible executor network
- Competitive fees
- Strong ecosystem support

**Note:** This decision is final. All implementation phases will use LayerZero v2.

---

## Trusted GMP Mode: Verifier as GMP Alternative

### The Concept

**The Trusted GMP Service** replaces the verifier as a message relay for testing. It is used for:

- **Local development** - No need for testnet GMP infrastructure
- **CI testing** - Fast, deterministic message delivery
- **Debugging** - Easier to trace and debug message flows
- **Testing environments** - Full control over message delivery

### How It Works

| Environment | GMP Provider | Flow |
|-------------|--------------|------|
| **Production** | Real GMP (LayerZero) | Contract → `lzSend()` → [Real GMP DVNs + Executors] → `lzReceive()` → Destination |
| **Local/CI/Testing** | Trusted GMP Service | Contract → `lzSend()` → [Mock endpoint emits event] → [Trusted GMP watches] → [Calls `lzReceive()`] → Destination |

### Key Principle

**Contracts remain identical across all environments** - they use the same GMP interfaces. Only the underlying endpoint implementation differs:

- **Production**: Real LayerZero endpoint
- **Local/CI/Testing**: Mock endpoint + Trusted GMP service

### Verifier Transformation

| Aspect | Current Verifier | Trusted GMP Mode | Production (Real GMP) |
|--------|-----------------|------------------|----------------------|
| **Watches events** | `IntentCreated`, `EscrowCreated` | `MessageSent` (from mock endpoints) | N/A - not running |
| **Validates logic** | 15+ validation checks | ❌ NONE - contracts validate | N/A |
| **Action taken** | Generates signatures | Calls `lzReceive()` on destination contracts | N/A - real GMP delivers |
| **Private keys** | ✅ YES (verifier signing) | ✅ YES (operator wallet privkey per chain) | N/A |
| **Can steal funds** | 🔴 YES (forge signatures) | 🔴 YES (forge messages) | 🟡 LayerZero DVNs can |
| **Security impact** | 🔴 CRITICAL | 🔴 SAME AS VERIFIER | 🟡 Trust LayerZero |

### Implementation

**Phase 0 splits the verifier into two separate services:**

1. **Coordinator Service** - UX functions (event monitoring, API, negotiation) - NO KEYS, CANNOT STEAL FUNDS
2. **Trusted GMP Service** - message relay for local/CI testing - REQUIRES FUNDED OPERATOR WALLET on each chain (private key in config, pays gas to call `lzReceive()`), CAN FORGE MESSAGES, CAN STEAL FUNDS

**Contracts use configurable GMP endpoint address:**

- **Local/CI**: Mock endpoint → Trusted GMP service relays messages
- **Production**: Real LayerZero endpoint → LayerZero handles delivery

> ⚠️ **No backwards compatibility.** The current verifier (with keys, validation, signatures) is completely replaced. Old architecture is deprecated and removed.

### Benefits

- **Production-ready contracts** - same code path in all environments
- **Fast local/CI tests** - message delivery in ~500ms vs 1-30s with real GMP
- **Deterministic tests** - no flaky tests from network delays
- **Cost efficient** - no testnet gas fees for every CI run
- **Easier debugging** - full control over message delivery timing
- **Local development** - developers can test without testnet GMP setup

### Environment Configuration

1. **Local/CI**: Deploy mock endpoints, run Trusted GMP service for message relay
2. **Testnet**: Use real LayerZero endpoints on testnets
3. **Production**: Use real LayerZero endpoints on mainnets

---

## Open Questions

1. **Gas Fee Economics:** Are GMP fees + on-chain validation costs acceptable for users?
2. **Solver Adoption:** Will solvers adapt to calling validation contracts instead of arbitrary transactions?
3. **Coordinator Role:** The coordinator handles negotiation/discovery - security doesn't depend on it, but UX does.
4. **Failure Modes:** How to handle GMP message delivery failures or delays? ✅ **RESOLVED** - On-chain expiry handles stuck intents, idempotency handles duplicate messages
5. **State Synchronization:** How to ensure intent requirements are always in sync across chains?
6. **Trusted GMP Mode:** Should the verifier be converted to a "trusted GMP" provider for local/CI/testing? ✅ **RECOMMENDED** - Provides seamless development/testing experience while keeping production contracts identical.

---

## Conclusion

### Summary

Replacing the trusted verifier with GMP integration is **FEASIBLE** but requires:

- **Major architectural redesign** - not a drop-in replacement
- **On-chain validation** - move logic from verifier to contracts
- **GMP integration** - add cross-chain messaging to all contracts
- **New validation contracts** - deploy on all connected chains
- **Coordinator service** - handles negotiation/discovery (no keys, no validation)
- **4-6 week timeline** - achievable with AI-assisted development (testnet only)

### Key Benefits

- ✅ **Eliminate trusted verifier** - no single point of failure
- ✅ **Decentralize trust** - leverage GMP validator networks
- ✅ **Increase transparency** - validation logic on-chain
- ✅ **Improve security** - no private keys to compromise
- ✅ **Enable censorship resistance** - permissionless execution

### Key Trade-offs

- ⚠️ **Higher gas costs** - GMP fees + on-chain validation
- ⚠️ **Contract complexity** - moderate increase (~50-100 lines per contract)
- ⚠️ **Development time** - 2-3 weeks for testnet (with AI assistance)
- ⚠️ **New dependencies** - rely on GMP protocol security
- ⚠️ **Solver UX changes** - must use validation contracts

---

**End of Proposal**

> See execution phase documents for detailed implementation plan.
