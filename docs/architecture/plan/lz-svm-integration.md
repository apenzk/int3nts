# LZ V2: Solana Integration Research

**Status:** Research
**Date:** 2026-01-28
**Purpose:** Document LZ V2's Solana integration for cross-chain messaging in the int3nts GMP system.

> **IMPORTANT: Verify all addresses, EIDs, and API details before implementation.**
> This document is based on research up to January 2026. Always cross-reference with:
>
> - <https://docs.layerzero.network/v2/developers/solana/oapp/overview>
> - <https://github.com/LZ-Labs/LZ-v2>
> - <https://layerzeroscan.com>

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [OApp Pattern on Solana](#2-oapp-pattern-on-solana)
3. [Sending Messages](#3-sending-messages)
4. [Receiving Messages](#4-receiving-messages)
5. [Endpoint Addresses and Program IDs](#5-endpoint-addresses-and-program-ids)
6. [LZ Endpoint IDs (EIDs)](#6-lz-endpoint-ids-eids)
7. [Payload Wrapping and Message Format](#7-payload-wrapping-and-message-format)
8. [Nonce Tracking](#8-nonce-tracking)
9. [Solana-Specific Quirks](#9-solana-specific-quirks)
10. [Integration Recommendations for int3nts](#10-integration-recommendations-for-int3nts)

---

## 1. Executive Summary

### Key Findings

| Topic | Finding |
|-------|---------|
| **LZ V2 on Solana** | Full mainnet and devnet support. Native Rust programs, not EVM-compiled. |
| **OApp Pattern** | CPI-based (Cross Program Invocation), not inheritance. OApp calls LZ endpoint program via CPI. |
| **PDAs** | Three required: OApp Store, Peer Config, lz_receive_types_accounts. |
| **Send** | CPI call to LZ endpoint program. OApp Store PDA is the signer seed. |
| **Receive** | Executor discovers accounts via `lz_receive_types_v2`, then calls `lz_receive` instruction. Must call `clear()` before modifying state. |
| **Payload Format** | Raw bytes. LZ wraps in its packet format. You encode/decode your payload manually. |
| **Nonces** | Per-pathway `(srcEid, sender, dstEid, receiver)`. Endpoint manages. Burned via `clear()` on receive. |
| **Major Quirk** | All accounts must be declared upfront (no dynamic dispatch). Account discovery via `lz_receive_types_v2` is critical for executors. |

---

## 2. OApp Pattern on Solana

### 2.1 Fundamental Differences from EVM

On EVM, an OApp inherits from `OApp.sol` and overrides `_lzReceive()`. On Solana, there is **no inheritance**. Instead:

1. Your program calls the LZ endpoint program via **CPI** (Cross Program Invocation) to send messages.
2. Your program exposes an `lz_receive` instruction that the LZ executor calls to deliver messages.
3. The LZ endpoint program verifies message authenticity. Your program verifies peer addresses.
4. OApp identity derives from **PDAs** (Program Derived Addresses), not program addresses.

### 2.2 Required PDAs

Three PDAs form the OApp foundation:

| PDA | Seed | Purpose |
|-----|------|---------|
| **OApp Store** | `[b"Store"]` (customizable) | Program state, admin config, endpoint details. Acts as the OApp's identity address for LZ. Also the signer seed for endpoint CPIs. |
| **Peer Config** | `[b"Peer", store_key, source_eid]` | One per remote chain. Validates incoming sender addresses. Initialized during wiring. |
| **lz_receive_types_accounts** | `[b"LzReceiveTypes", store_key]` | Stores account requirements for executor account discovery. |

### 2.3 Required Instructions

Your OApp program must implement these instructions:

| Instruction | Purpose |
|-------------|---------|
| `init` | Initialize OApp Store and lz_receive_types PDAs. Register with endpoint via `oapp::endpoint_cpi::register_oapp`. |
| `lz_receive` | Process inbound messages. Must call `endpoint::clear()` before modifying state. |
| `lz_receive_types_info` | Returns `(version, versioned_data)` for executor account discovery. |
| `lz_receive_types_v2` | Returns full execution plan including ALTs and instruction sequences. |

### 2.4 Key Architectural Differences from EVM

| Aspect | EVM (Solidity) | Solana (Rust) |
|--------|---------------|---------------|
| **OApp base** | Inherit from `OApp.sol` | CPI calls to endpoint program |
| **Contract identity** | Contract address | OApp Store PDA address |
| **Receive mechanism** | `_lzReceive()` override | `lz_receive` instruction |
| **Access control** | `onlyEndpoint` modifier | PDA constraints + signer checks |
| **State storage** | Contract storage slots | PDAs (Program Derived Addresses) |
| **Peer storage** | `mapping(uint32 => bytes32)` | Peer Config PDA per chain |
| **Fee payment** | `msg.value` in ETH | Lamports via SOL transfer |
| **Address format** | 20 bytes | 32 bytes (Pubkey) |
| **Account model** | Dynamic access | All accounts declared upfront |

---

## 3. Sending Messages

### 3.1 Send Flow

Sending a cross-chain message from your OApp:

```text
1. Your program builds the payload (application message)
2. Your program calls quote_send CPI to estimate fees
3. Your program calls send CPI to the LZ endpoint program
4. LZ endpoint assigns nonce, creates Packet
5. LZ endpoint routes to message library (ULN)
6. ULN emits PacketSent event
7. DVNs observe and verify
8. Executor picks up for delivery on destination chain
```

### 3.2 CPI Call Pattern

```rust
// Conceptual send pattern (Anchor-style)
pub fn send_message(
    ctx: Context<SendMessage>,
    dst_eid: u32,
    payload: Vec<u8>,
    options: Vec<u8>,
) -> Result<()> {
    // 1. Build the message
    let message = encode_intent_requirements(&payload);

    // 2. Quote the fee
    let (native_fee, zro_fee) = oapp::endpoint_cpi::quote(
        ctx.accounts.endpoint_program.to_account_info(),
        ctx.accounts.oapp_store.key(),
        dst_eid,
        peer_address,
        message.clone(),
        options.clone(),
        false, // pay_in_zro
    )?;

    // 3. Send via CPI
    // The OApp Store PDA provides the signer seed:
    // seeds = [STORE_SEED, &[store_bump]]
    let receipt = oapp::endpoint_cpi::send(
        ctx.accounts.endpoint_program.to_account_info(),
        ctx.accounts.oapp_store.to_account_info(),
        dst_eid,
        peer_address,
        message,
        options,
        native_fee,
        zro_fee,
    )?;

    // receipt contains: guid, nonce, native_fee, zro_fee
    Ok(())
}
```

**IMPORTANT:** The OApp Store PDA is used as the signer for CPI calls. This PDA's address is what gets registered as a peer on remote chains.

### 3.3 Options Encoding

The `options` parameter encodes executor settings for the destination chain:

```text
TYPE_3 options format:
[0x0003]                               -- options type (2 bytes)
[workerType][optionLength][optionData] -- worker options

For executor (workerType = 0x01):
TYPE 1 (gas limit only):
[0x01][0x0011][0x01][gasLimit(16 bytes)]

TYPE 2 (gas + native drop):
[0x01][0x0020][0x02][gasLimit(16 bytes)][nativeAmount(16 bytes)]
```

---

## 4. Receiving Messages

### 4.1 Account Discovery

Before delivering a message, the LZ executor must know which accounts the `lz_receive` instruction needs. This is done via a two-step discovery process:

```text
1. Executor calls lz_receive_types_info
   → Returns (version, versioned_data)
   → version identifies the protocol iteration

2. Executor calls lz_receive_types_v2
   → Returns full execution plan:
   - ALTs (Address Lookup Tables) to use
   - Instruction sequences
   - Account lists needed for lz_receive
```

This is necessary because Solana requires all accounts to be declared upfront in a transaction. The executor cannot dynamically discover accounts during execution.

### 4.2 lz_receive Instruction

```rust
// Conceptual lz_receive handler (Anchor-style)
pub fn lz_receive(
    ctx: Context<LzReceive>,
    src_eid: u32,
    sender: [u8; 32],
    nonce: u64,
    guid: [u8; 32],
    message: Vec<u8>,
    extra_data: Vec<u8>,
) -> Result<()> {
    // CRITICAL: Call clear() FIRST before modifying state.
    // Prevents replay attacks.
    oapp::endpoint_cpi::clear(
        ctx.accounts.endpoint_program.to_account_info(),
        ctx.accounts.oapp_store.to_account_info(),
        src_eid,
        sender,
        nonce,
    )?;

    // Verify peer
    let peer = &ctx.accounts.peer_config;
    require!(peer.address == sender, OAppError::InvalidPeer);

    // Decode and process message
    let msg_type = message[0];
    match msg_type {
        MSG_TYPE_INTENT_REQUIREMENTS => {
            handle_intent_requirements(&ctx, &message)?;
        }
        MSG_TYPE_FULFILLMENT_PROOF => {
            handle_fulfillment_proof(&ctx, &message)?;
        }
        MSG_TYPE_ESCROW_CONFIRMATION => {
            handle_escrow_confirmation(&ctx, &message)?;
        }
        _ => return Err(OAppError::UnknownMessageType.into()),
    }

    Ok(())
}
```

### 4.3 Critical: clear() Before State Changes

The `clear()` call burns the nonce and marks the message as delivered. This **must** happen before any state modifications:

```text
lz_receive() {
    clear()           // ← Burns nonce, prevents replay
    modify_state()    // ← Safe to change state now
}
```

If `clear()` is not called first, the message can be replayed.

### 4.4 Account Synchronization

The accounts returned by `lz_receive_types_v2` must match the accounts used in `lz_receive`. Specifically:

- Use `ctx.remaining_accounts` consistently across both instructions
- Include zero-pubkey signers for ATA initialization or rent payers
- Omitting required signers causes `AccountNotSigner` errors

---

## 5. Endpoint Addresses and Program IDs

### 5.1 Solana Program IDs (Mainnet)

| Program | Address |
|---------|---------|
| **Endpoint** | `76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6` |
| **Executor** | `6doghB248px58JSSwG4qejQ46kFMW4AMj7vzJnWZHNZn` |
| **DVN** | `HtEYV4xB4wvsj5fgTkcfuChYpvGYzgzwvNhgDZQNh7wW` |
| **ULN (Message Library)** | `7a4WjyR8VZ7yZz5XJAKm39BUGn5iT9CKcv2pmG9tdXVH` |
| **Price Feed** | `8ahPGPjEbpgGaZx2NV1iG5Shj7TDwvsjkEDcGWjt94TP` |
| **OFT** | `HRPXLCqspQocTjfcX4rvAPaY9q6Gwb1rrD3xXWrfJWdW` |
| **Blocked Message Library** | `2XrYqmhBMPJgDsb4SVbjV1PnJBprurd5bzRCkHwiFCJB` |

Source: [LZ-v2 verify-contracts.md](https://github.com/LZ-Labs/LZ-v2/blob/main/packages/layerzero-v2/solana/programs/verify-contracts.md)

### 5.2 Solana Program IDs (Devnet)

Devnet program IDs are **not listed** in the verification guide. The LZ devtools scaffold uses different endpoint addresses for devnet. **VERIFY** devnet addresses from:

- `@layerzerolabs/lz-definitions` npm package
- LZ devtools example configs

### 5.3 EVM Endpoint Addresses (for reference)

For the connected EVM chains in int3nts:

| Network | Endpoint Address |
|---------|-----------------|
| **EVM Mainnet (all chains)** | `0x1a44076050125825900e736c501f859c50fE728c` |
| **EVM Testnet (all chains)** | `0x6EDCE65403992e310A62460808c4b910D972f10f` |

LZ V2 uses the same endpoint address across most EVM chains.

---

## 6. LZ Endpoint IDs (EIDs)

### 6.1 Known EIDs

| Chain | EID | Type |
|-------|-----|------|
| **Solana Mainnet** | `30168` | Mainnet (chain ID 101) |
| **Solana Devnet** | `40168` | Testnet (chain ID 103) |
| **Ethereum Mainnet** | `30101` | Mainnet |
| **Base Mainnet** | `30184` | Mainnet |
| **Base Sepolia** | `40245` | Testnet |
| **Arbitrum Mainnet** | `30110` | Mainnet |
| **Aptos Mainnet** | `30108` | Mainnet |
| **Movement Mainnet** | `30325` | Mainnet |

### 6.2 EID Format

LZ V2 EIDs follow a convention:

- **Mainnet:** `30xxx` (e.g., 30101 for Ethereum, 30168 for Solana)
- **Testnet:** `40xxx` (e.g., 40168 for Solana devnet)

### 6.3 How EIDs Are Used

```rust
// When sending from Solana to Movement:
let dst_eid: u32 = 30325; // Movement mainnet

// When receiving on Solana from Movement:
// origin.src_eid will be 30325

// Peer config maps EID -> trusted remote address:
// set_peer(30325, <movement_oapp_address_as_bytes32>)
// set_peer(30101, <ethereum_oapp_address_as_bytes32>)
```

---

## 7. Payload Wrapping and Message Format

### 7.1 LZ Packet Format

LZ wraps your application payload in a **Packet** structure:

```text
LZ V2 Packet (internal, handled by LZ):
┌─────────────────────────────────────────────────┐
│ nonce     : u64    (8 bytes)                     │
│ srcEid    : u32    (4 bytes)                     │
│ sender    : bytes32 (32 bytes) - source OApp     │
│ dstEid    : u32    (4 bytes)                     │
│ receiver  : bytes32 (32 bytes) - dest OApp       │
│ guid      : bytes32 (32 bytes) - unique msg ID   │
│ message   : bytes  (variable) - YOUR PAYLOAD     │
└─────────────────────────────────────────────────┘
```

You only control the `message` field. Everything else is added by LZ.

### 7.2 Application Payload

Your payload is raw bytes that you encode/decode. LZ does not interpret it. For int3nts:

```text
Application Payload Format:
┌─────────────────────────────────────────────────┐
│ message_type  : u8     (1 byte) - discriminator  │
│ intent_id     : bytes32 (32 bytes)               │
│ ... (type-specific fields)                       │
└─────────────────────────────────────────────────┘

Message Types:
  0x01 = IntentRequirements (hub → connected)
  0x02 = EscrowConfirmation (connected → hub)
  0x03 = FulfillmentProof   (either direction)
```

### 7.3 Encoding in Rust

```rust
use borsh::{BorshSerialize, BorshDeserialize};

// Option A: Manual fixed-width encoding (recommended for cross-chain)
// Matches the encoding used on Move and Solidity sides.

pub const MSG_TYPE_INTENT_REQUIREMENTS: u8 = 0x01;
pub const MSG_TYPE_ESCROW_CONFIRMATION: u8 = 0x02;
pub const MSG_TYPE_FULFILLMENT_PROOF: u8 = 0x03;

/// Encode IntentRequirements message
/// Layout: [msg_type(1)][intent_id(32)][recipient(32)][amount(8)][token(32)][solver(32)][expiry(8)]
pub fn encode_intent_requirements(
    intent_id: &[u8; 32],
    recipient: &[u8; 32],
    amount: u64,
    token: &[u8; 32],
    solver: &[u8; 32],
    expiry: u64,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(1 + 32 + 32 + 8 + 32 + 32 + 8);
    payload.push(MSG_TYPE_INTENT_REQUIREMENTS);
    payload.extend_from_slice(intent_id);
    payload.extend_from_slice(recipient);
    payload.extend_from_slice(&amount.to_be_bytes());
    payload.extend_from_slice(token);
    payload.extend_from_slice(solver);
    payload.extend_from_slice(&expiry.to_be_bytes());
    payload
}

/// Decode IntentRequirements message
pub fn decode_intent_requirements(payload: &[u8]) -> Result<IntentRequirements, ProgramError> {
    if payload.len() < 1 + 32 + 32 + 8 + 32 + 32 + 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    if payload[0] != MSG_TYPE_INTENT_REQUIREMENTS {
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut offset = 1;

    let intent_id: [u8; 32] = payload[offset..offset + 32].try_into().unwrap();
    offset += 32;

    let recipient: [u8; 32] = payload[offset..offset + 32].try_into().unwrap();
    offset += 32;

    let amount = u64::from_be_bytes(payload[offset..offset + 8].try_into().unwrap());
    offset += 8;

    let token: [u8; 32] = payload[offset..offset + 32].try_into().unwrap();
    offset += 32;

    let solver: [u8; 32] = payload[offset..offset + 32].try_into().unwrap();
    offset += 32;

    let expiry = u64::from_be_bytes(payload[offset..offset + 8].try_into().unwrap());

    Ok(IntentRequirements {
        intent_id,
        recipient,
        amount,
        token,
        solver,
        expiry,
    })
}
```

### 7.4 Cross-Chain Address Encoding

LZ V2 uses **bytes32** for all addresses:

| Chain | Native Address Size | LZ bytes32 Encoding |
|-------|--------------------|--------------------|
| Solana | 32 bytes (Pubkey) | No padding needed |
| Aptos/Movement | 32 bytes | No padding needed |
| EVM | 20 bytes | Left-pad with 12 zero bytes |

```rust
/// Convert an EVM address (20 bytes) to bytes32 (32 bytes)
pub fn evm_to_bytes32(evm_addr: &[u8; 20]) -> [u8; 32] {
    let mut result = [0u8; 32];
    result[12..].copy_from_slice(evm_addr);
    result
}
```

---

## 8. Nonce Tracking

### 8.1 How Nonces Work

LZ V2 tracks nonces per **pathway**: `(srcEid, sender, dstEid, receiver)`.

- Each unique pathway has its own monotonically incrementing nonce counter.
- The endpoint assigns nonces on send and verifies them on receive.
- Nonces start from 1.

### 8.2 Nonce on Solana

The LZ endpoint program manages nonces internally. On receive, the `clear()` CPI call burns the nonce:

```text
Receive flow:
1. Executor submits lz_receive transaction
2. Your program calls clear(src_eid, sender, nonce)
3. Endpoint marks nonce as consumed
4. Your program processes the message
```

### 8.3 Ordered vs Unordered Delivery

| Mode | Behavior | Use Case |
|------|----------|----------|
| **Ordered** | Messages delivered in nonce order. Nonce N must be delivered before N+1. | Default. Use when message order matters. |
| **Unordered** | Messages delivered in any order. Each nonce tracked independently. | When messages are independent. |

**For int3nts:** Ordered delivery is recommended. Our flows have sequential steps (requirements -> escrow -> fulfillment). Step N must complete before step N+1.

### 8.4 Idempotency

Your OApp does not manage nonces directly. The endpoint handles it via `clear()`. For application-level idempotency, track processed messages:

```rust
// Track processed messages by intent_id + step_number
#[account]
pub struct ProcessedMessage {
    pub intent_id: [u8; 32],
    pub step: u8,
    pub processed: bool,
}

// PDA seed: [b"processed", intent_id, step]
```

---

## 9. Solana-Specific Quirks

### 9.1 Upfront Account Declaration

Solana requires all accounts to be declared in the transaction before execution. This means:

- The LZ executor must know every account your `lz_receive` handler needs
- The `lz_receive_types_v2` instruction provides this information
- If your handler touches different accounts depending on message type, all possible accounts must be discoverable

**Impact on int3nts:** The `lz_receive_types_v2` instruction must return accounts for all three message types (IntentRequirements, EscrowConfirmation, FulfillmentProof). The executor uses `ctx.remaining_accounts` for dynamic account access.

### 9.2 Transaction Size Limits

Solana transactions have a 1232-byte limit. Cross-chain payloads and account lists must fit within this. LZ uses **Address Lookup Tables (ALTs)** to compress account lists.

**Impact on int3nts:** Our payloads are small (< 200 bytes). The concern is the number of accounts in `lz_receive`, not payload size. ALT support in `lz_receive_types_v2` helps.

### 9.3 Priority Fees

Solana uses priority fees for transaction ordering during congestion:

```text
priorityFee = compute_budget × compute_unit_price (in micro-lamports)
```

**Impact on int3nts:** The LZ executor pays priority fees for `lz_receive` delivery. For sending (`lz_send`), the caller (requester/solver) pays. Configure appropriate compute unit prices.

### 9.4 Rent

Solana accounts must be rent-exempt. Creating PDAs for peer configs and processed message tracking requires SOL for rent.

**Impact on int3nts:** Budget for rent costs when initializing OApp PDAs and per-message tracking accounts.

### 9.5 Compute Budget

Solana has a per-transaction compute unit limit (default 200k, max 1.4M). Cross-chain message processing must fit within budget.

**Impact on int3nts:** Our message handlers are simple (decode + state update). Should fit within default compute budget. If not, request higher budget with `ComputeBudgetInstruction::set_compute_unit_limit`.

### 9.6 Token Decimals

Solana uses `u64` for token amounts. Maximum supply depends on decimals:

| Decimals | Max Supply |
|----------|-----------|
| 9 | ~18 billion |
| 6 | ~18 trillion |
| 4 | ~1.8 quadrillion |

**Impact on int3nts:** SPL tokens typically use 6 or 9 decimals. Cross-chain amount encoding must account for differing decimal precision between chains. Use a shared decimal standard in the wire format.

### 9.7 No Try/Catch

Like Move, Solana programs cannot catch errors. A failed `lz_receive` aborts the entire transaction. The executor will retry, but if the error is deterministic, the message gets stuck.

**Impact on int3nts:** Validate all inputs. Consider a "store then process" pattern for fault tolerance.

---

## 10. Integration Recommendations for int3nts

### 10.1 SVM Program Structure

```text
intent-frameworks/svm/programs/
├── intent-escrow/              # Existing escrow program (modify)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── instructions/
│   │   │   ├── create_escrow.rs    # Add: validate stored requirements
│   │   │   ├── claim.rs            # Remove: signature verification
│   │   │   └── lz_receive.rs       # New: handle incoming GMP messages
│   │   └── state/
│   │       ├── escrow.rs
│   │       └── intent_requirements.rs  # New: stored requirements from hub
├── outflow-validator/          # New program (outflow validation)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── instructions/
│   │   │   ├── lz_receive.rs       # Receive IntentRequirements from hub
│   │   │   ├── fulfill_intent.rs   # Solver calls: validate + transfer + lz_send
│   │   │   └── lz_receive_types.rs # Account discovery for executor
│   │   └── state/
│   │       └── stored_requirements.rs
├── native-gmp-endpoint/         # New program (local/CI only)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── instructions/
│   │   │   ├── send.rs             # Emits MessageSent event
│   │   │   └── deliver_message.rs  # Trusted-GMP calls this
│   │   └── state/
│   │       └── config.rs
└── gmp-common/                 # New crate (shared)
    ├── src/
    │   ├── lib.rs
    │   ├── messages.rs             # Encode/decode all 3 message types
    │   └── types.rs                # Shared types
    └── Cargo.toml
```

### 10.2 Key Design Decisions

1. **OApp Store PDA as identity:** The OApp Store PDA address is registered as a peer on remote chains. It's the "contract address" for LZ purposes.

2. **CPI for all endpoint calls:** Send and receive both use CPI to the LZ endpoint program. The Store PDA provides the signer seed.

3. **Payload encoding:** Fixed-width big-endian encoding (not Borsh, not ABI). This is cross-chain compatible with Move and Solidity encoders.

4. **Peer verification in lz_receive:** Always check the Peer Config PDA matches the sender. The endpoint verifies message authenticity; you verify the application-level peer.

5. **clear() before state:** Always call `endpoint::clear()` as the first operation in `lz_receive` to prevent replay.

6. **Account discovery:** Implement `lz_receive_types_v2` to return all accounts needed for all message types. The executor builds the transaction from this.

### 10.3 Items Requiring Verification

| Item | What to Verify | Where to Check |
|------|---------------|----------------|
| **Devnet program IDs** | Endpoint, executor, DVN addresses on devnet | `@layerzerolabs/lz-definitions` npm package |
| **CPI interface** | Exact accounts and data for send/clear CPIs | LZ V2 Solana SDK source on GitHub |
| **lz_receive signature** | Exact instruction data layout expected by executor | LZ V2 Solana OApp example |
| **ALT support** | How to register ALTs for lz_receive_types_v2 | LZ V2 Solana docs |
| **Compute budget** | Whether lz_receive fits in default 200k CU | Testing on devnet |
| **Anchor compatibility** | Whether to use Anchor or native Solana for the OApp | LZ V2 examples use Anchor |

### 10.4 Recommended Next Steps

1. **Clone and examine** the LZ V2 Solana OApp example:
   - `npx create-lz-oapp@latest` → select Solana OApp
   - Study the generated program structure, CPI calls, and account setup

2. **Study the devtools examples:**
   - <https://github.com/LZ-Labs/devtools/tree/main/examples/oapp-solana>
   - Focus on send, receive, and account discovery patterns

3. **Build native-gmp-endpoint first** (as planned in Phase 1)
   - Implements the same CPI interface as the real LZ endpoint
   - Emits `MessageSent` events for the Trusted-GMP relay
   - Unblocks development immediately

---

## Appendix A: LZ V2 Solana Programs Reference

```text
Mainnet Programs:
  Endpoint:               76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6
  Executor:               6doghB248px58JSSwG4qejQ46kFMW4AMj7vzJnWZHNZn
  DVN:                    HtEYV4xB4wvsj5fgTkcfuChYpvGYzgzwvNhgDZQNh7wW
  ULN (Message Library):  7a4WjyR8VZ7yZz5XJAKm39BUGn5iT9CKcv2pmG9tdXVH
  Price Feed:             8ahPGPjEbpgGaZx2NV1iG5Shj7TDwvsjkEDcGWjt94TP
  OFT:                    HRPXLCqspQocTjfcX4rvAPaY9q6Gwb1rrD3xXWrfJWdW
  Blocked Msglib:         2XrYqmhBMPJgDsb4SVbjV1PnJBprurd5bzRCkHwiFCJB
```

## Appendix B: References

- LZ V2 Solana OApp Overview: <https://docs.layerzero.network/v2/developers/solana/oapp/overview>
- LZ V2 Solana Getting Started: <https://docs.layerzero.network/v2/developers/solana/getting-started>
- LZ V2 Solana Guidance: <https://docs.layerzero.network/v2/developers/solana/technical-reference/solana-guidance>
- LZ V2 GitHub: <https://github.com/LZ-Labs/LZ-v2>
- LZ V2 Devtools (examples): <https://github.com/LZ-Labs/devtools>
- LZ Scan: <https://layerzeroscan.com>
- Verify Contracts: <https://github.com/LZ-Labs/LZ-v2/blob/main/packages/layerzero-v2/solana/programs/verify-contracts.md>

---

**End of Research Document**
