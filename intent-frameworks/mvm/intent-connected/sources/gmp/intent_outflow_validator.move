/// Outflow Validator (MVM as Connected Chain)
///
/// Validates and executes outflow intent fulfillments when MVM acts as a connected chain.
/// Receives intent requirements from the hub via GMP, validates solver fulfillments,
/// and sends fulfillment proofs back to the hub.
///
/// ## Flow
///
/// 1. Hub creates intent → sends IntentRequirements via GMP
/// 2. This module receives requirements via `receive_intent_requirements` (stores in table)
/// 3. Authorized solver calls `fulfill_intent`:
///    - Tokens pulled from solver to recipient
///    - FulfillmentProof sent back to hub via GMP
/// 4. Hub receives proof → releases escrowed funds to solver
module mvmt_intent::intent_outflow_validator_impl {
    use std::signer;
    use std::vector;
    use aptos_framework::event;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use mvmt_intent::gmp_common;
    use mvmt_intent::gmp_sender;

    // ============================================================================
    // ERROR CODES
    // ============================================================================

    /// Caller is not the admin
    const E_UNAUTHORIZED_ADMIN: u64 = 1;
    /// Invalid source chain (not the hub GMP endpoint)
    const E_INVALID_SOURCE_CHAIN: u64 = 2;
    /// Invalid source address (not the hub GMP endpoint)
    const E_INVALID_SOURCE_ADDRESS: u64 = 3;
    /// Requirements already exist for this intent (idempotent - not an error in normal flow)
    const E_REQUIREMENTS_ALREADY_STORED: u64 = 4;
    /// Requirements not found for this intent
    const E_REQUIREMENTS_NOT_FOUND: u64 = 5;
    /// Intent already fulfilled
    const E_ALREADY_FULFILLED: u64 = 6;
    /// Intent has expired
    const E_INTENT_EXPIRED: u64 = 7;
    /// Solver is not authorized for this intent
    const E_UNAUTHORIZED_SOLVER: u64 = 8;
    /// Token mint does not match requirements
    const E_TOKEN_MISMATCH: u64 = 9;
    /// Config not initialized
    const E_CONFIG_NOT_INITIALIZED: u64 = 10;

    // ============================================================================
    // EVENTS
    // ============================================================================

    #[event]
    /// Emitted when IntentRequirements is received from hub.
    struct IntentRequirementsReceived has drop, store {
        intent_id: vector<u8>,
        src_chain_id: u32,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    }

    #[event]
    /// Emitted when duplicate IntentRequirements is received (idempotent).
    struct IntentRequirementsDuplicate has drop, store {
        intent_id: vector<u8>,
    }

    #[event]
    /// Emitted when a solver successfully fulfills an intent.
    struct FulfillmentSucceeded has drop, store {
        intent_id: vector<u8>,
        solver: address,
        recipient: address,
        amount: u64,
        token_addr: vector<u8>,
    }

    #[event]
    /// Emitted when a fulfillment proof is sent to the hub.
    struct FulfillmentProofSent has drop, store {
        intent_id: vector<u8>,
        solver_addr: vector<u8>,
        amount_fulfilled: u64,
        timestamp: u64,
        dst_chain_id: u32,
    }

    // ============================================================================
    // STATE
    // ============================================================================

    /// Global configuration for the outflow validator.
    struct OutflowValidatorConfig has key {
        /// Admin address (can update config)
        admin: address,
        /// Hub chain ID (GMP endpoint ID)
        hub_chain_id: u32,
        /// Hub GMP endpoint address (32 bytes)
        hub_gmp_endpoint_addr: vector<u8>,
    }

    /// Stored intent requirements from the hub.
    struct StoredRequirements has store, drop, copy {
        /// The recipient address on this chain (32 bytes, can be converted to address)
        requester_addr: vector<u8>,
        /// Amount of tokens required
        amount_required: u64,
        /// Token address (32 bytes, object address of FA metadata)
        token_addr: vector<u8>,
        /// Authorized solver address (32 bytes, zero = any solver)
        solver_addr: vector<u8>,
        /// Expiry timestamp (Unix seconds)
        expiry: u64,
        /// Whether this intent has been fulfilled
        fulfilled: bool,
    }

    /// Storage for all received intent requirements.
    struct IntentRequirementsStore has key {
        /// Map from intent_id (as vector<u8>) to requirements
        /// Using vector<u8> as key since intent_id is 32 bytes
        requirements: Table<vector<u8>, StoredRequirements>,
    }

    // ============================================================================
    // INITIALIZATION
    // ============================================================================

    /// Initialize the outflow validator config.
    /// Can only be called once by the module publisher.
    public entry fun initialize(
        admin: &signer,
        hub_chain_id: u32,
        hub_gmp_endpoint_addr: vector<u8>,
    ) {
        let admin_addr = signer::address_of(admin);
        assert!(admin_addr == @mvmt_intent, E_UNAUTHORIZED_ADMIN);

        // Initialize config
        move_to(admin, OutflowValidatorConfig {
            admin: admin_addr,
            hub_chain_id,
            hub_gmp_endpoint_addr,
        });

        // Initialize requirements store
        move_to(admin, IntentRequirementsStore {
            requirements: table::new(),
        });
    }

