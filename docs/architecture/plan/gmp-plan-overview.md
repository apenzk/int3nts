# GMP Integration Plan

**Status:** In Progress (Phase 4 In Progress)
**Date:** 2026-01-22
**Summary:** Generic Message Passing (GMP) for cross-chain messaging using Integrated GMP as the relay for all environments. Validation is on-chain; cross-chain messaging uses GMP messages.

> **Design Standard: GMP Interfaces**
>
> Contracts use GMP interfaces (`gmpSend()`/`gmpReceive()`, wire format) for cross-chain messaging. The sender/receiver split pattern avoids circular dependencies. All current environments use the **Integrated GMP relay**.

---

## Executive Summary

### System Architecture

- **Coordinator** -- event monitoring, REST API, negotiation (no keys, cannot authorize releases)
- **Integrated GMP** -- message relay; watches `MessageSent` events, calls `deliver_message()` on destination chains. Has operator wallets for gas payment only.
- **On-chain contracts** -- all validation logic is on-chain; contracts authenticate via GMP message verification

### Key Properties

| Property | Description |
|----------|-------------|
| **Security** | Validation logic is transparent on-chain |
| **Operational simplicity** | Coordinator + Integrated GMP relay |
| **Future-ready** | GMP interfaces; future external GMP provider integration is config-only |
| **Transparency** | On-chain validation, auditable contracts |

---

## Implementation Roadmap

See execution phase documents for detailed implementation plan:

- [Phase 1: Research & Design](gmp-plan-execution-phase1.md) -- Interfaces, message schemas, wire format spec
- [Phase 2: Complete GMP Implementation (MVM, SVM, EVM) & Architecture Alignment](gmp-plan-execution-phase2.md) -- GMP for all three chains, native relay, package restructuring, naming alignment (19 commits)
- [Phase 3: Coordinator Readiness Tracking](gmp-plan-execution-phase3.md) -- Readiness tracking for outflow intents (commit f46eb3d)
- [Phase 4: Integration & Documentation](gmp-plan-execution-phase4.md) -- Commits 1-6 done (stripped API, frontend/solver cleanup, testnet deploy, architecture docs, GMP integration docs). Remaining: final cleanup

---

## Architecture

### System Components

```text
┌─────────────────────────────────┐  ┌─────────────────────────────────┐
│      COORDINATOR SERVICE        │  │      INTEGRATED GMP SERVICE     │
│  Event monitoring, REST API,    │  │  Relay: watches MessageSent,    │
│  negotiation. NO KEYS.          │  │  calls deliver_message().       │
│  Cannot steal funds             │  │  Operator wallet keys for gas.  │
└─────────────────────────────────┘  └─────────────────────────────────┘
```

| Environment | Who authorizes releases | Trust |
|-------------|-------------------------|--------|
| **All environments** | On-chain contracts via GMP messages, Integrated GMP relay delivers | Relay delivers, but validation is on-chain |

### On-Chain Validation

All validation logic is on-chain. The table below shows how each validation task is handled:

| # | Task | How It Works |
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
| 11 | **GMP Message Authentication** | Contracts verify message source via GMP endpoint verification |
| 12 | **Cache & Serve Events** | Coordinator service (no keys, no validation) for UX |
| 13 | **Negotiation Routing** | Coordinator includes negotiation API (application logic, not security-critical) |

### How It Works

```text
On-chain contract validates → Sends GMP message → Receiving contract accepts via GMP
```

**Key Insight:** Contracts must be explicitly called to initiate messages — contracts call `gmpSend()` as part of their logic. The Integrated GMP relay watches for `MessageSent` events and delivers messages to destination chains by calling `gmpReceive()` / `deliver_message()`.

---

## Detailed Flows

### Inflow (Connected Chain → Hub)

