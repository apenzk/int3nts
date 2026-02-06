# Phase 3 Commit 6: Intent Type Difference Analysis

**Status:** Complete
**Date:** 2026-02-06

---

## Executive Summary

The hub chain (MVM) uses two different intent types for inflow vs outflow:

| Flow | Intent Type | Module | Used By |
|------|-------------|--------|---------|
| **Inflow** | `FALimitOrder` | `fa_intent` | `fa_intent_inflow` |
| **Outflow** | `OracleGuardedLimitOrder` | `fa_intent_with_oracle` | `fa_intent_outflow` |

The types share a common core (desired token, amount, chain IDs, requester) but differ in **authorization model** and **cross-chain awareness**. `OracleGuardedLimitOrder` provides defense-in-depth via type-level authorization checks; `FALimitOrder` relies entirely on wrapper-level security.

---

## Field Comparison

### FALimitOrder (fa_intent.move:42)

```move
struct FALimitOrder has store, drop {
    desired_metadata: Object<Metadata>,
    desired_amount: u64,
    requester_addr: address,
    intent_id: Option<address>,    // None for regular, Some for cross-chain
    offered_chain_id: u64,
    desired_chain_id: u64
}
```

**6 fields.** No authorization requirement embedded in the type.

### OracleGuardedLimitOrder (fa_intent_with_oracle.move:61)

```move
struct OracleGuardedLimitOrder has store, drop {
    desired_metadata: Object<Metadata>,
    desired_amount: u64,
    desired_chain_id: u64,
    offered_chain_id: u64,
    requester_addr: address,
    requirement: OracleSignatureRequirement,  // UNIQUE
    intent_id: address,                       // NOT optional
    requester_addr_connected_chain: Option<address>,  // UNIQUE
}
```

```move
struct OracleSignatureRequirement has store, drop, copy {
    min_reported_value: u64,
    public_key: ed25519::UnvalidatedPublicKey,
}
```

**8 fields** (counting `requirement` sub-fields: effectively 9). Authorization requirement embedded in the type.

### Field-by-Field Comparison

| Field | `FALimitOrder` | `OracleGuardedLimitOrder` | Notes |
|-------|--------------------------|--------------------------|-------|
| `desired_metadata` | `Object<Metadata>` | `Object<Metadata>` | Identical |
| `desired_amount` | `u64` | `u64` | Identical |
| `requester_addr` | `address` | `address` | Identical |
| `offered_chain_id` | `u64` | `u64` | Identical |
| `desired_chain_id` | `u64` | `u64` | Identical |
| `intent_id` | `Option<address>` | `address` (non-optional) | Different types |
| `requirement` | **absent** | `OracleSignatureRequirement` | Oracle-only |
| `requester_addr_connected_chain` | **absent** | `Option<address>` | Oracle-only |

**5 fields shared**, 1 field differs in type (`intent_id`), 2 fields unique to `OracleGuardedLimitOrder`.

---

## intent_id Difference

| | `FALimitOrder` | `OracleGuardedLimitOrder` |
|-|--------------------------|--------------------------|
| **Type** | `Option<address>` | `address` |
| **Reason** | Used for both regular (no cross-chain) and cross-chain intents. Regular intents have `None`. | Always cross-chain (outflow). Always has an intent ID. |
| **Usage** | Event emission uses `intent_id` if `Some`, else `intent_addr` | Signature verification, GMP proof lookup |

This difference reflects that `FALimitOrder` predates cross-chain support and was retrofitted with an optional intent_id, while `OracleGuardedLimitOrder` was designed cross-chain-first.

---

## Finish Function Analysis

### FALimitOrder: `finish_fa_receiving_session_with_event` (fa_intent.move:411)

Checks performed:
1. `provided_metadata == desired_metadata` (token type match)
2. `provided_amount >= desired_amount` (amount sufficient)

That's it. **No authorization check.** Any caller that obtains a `Session<FALimitOrder>` can finish the intent by providing the correct tokens.

### OracleGuardedLimitOrder: Two finish paths

**Path 1: `finish_fa_receiving_session_with_oracle` (fa_intent_with_oracle.move:296)**

Checks performed:
1. `provided_metadata == desired_metadata`
2. `provided_amount >= required_payment_amount` (cross-chain aware: 0 if cross-chain)
3. `verify_oracle_requirement(argument, &oracle_witness_opt)` — Ed25519 signature verification

**Path 2: `finish_fa_receiving_session_for_gmp` (fa_intent_with_oracle.move:339)**

Checks performed:
1. `gmp_intent_state::is_fulfillment_proof_received(intent_id)` — GMP proof received
2. `provided_metadata == desired_metadata`
3. `provided_amount >= required_payment_amount` (cross-chain aware: 0 if cross-chain)

Both paths include **type-level authorization** (oracle signature or GMP proof) that cannot be bypassed even if a caller somehow obtains a session directly.

---

## Cross-Chain Payment Logic

| | `FALimitOrder` | `OracleGuardedLimitOrder` |
|-|--------------------------|--------------------------|
| **Payment validation** | Always `amount >= desired_amount` | `desired_chain_id == offered_chain_id` → `desired_amount`; otherwise → `0` |
| **Reason** | Inflow: solver pays desired tokens on hub (same chain as offered) | Outflow: solver pays on connected chain, not hub. Hub payment is 0 for cross-chain. |

`OracleGuardedLimitOrder` explicitly models the fact that in outflow, the actual token delivery happens on a different chain. The hub-side session completes with 0 payment because the real payment was verified via GMP proof from the connected chain.