    /// Update the hub chain configuration.
    /// Only admin can call this.
    public entry fun update_hub_config(
        admin: &signer,
        hub_chain_id: u32,
        hub_gmp_endpoint_addr: vector<u8>,
    ) acquires OutflowValidatorConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<OutflowValidatorConfig>(@mvmt_intent);
        assert!(config.admin == admin_addr, E_UNAUTHORIZED_ADMIN);

        config.hub_chain_id = hub_chain_id;
        config.hub_gmp_endpoint_addr = hub_gmp_endpoint_addr;
    }

    // ============================================================================
    // INBOUND: Hub -> Connected Chain
    // ============================================================================

    /// Receive and store IntentRequirements from the hub.
    ///
    /// Called by the integrated GMP endpoint when a message is delivered from the hub.
    /// Implements idempotency: if requirements already exist, silently succeeds.
    ///
    /// # Arguments
    /// - `src_chain_id`: Source chain endpoint ID (must match hub)
    /// - `remote_gmp_endpoint_addr`: Source address (must match hub GMP endpoint address)
    /// - `payload`: Raw GMP message payload (IntentRequirements encoded)
    public fun receive_intent_requirements(
        src_chain_id: u32,
        remote_gmp_endpoint_addr: vector<u8>,
        payload: vector<u8>,
    ) acquires OutflowValidatorConfig, IntentRequirementsStore {
        // Verify config exists
        assert!(exists<OutflowValidatorConfig>(@mvmt_intent), E_CONFIG_NOT_INITIALIZED);

        let config = borrow_global<OutflowValidatorConfig>(@mvmt_intent);

        // Verify source chain matches hub
        assert!(src_chain_id == config.hub_chain_id, E_INVALID_SOURCE_CHAIN);

        // Verify source address matches hub GMP endpoint
        assert!(remote_gmp_endpoint_addr == config.hub_gmp_endpoint_addr, E_INVALID_SOURCE_ADDRESS);

        // Decode the message
        let msg = gmp_common::decode_intent_requirements(&payload);

        let intent_id = *gmp_common::intent_requirements_intent_id(&msg);

        // Get requirements store
        let store = borrow_global_mut<IntentRequirementsStore>(@mvmt_intent);

        // Idempotency check: if requirements already exist, emit duplicate event and return
        if (table::contains(&store.requirements, intent_id)) {
            event::emit(IntentRequirementsDuplicate {
                intent_id,
            });
            return
        };

        // Store the requirements
        let requirements = StoredRequirements {
            requester_addr: *gmp_common::intent_requirements_requester_addr(&msg),
            amount_required: gmp_common::intent_requirements_amount_required(&msg),
            token_addr: *gmp_common::intent_requirements_token_addr(&msg),
            solver_addr: *gmp_common::intent_requirements_solver_addr(&msg),
            expiry: gmp_common::intent_requirements_expiry(&msg),
            fulfilled: false,
        };

        table::add(&mut store.requirements, intent_id, requirements);

        // Emit event for tracking
        event::emit(IntentRequirementsReceived {
            intent_id,
            src_chain_id,
            requester_addr: *gmp_common::intent_requirements_requester_addr(&msg),
            amount_required: gmp_common::intent_requirements_amount_required(&msg),
            token_addr: *gmp_common::intent_requirements_token_addr(&msg),
            solver_addr: *gmp_common::intent_requirements_solver_addr(&msg),
            expiry: gmp_common::intent_requirements_expiry(&msg),
        });
    }

    // ============================================================================
    // FULFILLMENT
    // ============================================================================

    /// Fulfill an intent by transferring tokens from solver to recipient.
    ///
    /// The solver must:
    /// 1. Be authorized for this intent (or any solver if solver_addr is zero)
    /// 2. Transfer the exact amount required
    /// 3. Use the correct token
    ///
    /// After successful transfer, a FulfillmentProof is sent to the hub via GMP.
    ///
    /// # Arguments
    /// - `solver`: The solver signer (must be authorized)
    /// - `intent_id`: 32-byte intent identifier
    /// - `token_metadata`: The fungible asset metadata object
    public entry fun fulfill_intent(
        solver: &signer,
        intent_id: vector<u8>,
        token_metadata: Object<Metadata>,
    ) acquires OutflowValidatorConfig, IntentRequirementsStore {
        let solver_addr = signer::address_of(solver);

        // Verify config exists
        assert!(exists<OutflowValidatorConfig>(@mvmt_intent), E_CONFIG_NOT_INITIALIZED);

        // Load requirements
        let store = borrow_global_mut<IntentRequirementsStore>(@mvmt_intent);
        assert!(table::contains(&store.requirements, intent_id), E_REQUIREMENTS_NOT_FOUND);

        let requirements = table::borrow_mut(&mut store.requirements, intent_id);

        // Verify not already fulfilled
        assert!(!requirements.fulfilled, E_ALREADY_FULFILLED);

        // Verify not expired
        let current_time = timestamp::now_seconds();
        assert!(current_time <= requirements.expiry, E_INTENT_EXPIRED);

        // Verify solver is authorized (zero address = any solver allowed)
        let zero_addr = create_zero_bytes32();
        if (requirements.solver_addr != zero_addr) {
            let solver_bytes = address_to_bytes32(solver_addr);
            assert!(solver_bytes == requirements.solver_addr, E_UNAUTHORIZED_SOLVER);
        };

        // Verify token matches
        let token_addr_from_metadata = address_to_bytes32(object::object_address(&token_metadata));
        assert!(token_addr_from_metadata == requirements.token_addr, E_TOKEN_MISMATCH);

        // Convert recipient address from bytes32 to address
        let recipient = bytes32_to_address(&requirements.requester_addr);

        // Extract values from requirements before modifying
        let amount_required = requirements.amount_required;
        let token_addr_copy = requirements.token_addr;

        // Transfer tokens from solver to recipient using primary fungible store
        primary_fungible_store::transfer(
            solver,
            token_metadata,
            recipient,
            amount_required,
        );

        // Mark as fulfilled
        requirements.fulfilled = true;

        // Emit success event
        event::emit(FulfillmentSucceeded {
            intent_id: copy intent_id,
            solver: solver_addr,
            recipient,
            amount: amount_required,
            token_addr: token_addr_copy,
        });

        // Create FulfillmentProof payload and send via GMP
        let config = borrow_global<OutflowValidatorConfig>(@mvmt_intent);
        let fulfillment_proof = gmp_common::new_fulfillment_proof(
            copy intent_id,
            address_to_bytes32(solver_addr),
            amount_required,
            current_time,
        );
        let payload = gmp_common::encode_fulfillment_proof(&fulfillment_proof);

        // Send FulfillmentProof to hub via gmp_sender::gmp_send
        // (Separate sender module avoids circular dependency with receiver)
        let nonce = gmp_sender::gmp_send(
            solver,
            config.hub_chain_id,
            config.hub_gmp_endpoint_addr,
            payload,
        );

        // Emit proof sent event for tracking
        event::emit(FulfillmentProofSent {
            intent_id,
            solver_addr: address_to_bytes32(solver_addr),
            amount_fulfilled: amount_required,
            timestamp: current_time,
            dst_chain_id: config.hub_chain_id,
        });

        // Suppress unused variable warning
        let _ = nonce;
    }

    // ============================================================================
    // VIEW FUNCTIONS
    // ============================================================================

    #[view]
    /// Check if requirements exist for an intent.
    public fun has_requirements(intent_id: vector<u8>): bool acquires IntentRequirementsStore {
        if (!exists<IntentRequirementsStore>(@mvmt_intent)) {
            return false
        };
        let store = borrow_global<IntentRequirementsStore>(@mvmt_intent);
        table::contains(&store.requirements, intent_id)
    }

    #[view]
    /// Check if an intent has been fulfilled.
    public fun is_fulfilled(intent_id: vector<u8>): bool acquires IntentRequirementsStore {
        if (!exists<IntentRequirementsStore>(@mvmt_intent)) {
            return false
        };
        let store = borrow_global<IntentRequirementsStore>(@mvmt_intent);
        if (!table::contains(&store.requirements, intent_id)) {
            return false
        };
        let requirements = table::borrow(&store.requirements, intent_id);
        requirements.fulfilled
    }

    #[view]
    /// Get the amount required for an intent.
    public fun get_amount_required(intent_id: vector<u8>): u64 acquires IntentRequirementsStore {
        let store = borrow_global<IntentRequirementsStore>(@mvmt_intent);
        let requirements = table::borrow(&store.requirements, intent_id);
        requirements.amount_required
    }

    #[view]
    /// Get the hub chain ID from config.
    public fun get_hub_chain_id(): u32 acquires OutflowValidatorConfig {
        let config = borrow_global<OutflowValidatorConfig>(@mvmt_intent);
        config.hub_chain_id
    }

    #[view]
    /// Get the hub GMP endpoint address from config.
    public fun get_hub_gmp_endpoint_addr(): vector<u8> acquires OutflowValidatorConfig {
        let config = borrow_global<OutflowValidatorConfig>(@mvmt_intent);
        config.hub_gmp_endpoint_addr
    }

    #[view]
    /// Check if the outflow validator is initialized.
    public fun is_initialized(): bool {
        exists<OutflowValidatorConfig>(@mvmt_intent) && exists<IntentRequirementsStore>(@mvmt_intent)
    }

    // ============================================================================
    // INTERNAL HELPERS
    // ============================================================================

    /// Create a 32-byte zero vector.
    fun create_zero_bytes32(): vector<u8> {
        let result = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut result, 0);
            i = i + 1;
        };
        result
    }

    /// Convert an address to a 32-byte vector (big-endian).
    fun address_to_bytes32(addr: address): vector<u8> {
        std::bcs::to_bytes(&addr)
    }

    /// Convert a 32-byte vector to an address.
    /// Assumes the input is a valid 32-byte address representation.
    fun bytes32_to_address(bytes: &vector<u8>): address {
        // BCS-encoded address is just the 32 bytes directly
        std::from_bcs::to_address(*bytes)
    }
}
