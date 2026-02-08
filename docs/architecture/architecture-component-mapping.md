# Component-to-Domain Mapping Analysis

This document provides a comprehensive mapping of all source files in the Intent Framework to their respective domains. A domain is a logical grouping of related functionality that handles a specific set of responsibilities within the system. Domains organize the codebase into major functional areas with the following characteristics:

- Each domain has a clear purpose and responsibility
- Components (source files) belong to domains based on their functionality
- Domains interact with each other while maintaining clear boundaries
- This organization facilitates understanding of system interactions

This analysis forms the foundation for the architecture document.

## Topological Order (Build Sequence)

Following RPG methodology, domains are organized in topological order from foundation to dependent layers:

```mermaid
graph TB
    subgraph Foundation["Foundation Layer (No Dependencies)"]
        IM[Intent Management Domain<br/>intent.move, fa_intent.move<br/>fa_intent_with_oracle.move<br/>fa_intent_cross_chain.move<br/>intent_reservation.move]
    end
    
    subgraph Layer1["Layer 1 (Depends on Foundation)"]
        EM[Escrow Domain<br/>intent_escrow.move<br/>intent_inflow_escrow.move<br/>IntentInflowEscrow.sol]
    end
    
    subgraph Layer2["Layer 2 (Depends on Foundation + Layer 1)"]
        SM[Settlement Domain<br/>Fulfillment Functions<br/>Completion Functions<br/>Claim Functions]
        VM[Validation Domain<br/>coordinator: monitor/, api/<br/>trusted-gmp: validator/, crypto/, api/]
    end
    
    IM -->|Provides reservation &<br/>oracle-intent systems| EM
    IM -->|Provides fulfillment<br/>functions| SM
    IM -->|Emits events| VM
    EM -->|Provides completion<br/>functions| SM
    EM -->|Emits escrow events| VM
    VM -->|Validates & approves<br/>via trusted-gmp| SM
    
    style IM fill:#e1f5ff,stroke:#0066cc,stroke-width:3px,color:#333
    style EM fill:#fff4e1,stroke:#cc6600,stroke-width:2px,color:#333
    style SM fill:#e8f5e9,stroke:#006600,stroke-width:2px,color:#333
    style VM fill:#f3e5f5,stroke:#6600cc,stroke-width:2px,color:#333
    style Foundation fill:#f0f0f0,stroke:#333,stroke-width:2px,color:#333
    style Layer1 fill:#f5f5f5,stroke:#666,stroke-width:1px,color:#333
    style Layer2 fill:#fafafa,stroke:#999,stroke-width:1px,color:#333
```

**Build Order**:

1. **Foundation**: Intent Management (implement first - no dependencies)
2. **Layer 1**: Escrow (depends on Intent Management)
3. **Layer 2**: Settlement and Validation (Coordinator/Trusted GMP) (depend on Foundation + Layer 1)

## Domain Architecture Overview

This diagram shows all domain relationships and interactions, while the Topological Order diagram above focuses on build sequence and layering.

```mermaid
graph TB
    subgraph "Intent Management Domain"
        IM[intent.move<br/>fa_intent.move<br/>fa_intent_with_oracle.move<br/>fa_intent_cross_chain.move<br/>intent_reservation.move]
    end
    
    subgraph "Escrow Domain"
        EM[intent_escrow.move<br/>intent_inflow_escrow.move<br/>IntentInflowEscrow.sol]
    end
    
    subgraph "Settlement Domain"
        SM[Fulfillment Functions<br/>Completion Functions<br/>Claim Functions]
    end
    
    subgraph "Validation Domain (Coordinator + Trusted GMP)"
        VM[coordinator: monitor/, api/<br/>trusted-gmp: validator/, crypto/, api/]
    end
    
    IM -->|Creates intents<br/>Emits events| VM
    IM -->|Uses reservation| EM
    EM -->|Emits escrow events| VM
    SM -.->|Fulfillment functions<br/>in fa_intent.move| IM
    SM -.->|Completion functions<br/>in intent_escrow.move| EM
    SM -.->|Claim functions<br/>in IntentInflowEscrow.sol| EM
    VM -->|Validates & approves<br/>via trusted-gmp| SM
    VM -->|Monitors events<br/>via coordinator| IM
    VM -->|Monitors events<br/>via coordinator| EM
    
    style IM fill:#e1f5ff,color:#333
    style EM fill:#fff4e1,color:#333
    style SM fill:#e8f5e9,color:#333
    style VM fill:#f3e5f5,color:#333
```