`FALimitOrder` doesn't need this distinction because inflow always involves the solver delivering tokens on the hub chain itself.

---

## Security Model Comparison

### FALimitOrder — Wrapper-Level Security Only

```text
User creates intent (fa_intent_inflow.create_inflow_intent)
    → Registers in gmp_intent_state
    → Sends IntentRequirements via GMP to connected chain

Solver fulfills (fa_intent_inflow.fulfill_inflow_intent)
    → WRAPPER CHECK: gmp_intent_state::is_escrow_confirmed()  ← security gate
    → Calls fa_intent::finish_fa_receiving_session_with_event()
    → TYPE CHECK: metadata + amount only (no auth check)
```

**Risk:** If someone bypasses `fa_intent_inflow.fulfill_inflow_intent` and calls `fa_intent::finish_fa_receiving_session_with_event` directly with a valid session + correct tokens, the intent completes with **no escrow confirmation check**. The authorization check lives solely in the wrapper.

**Mitigation:** Obtaining a `Session<FALimitOrder>` requires calling `intent::start_session()`, which requires the solver to be registered. This is a meaningful barrier but is not an authorization check tied to the specific fulfillment.

### OracleGuardedLimitOrder — Defense-in-Depth

```text
User creates intent (fa_intent_outflow.create_outflow_intent)
    → Registers in gmp_intent_state
    → Sends IntentRequirements via GMP to connected chain

GMP delivers proof (fa_intent_outflow.receive_fulfillment_proof)
    → Records proof in gmp_intent_state

Solver fulfills (fa_intent_outflow.fulfill_outflow_intent)
    → WRAPPER CHECK: gmp_intent_state::is_fulfillment_proof_received()
    → Calls fa_intent_with_oracle::finish_fa_receiving_session_for_gmp()
    → TYPE CHECK: GMP proof received + metadata + amount  ← ALSO checks auth
```

**Protection:** Even if someone bypasses `fa_intent_outflow.fulfill_outflow_intent` and calls `finish_fa_receiving_session_for_gmp` directly, the function **still checks** `gmp_intent_state::is_fulfillment_proof_received()`. The authorization is enforced at the type level.

Similarly, `finish_fa_receiving_session_with_oracle` enforces oracle signature verification at the type level.

---

## Revocation

| | `FALimitOrder` | `OracleGuardedLimitOrder` |
|-|--------------------------|--------------------------|
| **Revocable?** | Yes — `revoke_fa_intent()` in `fa_intent.move` | **No** — revocation removed for security |
| **Reason** | Inflow intents can be safely cancelled (escrow on connected chain is separate) | Outflow intents lock tokens on hub; allowing revocation after solver has fulfilled on connected chain would cause loss |

---

## Witness Types

| | `FALimitOrder` | `OracleGuardedLimitOrder` |
|-|--------------------------|--------------------------|
| **Witness** | `FungibleAssetRecipientWitness {}` (empty) | `OracleGuardedWitness {}` (empty) |
| **Purpose** | Proves intent completed | Proves intent completed |

Both witnesses are empty structs. The difference is that `OracleGuardedWitness` can only be created by `finish_fa_receiving_session_with_oracle` or `finish_fa_receiving_session_for_gmp`, both of which enforce authorization. `FungibleAssetRecipientWitness` is created by functions that do not enforce authorization.

---

## Summary of Differences

| Aspect | `FALimitOrder` | `OracleGuardedLimitOrder` |
|--------|--------------------------|--------------------------|
| **Fields** | 6 | 8 (+2 sub-fields in requirement) |
| **intent_id** | `Option<address>` | `address` (always present) |
| **Authorization in type** | None | Oracle signature OR GMP proof |
| **Cross-chain payment** | Always required | 0 when cross-chain |
| **Connected chain address** | Not tracked | `requester_addr_connected_chain` |
| **Revocable** | Yes | No |
| **Security model** | Wrapper-level only | Defense-in-depth |
| **Used for** | Inflow (tokens come TO hub) | Outflow (tokens go FROM hub) |
| **Finish paths** | 1 (`finish_fa_receiving_session_with_event`) | 2 (oracle path + GMP path) |

---

## Security Implications by Field

| Field | Security Role |
|-------|---------------|
| `desired_metadata` | Prevents token type substitution attacks |
| `desired_amount` | Prevents underpayment |
| `requester_addr` | Ensures tokens are deposited to the correct recipient |
| `offered_chain_id` / `desired_chain_id` | Determines payment model (same-chain vs cross-chain) |
| `intent_id` | Links hub intent to connected chain escrow/validation — critical for GMP proof lookup |
| `requirement` (oracle) | Embeds public key + minimum value — enables type-level signature verification |
| `requester_addr_connected_chain` | Tells solver where to deliver tokens on connected chain — if wrong, solver sends to wrong address |

---

## Potential Unification Considerations

The key tension is:

1. **Inflow** doesn't need type-level authorization because the security gate (escrow confirmation) is inherently external — it comes from the connected chain via GMP. The check happens in the wrapper.

2. **Outflow** needs type-level authorization because the security gate (fulfillment proof) must be verified even if someone bypasses the wrapper. Defense-in-depth is critical here because outflow locks real tokens on the hub.

A unified type would need to either:
- Always embed authorization (adds unnecessary complexity to inflow)
- Conditionally require authorization (runtime check instead of compile-time)
- Use generics to parameterize the authorization strategy

These approaches are evaluated in Commits 7-8.
