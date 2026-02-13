# GMP Sender/Receiver Architecture Pattern

**Status:** Implemented
**Date:** 2026-02-02
**Applies to:** MVM, SVM

This document explains the architectural pattern used for GMP (Generic Message Passing) in both MVM and SVM, why it was chosen, and how it mirrors LZ's design.

---

## Problem: Circular Dependencies

When implementing cross-chain messaging, application modules need to both **send** and **receive** GMP messages:

```text
Application Module (e.g., outflow_validator)
├── Needs to SEND FulfillmentProof → calls GMP endpoint
└── Needs to RECEIVE IntentRequirements ← called BY GMP endpoint
```

This creates a potential circular dependency:

```text
GMP Endpoint ──imports──> Application (for routing received messages)
Application ──imports──> GMP Endpoint (for sending messages)
     ↑                        │
     └────── CYCLE! ──────────┘
```

### Platform-Specific Manifestations

| Platform | Dependency Type | Cycle Issue? |
|----------|-----------------|--------------|
| **MVM (Move)** | Compile-time module imports | YES - Move compiler rejects cycles |
| **SVM (Solana)** | Runtime CPI (Cross-Program Invocation) | NO - Programs are separate binaries |

---

## Solution: LZ's Sender/Receiver Split

LZ V2 solves this by **separating send and receive into distinct components**:

### LZ on Aptos/Movement

```text
┌─────────────────────────┐     ┌─────────────────────────┐
│   oapp_core             │     │   oapp_receive          │
│   - gmp_send()           │     │   - gmp_receive()        │
│   - gmp_quote()          │     │   - routes to app       │
│   (no app imports)      │     │   (imports app)         │
└─────────────────────────┘     └─────────────────────────┘
         ↑                                │
    app imports                     calls via friend
         │                                ↓
┌─────────────────────────────────────────────────────────┐
│                    Your OApp                             │
│   - imports oapp_core for sending                       │
│   - exposes gmp_receive_impl() for receiving             │
└─────────────────────────────────────────────────────────┘
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
┌─────────────────────────────────────────────────────────┐
│                    Your OApp Program                     │
│   - CPIs to endpoint for sending                        │
│   - exposes gmp_receive instruction for receiving        │
└─────────────────────────────────────────────────────────┘
```

**Key insight:** The **sender component has no imports of application code**. Applications import the sender to send, and the receiver imports applications to route.

---

## Our Implementation

### MVM Architecture

```text
┌─────────────────────────┐     ┌─────────────────────────┐
│   gmp_sender.move       │     │  intent_gmp.move│
│                         │     │  (receiver)             │
│   - gmp_send()           │     │  - deliver_message()    │
│   - MessageSent event   │     │  - route_message()      │
│   - nonce tracking      │     │  - remote GMP endpoints      │
│                         │     │  - replay protection    │
│   NO APP IMPORTS        │     │  IMPORTS APPS           │
└─────────────────────────┘     └─────────────────────────┘
         ↑                                │
    imports                          imports
         │                                │
┌────────┴────────────────────────────────┴─────────────────┐
│              outflow_validator_impl.move                   │
│                                                           │
│   - imports gmp_sender::gmp_send (to send FulfillmentProof)│
│   - exposes receive_intent_requirements() (called by      │
│     intent_gmp when routing)                     │
└───────────────────────────────────────────────────────────┘
```

**Dependency graph (no cycles):**

```text
gmp_sender ← outflow_validator_impl
                      ↑
         intent_gmp (receiver)
```

### SVM Architecture

```text
┌─────────────────────────┐     ┌─────────────────────────┐
│   integrated-gmp-endpoint   │     │   outflow-validator     │
│   (Program ID: ABC)     │     │   (Program ID: XYZ)     │
│                         │     │                         │
│   - Send instruction    │ ←── │  - CPI to Send          │
│   - DeliverMsg instr    │ ──→ │  - GmpReceive handler    │
└─────────────────────────┘     └─────────────────────────┘
```

**No compile-time dependency:** Programs are separate binaries that invoke each other via CPI at runtime.

---

## Why This Pattern?

### 1. Eliminates Circular Dependencies

The sender module (`gmp_sender`) has **zero imports of application modules**. It only contains:

- Send function
- Event emission
- Nonce tracking

This makes it safe for any application module to import without creating cycles.

### 2. Matches LZ's Production Architecture

By following LZ's pattern, our code structure mirrors what we'll use in production:

- Same mental model
- Easy to swap integrated GMP with real LZ endpoint
- Consistent patterns across MVM and SVM

### 3. Single Responsibility

| Module | Responsibility |
|--------|---------------|
| `gmp_sender` | Outbound message emission only |
| `intent_gmp` | Inbound message delivery and routing only |
| Application modules | Business logic only |

### 4. Testability

Each component can be tested independently:

- Test sender without receiver
- Test receiver without sender
- Mock either for application tests

---

## Code References

### MVM

| File | Purpose |
|------|---------|
| [gmp_sender.move](../../intent-frameworks/mvm/intent-gmp/sources/gmp/gmp_sender.move) | Outbound GMP send functionality |
| [intent_gmp.move](../../intent-frameworks/mvm/intent-connected/sources/gmp/intent_gmp.move) | Inbound message delivery and routing |
| [intent_outflow_validator.move](../../intent-frameworks/mvm/intent-connected/sources/gmp/intent_outflow_validator.move) | Example app that imports gmp_sender |

### SVM

| File | Purpose |
|------|---------|
| [integrated-gmp-endpoint/](../../intent-frameworks/svm/programs/integrated-gmp-endpoint/) | GMP endpoint program (send + deliver) |
| [outflow-validator/](../../intent-frameworks/svm/programs/outflow-validator/) | Validator program (CPIs to endpoint) |

---

## Alternative Approaches Considered

### 1. Event-Based Relay (Original MVM Approach)

Instead of calling `gmp_send` directly, emit an event and let the relay handle sending.

```move
// OLD: outflow_validator emits event
event::emit(FulfillmentProofPayload { dst_chain_id, dst_addr, payload });
// Relay picks up event and calls gmp_sender::gmp_send externally
```

**Rejected because:**

- Extra indirection
- Relay must monitor additional event type
- Doesn't match LZ pattern
- Not how SVM works (SVM uses direct CPI)

### 2. Single Monolithic Endpoint

Keep all GMP functionality in one module.

**Rejected because:**

- Creates circular dependency in MVM
- Violates single responsibility
- Harder to test

### 3. Friend Functions Only

Use Move's friend mechanism to break the cycle.

**Rejected because:**

- More complex
- Doesn't apply to SVM
- LZ's split pattern is cleaner

---

## Initialization Order

When deploying, initialize in this order:

```text
1. gmp_sender::initialize()        // Sender first (no dependencies)
2. intent_gmp::initialize() // Receiver second
3. outflow_validator_impl::initialize() // Apps last
```

Tests must follow the same order - see `setup_test()` functions in test files.

---

## References

- [LZ V2 MVM OApp Architecture](../architecture/plan/lz-mvm-integration.md)
- [LZ V2 SVM OApp Architecture](../architecture/plan/lz-svm-integration.md)
- [GMP Architecture Integration](../architecture/plan/gmp-architecture-integration.md)