## Domain Definitions

### 1. Intent Management Domain

**Responsibility**: Core intent creation, validation, and lifecycle management. Handles intent types, witness systems, reservation mechanisms, and event emissions.

**Key Characteristics**:

- Manages intent lifecycle (creation, expiry, revocation)
- Enforces type-safe witness validation
- Handles intent reservation for specific solvers
- Emits events for external monitoring

### 2. Escrow Domain

**Responsibility**: Asset custody and conditional release mechanisms on connected chains. Handles fund locking on individual chains, trusted-gmp integration, and escrow-specific security requirements. The cross-chain aspect comes from escrows being created on chains different from where intents are created (hub chain).

**Key Characteristics**:

- Locks assets awaiting trusted-gmp approval
- Enforces non-revocable requirement (CRITICAL security constraint)
- Supports both Move and EVM implementations
- Manages reserved solver addresses

### 3. Settlement Domain

**Responsibility**: Transaction completion and finalization processes across chains. Handles intent fulfillment, escrow release, and asset transfers.

**Note**: Unlike other domains, Settlement is not a separate module but rather represents completion/finalization functionality distributed across Intent Management and Escrow modules. This reflects the architectural pattern where settlement is the natural conclusion of intent/escrow operations.

**Key Characteristics**:

- Processes intent fulfillment by solvers
- Releases escrowed funds upon trusted-gmp approval
- Coordinates cross-chain asset transfers
- Handles expiry and cancellation scenarios

### 4. Validation Domain (Coordinator + Trusted GMP)

**Responsibility**: Two services that together handle event monitoring, cross-chain validation, and cryptographic approvals for escrow releases. The **Coordinator** handles read-only event monitoring, event caching, and negotiation routing (no private keys). The **Trusted GMP** handles cross-chain validation, approval signature generation, and crypto operations (has private keys).

**Key Characteristics**:

- Coordinator monitors events from multiple chains
- Trusted GMP validates cross-chain state consistency
- Trusted GMP generates cryptographic approval signatures
- Both services provide REST APIs for external integration

---

## Component Mapping

### Intent Management Domain

#### Core Intent Framework

- **`intent-frameworks/mvm/intent-hub/sources/intent.move`**
  - **Purpose**: Generic intent framework providing abstract structures and functions
  - **Key Structures**: `TradeIntent<Source, Args>`, `TradeSession<Args>`
  - **Key Functions**: `create_intent()`, `start_intent_session()`, `finish_intent_session()`, `revoke_intent()`
  - **Responsibilities**: Intent lifecycle, witness validation, expiry handling, revocation logic

#### Fungible Asset Intent Implementation

- **`intent-frameworks/mvm/intent-hub/sources/fa_intent.move`**
  - **Purpose**: Fungible asset trading intent implementation
  - **Key Structures**: `FALimitOrder`, `FungibleStoreManager`, `FungibleAssetRecipientWitness`
  - **Key Functions**: `create_fa_to_fa_intent()`, `fulfill_cross_chain_request_intent()`
  - **Key Events**: `LimitOrderEvent`, `LimitOrderFulfillmentEvent`
  - **Responsibilities**: FA-specific intent creation, fulfillment logic, event emission

#### Oracle-Guarded Intent Implementation

- **`intent-frameworks/mvm/intent-hub/sources/fa_intent_with_oracle.move`**
  - **Purpose**: Oracle signature requirement layer on top of base intent mechanics
  - **Key Structures**: `OracleGuardedLimitOrder`, `OracleSignatureRequirement`
  - **Key Functions**: `create_fa_to_fa_intent_with_oracle()`, `start_oracle_intent_session()`, `finish_oracle_intent_session()`
  - **Key Events**: `OracleLimitOrderEvent`
  - **Responsibilities**: Oracle signature verification, threshold validation

