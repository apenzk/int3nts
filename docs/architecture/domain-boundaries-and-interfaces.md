# Domain Boundaries and Interfaces

This document provides precise definitions of domain boundaries, external interfaces, internal components, data ownership, and interaction protocols following RPG methodology principles.

## Intent Management: Boundaries and Interfaces

### Intent Management: Domain Boundaries

**In Scope**:

- Intent creation, lifecycle management, and validation
- Witness type system and verification
- Intent reservation mechanisms
- Event emission for external monitoring
- Cross-chain intent creation (zero-amount source intents)

**Out of Scope**:

- Asset custody (belongs to Escrow Domain)
- Integrated GMP approval logic (belongs to Integrated GMP Domain)
- Escrow-specific operations (belongs to Escrow Domain)

### External Interfaces

**Public Entry Functions** (Move):

- `create_fa_to_fa_intent_entry()` - Create fungible asset intent
- `create_cross_chain_request_intent_entry()` - Create cross-chain request-intent
- `fulfill_cross_chain_request_intent()` - Fulfill cross-chain intent
- `create_reserved_intent()` - Create reserved intent with solver signature

**Public Functions** (Move):

- `create_intent<Source, Args>()` - Generic intent creation
- `start_intent_session<Source, Args>()` - Start intent session
- `finish_intent_session<Witness, Args>()` - Complete intent session
- `revoke_intent()` - Revoke intent (if revocable)

**Events Emitted**:

- `LimitOrderEvent` - Intent creation event (fa_intent.move)
- `LimitOrderFulfillmentEvent` - Intent fulfillment event (fa_intent.move)
- `OracleLimitOrderEvent` - Oracle-guarded intent event (fa_intent_with_oracle.move)

**Data Structures Exported**:

- `TradeIntent<Source, Args>` - Core intent structure
- `TradeSession<Args>` - Active trading session
- `FALimitOrder` - FA trading conditions
- `OracleGuardedLimitOrder` - Oracle-guarded trading conditions
- `IntentReserved` - Solver reservation structure

### Intent Management: Internal Components

- Witness type system (`FungibleAssetRecipientWitness`, etc.)
- Intent expiry validation logic
- Reservation signature verification
- Event emission infrastructure

### Intent Management: Data Ownership

- **Intent Objects**: Owned by intent creator until fulfilled or revoked
- **Intent State**: Stored in Move object system, managed by Intent Management domain
- **Session State**: Hot potato types, must be consumed by completion

### Intent Management: Interaction Protocols

