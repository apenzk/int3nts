# GMP Message Wire Format Specification

**Status:** Draft
**Date:** 2026-01-28
**Purpose:** Define the exact byte layout for all cross-chain GMP messages in the int3nts protocol.

---

## Overview

Three message types flow between the hub (Movement) and connected chains (Solana, EVM):

| Type | Discriminator | Direction | When |
| ---- | ------------- | --------- | ---- |
| `IntentRequirements` | `0x01` | Hub → Connected | On intent creation |
| `EscrowConfirmation` | `0x02` | Connected → Hub | On escrow creation (inflow only) |
| `FulfillmentProof` | `0x03` | Either direction | On fulfillment |

All messages are the `message` field inside an LZ packet. LZ wraps them with its own header (nonce, srcEid, sender, dstEid, receiver, guid). We only define our application payload here.

---

## Encoding Rules

- **Fixed-width fields.** Every field has a known size. No length prefixes, no delimiters.
- **Big-endian integers.** All multi-byte integers use big-endian (network byte order).
- **32-byte addresses.** All addresses are padded to 32 bytes. EVM 20-byte addresses are left-padded with 12 zero bytes.
- **No serialization library.** Not BCS (Move-specific), not Borsh (Rust-specific), not ABI (Solidity-specific). Plain fixed-width bytes so every chain can encode/decode without dependencies.

### Why This Format

1. **Simplicity.** No schema, no versioning overhead, no variable-length fields. Every message type has a fixed size.
2. **Cross-chain compatible.** Move, Rust, and Solidity can all read/write fixed-width big-endian bytes without importing a serialization library.
3. **Deterministic.** Same input always produces the same bytes. No field ordering ambiguity, no optional fields.
4. **Small.** Our largest message is 145 bytes. Well within LZ packet limits and Solana transaction size constraints.

---

## Message Type 0x01: IntentRequirements

**Direction:** Hub → Connected chain
**Sent by:** Hub intent contract on `create_outflow_intent()` or `create_inflow_intent()`
**Received by:** Connected chain validation contract (outflow) or escrow contract (inflow)
**Purpose:** Tell the connected chain what requirements must be met for this intent.

### Byte Layout

```text
Offset  Size   Field              Type       Description
──────  ────   ─────              ────       ───────────
0       1      message_type       u8         0x01
1       32     intent_id          bytes32    Unique intent identifier
33      32     requester_addr     bytes32    Requester's address on the connected chain
65      8      amount_required    u64 BE     Required token amount
73      32     token_addr         bytes32    Token address on the connected chain
105     32     solver_addr        bytes32    Authorized solver address
137     8      expiry             u64 BE     Unix timestamp after which intent expires
```

**Total: 145 bytes**

### Field Notes

- `requester_addr`: The requester's address on the connected chain. For outflow, this is where the solver must deliver tokens. For inflow, this is who is allowed to create the escrow.
- `token_addr`: The token address on the **connected chain** (where the action happens). Not the hub token.
- `solver_addr`: The authorized solver's address on the chain where they will act. `bytes32(0)` if any solver is allowed.
- `expiry`: Unix seconds. `0` means no expiry (not recommended).

---

## Message Type 0x02: EscrowConfirmation

**Direction:** Connected chain → Hub
**Sent by:** Connected chain escrow contract on `create_escrow()`
**Received by:** Hub intent contract
**Purpose:** Confirm that an escrow matching the intent requirements has been created on the connected chain. The hub gates solver fulfillment on this confirmation.

### Byte Layout

```text
Offset  Size   Field              Type       Description
──────  ────   ─────              ────       ───────────
0       1      message_type       u8         0x02
1       32     intent_id          bytes32    The intent this escrow is for
33      32     escrow_id          bytes32    Unique escrow identifier on the connected chain
65      8      amount_escrowed    u64 BE     Escrowed token amount
73      32     token_addr         bytes32    Escrowed token address
105     32     creator_addr       bytes32    Address that created the escrow (requester)
```

**Total: 137 bytes**

### Field Notes

- `escrow_id`: The connected chain's identifier for the escrow. The hub stores this for reference but does not use it for validation — the hub trusts the GMP message origin (verified peer).
- `amount_escrowed` and `token_addr`: The hub can optionally cross-check these against the original intent requirements. The connected chain already validated them before creating the escrow.
- `creator_addr`: The requester's address on the connected chain.

---

## Message Type 0x03: FulfillmentProof

**Direction:** Either direction

- **Outflow:** Connected chain → Hub (solver fulfilled on connected chain, hub releases locked tokens)
- **Inflow:** Hub → Connected chain (solver fulfilled on hub, connected chain releases escrow)

**Sent by:** The contract where fulfillment happened
**Received by:** The contract that needs to release locked tokens
**Purpose:** Prove that a solver has fulfilled the intent, triggering token release on the other chain.

### Byte Layout

```text
Offset  Size   Field              Type       Description
──────  ────   ─────              ────       ───────────
0       1      message_type       u8         0x03
1       32     intent_id          bytes32    The fulfilled intent
33      32     solver_addr        bytes32    Address of the solver that fulfilled
65      8      amount_fulfilled   u64 BE     Fulfilled token amount
73      8      timestamp          u64 BE     Unix timestamp of fulfillment
```

**Total: 81 bytes**

### Field Notes