#### Cross-Chain Intent Creation

- **`intent-frameworks/mvm/intent-hub/sources/fa_intent_cross_chain.move`**
  - **Purpose**: Cross-chain request-intent creation (tokens locked on different chain)
  - **Key Functions**: `create_cross_chain_request_intent()`, `create_cross_chain_request_intent_entry()`
  - **Responsibilities**: Creates reserved intents with `intent_id` for cross-chain linking, zero-amount source (tokens on other chain). Uses solver registry to verify solver signatures.

#### Intent Reservation System

- **`intent-frameworks/mvm/intent-hub/sources/intent_reservation.move`**
  - **Purpose**: Reserved intent system for specific solver addresses
  - **Key Structures**: `IntentReserved`, `IntentToSign`, `IntentDraft`
  - **Key Functions**: `verify_and_create_reservation()`, `verify_and_create_reservation_from_registry()`
  - **Responsibilities**: Solver reservation, signature verification for reserved intents. Supports both authentication key extraction (old format) and registry-based lookup (new format, cross-chain).

#### Solver Registry

- **`intent-frameworks/mvm/intent-hub/sources/solver_registry.move`**
  - **Purpose**: On-chain registry for solver public keys and EVM addresses
  - **Key Structures**: `SolverRegistry`, `SolverInfo`
  - **Key Functions**: `register_solver()`, `update_solver()`, `deregister_solver()`, `get_public_key()`, `get_evm_address()`
  - **Responsibilities**: Stores solver Ed25519 public keys for signature verification and EVM addresses for cross-chain escrow creation. Required for cross-chain intents and accounts with new authentication key format.

#### Test Utilities

- **`intent-frameworks/mvm/intent-hub/sources/test_fa_helper.move`**
  - **Purpose**: Test helper utilities for intent framework testing
  - **Domain**: Testing infrastructure (not part of production domains)

---

### Escrow Domain

#### Move-Based Escrow (Hub)

- **`intent-frameworks/mvm/intent-hub/sources/intent_escrow.move`**
  - **Purpose**: Simplified escrow abstraction using oracle-intent system
  - **Key Structures**: `EscrowConfig`
  - **Key Functions**: `create_escrow()`, `start_escrow_session()`, `complete_escrow()`
  - **Security**: **CRITICAL** - Enforces non-revocable requirement (`revocable = false`)
  - **Responsibilities**: Escrow creation, session management, trusted-gmp approval handling

- **`intent-frameworks/mvm/intent-hub/sources/intent_escrow_entry.move`**
  - **Purpose**: Entry function wrappers for CLI convenience
  - **Key Functions**: `create_escrow_from_fa()`, `complete_escrow_from_fa()`
  - **Responsibilities**: User-friendly entry points for escrow operations

#### Move-Based Escrow (Connected Chain)

- **`intent-frameworks/mvm/intent-connected/sources/gmp/intent_inflow_escrow.move`**
  - **Purpose**: Inflow escrow for MVM as connected chain with GMP integration
  - **Responsibilities**: Escrow creation with GMP requirements validation, auto-release on fulfillment proof receipt

#### EVM-Based Escrow

- **`intent-frameworks/evm/contracts/IntentInflowEscrow.sol`**
  - **Purpose**: Solidity inflow escrow contract for EVM chains
  - **Key Structures**: `Escrow` struct
  - **Key Functions**: `createEscrow()`, `deposit()`, `claim()`, `cancel()`
  - **Key Events**: `EscrowInitialized`, `DepositMade`, `EscrowClaimed`, `EscrowCancelled`
  - **Security**: Enforces reserved solver addresses, expiry-based cancellation
  - **Responsibilities**: EVM escrow creation, fund locking, trusted-gmp signature verification, fund release

#### Mock Contracts (Testing)

