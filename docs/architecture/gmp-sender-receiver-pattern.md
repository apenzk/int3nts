# GMP Sender/Receiver Architecture Pattern

This document describes the GMP (Generic Message Passing) sender/receiver split pattern used across all three VMs, mirroring LZ's design.

---

## LZ's Sender/Receiver Split

LZ V2 separates send and receive into distinct components:

### LZ on Aptos/Movement

```text
┌─────────────────────────┐     ┌─────────────────────────┐
│   oapp_core             │     │   oapp_receive          │
│   - gmp_send()          │     │   - gmp_receive()       │
│   - gmp_quote()         │     │   - routes to app       │
│   (no app imports)      │     │   (imports app)         │
└─────────────────────────┘     └─────────────────────────┘
         ↑                                │
    app imports                     calls via friend
         │                                ↓
┌─────────────────────────────────────────────────────┐
│                    Your OApp                        │
│   - imports oapp_core for sending                  │
│   - exposes gmp_receive_impl() for receiving       │
└─────────────────────────────────────────────────────┘
```

### LZ on Solana

```text
┌─────────────────────────┐     ┌─────────────────────────┐
│   LZ Endpoint Program   │     │   LZ Executor           │
│   - send() instruction  │     │   - delivers messages   │
│   (separate binary)     │     │   (calls your program)  │
└─────────────────────────┘     └─────────────────────────┘
         ↑                                │
    CPI call                         CPI call
         │                                ↓
┌─────────────────────────────────────────────────────┐
│                    Your OApp Program                │
│   - CPIs to endpoint for sending                   │
│   - exposes gmp_receive instruction for receiving  │
└─────────────────────────────────────────────────────┘
```

**Key insight:** The **sender component has no imports of application code**. Applications import the sender to send, and the receiver imports applications to route.

---

## Our Implementation

### MVM Architecture

The sender ([gmp_sender.move](../../intent-frameworks/mvm/intent-gmp/sources/gmp/gmp_sender.move)) has no app imports. The receiver ([intent_gmp.move](../../intent-frameworks/mvm/intent-connected/sources/gmp/intent_gmp.move)) imports apps to route received messages. App modules like [intent_outflow_validator.move](../../intent-frameworks/mvm/intent-connected/sources/gmp/intent_outflow_validator.move) import the sender for outbound messages and expose handler functions for the receiver.

```text
┌─────────────────────────┐     ┌─────────────────────────┐
│   gmp_sender.move       │     │  intent_gmp.move        │
│                         │     │  (receiver)             │
│   - gmp_send()          │     │  - deliver_message()    │
│   - MessageSent event   │     │  - route_message()      │
│   - nonce tracking      │     │  - remote GMP endpoints │
│                         │     │  - replay protection    │
│   NO APP IMPORTS        │     │  IMPORTS APPS           │
└─────────────────────────┘     └─────────────────────────┘
         ↑                                │
    imports                          imports
         │                                │
┌────────┴──────────────────────────────┴─────────────┐
│              outflow_validator_impl.move             │
│                                                     │
│   - imports gmp_sender::gmp_send (to send proofs)   │
│   - exposes receive_intent_requirements()           │
│     (called by intent_gmp when routing)             │
└─────────────────────────────────────────────────────┘
```

### EVM Architecture

[IntentGmp.sol](../../intent-frameworks/evm/contracts/IntentGmp.sol) is the GMP endpoint handling both send and receive. App contracts like [IntentOutflowValidator.sol](../../intent-frameworks/evm/contracts/IntentOutflowValidator.sol) and [IntentInflowEscrow.sol](../../intent-frameworks/evm/contracts/IntentInflowEscrow.sol) call `sendMessage()` for outbound messages and implement the `IMessageHandler` interface for inbound routing.

```text
┌─────────────────────────┐     ┌──────────────────────────────┐
│   IntentGmp.sol         │     │   IntentOutflowValidator     │
│   (GMP endpoint)        │     │   / IntentInflowEscrow       │
│                         │     │                              │
│   - sendMessage()       │ ←── │  - calls sendMessage()       │
│   - deliverMessage()    │ ──→ │  - implements IMessageHandler│
│   - route by msg type   │     │                              │
└─────────────────────────┘     └──────────────────────────────┘
```

Contracts interact via address at runtime — no compile-time dependency.

### SVM Architecture

The [intent-gmp](../../intent-frameworks/svm/programs/intent-gmp/) program handles send and deliver instructions. App programs like [intent-outflow-validator](../../intent-frameworks/svm/programs/intent-outflow-validator/) CPI into the endpoint to send and expose a `GmpReceive` handler for inbound messages.

```text
┌───────────────────────────────┐     ┌─────────────────────────┐
│   intent-gmp                  │     │   outflow-validator     │
│                               │     │                         │
│   - Send instruction          │ ←── │  - CPI to Send          │
│   - DeliverMsg instr          │ ──→ │  - GmpReceive handler   │
└───────────────────────────────┘     └─────────────────────────┘
```

Programs are separate binaries that invoke each other via CPI at runtime — no compile-time dependency.