1. Hub: Requester creates intent (wants assets on hub), **sends GMP message to connected chain** with escrow requirements
2. Connected Chain: **Receives escrow requirements via GMP** (idempotent: if requirements already exist for intent_id + step number, ignore duplicate message), stores them in validation contract
3. Connected Chain: Requester creates escrow (offers their assets, reserved for solver) → **contract validates requirements exist and match escrow details** (reverts if requirements don't exist or don't match) → **sends GMP message to hub** (escrow confirmed)
4. Hub: **Receives escrow confirmation via GMP**, allows fulfillment
5. Hub: Solver fulfills intent (provides desired assets to requester) → **sends GMP message to connected chain** (fulfillment confirmed)
6. Connected Chain: **Receives fulfillment confirmation via GMP** → escrow automatically releases requester's offering to solver

### Outflow (Hub → Connected Chain)

1. Hub: Intent created (locks funds) → **sends GMP message to connected chain** with requirements (recipient, amount, token, authorized solver address)
2. Connected Chain: **Validation contract receives requirements via GMP** (idempotent: if requirements already exist for intent_id + step number, ignore duplicate message), stores them (maps `intent_id/step → {requirements, authorizedSolver}`)
3. Connected Chain: **Authorized solver approves validation contract** to spend tokens (one-time, with large amount like MAX_UINT256)
4. Connected Chain: **Authorized solver calls validation contract function** (e.g., `fulfillIntent(intent_id, token, amount)`)
5. Validation Contract: **Pulls tokens via `transferFrom(authorizedSolver, contract, amount)`** (requires approval)
6. Validation Contract: **Validates** (amount, token match stored requirements, solver matches authorized solver)
7. Validation Contract: **Forwards tokens to user wallet**
8. Validation Contract: **Sends GMP message to hub** (calls `gmpSend()`)
9. Hub: **Receives fulfillment proof via GMP** → releases escrow to solver

**Note:** Steps 3-8 happen atomically in one transaction (after the one-time approval in step 3). The contract pulls tokens from the authorized solver's wallet, validates, forwards, and sends GMP - all in the same transaction.

**Idempotency & Failure Handling:**

- Each `intent_id` can have multiple sequenced GMP messages (e.g., `intent_id/step1`, `intent_id/step2`, etc.) that can go in both directions (hub → connected chain, connected chain → hub)
- Messages are sequenced with step numbers, so out-of-order delivery is not a concern (step1 must come before step2)
- Duplicate GMP messages (same step) are ignored (idempotent: if requirements already exist for `intent_id + step number`, ignore the duplicate)
- Messages don't have timeouts - if a message never arrives, the intent/escrow will expire on-chain (existing expiry mechanism)
- No retry logic needed - on-chain expiry handles stuck intents

---

## Contract Deployment

| Chain Type | Contracts Deployed |
|------------|-------------------|
| Hub (MVM) | Intent contracts (reservation, registry, escrow, outflow) + GMP receiver/sender modules |
| Connected MVM | Escrow contract + GMP integration + Outflow validation contract |
| Connected EVM | Escrow contract + GMP integration + Outflow validation contract |
| Connected SVM | Escrow program + GMP integration + Outflow validation program |

Each connected chain has **two** contract types:

1. **Inflow**: Escrow contract (receives intent requirements, validates on creation, sends confirmations, auto-releases on fulfillment proof)
2. **Outflow**: Validation contract (receives intent requirements, validates solver fulfillment, sends GMP proof)

---

## Infrastructure

**Services:**

- **Coordinator** (event monitoring, caching, REST API, negotiation) -- no keys
- **Integrated GMP relay** -- operator wallets for gas, delivers messages

| Aspect | Description |
|--------|-------------|
| **Infrastructure** | Coordinator + Integrated GMP relay |
| **Relay criticality** | Relay delivers, contracts validate |
| **Security** | Relay has operator wallets for gas only |
| **Validation location** | On-chain (contracts) |
| **Upgrade path** | Swap endpoint address to external GMP provider (config change only) |

---

## Security Model

### Trust Model

| Aspect | Description |
|--------|-------------|
| **Authority source** | On-chain contracts (Integrated GMP relay delivers) |
| **Validation location** | On-chain (smart contracts) |
| **Liveness dependency** | Relay must be online for message delivery |
| **Security assumption** | Trust on-chain validation + relay for delivery |
| **Key compromise impact** | Relay keys = gas wallets only |
| **Transparency** | On-chain logic, transparent and auditable |