- **`intent-frameworks/evm/contracts/MockERC20.sol`**
  - **Purpose**: Mock ERC20 token for testing
  - **Domain**: Testing infrastructure (not part of production domains)

---

### Settlement Domain

#### Intent Fulfillment (Move)

- **`intent-frameworks/mvm/intent-hub/sources/fa_intent.move`** (fulfillment functions)
  - **Key Functions**: `fulfill_cross_chain_request_intent()`, `finish_fa_intent_session()`
  - **Responsibilities**: Processes solver fulfillment, validates conditions, transfers assets

#### Escrow Completion (Move)

- **`intent-frameworks/mvm/intent-hub/sources/intent_escrow.move`** (completion functions)
  - **Key Functions**: `complete_escrow()`
  - **Responsibilities**: Verifies trusted-gmp approval, releases escrowed funds to solver

#### Escrow Claim (EVM)

- **`intent-frameworks/evm/contracts/IntentInflowEscrow.sol`** (claim function)
  - **Key Functions**: `claim()`
  - **Responsibilities**: Verifies trusted-gmp signature, transfers funds to reserved solver

#### Escrow Cancellation

- **`intent-frameworks/evm/contracts/IntentInflowEscrow.sol`** (cancel function)
  - **Key Functions**: `cancel()`
  - **Responsibilities**: Returns funds to requester after expiry

---

### Validation Domain (Coordinator + Trusted GMP)

The former monolithic signer service has been split into two services:

- **Coordinator** (`coordinator/src/`): Read-only event monitoring, event caching, negotiation routing. No private keys.
- **Trusted GMP** (`trusted-gmp/src/`): Cross-chain validation, approval signature generation, crypto operations. Has private keys.

#### Event Monitoring (Coordinator)

- **`coordinator/src/monitor/`**
  - **`mod.rs`**: Main monitor module with `EventMonitor` struct, shared types, and generic monitoring logic
  - **`inflow_mvm.rs`**: Move VM-specific escrow event polling (`poll_mvm_escrow_events()`)
  - **`inflow_evm.rs`**: EVM-specific escrow event polling (`poll_evm_escrow_events()`)
  - **`outflow_mvm.rs`**: MVM readiness monitoring (`poll_mvm_outflow_readiness()`)
  - **`outflow_evm.rs`**: EVM readiness monitoring (`poll_evm_outflow_readiness()`)
  - **`outflow_svm.rs`**: SVM readiness monitoring (`poll_svm_outflow_readiness()`)
  - **Purpose**: Monitors blockchain events from hub and connected chains (MVM, EVM, SVM)
  - **Key Structures**: `RequestIntentEvent`, `EscrowEvent`, `FulfillmentEvent`, `EventMonitor`
  - **Key Functions**: `poll_hub_events()`, `poll_connected_events()`, `poll_evm_events()`, `monitor_hub_chain()`, `monitor_connected_chain()`, `monitor_evm_chain()`, `get_cached_events()`
  - **Responsibilities**:
    - Event polling from multiple chains
    - Event caching (MVM, EVM, SVM escrows)
    - Cross-chain event correlation
    - **Readiness tracking**: Monitors IntentRequirementsReceived events on connected chains, sets `ready_on_connected_chain` flag when requirements arrive
    - Enables frontend to know when intents can proceed without polling connected chains directly

#### Cross-Chain Validation (Trusted GMP)