- `solver_addr`: The solver's address on the chain where fulfillment happened. The receiving chain already knows who to pay from its own state (intent reservation or escrow `reserved_solver`). This field is for auditing and optional cross-referencing.
- `amount_fulfilled`: The actual amount fulfilled. For outflow (connected → hub), the hub validates this against intent requirements. For inflow (hub → connected), the connected chain does not validate — receipt of the message is sufficient to release the escrow.
- `timestamp`: When fulfillment occurred. Used for auditing, not for validation logic.

---

## Summary

| Message | Discriminator | Size | Direction |
| ------- | ------------- | ---- | --------- |
| IntentRequirements | `0x01` | 145 bytes | Hub → Connected |
| EscrowConfirmation | `0x02` | 137 bytes | Connected → Hub |
| FulfillmentProof | `0x03` | 81 bytes | Either |

### Discriminator Byte

The first byte of every message is the type discriminator. Receivers switch on this byte to determine how to decode the rest:

```text
match message[0] {
    0x01 => decode_intent_requirements(message)
    0x02 => decode_escrow_confirmation(message)
    0x03 => decode_fulfillment_proof(message)
    _    => reject
}
```

### Address Encoding

All addresses are 32 bytes (`bytes32`):

| Chain | Native Size | Encoding |
| ----- | ----------- | -------- |
| Movement | 32 bytes | Use as-is |
| Solana | 32 bytes | Use as-is |
| EVM | 20 bytes | Left-pad with 12 zero bytes: `0x000000000000000000000000` + address |

---

## Implementation Mapping

Maps each spec field to the source variable in existing contracts. Use this when implementing encode/decode.

### IntentRequirements (0x01) — Sender: Hub

The hub populates this message differently per flow. Both flows call `lzSend()` as part of intent creation.

**Outflow** — source: `fa_intent_outflow::create_outflow_intent()`

| Spec field | Source variable | File |
|---|---|---|
| `intent_id` | `intent_id: address` | `fa_intent_outflow.move` |
| `requester_addr` | `requester_addr_connected_chain: address` | `fa_intent_outflow.move` |
| `amount_required` | `desired_amount: u64` | `fa_intent_outflow.move` |
| `token_addr` | `desired_metadata_addr: address` | `fa_intent_outflow.move` |
| `solver_addr` | `solver: address` | `fa_intent_outflow.move` |
| `expiry` | `expiry_time: u64` | `fa_intent_outflow.move` |

**Inflow** — source: `fa_intent_inflow::create_inflow_intent()`

| Spec field | Source variable | File |
|---|---|---|
| `intent_id` | `intent_id: address` | `fa_intent_inflow.move` |
| `requester_addr` | `requester_addr_connected_chain: address` | `fa_intent_inflow.move` |
| `amount_required` | `offered_amount: u64` | `fa_intent_inflow.move` |
| `token_addr` | `offered_metadata_addr: address` | `fa_intent_inflow.move` |
| `solver_addr` | `solver: address` | `fa_intent_inflow.move` |
| `expiry` | `expiry_time: u64` | `fa_intent_inflow.move` |

Note: `amount_required` maps to `desired_amount` (outflow) or `offered_amount` (inflow). In both cases it is the token amount relevant on the connected chain.

### EscrowConfirmation (0x02) — Sender: Connected chain

**SVM** — source: `Escrow` struct after `process_create_escrow()`

| Spec field | Source variable | File |
|---|---|---|
| `intent_id` | `escrow.intent_id: [u8; 32]` | `state.rs` |
| `escrow_id` | Escrow PDA pubkey (derived from `[ESCROW_SEED, intent_id]`) | `processor.rs` |
| `amount_escrowed` | `escrow.amount: u64` | `state.rs` |
| `token_addr` | `escrow.token_mint: Pubkey` | `state.rs` |
| `creator_addr` | `escrow.requester: Pubkey` | `state.rs` |

**EVM** — source: `Escrow` struct after `createEscrow()`

| Spec field | Source variable | File |
|---|---|---|
| `intent_id` | `intentId` (mapping key, `uint256`) | `IntentEscrow.sol` |
| `escrow_id` | `address(this)` (contract address) | `IntentEscrow.sol` |
| `amount_escrowed` | `escrow.amount` (`uint256`, must fit `u64`) | `IntentEscrow.sol` |
| `token_addr` | `escrow.token` (`address`, left-pad to 32 bytes) | `IntentEscrow.sol` |
| `creator_addr` | `escrow.requester` (`address`, left-pad to 32 bytes) | `IntentEscrow.sol` |

### FulfillmentProof (0x03) — Sender: Either

**Outflow (connected → hub)** — sent by the new outflow validation contract after solver fulfills on connected chain. Fields come from the fulfillment transaction context.

**Inflow (hub → connected)** — source: `fa_intent::finish_fa_receiving_session_with_event()`

| Spec field | Source variable | File |
|---|---|---|
| `intent_id` | `argument.intent_id` (from `FALimitOrder`) | `fa_intent.move` |
| `solver_addr` | `solver: address` (param) | `fa_intent.move` |
| `amount_fulfilled` | `provided_amount` (from `fungible_asset::amount()`) | `fa_intent.move` |
| `timestamp` | `timestamp::now_seconds()` | `fa_intent.move` |

These match the `LimitOrderFulfillmentEvent` fields emitted at the same point.

---

## Versioning

This spec has no version field. If the wire format changes in the future, we will introduce a new message type discriminator (e.g., `0x04`) rather than adding a version header. This keeps decoding simple and avoids breaking existing handlers.