For comprehensive inter-domain interaction patterns, see [Inter-Domain Interaction Patterns and Dependencies](architecture-component-mapping.md#inter-domain-interaction-patterns-and-dependencies) in the architecture component mapping document.

---

## Escrow: Boundaries and Interfaces

### Escrow: Domain Boundaries

**In Scope**:

- Asset custody and fund locking on individual chains
- Escrow creation with GMP requirements validation
- Auto-release on FulfillmentProof receipt via GMP
- Reserved solver address enforcement
- Non-revocable requirement enforcement

**Out of Scope**:

- Intent creation logic (belongs to Intent Management Domain)
- GMP message relay (belongs to Integrated GMP Domain)
- Cross-chain intent creation (belongs to Intent Management Domain)

### Escrow: External Interfaces

**Public Entry Functions** (Move):

- `create_escrow_from_fa()` - Create escrow from fungible asset
- `complete_escrow_from_fa()` - Complete escrow

**Public Functions** (Move):

- `create_escrow()` - Create escrow with GMP requirements
- `start_escrow_session()` - Start escrow session (solver takes escrowed assets)
- `complete_escrow()` - Complete escrow

**Public Functions** (Solidity):

- `createEscrow(uint256 intentId, address token, uint256 amount, address reservedSolver)` - Create and deposit escrow
- `deposit(uint256 intentId, address token, uint256 amount)` - Additional deposit to escrow
- `claim(uint256 intentId, bytes signature)` - Claim escrow
- `cancel(uint256 intentId)` - Cancel escrow after expiry

**Events Emitted**:

- `OracleLimitOrderEvent` - Escrow creation event (Move)
- `EscrowInitialized` - Escrow creation event (EVM)
- `DepositMade` - Additional deposit event (EVM)
- `EscrowClaimed` - Escrow claim event (EVM)
- `EscrowCancelled` - Escrow cancellation event (EVM)

**Data Structures Exported**:

- `EscrowConfig` - Escrow configuration (Move)
- `Escrow` struct - Escrow data structure (EVM)

### Escrow: Internal Components

- Non-revocable enforcement logic (`revocable = false` requirement)
- Reserved solver address validation
- GMP message verification (IntentRequirements validation, FulfillmentProof auto-release)
- Expiry-based cancellation logic

### Escrow: Data Ownership

- **Escrowed Assets**: Locked in escrow contract/module until released or cancelled
- **Escrow State**: Owned by escrow contract, managed by Escrow domain
- **Reserved Solver**: Enforced at creation, cannot be changed

### Escrow: Interaction Protocols

For comprehensive inter-domain interaction patterns, see [Inter-Domain Interaction Patterns and Dependencies](architecture-component-mapping.md#inter-domain-interaction-patterns-and-dependencies) in the architecture component mapping document.

---

## Settlement: Boundaries and Interfaces

### Settlement: Domain Boundaries

**In Scope**:

- Intent fulfillment operations
- Escrow completion and claim operations
- Asset transfer coordination
- Expiry and cancellation handling

**Out of Scope**:

- Intent creation (belongs to Intent Management Domain)
- Escrow creation (belongs to Escrow Domain)
- GMP message relay (belongs to Integrated GMP Domain)

**Note**: Settlement functionality is distributed across Intent Management and Escrow modules, not a separate structural module.

### Settlement: External Interfaces

**Public Entry Functions** (Move):

- `fulfill_cross_chain_request_intent()` - Fulfill cross-chain intent (in fa_intent.move)
- `complete_escrow_from_fa()` - Complete escrow (in intent_escrow_entry.move)

**Public Functions** (Move):

- `finish_fa_intent_session()` - Complete FA intent session (in fa_intent.move)
- `complete_escrow()` - Complete escrow (in intent_escrow.move)

**Public Functions** (Solidity):

- `claim(uint256 intentId, bytes signature)` - Claim escrow (in IntentInflowEscrow.sol)
- `cancel(uint256 intentId)` - Cancel escrow after expiry (in IntentInflowEscrow.sol)

### Settlement: Internal Components

- Fulfillment validation logic (witness verification, condition checking)
- GMP FulfillmentProof validation
- Asset transfer execution
- Expiry validation

### Settlement: Data Ownership

- **Fulfilled Assets**: Transferred from intent creator to solver
- **Escrowed Assets**: Transferred from escrow to reserved solver
- **Session State**: Consumed during completion (hot potato pattern)

### Settlement: Interaction Protocols

For comprehensive inter-domain interaction patterns, see [Inter-Domain Interaction Patterns and Dependencies](architecture-component-mapping.md#inter-domain-interaction-patterns-and-dependencies) in the architecture component mapping document.

---

## Coordinator: Boundaries and Interfaces

### Coordinator: Domain Boundaries

**In Scope**:

- Read-only event monitoring from hub and connected chains (Move VM and EVM)
- Symmetrical monitoring of Move VM and EVM escrows (both cached when created)
- Event caching and retrieval
- Event correlation and matching
- Negotiation routing
- REST API for event queries and negotiation

**Out of Scope**:

- Intent creation (belongs to Intent Management Domain)
- Escrow creation (belongs to Escrow Domain)
- Asset custody (belongs to Escrow Domain)
- Cross-chain validation (belongs to Integrated GMP Domain)
- GMP message relay (belongs to Integrated GMP Domain)

### Coordinator: External Interfaces

**REST API Endpoints**:

- `GET /health` - Health check
- `GET /events` - Get cached events (intents, escrows, fulfillments)
- Negotiation routing endpoints (solver discovery and matching)

**Public Functions** (Rust):

- `EventMonitor::poll_hub_events()` - Poll hub chain for intent events
- `EventMonitor::poll_connected_events()` - Poll Move VM connected chain for escrow events
- `EventMonitor::poll_evm_events()` - Poll EVM connected chain for escrow events
- `EventMonitor::monitor_hub_chain()` - Monitor hub chain continuously
- `EventMonitor::monitor_connected_chain()` - Monitor Move VM connected chain continuously
- `EventMonitor::monitor_evm_chain()` - Monitor EVM connected chain continuously
- `EventMonitor::get_cached_events()` - Get cached events

**Data Structures Exported**:

- `RequestIntentEvent` - Normalized request-intent event structure
- `EscrowEvent` - Normalized escrow event structure with `chain_type` field (Mvm, Evm, Svm) set by coordinator based on monitor that discovered it
- `FulfillmentEvent` - Normalized fulfillment event structure

### Coordinator: Internal Components

- Event polling and caching mechanisms (symmetrical for Move VM and EVM)
- Event correlation logic (`intent_id` matching)
- Negotiation routing logic
- Configuration management
- Blockchain RPC clients (MvmClient for Move VM chains)

### Coordinator: Data Ownership

- **Event Cache**: Owned by Coordinator domain, populated from blockchain events
- **Configuration**: Owned by Coordinator domain, loaded from config files

### Coordinator: Interaction Protocols

For comprehensive inter-domain interaction patterns, see [Inter-Domain Interaction Patterns and Dependencies](architecture-component-mapping.md#inter-domain-interaction-patterns-and-dependencies) in the architecture component mapping document.

---

## Integrated GMP: Boundaries and Interfaces

### Integrated GMP: Domain Boundaries

**In Scope**:

- GMP message relay (watches `MessageSent` events on source chains, delivers messages to destination chains via `deliver_message`)
- Operator wallet management (Ed25519 keys for MVM, derived ECDSA for EVM, derived Solana keypair for SVM)
- Relay authorization verification at startup

**Out of Scope**:

- Intent creation (belongs to Intent Management Domain)
- Escrow creation (belongs to Escrow Domain)
- Asset custody (belongs to Escrow Domain)
- Event monitoring and caching (belongs to Coordinator Domain)
- Negotiation routing (belongs to Coordinator Domain)
- Validation logic (now on-chain in GMP endpoint contracts/modules)
- Approval signatures (GMP messages -- IntentRequirements and FulfillmentProof -- handle cross-chain authorization on-chain)

### Integrated GMP: External Interfaces

**REST API Endpoints**:

- `GET /health` - Health check

The relay has no other client-facing API. It is a pure relay -- invisible to clients. The coordinator is the single API surface for frontends and solvers.

**Public Functions** (Rust):

- `NativeGmpRelay::new(config, crypto_service)` - Create a new relay instance
- `NativeGmpRelay::run()` - Run the relay (polls all configured chains, delivers messages)
- `NativeGmpRelay::poll_mvm_events()` - Poll MVM hub chain for `MessageSent` events
- `NativeGmpRelay::poll_mvm_connected_events()` - Poll MVM connected chain for `MessageSent` events
- `NativeGmpRelay::poll_svm_events()` - Poll SVM chain for `MessageSent` events
- `NativeGmpRelay::poll_evm_events()` - Poll EVM chain for `MessageSent` events
- `NativeGmpRelay::deliver_message(message)` - Route and deliver a GMP message to the appropriate destination chain
- `NativeGmpRelay::deliver_to_mvm_hub(message)` - Deliver message to MVM hub chain
- `NativeGmpRelay::deliver_to_mvm_connected(message)` - Deliver message to MVM connected chain
- `NativeGmpRelay::deliver_to_evm(message)` - Deliver message to EVM chain
- `NativeGmpRelay::deliver_to_svm(message)` - Deliver message to SVM chain

**Data Structures Exported**:

- `NativeGmpRelayConfig` - Relay configuration (chain RPC URLs, module addresses, chain IDs, operator key, polling interval)
- `GmpMessage` - GMP message structure (src_chain_id, remote_gmp_endpoint_addr, dst_chain_id, dst_addr, payload, nonce)

### Integrated GMP: Internal Components

- `MessageSent` event parsing (MVM view-function-based outbox polling, SVM transaction log parsing, EVM log filtering)
- `deliver_message` transaction building (MVM CLI-based submission, EVM `eth_sendTransaction`, SVM instruction building)
- Nonce tracking and replay protection (per-chain processed nonce sets, outbox cursor tracking)
- Relay authorization checking at startup (verifies operator is authorized on all destination chains)
- Configuration management (`NativeGmpRelayConfig` from TOML config)
- Blockchain RPC clients (MvmClient for Move VM chains, SvmClient for SVM chains, HTTP/JSON-RPC for EVM chains)

### Integrated GMP: Data Ownership

- **Message delivery state**: Processed nonce sets per source chain, outbox cursors for MVM and SVM, last polled EVM block number
- **Nonce tracking**: Per-chain nonce tracking for replay protection
- **Configuration**: Owned by Integrated GMP domain, loaded from config files

### Integrated GMP: Interaction Protocols

For comprehensive inter-domain interaction patterns, see [Inter-Domain Interaction Patterns and Dependencies](architecture-component-mapping.md#inter-domain-interaction-patterns-and-dependencies) in the architecture component mapping document.