- **`trusted-gmp/src/validator/`**
  - **`mod.rs`**: Module declarations and re-exports of public types and functions
  - **`generic.rs`**: Shared structures (`ValidationResult`, `FulfillmentTransactionParams`) and `CrossChainValidator` struct definition and implementation
  - **`inflow_generic.rs`**: Chain-agnostic inflow validation logic (`validate_request_intent_fulfillment()`)
  - **`outflow_generic.rs`**: Chain-agnostic outflow validation logic (`validate_outflow_fulfillment()`)
  - **`inflow_mvm.rs`**: Move VM-specific inflow validation (reserved for future)
  - **`inflow_evm.rs`**: EVM-specific inflow validation (`validate_evm_escrow_solver()`)
  - **`outflow_mvm.rs`**: Move VM-specific outflow transaction parameter extraction (`extract_mvm_fulfillment_params()`)
  - **`outflow_evm.rs`**: EVM-specific outflow transaction parameter extraction (`extract_evm_fulfillment_params()`)
  - **Purpose**: Validates cross-chain state consistency and escrow safety
  - **Key Structures**: `ValidationResult`, `CrossChainValidator`, `FulfillmentTransactionParams`
  - **Key Functions**: `validate_intent_safety()`, `validate_fulfillment()`, `validate_request_intent_fulfillment()`, `validate_outflow_fulfillment()`
  - **Security**: **CRITICAL** - Validates `revocable = false` requirement
  - **Responsibilities**: Intent safety checks, fulfillment validation, approval decision logic

#### Cryptographic Operations (Trusted GMP)

- **`trusted-gmp/src/crypto/mod.rs`**
  - **Purpose**: Cryptographic operations for approval signatures
  - **Key Structures**: `ApprovalSignature`, `CryptoService`
  - **Key Functions**: `create_mvm_approval_signature(intent_id)`, `create_evm_approval_signature(intent_id)`, `verify_signature()`, `get_public_key()`
  - **Responsibilities**: Ed25519 (Move VM) and ECDSA (EVM) signature generation/verification - trusted-gmp signs the `intent_id`, signature itself is the approval

#### REST API Servers

- **`coordinator/src/api/`** (Coordinator API - negotiation endpoints, event queries)
  - **`mod.rs`**: Main API module with route definitions, shared handlers, and `ApiServer` struct
  - **Purpose**: REST API for event queries and negotiation routing
  - **Key Endpoints**: `/health`, `/events`
  - **Key Structures**: `ApiServer`, `ApiResponse<T>`
  - **Responsibilities**: HTTP request handling, event retrieval, negotiation routing

- **`trusted-gmp/src/api/`** (Trusted GMP API - validation and approval endpoints)
  - **`mod.rs`**: Main API module with route definitions, shared handlers, and `ApiServer` struct
  - **`outflow_mvm.rs`**: Move VM-specific transaction querying (`query_mvm_fulfillment_transaction()`)
  - **`outflow_evm.rs`**: EVM-specific transaction querying (`query_evm_fulfillment_transaction()`)
  - **Purpose**: REST API for validation and approval operations
  - **Key Endpoints**: `/health`, `/public-key`, `/approvals`, `/approval`, `/validate-outflow-fulfillment`, `/validate-inflow-escrow`
  - **Key Structures**: `ApiServer`, `ApiResponse<T>`
  - **Responsibilities**: HTTP request handling, approval retrieval, manual approval creation, fulfillment validation

#### Configuration Management

- **`coordinator/src/config/mod.rs`** (monitoring and routing configuration)
  - **Purpose**: Coordinator service configuration management
  - **Key Structures**: `Config`, `ChainConfig`, `EvmChainConfig`, `ApiConfig`
  - **Responsibilities**: Configuration loading, validation, chain-specific settings for monitoring

- **`trusted-gmp/src/config/mod.rs`** (key-related and validation configuration)
  - **Purpose**: Trusted GMP service configuration management
  - **Key Structures**: `Config`, `ChainConfig`, `EvmChainConfig`, `ApproverConfig`, `ApiConfig`
  - **Responsibilities**: Configuration loading, validation, chain-specific settings, key management configuration

#### Move VM Client

- **`coordinator/src/mvm_client.rs`** (event monitoring context)
  - **Purpose**: Move VM blockchain client for event querying
  - **Key Functions**: `get_events()`, `get_limit_order_events()`, `get_escrow_events()`
  - **Responsibilities**: Blockchain RPC communication, event parsing

- **`trusted-gmp/src/mvm_client.rs`** (validation context)
  - **Purpose**: Move VM blockchain client for validation queries
  - **Key Functions**: `get_intent_solver()`, `get_solver_evm_address()`, `call_view_function()`
  - **Responsibilities**: Blockchain RPC communication, solver registry queries