### Attack Surface

**Attack vectors:**

- Integrated GMP relay compromise (relay has operator wallets; can deliver forged messages to integrated GMP endpoints)
- On-chain validation logic bugs (mitigated by audits, formal verification)

**Mitigations:**

- All validation logic is on-chain and auditable
- No off-chain validation or transaction parsing
- No approval signature generation
- Future external GMP provider integration would remove relay trust requirement

> **Trust model:** The Integrated GMP relay can forge messages to integrated GMP endpoints. Validation logic is on-chain and transparent, but the relay is still a trusted component. Future external GMP provider integration would remove this trust requirement.

---

## Cost Model

| Component | Cost Type | Estimate |
|-----------|-----------|----------|
| **Relay Gas Costs** | Per-message | Operator wallet pays gas for `deliver_message()` on each chain |
| **On-chain Validation Gas** | Per-transaction | Validation logic runs on-chain |
| **Coordinator + Relay Infrastructure** | Fixed monthly | $X/month (servers, monitoring) |
| **Security Audits** | Periodic | Contracts are auditable |

No third-party GMP fees. Relay operator pays gas for delivery on the destination chain.

---

## GMP Design Reference

Contracts follow LZ v2 conventions as a design standard:

- **Industry-standard interfaces** — `gmpSend()` / `gmpReceive()` naming, OApp patterns
- **Future upgrade path** — swapping to real LZ endpoints is a config change (endpoint address), not a code change
- **Proven patterns** — LZ's OApp architecture is battle-tested across Aptos, EVM, and Solana

All environments currently use the **Integrated GMP relay** with integrated GMP endpoints. See [LZ SVM Integration Research](lz-svm-integration.md) and [LZ MVM Integration Research](lz-mvm-integration.md) for the design reference research that informed our interfaces.

---

## Integrated GMP Relay

### Message Flow

| Environment | GMP Provider | Flow |
|-------------|--------------|------|
| **All environments** | Integrated GMP Service | Contract → `gmpSend()` → [Integrated GMP endpoint emits event] → [Integrated GMP watches] → [Calls `gmpReceive()`] → Destination |

### Key Principle

**Contracts use GMP interfaces** (`gmpSend()`/`gmpReceive()`). The Integrated GMP endpoints implement these interfaces. Swapping to an external GMP provider is a configuration change only.

### Integrated GMP Role

| Aspect | Description |
|--------|-------------|
| **Watches events** | `MessageSent` events on integrated GMP endpoints |
| **Validates logic** | None (contracts validate on-chain) |
| **Action taken** | Calls `deliver_message()` / `gmpReceive()` on destination |
| **Private keys** | Operator wallet per chain (gas only) |
| **Can forge messages** | Yes (can forge messages to integrated GMP endpoints) |

### Services

1. **Coordinator Service** -- UX functions (event monitoring, API, negotiation) -- NO KEYS, CANNOT STEAL FUNDS
2. **Integrated GMP Service** -- message relay for all environments -- REQUIRES FUNDED OPERATOR WALLET on each chain (private key in config, pays gas to call `deliver_message()`)

> **Trust model:** The Integrated GMP relay can forge messages. Validation logic is on-chain and transparent, but the relay is still a trusted component. Future external GMP provider integration would remove this trust requirement.

### Benefits

- **GMP-compatible contracts** — same code path, future external provider swap is config-only
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

> **Note:** All environments use Integrated GMP. Contracts follow GMP conventions so that future external GMP provider integration is a configuration change (swap endpoint address).

---

## Open Questions

1. **Gas Fee Economics:** Are relay gas + on-chain validation costs acceptable for users?
2. **Solver Adoption:** Will solvers adapt to calling validation contracts instead of arbitrary transactions?
3. **Coordinator Role:** The coordinator handles negotiation/discovery -- security doesn't depend on it, but UX does.
4. **Failure Modes:** How to handle GMP message delivery failures or delays? **RESOLVED** -- On-chain expiry handles stuck intents, idempotency handles duplicate messages
5. **State Synchronization:** How to ensure intent requirements are always in sync across chains?

---

> See execution phase documents for detailed implementation plan.
