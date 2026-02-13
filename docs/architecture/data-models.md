# Data Models Documentation

This document provides architectural guidance on how data structures relate across chains and domains in the Intent Framework.

For detailed field-by-field documentation, see:

- [Move Intent Framework API Reference](../../docs/intent-frameworks/mvm/api-reference.md#type-definitions) - TradeIntent, TradeSession, FALimitOrder
- [Move event structures](../../intent-frameworks/mvm/intent-hub/sources/fa_intent.move) - LimitOrderEvent, LimitOrderFulfillmentEvent
- [EVM Escrow documentation](../../docs/intent-frameworks/evm/README.md)
- [SVM Escrow documentation](../../docs/intent-frameworks/svm/README.md)
- [Rust coordinator structures](../../coordinator/src/monitor/mod.rs) - IntentEvent, EscrowEvent, FulfillmentEvent
- [GMP message definitions (Move)](../../intent-frameworks/mvm/intent-gmp/sources/gmp_common/messages.move) - IntentRequirements, EscrowConfirmation, FulfillmentProof
- [GMP message definitions (Solidity)](../../intent-frameworks/evm/contracts/gmp-common/Messages.sol) - IntentRequirements, EscrowConfirmation, FulfillmentProof
- [GMP message definitions (Rust/SVM)](../../intent-frameworks/svm/programs/gmp-common/src/messages.rs) - IntentRequirements, EscrowConfirmation, FulfillmentProof

## Overview

The Intent Framework uses data structures across three implementation languages (Move, Solidity, Rust) that work together to enable cross-chain escrow operations. This document focuses on:

- **Cross-chain data linking patterns** - How structures link across chains using `intent_id`
- **GMP message types** - Cross-chain message structures for on-chain validation
- **Event correlation mechanisms** - How coordinator normalizes and matches events
- **Domain relationships** - How data structures map to architectural domains
- **State transition patterns** - How data flows between chains

**Key Data Flow Patterns**:

- **Hub Chain (Move)**: Intent creation and fulfillment with event emissions; sends/receives GMP messages
- **Connected Chains (Move/EVM/SVM)**: Escrow creation and release; validates GMP-delivered requirements on-chain
- **Coordinator Service (Rust)**: Event monitoring, normalization, and caching (read-only, no keys)
- **Integrated GMP Relay (Rust)**: Watches `MessageSent` events, delivers messages to destination chains

## GMP Message Types

Three message types enable cross-chain communication. All implementations (Move, Solidity, Rust) produce bitwise identical encodings, verified by shared test vectors in `intent-frameworks/common/testing/gmp-encoding-test-vectors.json`.

### Wire Format

All messages use fixed-width encoding with big-endian integers, 32-byte addresses, and a 1-byte message type discriminator as the first byte.

| Message Type | Byte | Direction | Size | Purpose |
|-------------|------|-----------|------|---------|
| IntentRequirements | `0x01` | Hub → Connected | 145 bytes | Delivers requirements for escrow/fulfillment validation |
| EscrowConfirmation | `0x02` | Connected → Hub | 137 bytes | Confirms escrow was created matching requirements |
| FulfillmentProof | `0x03` | Bidirectional | 81 bytes | Proves solver fulfilled; triggers token release |

### IntentRequirements (0x01)

Sent from hub chain when an intent is created. The connected chain stores these requirements and validates escrow creation (inflow) or solver fulfillment (outflow) against them.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | type | `0x01` |
| 1 | 32 | intent_id | Cross-chain intent identifier |
| 33 | 32 | requester_addr | Requester's address on the connected chain |
| 65 | 8 | amount_required | Required escrow/fulfillment amount (big-endian) |
| 73 | 32 | token_addr | Token address on the connected chain |
| 105 | 32 | solver_addr | Authorized solver's address on the connected chain |
| 137 | 8 | expiry | Expiry timestamp (big-endian) |

### EscrowConfirmation (0x02)

Sent from connected chain after a validated escrow is created. The hub gates solver fulfillment on this confirmation (inflow only).

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | type | `0x02` |
| 1 | 32 | intent_id | Cross-chain intent identifier |
| 33 | 32 | escrow_id | Escrow identifier on the connected chain |
| 65 | 8 | amount_escrowed | Actual amount locked in escrow (big-endian) |
| 73 | 32 | token_addr | Token address locked in escrow |
| 105 | 32 | creator_addr | Address that created the escrow |

### FulfillmentProof (0x03)

Sent in either direction after solver fulfills. Triggers automatic token release on the receiving chain.

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 1 | type | `0x03` |
| 1 | 32 | intent_id | Cross-chain intent identifier |
| 33 | 32 | solver_addr | Solver that fulfilled |
| 65 | 8 | amount_fulfilled | Amount fulfilled (big-endian) |
| 73 | 8 | timestamp | Fulfillment timestamp (big-endian) |

### GMP State Tracking (Hub)

The hub chain tracks GMP message delivery state per intent via `IntentGmpState` (`intent-frameworks/mvm/intent-gmp/sources/gmp/gmp_intent_state.move`):

| Field | Type | Description |
|-------|------|-------------|
| intent_id | `vector<u8>` | Cross-chain intent identifier |
| intent_addr | `address` | On-chain intent object address |
| dst_chain_id | `u32` | Destination chain ID |
| flow_type | `u8` | Inflow or outflow |
| escrow_confirmed | `bool` | Set when EscrowConfirmation received |
| fulfillment_proof_received | `bool` | Set when FulfillmentProof received |
| solver_addr_connected_chain | `vector<u8>` | Solver's address on connected chain |

## Coordinator Domain: Normalized Event Structures

The coordinator normalizes blockchain events from different chains into common Rust structures for event caching and correlation. These structures enable unified processing of events from Move, EVM, and SVM chains.

**Key Normalization Patterns**:

- **RequestIntentEvent** (`coordinator/src/monitor/mod.rs`) - Normalizes `LimitOrderEvent` from Move hub chain
- **EscrowEvent** (`coordinator/src/monitor/mod.rs`) - Normalizes `OracleLimitOrderEvent` (Move) and `EscrowInitialized` (EVM) from connected chains
- **FulfillmentEvent** (`coordinator/src/monitor/mod.rs`) - Normalizes `LimitOrderFulfillmentEvent` from hub chain
- **ChainType** (`coordinator/src/monitor/mod.rs`) - Enum representing blockchain type (`Mvm`, `Evm`, `Svm`) for escrow events

**Normalization Purpose**: These structures abstract away chain-specific differences (Move address types vs EVM address types, BCS vs ABI encoding) to enable unified event caching and correlation. See [`coordinator/src/monitor/mod.rs`](../../coordinator/src/monitor/mod.rs) for complete field definitions.

## Cross-Chain Data Linking

Data structures link across chains using `intent_id` fields, GMP messages, and event correlation patterns.

### Intent ID Pattern

The `intent_id` field serves as the primary cross-chain linking mechanism:

- **Hub Chain Intents**: `intent_id` is set to `intent_address` for regular intents, or a shared address for cross-chain request-intents
- **Connected Chain Escrows**: `intent_id` is passed during escrow creation to link back to the hub intent
- **GMP Messages**: All three message types carry `intent_id` to correlate across chains
- **Event Correlation**: Coordinator matches events across chains using `intent_id` field

**References**:

- `FALimitOrder.intent_id: Option<address>` - Optional cross-chain linking field
- `LimitOrderEvent.intent_id: address` - Event correlation field
- `EscrowEvent.intent_id: String` - Coordinator event matching field
- `EscrowEvent.chain_id: u64` - Chain ID where escrow is located (set by coordinator from config, trusted)
- `EscrowEvent.chain_type: ChainType` - Blockchain type (`Mvm`, `Evm`, `Svm`) - set by coordinator based on which monitor discovered the event (trusted)

### Reserved Solver Addressing

Solver addresses are preserved across chains:

- **Hub Chain**: `TradeIntent.reservation: Option<IntentReserved>` - Optional reserved solver
- **Connected Chain**: `Escrow.reservedSolver: address` - Always set, never address(0)
- **GMP Delivery**: Hub sends solver's connected-chain address via IntentRequirements; escrow/validation contracts validate on-chain

### Event Correlation Logic

The coordinator correlates events across chains using:

1. **Intent Creation**: `IntentEvent` from hub chain with `intent_id`
2. **Escrow Creation**: `EscrowEvent` from connected chain with matching `intent_id`
3. **Fulfillment**: `FulfillmentEvent` from hub chain with matching `intent_id`
4. **Readiness Tracking**: `IntentRequirementsReceived` event on connected chain sets `ready_on_connected_chain` flag

## Serialization Formats

Data structures are serialized differently depending on the chain and communication protocol:

- **Move Contracts**: BCS (Binary Canonical Serialization) for on-chain storage
- **EVM Contracts**: ABI encoding for Solidity structs
- **SVM Programs**: Borsh serialization for on-chain account data
- **GMP Messages**: Fixed-width binary encoding (big-endian integers, 32-byte addresses) - identical across all chains
- **Coordinator Service**: JSON for REST API communication
- **Cross-Chain Events**: JSON serialization for event data passed between chains

## State Transitions

Key data structure state transitions:

1. **Intent Creation**: `TradeIntent` created → `LimitOrderEvent` emitted → IntentRequirements sent via GMP to connected chain
2. **Requirements Delivery**: Connected chain receives IntentRequirements → stores requirements → emits `IntentRequirementsReceived`
3. **Escrow Creation** (inflow): `Escrow` created → validates against stored IntentRequirements → EscrowConfirmation sent via GMP to hub
4. **Escrow Confirmation**: Hub receives EscrowConfirmation → sets `escrow_confirmed = true` → gates solver fulfillment
5. **Intent Fulfillment**: `TradeIntent` consumed → `LimitOrderFulfillmentEvent` emitted → FulfillmentProof sent via GMP to connected chain
6. **Escrow Auto-Release** (inflow): Connected chain receives FulfillmentProof → escrow funds transferred to solver automatically
7. **Solver Fulfillment** (outflow): Solver calls validation contract on connected chain → validates against IntentRequirements → FulfillmentProof sent via GMP to hub
8. **Hub Release** (outflow): Hub receives FulfillmentProof → sets `fulfillment_proof_received = true` → solver claims locked tokens