#### EVM Client (Trusted GMP)

- **`trusted-gmp/src/evm_client.rs`**
  - **Purpose**: EVM blockchain client for event querying via JSON-RPC
  - **Key Functions**: `get_escrow_initialized_events()`, `get_block_number()`
  - **Responsibilities**: EVM JSON-RPC communication, event log parsing, EscrowInitialized event extraction

#### Core Libraries

- **`coordinator/src/lib.rs`**
  - **Purpose**: Coordinator library root, re-exports common types
  - **Responsibilities**: Module organization, public API definition

- **`trusted-gmp/src/lib.rs`**
  - **Purpose**: Trusted GMP library root, re-exports common types
  - **Responsibilities**: Module organization, public API definition

#### Main Entry Points

- **`coordinator/src/main.rs`**
  - **Purpose**: Coordinator application entry point
  - **Responsibilities**: Service initialization, event monitoring loop orchestration

- **`trusted-gmp/src/main.rs`**
  - **Purpose**: Trusted GMP application entry point
  - **Responsibilities**: Service initialization, validation and approval loop orchestration

#### Utility Binaries (Trusted GMP)

- **`trusted-gmp/src/bin/generate_keys.rs`**
  - **Purpose**: Key pair generation utility
  - **Domain**: Development tooling

- **`trusted-gmp/src/bin/get_approver_eth_address.rs`**
  - **Purpose**: Derive Ethereum address from Ed25519 key
  - **Domain**: Development tooling

---

## Inter-Domain Interaction Patterns and Dependencies

This section documents comprehensive communication patterns between domains, including event flows, data sharing, API calls, and error handling. Dependencies follow topological order: Foundation → Layer 1 → Layer 2.

### Event Flow Patterns

**Intent Management → Validation Domain** (Event Emission):

- `LimitOrderEvent`: Emitted when intent is created (`fa_intent.move`)
  - Contains: `intent_id`, `offered_metadata`, `offered_amount`, `desired_metadata`, `desired_amount`, `expiry_time`, `revocable`
  - Purpose: Coordinator monitors for new intents; trusted-gmp validates them
- `LimitOrderFulfillmentEvent`: Emitted when intent is fulfilled (`fa_intent.move`)
  - Contains: `intent_id`, `solver`, `provided_metadata`, `provided_amount`, `timestamp`
  - Purpose: Trusted-gmp validates fulfillment before approving escrow release
- `OracleLimitOrderEvent`: Emitted for oracle-guarded intents (`fa_intent_with_oracle.move`)
  - Contains: Same as `LimitOrderEvent` plus `min_reported_value`
  - Purpose: Used by escrow system and monitored by coordinator

**Escrow → Validation Domain** (Event Emission):

- `OracleLimitOrderEvent` (Move): Emitted when escrow is created (`intent_escrow.move`)
  - Contains: Escrow details with `intent_id` for cross-chain correlation, `reserved_solver`
  - Purpose: Coordinator monitors Move VM escrow creation; trusted-gmp validates safety
  - Monitoring: Coordinator actively polls Move VM connected chain and caches escrows when created
- `EscrowInitialized` (EVM): Emitted when escrow is created (`IntentInflowEscrow.sol`)
  - Contains: `intentId`, `requester`, `token`, `reservedSolver`
  - Purpose: Coordinator monitors EVM escrow creation; trusted-gmp validates safety
  - Monitoring: Coordinator actively polls EVM connected chain and caches escrows when created (symmetrical with Move VM)
- `EscrowClaimed`, `EscrowCancelled` (EVM): Emitted on escrow completion/cancellation
  - Purpose: Coordinator tracks escrow lifecycle

**Trusted GMP → Settlement** (Approval Provision):

- Approval signatures provided via trusted-gmp REST API (`/approvals/:escrow_id`) or direct function calls
- Contains: Cryptographic signature (Ed25519 for Move, ECDSA for EVM) - signature itself is the approval
- Purpose: Settlement uses signatures to authorize escrow release (trusted-gmp signs the `intent_id`)

