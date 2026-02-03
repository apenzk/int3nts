# LZ V2: Movement/Aptos Integration Research

**Status:** Research
**Date:** 2026-01-28
**Purpose:** Document LZ V2's Aptos and Movement integration for cross-chain messaging in the int3nts GMP system.

> **IMPORTANT: Verify all addresses, EIDs, and API details before implementation.**
> This document is based on research up to January 2026. Always cross-reference with:
>
> - <https://docs.layerzero.network/v2>
> - <https://github.com/LZ-Labs/LZ-v2>
> - <https://layerzeroscan.com>
> - Movement Labs announcements

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [OApp Pattern on Aptos/Movement](#2-oapp-pattern-on-aptosmovement)
3. [lz_send and lz_receive in Move](#3-lz_send-and-lz_receive-in-move)
4. [Endpoint Addresses](#4-endpoint-addresses)
5. [LZ Endpoint IDs (EIDs)](#5-lz-endpoint-ids-eids)
6. [Payload Wrapping and Message Format](#6-payload-wrapping-and-message-format)
7. [Nonce Tracking](#7-nonce-tracking)
8. [Chain-Specific Quirks (Move/Aptos)](#8-chain-specific-quirks-moveaptos)
9. [Movement-Specific Considerations](#9-movement-specific-considerations)
10. [Integration Recommendations for int3nts](#10-integration-recommendations-for-int3nts)

---

## 1. Executive Summary

### Key Findings

| Topic | Finding |
|-------|---------|
| **LZ V2 on Aptos** | LZ V2 has full Aptos mainnet support. The integration uses native Move modules, not EVM-style contracts. |
| **LZ V2 on Movement** | **Movement mainnet is supported** (EID 30325). **Movement testnet support is uncertain** -- may require using mock endpoints + Trusted GMP for testnet. Verify current status. |
| **OApp Pattern** | Different from EVM. Move modules use friend functions and resource accounts rather than inheritance. OApp is a module that interacts with the LZ endpoint module via function calls. |
| **lz_send / lz_receive** | Move does not have `msg.sender` or inheritance. `lz_send` is called via the LZ endpoint module. `lz_receive` is a function in your module called by the LZ executor via the endpoint. |
| **Payload Format** | Raw bytes (`vector<u8>`). LZ wraps your payload in its own packet format. You encode/decode your application payload manually. |
| **Nonces** | LZ V2 on Aptos uses nonces per (srcEid, sender, dstEid, receiver) pathway. Nonces are tracked by the endpoint. |
| **Major Quirk** | Move's resource model means OApps are modules published at addresses, not instantiated contracts. Module upgrade policies and resource account patterns are critical. |

---

## 2. OApp Pattern on Aptos/Movement

### 2.1 How OApp Works on Aptos (Different from EVM)

On EVM, an OApp inherits from `OApp.sol` and overrides `_lzSend()` and `_lzReceive()`. On Aptos/Move, there is **no inheritance**. Instead:

1. Your Move module calls the LZ endpoint module's `send()` function to send messages.
2. Your Move module exposes a `lz_receive()` (or similarly named) entry function that the LZ executor calls via the endpoint to deliver messages.
3. The LZ endpoint module verifies the message authenticity before calling your module's receive function.

### 2.2 Move Module Structure for an OApp

The LZ V2 Aptos SDK provides a set of Move modules that your application interacts with. The key pattern is:

```text
your_oapp/
├── Move.toml                    # Dependencies include LZ packages
├── sources/
│   ├── oapp.move                # Your OApp module (main logic)
│   ├── oapp_config.move         # Configuration (peers, DVN settings)
│   └── oapp_core.move           # Core send/receive wrappers
```

### 2.3 LZ V2 Aptos OApp SDK Structure

The LZ V2 OApp SDK provides 5 interconnected modules:

| Module | Responsibility |
| -------- | --------------- |
| `oapp::oapp` | Entry functions: custom send/receive logic. Developer's main module. |
| `oapp::oapp_core` | Message sending (`lz_send`), fee quoting (`lz_quote`), peer/admin management. |
| `oapp::oapp_receive` | Low-level message reception and validation. Routes to `lz_receive_impl`. |
| `oapp::oapp_compose` | Optional composable message logic for multi-step workflows. |
| `oapp::oapp_store` | Persistent state: `OAppStore` resource with peers, admin, delegate, enforced options. |

The LZ V2 Aptos packages (published on-chain) include:

```text
layerzero-v2/aptos/
├── endpoint/                    # Core endpoint module
│   ├── sources/
│   │   ├── endpoint.move        # Main endpoint: send(), verify(), lz_receive()
│   │   ├── messaging_channel.move # Channel management, nonce tracking
│   │   ├── messaging_composer.move # Compose message support
│   │   └── endpoint_codec.move  # Packet encoding/decoding
├── protocol/
│   ├── sources/
│   │   ├── msglib/              # Message library (ULN302)
│   │   ├── uln/                 # Ultra Light Node
│   │   └── executor/            # Executor module
├── oapp/
│   ├── sources/
│   │   ├── oapp.move            # Developer entry functions
│   │   ├── oapp_core.move       # lz_send, lz_quote, peer management
│   │   ├── oapp_receive.move    # lz_receive entry, routes to lz_receive_impl
│   │   ├── oapp_compose.move    # Compose message support (optional)
│   │   └── oapp_store.move      # OAppStore resource, admin/delegate
└── oft/                         # OFT (Omnichain Fungible Token) example
    ├── sources/
    │   ├── oft.move             # OFT implementation
    │   └── oft_core.move        # OFT core logic
```

### 2.4 Required Functions for an OApp

Your OApp module needs to implement/interact with:

| Function | Direction | Purpose |
|----------|-----------|---------|
| `send()` (calls endpoint) | Outbound | Send a cross-chain message |
| `lz_receive()` | Inbound | Receive and process a cross-chain message |
| `set_peer()` | Config | Register trusted remote OApp addresses |
| `set_enforced_options()` | Config | Set execution options per destination |

### 2.5 Minimal OApp Module Example (Conceptual)

```move
module your_addr::your_oapp {
    use std::vector;
    use std::signer;
    use aptos_framework::event;

    // Import LZ endpoint modules
    use layerzero_v2::endpoint;
    use layerzero_v2::oapp_core;

    // ============================================================
    // State
    // ============================================================

    /// Stores the OApp configuration
    struct OAppConfig has key {
        admin: address,
        lz_endpoint_addr: address,
        // Peers: mapping of dst_eid -> remote OApp address (as bytes32)
        // In Move, this would be a Table or SimpleMap
    }

    /// Stores peer addresses per destination chain
    struct PeerStore has key {
        peers: aptos_std::simple_map::SimpleMap<u32, vector<u8>>,
    }

    // ============================================================
    // Initialize
    // ============================================================

    /// Initialize the OApp with LZ endpoint address
    public entry fun initialize(
        admin: &signer,
        lz_endpoint_addr: address,
    ) {
        move_to(admin, OAppConfig {
            admin: signer::address_of(admin),
            lz_endpoint_addr,
        });
        move_to(admin, PeerStore {
            peers: aptos_std::simple_map::new(),
        });
    }

    // ============================================================
    // Configuration
    // ============================================================

    /// Set a trusted peer for a destination chain
    /// The peer_address is the remote OApp's address as bytes32
    public entry fun set_peer(
        admin: &signer,
        dst_eid: u32,
        peer_address: vector<u8>, // 32 bytes, left-padded for EVM addresses
    ) acquires OAppConfig, PeerStore {
        let config = borrow_global<OAppConfig>(signer::address_of(admin));
        assert!(signer::address_of(admin) == config.admin, 1);

        let store = borrow_global_mut<PeerStore>(signer::address_of(admin));
        if (aptos_std::simple_map::contains_key(&store.peers, &dst_eid)) {
            aptos_std::simple_map::remove(&mut store.peers, &dst_eid);
        };
        aptos_std::simple_map::add(&mut store.peers, dst_eid, peer_address);
    }

    // ============================================================
    // Send (Outbound)
    // ============================================================

    /// Send a cross-chain message
    /// The caller pays LZ fees (in native APT/MOVE tokens)
    public entry fun send_message(
        sender: &signer,
        dst_eid: u32,
        payload: vector<u8>,
        // options: vector<u8>,  // Executor options (gas limit, etc.)
        // native_fee: u64,      // Fee in native tokens
        // zro_fee: u64,         // Fee in ZRO tokens (usually 0)
    ) acquires OAppConfig, PeerStore {
        let sender_addr = signer::address_of(sender);
        let config = borrow_global<OAppConfig>(sender_addr);
        let store = borrow_global<PeerStore>(sender_addr);

        // Verify peer is set for destination
        assert!(
            aptos_std::simple_map::contains_key(&store.peers, &dst_eid),
            2 // EPEER_NOT_SET
        );

        let peer = *aptos_std::simple_map::borrow(&store.peers, &dst_eid);

        // Call LZ endpoint to send the message
        // The actual LZ SDK call would be:
        // endpoint::send(
        //     sender,
        //     dst_eid,
        //     peer,        // receiver address (bytes32)
        //     payload,     // your application payload
        //     options,     // executor options
        //     native_fee,  // messaging fee
        //     zro_fee,     // ZRO fee
        // );
    }

    // ============================================================
    // Receive (Inbound)
    // ============================================================

    /// Called by the LZ endpoint when a message arrives
    /// This function MUST be callable only by the LZ endpoint
    ///
    /// In LZ V2 Aptos, the endpoint calls this through a verified pathway.
    /// The function signature must match what the endpoint expects.
    public entry fun lz_receive(
        // The LZ endpoint passes these parameters:
        src_eid: u32,           // Source chain endpoint ID
        sender: vector<u8>,     // Source OApp address (bytes32)
        nonce: u64,             // Message nonce
        guid: vector<u8>,       // Globally unique message ID
        payload: vector<u8>,    // Your application payload
        extra_data: vector<u8>, // Additional data (usually empty)
    ) acquires OAppConfig, PeerStore {
        // 1. Verify the sender is a trusted peer
        // (The LZ endpoint already verifies the message, but we also check peer)
        let oapp_addr = @your_addr;
        let store = borrow_global<PeerStore>(oapp_addr);
        assert!(
            aptos_std::simple_map::contains_key(&store.peers, &src_eid),
            3 // EUNKNOWN_PEER
        );
        let expected_peer = *aptos_std::simple_map::borrow(&store.peers, &src_eid);
        assert!(sender == expected_peer, 4); // EINVALID_SENDER

        // 2. Decode and process the payload
        // Your application logic here
        // e.g., decode intent_id, amount, token, etc. from payload
        process_message(src_eid, payload);
    }

    /// Internal message processing
    fun process_message(src_eid: u32, payload: vector<u8>) {
        // Decode payload and execute business logic
        // Example: release escrow, store requirements, etc.
    }
}
```

**IMPORTANT CAVEATS about this example:**

- The actual LZ V2 Aptos API may differ in function signatures. The above is a conceptual illustration based on the known LZ V2 architecture pattern for Aptos.
- The real LZ V2 Aptos SDK uses resource accounts and specific module patterns. Check the actual SDK.
- The `lz_receive` function's exact parameters and calling convention depend on the LZ endpoint implementation.

### 2.6 Key Architectural Differences from EVM

| Aspect | EVM (Solidity) | Aptos (Move) |
|--------|---------------|--------------|
| **OApp base** | Inherit from `OApp.sol` | Import LZ endpoint module, no inheritance |
| **Contract identity** | Contract address (deployed instance) | Module address (published module) |
| **Receive mechanism** | `_lzReceive()` override, called via `receive()` | `lz_receive()` entry function, called by executor |
| **Access control** | `onlyEndpoint` modifier, `msg.sender` check | Must verify caller is LZ endpoint |
| **State storage** | Contract storage slots | Move resources under account addresses |
| **Peer storage** | `mapping(uint32 => bytes32)` | `SimpleMap<u32, vector<u8>>` or `Table` |
| **Fee payment** | `msg.value` in ETH | Aptos Coin transfer in APT |
| **Address format** | 20 bytes (EVM address) | 32 bytes (Aptos address) |
| **Composability** | External calls between contracts | Friend functions or public entry functions |

---

## 3. lz_send and lz_receive in Move

### 3.1 Sending Messages (lz_send)

In LZ V2 on Aptos, sending a message involves calling `oapp_core::lz_send()`. The confirmed function signature:

```move
/// oapp_core::lz_send — confirmed from LZ V2 Aptos OApp docs
public(friend) fun lz_send(
    dst_eid: u32,
    message: vector<u8>,
    options: vector<u8>,
    native_fee: &mut FungibleAsset,        // Fee in native APT/MOVE tokens
    zro_fee: &mut Option<FungibleAsset>,   // Fee in ZRO tokens (usually None)
): MessagingReceipt
```

Fee estimation before sending:

```move
/// oapp_core::lz_quote — confirmed from LZ V2 Aptos OApp docs
#[view]
public fun lz_quote(
    dst_eid: u32,
    message: vector<u8>,
    options: vector<u8>,
    pay_in_zro: bool,
): (u64, u64)  // (native_fee, zro_fee)
```

The endpoint:

1. Assigns a nonce
2. Creates a Packet (nonce + srcEid + sender + dstEid + receiver + guid + message)
3. Sends to the configured message library (ULN302)
4. ULN302 emits events for DVNs and executors to pick up
5. Returns a `MessagingReceipt` with nonce, guid, fee paid

**Note:** Uses `FungibleAsset` (Aptos Fungible Asset standard), not the legacy `Coin<AptosCoin>` type.

**Constructing the `options` parameter:**

The options parameter encodes execution parameters for the destination chain. For Aptos:

```text
// Options encoding (LZ V2 options format):
// TYPE_3 options format:
// [0x0003]                    -- options type (2 bytes)
// [workerType][optionLength][optionData]...
//
// For executor (workerType = 0x01):
// [0x01][0x0011][0x01][gasLimit(16 bytes)]  -- TYPE 1: gas limit
// [0x01][0x0020][0x02][gasLimit(16 bytes)][nativeAmount(16 bytes)] -- TYPE 2: gas + native drop
//
// Example: 200,000 gas limit
// 0x0003 0x01 0x0011 0x01 0x000000000000000000000000000030d40
```

**Fee Estimation:**

Before sending, you should estimate the fee:

```move
// Conceptual fee estimation
// endpoint::quote(dst_eid, receiver, message, options, pay_in_zro)
// Returns (native_fee: u64, zro_fee: u64)
```

### 3.2 Receiving Messages (lz_receive)

LZ V2 uses a two-layer receive pattern:

1. `oapp::oapp_receive::lz_receive` — entry point called by the executor. Validates the message, checks peer, then dispatches.
2. `oapp::oapp::lz_receive_impl` — friend function where you implement your business logic.

**Confirmed entry point (from oapp_receive module):**

```move
/// Called by the LZ executor after message verification
public entry fun lz_receive(
    src_eid: u32,
    sender: vector<u8>,     // 32 bytes (Bytes32)
    nonce: u64,
    guid: vector<u8>,       // 32 bytes
    message: vector<u8>,    // Your application payload
    extra_data: vector<u8>,
)
```

**Confirmed developer handler (from oapp module):**

```move
/// Developer implements this — called by oapp_receive::lz_receive via friend
public(friend) fun lz_receive_impl(
    _src_eid: u32,
    _sender: Bytes32,
    _nonce: u64,
    _guid: Bytes32,
    _message: vector<u8>,
    _extra_data: vector<u8>,
    receive_value: Option<FungibleAsset>,
)
```

**int3nts implementation pattern:**

```move
public(friend) fun lz_receive_impl(
    src_eid: u32,
    sender: Bytes32,
    _nonce: u64,
    _guid: Bytes32,
    message: vector<u8>,
    _extra_data: vector<u8>,
    _receive_value: Option<FungibleAsset>,
) {
    // 1. Peer already verified by oapp_receive module

    // 2. Decode and process message
    let message_type = *vector::borrow(&message, 0);

    if (message_type == MSG_TYPE_INTENT_REQUIREMENTS) {
        handle_intent_requirements(message);
    } else if (message_type == MSG_TYPE_FULFILLMENT_PROOF) {
        handle_fulfillment_proof(message);
    } else if (message_type == MSG_TYPE_ESCROW_CONFIRMATION) {
        handle_escrow_confirmation(message);
    } else {
        abort 100 // Unknown message type
    };
}
```

### 3.3 The OAppStore Resource

The OApp SDK stores state in an `OAppStore` resource at the `@oapp` address:

```move
/// Confirmed from LZ V2 Aptos OApp docs
struct OAppStore has key {
    contract_signer: ContractSigner,
    admin: address,
    peers: Table<u32, Bytes32>,                         // EID -> remote peer address
    delegate: address,
    enforced_options: Table<EnforcedOptionsKey, vector<u8>>,
}
```

**Admin vs Delegate roles:**

- **Admin**: Sets local storage (peers, enforced options) via `set_peer()`, `transfer_admin()`
- **Delegate**: Calls endpoint-level changes (DVNs, executors, message libraries) via `set_delegate()`

### 3.4 The LZ V2 Aptos Endpoint Interface

The LZ V2 endpoint on Aptos exposes these key functions:

```move
module layerzero_v2::endpoint {
    /// Send a message to a remote chain
    public fun send(
        sender: &signer,
        dst_eid: u32,
        receiver: vector<u8>,     // bytes32
        message: vector<u8>,
        options: vector<u8>,
        native_fee: &mut FungibleAsset,
        zro_fee: &mut Option<FungibleAsset>,
    ): MessagingReceipt;

    /// Quote the fee for sending a message
    public fun quote(
        sender: address,
        dst_eid: u32,
        receiver: vector<u8>,
        message: vector<u8>,
        options: vector<u8>,
        pay_in_zro: bool,
    ): (u64, u64); // (native_fee, zro_fee)

    struct Origin has copy, drop, store {
        src_eid: u32,
        sender: vector<u8>,  // bytes32
        nonce: u64,
    }

    struct MessagingReceipt has copy, drop, store {
        guid: vector<u8>,
        nonce: u64,
        native_fee: u64,
        zro_fee: u64,
    }
}
```

**Note:** Fee parameters use `FungibleAsset` (Aptos Fungible Asset standard), not the legacy `Coin<T>` type.

### 3.4 Resource Account Pattern

On Aptos, OApps typically use a **resource account** pattern:

```move
/// The OApp owns a resource account that acts as the "contract address"
/// This resource account's address is what gets registered as a peer on other chains
struct OAppResourceAccount has key {
    signer_cap: account::SignerCapability,
}

/// Create the resource account during initialization
public entry fun initialize(admin: &signer) {
    let (resource_signer, signer_cap) = account::create_resource_account(
        admin,
        b"my_oapp_seed", // Deterministic seed
    );
    move_to(&resource_signer, OAppResourceAccount { signer_cap });

    // Register with LZ endpoint using resource account address
    // The resource account address is your OApp's "contract address"
}

/// Get the resource account signer for sending messages
fun get_resource_signer(): signer acquires OAppResourceAccount {
    let cap = &borrow_global<OAppResourceAccount>(@your_addr).signer_cap;
    account::create_signer_with_capability(cap)
}
```

---

## 4. Endpoint Addresses

### 4.1 Aptos Endpoint Addresses

| Network | Endpoint Address | Status |
|---------|-----------------|--------|
| **Aptos Mainnet** | `0x54ad3d30af77b60d939ae356e6606de9a4da67583f02b962d2d3f2e481484e90` | **VERIFY** - This is the known LZ V2 endpoint on Aptos mainnet. Verify on <https://docs.layerzero.network> |
| **Aptos Testnet** | `0x...` (check docs) | **VERIFY** - Aptos testnet endpoint exists but address should be confirmed |

**Important:** Aptos addresses are 32 bytes (64 hex chars), not 20 bytes like EVM.

### 4.2 Movement Endpoint Addresses

| Network | Endpoint Address | Status |
|---------|-----------------|--------|
| **Movement Mainnet** | Unknown / To be confirmed | **VERIFY** - LZ has announced Movement mainnet support (EID 30325 is allocated). Check if endpoint is deployed. |
| **Movement Testnet** | **NOT AVAILABLE** (as of early 2025) | LZ does not appear to have testnet support for Movement. Use mock endpoints + Trusted GMP for testnet. |

### 4.3 Relevant EVM Endpoint Addresses (for reference)

For the connected EVM chains in int3nts:

| Network | Endpoint Address |
|---------|-----------------|
| **Ethereum Mainnet** | `0x1a44076050125825900e736c501f859c50fE728c` |
| **Base Mainnet** | `0x1a44076050125825900e736c501f859c50fE728c` |
| **Base Sepolia (testnet)** | `0x6EDCE65403992e310A62460808c4b910D972f10f` |
| **Arbitrum Mainnet** | `0x1a44076050125825900e736c501f859c50fE728c` |

**Note:** LZ V2 uses the same endpoint address across most EVM chains: `0x1a44076050125825900e736c501f859c50fE728c` for mainnet and `0x6EDCE65403992e310A62460808c4b910D972f10f` for testnet.

---

## 5. LZ Endpoint IDs (EIDs)

LZ V2 assigns each chain a unique 32-bit Endpoint ID (EID). These are used in `lz_send()` as `dst_eid` and in `lz_receive()` as `src_eid`.

### 5.1 Known EIDs

| Chain | EID | Type |
|-------|-----|------|
| **Aptos Mainnet** | `30108` | Mainnet |
| **Aptos Testnet** | `40108` | Testnet |
| **Movement Mainnet** | `30325` | Mainnet |
| **Movement Testnet** | `40325` (unconfirmed) | Testnet -- **VERIFY: May not exist yet** |
| **Ethereum Mainnet** | `30101` | Mainnet |
| **Base Mainnet** | `30184` | Mainnet |
| **Base Sepolia** | `40245` | Testnet |
| **Arbitrum Mainnet** | `30110` | Mainnet |
| **Solana Mainnet** | `30168` | Mainnet |
| **Solana Devnet** | `40168` | Testnet |

### 5.2 EID Format

LZ V2 EIDs follow a convention:

- **Mainnet:** `30xxx` (e.g., 30101 for Ethereum, 30108 for Aptos)
- **Testnet:** `40xxx` (e.g., 40108 for Aptos testnet)

### 5.3 How EIDs Are Used

```move
// When sending from Movement to Ethereum:
let dst_eid: u32 = 30101; // Ethereum mainnet

// When receiving on Movement from Ethereum:
// origin.src_eid will be 30101

// Your peer configuration maps EID -> trusted remote address:
// set_peer(30101, <ethereum_oapp_address_as_bytes32>)
// set_peer(30168, <solana_oapp_address_as_bytes32>)
```

---

## 6. Payload Wrapping and Message Format

### 6.1 LZ Packet Format

LZ wraps your application payload in a **Packet** structure:

```text
LZ V2 Packet (internal format, handled by LZ):
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

**You only control the `message` field.** Everything else is added by LZ.

### 6.2 Your Application Payload

Your payload (`message` field) is raw bytes that you encode/decode. LZ does not interpret your payload. You design the format.

For int3nts, the recommended encoding (from the wire format spec in phase 1):

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

### 6.3 Encoding in Move

Move uses `vector<u8>` for byte arrays. Encoding/decoding is manual:

```move
module your_addr::gmp_messages {
    use std::vector;
    use std::bcs;

    const MSG_TYPE_INTENT_REQUIREMENTS: u8 = 0x01;
    const MSG_TYPE_ESCROW_CONFIRMATION: u8 = 0x02;
    const MSG_TYPE_FULFILLMENT_PROOF: u8 = 0x03;

    /// Encode IntentRequirements message
    /// Layout: [msg_type(1)][intent_id(32)][recipient(32)][amount(8)][token(32)][solver(32)][expiry(8)]
    public fun encode_intent_requirements(
        intent_id: vector<u8>,    // 32 bytes
        recipient: vector<u8>,    // 32 bytes
        amount: u64,
        token: vector<u8>,        // 32 bytes
        solver: vector<u8>,       // 32 bytes
        expiry: u64,
    ): vector<u8> {
        let payload = vector::empty<u8>();

        // Message type
        vector::push_back(&mut payload, MSG_TYPE_INTENT_REQUIREMENTS);

        // Intent ID (32 bytes)
        vector::append(&mut payload, intent_id);

        // Recipient (32 bytes)
        vector::append(&mut payload, recipient);

        // Amount (8 bytes, big-endian)
        append_u64_be(&mut payload, amount);

        // Token address (32 bytes)
        vector::append(&mut payload, token);

        // Solver address (32 bytes)
        vector::append(&mut payload, solver);

        // Expiry (8 bytes, big-endian)
        append_u64_be(&mut payload, expiry);

        payload
    }

    /// Decode IntentRequirements message
    public fun decode_intent_requirements(payload: vector<u8>): (
        vector<u8>,  // intent_id
        vector<u8>,  // recipient
        u64,         // amount
        vector<u8>,  // token
        vector<u8>,  // solver
        u64,         // expiry
    ) {
        let offset = 0;

        // Verify message type
        assert!(*vector::borrow(&payload, offset) == MSG_TYPE_INTENT_REQUIREMENTS, 1);
        offset = offset + 1;

        // Intent ID (32 bytes)
        let intent_id = slice(&payload, offset, 32);
        offset = offset + 32;

        // Recipient (32 bytes)
        let recipient = slice(&payload, offset, 32);
        offset = offset + 32;

        // Amount (8 bytes, big-endian)
        let amount = read_u64_be(&payload, offset);
        offset = offset + 8;

        // Token (32 bytes)
        let token = slice(&payload, offset, 32);
        offset = offset + 32;

        // Solver (32 bytes)
        let solver = slice(&payload, offset, 32);
        offset = offset + 32;

        // Expiry (8 bytes, big-endian)
        let expiry = read_u64_be(&payload, offset);

        (intent_id, recipient, amount, token, solver, expiry)
    }

    /// Helper: append u64 as 8 bytes big-endian
    fun append_u64_be(v: &mut vector<u8>, val: u64) {
        vector::push_back(v, ((val >> 56) & 0xff as u8));
        vector::push_back(v, ((val >> 48) & 0xff as u8));
        vector::push_back(v, ((val >> 40) & 0xff as u8));
        vector::push_back(v, ((val >> 32) & 0xff as u8));
        vector::push_back(v, ((val >> 24) & 0xff as u8));
        vector::push_back(v, ((val >> 16) & 0xff as u8));
        vector::push_back(v, ((val >> 8) & 0xff as u8));
        vector::push_back(v, ((val & 0xff) as u8));
    }

    /// Helper: read u64 from 8 bytes big-endian
    fun read_u64_be(v: &vector<u8>, offset: u64): u64 {
        let result: u64 = 0;
        let i = 0;
        while (i < 8) {
            result = (result << 8) | (*vector::borrow(v, offset + i) as u64);
            i = i + 1;
        };
        result
    }

    /// Helper: extract a slice from a vector
    fun slice(v: &vector<u8>, start: u64, len: u64): vector<u8> {
        let result = vector::empty<u8>();
        let i = 0;
        while (i < len) {
            vector::push_back(&mut result, *vector::borrow(v, start + i));
            i = i + 1;
        };
        result
    }
}
```

### 6.4 Cross-Chain Address Encoding

LZ V2 uses **bytes32** for all addresses. Different chains need different padding:

| Chain | Native Address Size | LZ bytes32 Encoding |
|-------|--------------------|--------------------|
| Aptos/Movement | 32 bytes | No padding needed (already 32 bytes) |
| EVM | 20 bytes | Left-pad with 12 zero bytes: `0x000000000000000000000000<20-byte-addr>` |
| Solana | 32 bytes | No padding needed (already 32 bytes) |

```move
/// Convert an EVM address (20 bytes) to bytes32 (32 bytes)
fun evm_to_bytes32(evm_addr: vector<u8>): vector<u8> {
    assert!(vector::length(&evm_addr) == 20, 1);
    let result = vector::empty<u8>();
    let i = 0;
    // Left-pad with 12 zero bytes
    while (i < 12) {
        vector::push_back(&mut result, 0u8);
        i = i + 1;
    };
    vector::append(&mut result, evm_addr);
    result
}
```

---

## 7. Nonce Tracking

### 7.1 How Nonces Work in LZ V2

LZ V2 tracks nonces per **pathway**: `(srcEid, sender, dstEid, receiver)`.

- Each unique (source chain, source OApp, destination chain, destination OApp) pair has its own nonce counter.
- Nonces increment monotonically starting from 1.
- The endpoint assigns nonces on send and verifies them on receive.

### 7.2 Nonce on Aptos

On Aptos, the LZ endpoint module manages nonces internally:

```text
Nonce Storage (conceptual):
messaging_channel.move stores:
  - outbound_nonce: Table<PathwayKey, u64>  -- next nonce to assign on send
  - inbound_nonce: Table<PathwayKey, u64>   -- next expected nonce on receive
  - lazy_inbound_nonce: Table<PathwayKey, u64> -- for unordered delivery
```

### 7.3 Ordered vs Unordered Delivery

LZ V2 supports two delivery modes:

| Mode | Behavior | Use Case |
|------|----------|----------|
| **Ordered** | Messages delivered in nonce order. If nonce N is pending, nonce N+1 blocks. | Default. Use when message order matters. |
| **Unordered** | Messages delivered in any order. Each nonce tracked independently. | When messages are independent. |

**For int3nts:** Ordered delivery is recommended because our flows have sequential steps (requirements -> escrow -> fulfillment). Step N must complete before step N+1.

### 7.4 Nonce in Your OApp

Your OApp does NOT manage nonces. The LZ endpoint handles it:

```move
// On send: endpoint assigns nonce automatically
// let receipt = endpoint::send(...);
// receipt.nonce contains the assigned nonce

// On receive: endpoint verifies nonce before calling your lz_receive()
// origin.nonce contains the message's nonce
// You can use it for idempotency: if already processed nonce N, skip
```

### 7.5 Idempotency with Nonces

For int3nts, combine `intent_id + nonce` or `intent_id + step_number` for idempotency:

```move
/// Track processed messages for idempotency
struct ProcessedMessages has key {
    // Key: intent_id + step_number
    processed: aptos_std::simple_map::SimpleMap<vector<u8>, bool>,
}

/// Check and mark a message as processed
fun ensure_not_processed(intent_id: vector<u8>, step: u8) acquires ProcessedMessages {
    let key = intent_id;
    vector::push_back(&mut key, step);

    let store = borrow_global_mut<ProcessedMessages>(@your_addr);
    assert!(
        !aptos_std::simple_map::contains_key(&store.processed, &key),
        1 // EALREADY_PROCESSED
    );
    aptos_std::simple_map::add(&mut store.processed, key, true);
}
```

---

## 8. Chain-Specific Quirks (Move/Aptos)

### 8.1 No Inheritance / No Interface Contracts

Move does not support inheritance. Your OApp cannot "extend" a base OApp contract. Instead:

- Import the LZ endpoint module
- Call its functions directly
- Implement the expected `lz_receive()` function signature

**Impact on int3nts:** We cannot use the LZ OApp SDK as a base class. We must implement the OApp pattern manually by calling the endpoint module's functions.

### 8.2 Resource Model

Move uses a resource model instead of contract storage:

- State is stored as **resources** under account addresses
- Resources have abilities: `key`, `store`, `copy`, `drop`
- Resources with `key` ability can be stored as top-level values
- **No dynamic dispatch** -- you cannot call arbitrary functions by address

**Impact on int3nts:**

- OApp state (peers, processed messages, intent requirements) stored as resources
- Resource account needed to act as the OApp "contract address"
- Cannot dynamically dispatch to different handler modules

### 8.3 Module Publishing

Move modules are published to an account address. Key considerations:

- **Module upgrade policy:** By default, modules can be upgraded. For production, consider making them immutable or using governance-controlled upgrades.
- **Named addresses:** Your module uses `@your_addr` which maps to the publishing account address.
- **One module per file:** Each `.move` file contains one module.
- **Friend declarations:** Modules can declare friends that can call private/friend functions.

**Impact on int3nts:**

- The GMP module must be a friend of intent modules (or vice versa) for internal function access
- Module address must be consistent across deployments (use named addresses in `Move.toml`)

### 8.4 No msg.sender Equivalent

Move does not have `msg.sender`. Access control patterns:

```move
// Pattern 1: Signer-based (for entry functions)
public entry fun my_function(caller: &signer) {
    let caller_addr = signer::address_of(caller);
    assert!(caller_addr == @allowed_address, 1);
}

// Pattern 2: For LZ receive, the endpoint calls your function
// You trust the endpoint to only call with verified messages
// But you MUST verify the peer address yourself
```

### 8.5 No Try/Catch

Move does not have try/catch. Failed transactions abort entirely.

**Impact on int3nts:** If `lz_receive()` aborts, the message delivery transaction fails. The LZ executor will retry, but if the abort is deterministic (e.g., invalid payload), the message will be stuck. Consider:

- Validating all inputs carefully
- Using a "store then process" pattern (store the raw message, process later)
- The LZ V2 "lzCompose" pattern for multi-step processing

### 8.6 Transaction Size and Gas Limits

Aptos has transaction size limits (~64KB) and gas limits. Cross-chain payloads should be kept small.

**Impact on int3nts:** Our payloads are small (< 200 bytes per message), so this is not a concern.

### 8.7 Event Model

Aptos V2 events (which Movement also supports) use the `#[event]` attribute:

```move
#[event]
struct MessageSent has store, drop {
    dst_eid: u32,
    receiver: vector<u8>,
    payload: vector<u8>,
    nonce: u64,
}
```

**Impact on int3nts:** The native GMP endpoint will emit events that the Trusted GMP relay watches. This aligns well with the Aptos event model.

### 8.8 BCS Serialization

Aptos uses BCS (Binary Canonical Serialization) as its standard serialization format. However, for cross-chain messages, we should **NOT** use BCS because:

- BCS is Aptos-specific
- EVM and Solana do not natively support BCS
- Cross-chain payloads must use a format all chains can encode/decode

**Recommendation:** Use fixed-width big-endian encoding for cross-chain payloads (as shown in Section 6.3). Use BCS only for Aptos-internal data (e.g., event parameters, on-chain storage).

---

## 9. Movement-Specific Considerations

### 9.1 Movement Architecture Overview

Movement is an Aptos-compatible L2 that:

- Uses the **Move language** (same as Aptos)
- Runs the **Aptos framework** (same Move modules: `aptos_framework`, `aptos_std`, etc.)
- Has its own **consensus layer** (based on the Movement SDK)
- Settles to **Ethereum** for security

### 9.2 Aptos Compatibility

Movement aims for high compatibility with Aptos:

| Aspect | Aptos | Movement | Compatible? |
|--------|-------|----------|-------------|
| Move language | Full support | Full support | Yes |
| Aptos framework modules | Native | Ported | Yes (with caveats) |
| Object model | Native | Supported | Yes |
| Fungible Asset standard | Native | Supported | Yes |
| Resource accounts | Native | Supported | Yes |
| Events (V2) | Native | Supported | Yes |
| Module publishing | Standard | Standard | Yes |
| BCS serialization | Standard | Standard | Yes |
| Chain ID | Aptos chain IDs | Movement chain IDs | **Different** |
| Gas model | Aptos gas schedule | Movement gas schedule | **May differ** |
| Block time | ~1-4 seconds | ~sub-second | **Different** |
| Finality | Fast finality | Dependent on L1 settlement | **Different** |

### 9.3 Known Differences and Potential Incompatibilities

#### 9.3.1 Chain ID

Movement uses its own chain IDs, not Aptos chain IDs. When building cross-chain messages, ensure you use the correct chain ID / EID for Movement (30325 for LZ EID).

#### 9.3.2 Framework Version

Movement may lag behind Aptos in framework version. Some newer Aptos framework features may not be available. Check:

- `aptos_framework::dispatchable_fungible_asset` -- may or may not be available
- `aptos_framework::function_info` -- may or may not be available
- Latest object model features

**For int3nts:** Our existing MVM code already works on Movement (we test against the Movement CLI). The GMP modules should also be compatible as long as we use standard framework features.

#### 9.3.3 Address Format

Both Aptos and Movement use 32-byte addresses. No conversion needed between them for LZ messaging.

#### 9.3.4 Gas and Fees

Movement may have different gas pricing. LZ executor options (gas limit) need to be configured appropriately for Movement's gas schedule. If Movement has lower gas costs, you may need less gas allocated in LZ options.

#### 9.3.5 RPC Endpoints

Movement has its own RPC endpoints (different from Aptos):

- **Movement Mainnet:** `https://mainnet.movementnetwork.xyz/v1`
- **Movement Testnet:** `https://aptos.testnet.suzuka.movementlabs.xyz/v1` (or similar -- verify)

### 9.4 LZ on Movement -- Current Status

**Mainnet: LIVE**

- EID `30325` confirmed for Movement mainnet
- LZ is Movement's **official interoperability provider** and powers the canonical bridge
- The Movement-LZ bridge enables transactions between Ethereum and Movement for $MOVE, $WBTC, $WETH, $USDC, $USDT
- An Immunefi attackathon has been completed for the Movement-LZ bridge contracts, confirming production readiness
- Movement Labs is the sole owner of the LZ bridge contracts on L1 and L2
- Bridge implementation uses OFT adapter pattern: `MOVEOFTAdapter.sol` (L1) + `move_oft_adapter.move` (L2)
- Source: [immunefi-team/attackathon-movement-layerzero-devtools](https://github.com/immunefi-team/attackathon-movement-layerzero-devtools)

**Testnet:**

- **Status: Uncertain**
- LZ may not have testnet support for Movement
- **Recommendation:** Use native GMP endpoints + Trusted-GMP relay for testnet

**Development:**

- Use local Movement node + native GMP endpoints + Trusted-GMP relay
- This is the same pattern described in `gmp-architecture-integration.md`

**Known concern:** LZ engineers flagged potential "stuck in transit" scenarios with dual-adapter designs (L1 adapter + L2 adapter). Rate limit parameters must be precisely aligned between both sides.

### 9.5 Deploying LZ Modules on Movement

If LZ modules are not yet deployed on Movement, there are two approaches:

**Option A: LZ deploys their modules** (preferred)

- LZ Labs deploys their endpoint, ULN, executor modules on Movement
- You deploy your OApp modules that reference LZ's deployed modules
- Same as how it works on Aptos

**Option B: Deploy LZ modules yourself** (if LZ hasn't deployed yet)

- Clone LZ V2 Aptos packages from GitHub
- Modify `Move.toml` named addresses to match Movement deployment
- Deploy the endpoint and protocol modules to Movement
- This is complex and not recommended for production

**For int3nts:** We should use Option A for mainnet (wait for / verify LZ deployment). For testnet and local development, use mock GMP endpoints.

### 9.6 Movement-Specific LZ Integration Pattern

Since Movement is Aptos-compatible, the LZ integration code is identical:

```move
// This code works on BOTH Aptos AND Movement
// The only difference is:
// 1. The endpoint address (configured at deployment)
// 2. The chain's EID (30108 for Aptos, 30325 for Movement)
// 3. The RPC endpoint for off-chain tooling

module your_addr::cross_chain_intent {
    // Same Move code for both chains
    // Endpoint address is a configuration parameter, not hardcoded

    struct Config has key {
        lz_endpoint: address,  // Different per deployment
        this_eid: u32,         // 30108 for Aptos, 30325 for Movement
    }
}
```

---

## 10. Integration Recommendations for int3nts

### 10.1 Architecture Approach

Based on this research, here is how the GMP integration should work:

```text
Production (Movement mainnet as hub):
  - LZ endpoint on Movement (EID 30325)
  - LZ endpoint on EVM chains (EID 30101, 30184, etc.)
  - LZ endpoint on Solana (EID 30168)
  - All OApps register peers (trusted remote addresses)

Local/CI:
  - Mock GMP endpoints on each chain (local Movement, local Solana, local EVM)
  - Trusted GMP relay service watches mock events and delivers messages
  - Same contract code, different endpoint addresses

Testnet:
  - Movement testnet: Mock GMP endpoint + Trusted GMP relay
    (because LZ likely not available on Movement testnet)
  - Solana devnet: Real LZ endpoint (EID 40168)
  - Base Sepolia: Real LZ endpoint (EID 40245)
```

### 10.2 MVM Module Structure

```text
intent-frameworks/mvm/sources/
├── gmp/
│   ├── gmp_messages.move        # Message encoding/decoding (wire format)
│   ├── gmp_endpoint.move        # LZ endpoint interaction (send/quote)
│   ├── native_gmp_endpoint.move  # Mock endpoint for local/CI
│   └── oapp_config.move         # Peer management, endpoint config
├── fa_intent.move               # Existing (no changes)
├── fa_intent_outflow.move       # Modified: add lz_send on create, lz_receive for proof
├── fa_intent_inflow.move        # Modified: add lz_send on create/fulfill, lz_receive
├── intent_as_escrow.move        # Modified: add lz_receive for requirements/proof
├── intent.move                  # Existing (no changes)
├── intent_reservation.move      # Existing (no changes)
└── solver_registry.move         # Existing (no changes)
```

### 10.3 Key Design Decisions

1. **Endpoint address as config:** Store the LZ endpoint address in a resource. Different for each environment.

2. **Peer verification:** Always verify the source OApp address in `lz_receive()`. Trust the endpoint for message authenticity, but verify the peer for application-level security.

3. **Payload encoding:** Use fixed-width big-endian encoding (not BCS, not ABI). This works across Move, Rust (Solana), and Solidity.

4. **Resource account for OApp identity:** Use a deterministic resource account as the OApp's "contract address" that gets registered as a peer on other chains.

5. **Friend modules for internal access:** Use Move's friend declarations so the GMP module can call internal functions on the intent/escrow modules.

6. **Idempotency:** Track processed messages by `intent_id + step_number` to handle duplicate deliveries.

### 10.4 Items Requiring Verification

Before implementation, the following must be verified against current LZ documentation and deployments:

| Item | What to Verify | Where to Check |
|------|---------------|----------------|
| **Aptos endpoint address** | Exact deployed address on Aptos mainnet | <https://docs.layerzero.network/v2/developers/evm/technical-reference/deployed-contracts> |
| **Movement endpoint deployment** | Whether LZ endpoint is deployed on Movement mainnet | <https://docs.layerzero.network> or LZ Discord |
| **Movement EID** | Confirm EID 30325 for Movement mainnet | <https://docs.layerzero.network> |
| **Movement testnet LZ** | Confirm LZ is NOT on Movement testnet | LZ Discord / docs |
| **lz_receive signature** | Exact function signature expected by LZ endpoint on Aptos | LZ V2 Aptos SDK source code on GitHub |
| **Send function API** | Exact parameters for endpoint::send() on Aptos | LZ V2 Aptos SDK source code |
| **Fee payment** | How fees are paid on Aptos (Coin<AptosCoin> vs u64) | LZ V2 Aptos SDK source code |
| **Options encoding** | Exact encoding for executor options | LZ V2 docs for Aptos |
| **OApp SDK for Move** | Whether LZ provides an OApp SDK for Move or just raw endpoint | GitHub: LZ-Labs/LZ-v2 |

### 10.5 Recommended Next Steps

1. **Clone and examine** the LZ V2 repo: `https://github.com/LZ-Labs/LZ-v2`
   - Look in `packages/layerzero-v2/aptos/` for Move source code
   - Study `endpoint.move`, `oapp_core.move`, and any OFT examples

2. **Check LZ scan** for Movement: <https://layerzeroscan.com>
   - Search for transactions on Movement (chain EID 30325)
   - Verify if messages are flowing

3. **Contact LZ team** or check their Discord for:
   - Movement testnet timeline
   - Any Movement-specific documentation
   - Known issues with Movement compatibility

4. **Build mock GMP endpoints first** (as planned in Phase 1)
   - This unblocks development immediately
   - Real LZ integration can be swapped in later by changing endpoint config

---

## Appendix A: LZ V2 Architecture Quick Reference

```text
LZ V2 Message Lifecycle:

1. Source Chain:
   OApp calls endpoint.send(dstEid, receiver, message, options)
   → Endpoint assigns nonce, creates Packet
   → Endpoint calls MessageLib (ULN302)
   → ULN302 emits PacketSent event
   → DVNs (Decentralized Verifier Networks) observe and verify

2. Cross-Chain:
   DVNs submit verification proofs to destination chain
   Executor submits the Packet for delivery

3. Destination Chain:
   Executor calls endpoint.lzReceive()
   → Endpoint verifies DVN attestations
   → Endpoint verifies nonce ordering
   → Endpoint calls OApp.lzReceive(origin, guid, message, extraData)
   → OApp processes the message
```

## Appendix B: LZ V2 Security Model

```text
Trust Assumptions:

1. DVN Security:
   - Messages are verified by configurable DVN sets
   - Default: Google Cloud + LZ Labs DVN (2-of-2)
   - Custom: Configure any DVN combination
   - OApp owner chooses their security configuration

2. Executor:
   - Permissionless execution (anyone can submit verified packets)
   - Default executor provided by LZ
   - Cannot forge messages (must be verified by DVNs first)

3. For int3nts:
   - Production: Use default DVN config (Google Cloud + LZ Labs)
   - The GMP message authenticity is guaranteed by DVN consensus
   - Our contracts verify the peer address (application-level trust)
   - No our private keys needed for message authentication
```

## Appendix C: References

- LZ V2 Documentation: <https://docs.layerzero.network/v2>
- LZ V2 GitHub: <https://github.com/LZ-Labs/LZ-v2>
- LZ Scan: <https://layerzeroscan.com>
- Aptos Move Documentation: <https://aptos.dev/en/build/smart-contracts>
- Movement Documentation: <https://docs.movementlabs.xyz>
- LZ Supported Chains: <https://docs.layerzero.network/v2/developers/evm/technical-reference/deployed-contracts>
- LZ Discord: <https://discord.gg/layerzero> (for Movement-specific questions)

---

**End of Research Document**