### Functional Dependencies

**Escrow → Intent Management** (Layer 1 → Foundation):

- **Reservation System**: Escrow uses `IntentReserved` structure from `intent_reservation.move` to enforce reserved solver addresses
- **Oracle-Intent System**: Hub escrow implementation uses `fa_intent_with_oracle.move` for oracle-guarded intent mechanics
- **Function Calls**: `create_escrow()` (hub) internally uses `create_fa_to_fa_intent_with_oracle()` from Intent Management

**Settlement → Intent Management** (Layer 2 → Foundation):

- **Fulfillment Functions**: Settlement calls `fulfill_cross_chain_request_intent()` and `finish_fa_intent_session()` from `fa_intent.move`
- **Witness Validation**: Settlement uses witness type system from `intent.move` to validate fulfillment conditions
- **Session Management**: Settlement consumes `TradeSession` hot potato types from Intent Management

**Settlement → Escrow** (Layer 2 → Layer 1):

- **Completion Functions**: Settlement calls `complete_escrow()` (Move) or `claim()` (EVM) to release escrowed funds
- **Approval Verification**: Settlement verifies trusted-gmp signatures before releasing funds
- **Reserved Solver Enforcement**: Settlement ensures funds go to reserved solver regardless of transaction sender

**Validation Domain → Intent Management** (Layer 2 → Foundation):

- **Event Monitoring**: Coordinator polls `LimitOrderEvent` and `LimitOrderFulfillmentEvent` via blockchain RPC
- **Readiness Tracking**: Coordinator monitors IntentRequirementsReceived events on connected chains, sets `ready_on_connected_chain` flag for outflow intents
- **Safety Validation**: Trusted-gmp calls `validate_intent_safety()` to check intent requirements (expiry, revocability)
- **Fulfillment Validation**: Trusted-gmp calls `validate_fulfillment()` to verify fulfillment conditions match intent

**Validation Domain → Escrow** (Layer 2 → Layer 1):

- **Event Monitoring**: Coordinator polls `OracleLimitOrderEvent` (Move) and `EscrowInitialized` (EVM) actively
- **Symmetrical Monitoring**: Both Move VM and EVM escrows are monitored, cached, and validated when created (not retroactively)
- **Safety Validation**: Trusted-gmp calls `validate_intent_fulfillment()` to ensure `revocable = false` (CRITICAL) and validates solver addresses match
- **Chain Type Detection**: Each `EscrowEvent` includes a `chain_type` field (Mvm, Evm, Svm) set by the coordinator based on which monitor discovered the event. This is trusted because it comes from the coordinator's configuration, not from untrusted event data.
- **Solver Validation**: For Move VM escrows, compares Move VM addresses directly; for EVM escrows, queries solver registry for EVM address and compares. Chain type is determined from `EscrowEvent.chain_type` enum field.
- **Chain ID Validation**: Trusted-gmp validates that escrow `chain_id` matches intent `offered_chain_id`
- **Approval Generation**: Trusted-gmp calls `create_mvm_approval_signature(intent_id)` (Ed25519) or `create_evm_approval_signature(intent_id)` (ECDSA) to generate cryptographic signatures for escrow release. The signature itself is the approval - trusted-gmp signs the `intent_id`.

### Data Flow Patterns

**Cross-Chain Correlation**:

- `intent_id` field links intents on hub chain to escrows on connected chains
- Coordinator uses `intent_id` to match events across chains via `match_events_by_intent_id()`; trusted-gmp uses correlated data for validation
- Data flows: Hub Intent → `intent_id` → Connected Escrow → Coordinator Correlation → Trusted GMP Validation → Approval

**Reserved Solver Flow**:

- Intent Management: Provides `IntentReserved` structure with solver address
- Escrow: Stores `reserved_solver` / `reservedSolver` at creation (immutable)
- Settlement: Transfers funds to reserved solver regardless of transaction sender
- Trusted GMP: Validates reserved solver matches intent fulfillment

**Approval Signature Flow**:

- Trusted GMP: Generates approval signature using Ed25519 (Move) or ECDSA (EVM)
- Settlement: Retrieves signature via trusted-gmp REST API (`/approvals/:escrow_id`) or cached events
- Escrow: Verifies signature matches approver public key before releasing funds (on-chain parameter name `approver_public_key`)

### API Call Patterns

**External Systems → Coordinator**:

- `GET /events`: Retrieve cached events (intents, escrows, fulfillments)

**External Systems → Trusted GMP**:

- `GET /approvals/:escrow_id`: Retrieve approval signature for specific escrow
- `POST /approval`: Manually trigger approval generation (for testing/debugging)

**Settlement → Trusted GMP**:

- Settlement queries trusted-gmp `/approvals/:escrow_id` to retrieve approval signatures
- Settlement validates signature format and approver public key before use (on-chain parameter name kept as-is)

### Error Handling and Rollback Scenarios

**Intent Expiry**:

- Intent Management: Rejects fulfillment attempts after `expiry_time`
- Settlement: Cannot fulfill expired intents
- Escrow: Can be cancelled after expiry (EVM only), returns funds to requester

**Invalid Trusted GMP Signature**:

- Escrow: Rejects `complete_escrow()` / `claim()` calls with invalid signatures
- Settlement: Must retry with valid signature or wait for trusted-gmp approval
- Trusted GMP: Signature validation failures logged but don't prevent retry

**Escrow Safety Validation Failure**:

- Trusted GMP: Rejects escrows with `revocable = true` (CRITICAL security check)
- Escrow: Creation fails if trusted-gmp validation rejects (pre-creation validation)
- Settlement: Cannot proceed if trusted-gmp hasn't approved

**Cross-Chain Correlation Failure**:

- Coordinator/Trusted GMP: Cannot match events if `intent_id` mismatch or missing
- Settlement: Cannot proceed without trusted-gmp approval (requires correlation)
- Error: Escrow remains locked until manual intervention or expiry

**Reserved Solver Mismatch**:

- Escrow: Rejects completion if reserved solver doesn't match (Move: session validation, EVM: enforced in `claim()`)
- Settlement: Funds always go to reserved solver, transaction sender irrelevant
- Trusted GMP: Validates reserved solver matches fulfillment before approval

---

## Domain Boundaries and Interfaces

Detailed architectural definitions of domain boundaries, external interfaces, internal components, data ownership, and interaction protocols are documented in [`domain-boundaries-and-interfaces.md`](domain-boundaries-and-interfaces.md). This document follows RPG methodology principles and serves as architectural guidance for development decisions.

---

## Domain Boundaries Summary

This table provides a concise overview of domain boundaries, listing the primary source files for each domain and their core responsibilities. It serves as a quick reference for understanding which components belong to which domain and what each domain's primary function is within the Intent Framework system.

| Domain | Primary Files | Key Responsibility |
|--------|--------------|-------------------|
| **Intent Management** | `intent.move`, `fa_intent.move`, `fa_intent_with_oracle.move`, `fa_intent_cross_chain.move`, `intent_reservation.move` | Intent lifecycle, creation, validation, event emission |
| **Escrow** | `intent_escrow.move`, `intent_escrow_entry.move`, `intent_inflow_escrow.move`, `IntentInflowEscrow.sol` | Asset custody, fund locking, trusted-gmp integration |
| **Settlement** | Functions in `fa_intent.move`, `intent_escrow.move`, `IntentInflowEscrow.sol` | Intent fulfillment, escrow completion, asset transfers |
| **Validation (Coordinator + Trusted GMP)** | Coordinator: `monitor/`, `api/`, `config/`, `mvm_client.rs`, `svm_client.rs`, `storage/`; Trusted GMP: `validator/`, `crypto/`, `api/`, `config/`, `mvm_client.rs`, `evm_client.rs` | Coordinator: event monitoring (hub, MVM, EVM, SVM), readiness tracking (IntentRequirementsReceived), caching, negotiation routing; Trusted GMP: cross-chain validation, approval signatures (Ed25519 & ECDSA) |
