/// Inflow Escrow (MVM as Connected Chain)
///
/// Handles inflow intent escrows when MVM acts as a connected chain.
/// Receives intent requirements from the hub via GMP, validates escrow creation,
/// and sends escrow confirmation back to the hub. When fulfillment proof is
/// received, automatically releases escrowed funds to the solver (single-step).
///
/// ## Inflow Definition
/// Inflow = tokens locked on connected chain (MVM), desired on hub
///
/// ## Flow
///
/// 1. Hub creates intent → sends IntentRequirements via GMP
/// 2. This module receives requirements via `receive_intent_requirements` (stores in table)
/// 3. User creates escrow via `create_escrow_with_validation`:
///    - Validates escrow matches requirements (amount, token, requester)
///    - Stores escrow info
///    - Sends EscrowConfirmation back to hub via GMP
/// 4. Hub receives confirmation → solver fulfills on hub
/// 5. Hub sends FulfillmentProof via GMP
/// 6. This module receives proof via `receive_fulfillment_proof`:
///    - Auto-releases escrowed funds to the solver (single-step)
///    - Emits both FulfillmentProofReceived and EscrowReleased events
module mvmt_intent::intent_inflow_escrow {
    use std::signer;
    use std::vector;
    use aptos_framework::event;
    use aptos_framework::fungible_asset::{Self, FungibleAsset, Metadata};
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
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
    /// Escrow already created for this intent
    const E_ESCROW_ALREADY_CREATED: u64 = 6;
    /// Intent has expired
    const E_INTENT_EXPIRED: u64 = 7;
    /// Amount does not match requirements
    const E_AMOUNT_MISMATCH: u64 = 8;
    /// Token does not match requirements
    const E_TOKEN_MISMATCH: u64 = 9;
    /// Config not initialized
    const E_CONFIG_NOT_INITIALIZED: u64 = 10;
    /// Escrow not found
    const E_ESCROW_NOT_FOUND: u64 = 11;
    /// Already fulfilled (fulfillment proof already received)
    const E_ALREADY_FULFILLED: u64 = 12;
    /// Escrow not fulfilled yet (cannot release without fulfillment proof)
    const E_NOT_FULFILLED: u64 = 13;
    /// Unauthorized solver (not the authorized solver for this escrow)
    const E_UNAUTHORIZED_SOLVER: u64 = 14;
    /// Requester mismatch
    const E_REQUESTER_MISMATCH: u64 = 16;
    /// Escrow has not expired yet (cannot cancel before expiry)
    const E_ESCROW_NOT_EXPIRED: u64 = 15;
    /// Caller is not authorized (not the requester or admin)
    const E_UNAUTHORIZED_CALLER: u64 = 17;

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
    /// Emitted when an escrow is created and validated.
    struct EscrowCreated has drop, store {
        intent_id: vector<u8>,
        escrow_id: vector<u8>,
        requester: address,
        amount: u64,
        token_addr: vector<u8>,
        reserved_solver: vector<u8>,
        expiry: u64,
    }

    #[event]
    /// Emitted when EscrowConfirmation is sent to hub.
    struct EscrowConfirmationSent has drop, store {
        intent_id: vector<u8>,
        escrow_id: vector<u8>,
        amount_escrowed: u64,
        dst_chain_id: u32,
    }

    #[event]
    /// Emitted when FulfillmentProof is received from hub.
    struct FulfillmentProofReceived has drop, store {
        intent_id: vector<u8>,
        src_chain_id: u32,
        solver_addr: vector<u8>,
        amount_fulfilled: u64,
        timestamp: u64,
    }

    #[event]
    /// Emitted when escrowed funds are released to solver.
    struct EscrowReleased has drop, store {
        intent_id: vector<u8>,
        solver: address,
        amount: u64,
    }

    #[event]
    /// Emitted when escrow is cancelled and funds returned to requester.
    struct EscrowCancelled has drop, store {
        intent_id: vector<u8>,
        requester: address,
        amount: u64,
    }

    // ============================================================================
    // STATE
    // ============================================================================

    /// Global configuration for the inflow escrow GMP.
    struct InflowEscrowConfig has key {
        /// Admin address (can update config)
        admin: address,
        /// Hub chain ID (GMP endpoint ID)
        hub_chain_id: u32,
        /// Hub GMP endpoint address (32 bytes)
        hub_gmp_endpoint_addr: vector<u8>,
    }

    /// Stored intent requirements from the hub.
    struct StoredRequirements has store, drop, copy {
        /// The requester address on this chain (32 bytes)
        requester_addr: vector<u8>,
        /// Amount of tokens required in escrow
        amount_required: u64,
        /// Token address (32 bytes, object address of FA metadata)
        token_addr: vector<u8>,
        /// Authorized solver address (32 bytes, zero = any solver)
        solver_addr: vector<u8>,
        /// Expiry timestamp (Unix seconds)
        expiry: u64,
        /// Whether escrow has been created
        escrow_created: bool,
    }

    /// Stored escrow info after escrow is created.
    struct StoredEscrow has store, drop, copy {
        /// Escrow ID (derived from intent_id + creator)
        escrow_id: vector<u8>,
        /// Creator address
        creator_addr: vector<u8>,
        /// Amount escrowed
        amount: u64,
        /// Token address
        token_addr: vector<u8>,
        /// Authorized solver from requirements
        solver_addr: vector<u8>,
        /// Whether fulfillment proof has been received
        fulfilled: bool,
        /// Whether funds have been released
        released: bool,
    }

    /// Storage for all received intent requirements.
    struct IntentRequirementsStore has key {
        /// Map from intent_id (as vector<u8>) to requirements
        requirements: Table<vector<u8>, StoredRequirements>,
    }

    /// Storage for created escrows.
    /// Note: Using table to store FungibleAsset requires a resource account,
    /// but for simplicity we track amount and do transfers at release time.
    struct EscrowStore has key {
        /// Map from intent_id (as vector<u8>) to escrow info
        escrows: Table<vector<u8>, StoredEscrow>,
    }

    /// Resource account that holds escrowed funds.
    struct EscrowVault has key {
        /// Signer capability for the escrow vault
        signer_cap: aptos_framework::account::SignerCapability,
    }

    // ============================================================================
    // INITIALIZATION
    // ============================================================================

    /// Initialize the inflow escrow GMP config.
    /// Can only be called once by the module publisher.
    public entry fun initialize(
        admin: &signer,
        hub_chain_id: u32,
        hub_gmp_endpoint_addr: vector<u8>,
    ) {
        let admin_addr = signer::address_of(admin);
        assert!(admin_addr == @mvmt_intent, E_UNAUTHORIZED_ADMIN);

        // Initialize config
        move_to(admin, InflowEscrowConfig {
            admin: admin_addr,
            hub_chain_id,
            hub_gmp_endpoint_addr,
        });

        // Initialize requirements store
        move_to(admin, IntentRequirementsStore {
            requirements: table::new(),
        });

        // Initialize escrow store
        move_to(admin, EscrowStore {
            escrows: table::new(),
        });

        // Create resource account for escrow vault
        let (vault_signer, signer_cap) = aptos_framework::account::create_resource_account(
            admin,
            b"inflow_escrow_vault"
        );
        move_to(&vault_signer, EscrowVault { signer_cap });
    }

    /// Update the hub chain configuration.
    /// Only admin can call this.
    public entry fun update_hub_config(
        admin: &signer,
        hub_chain_id: u32,
        hub_gmp_endpoint_addr: vector<u8>,
    ) acquires InflowEscrowConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<InflowEscrowConfig>(@mvmt_intent);
        assert!(config.admin == admin_addr, E_UNAUTHORIZED_ADMIN);

        config.hub_chain_id = hub_chain_id;
        config.hub_gmp_endpoint_addr = hub_gmp_endpoint_addr;
    }

    // ============================================================================
    // INBOUND: Hub -> Connected Chain (IntentRequirements)
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
    ) acquires InflowEscrowConfig, IntentRequirementsStore {
        // Verify config exists
        assert!(exists<InflowEscrowConfig>(@mvmt_intent), E_CONFIG_NOT_INITIALIZED);

        let config = borrow_global<InflowEscrowConfig>(@mvmt_intent);

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
            escrow_created: false,
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
    // ESCROW CREATION
    // ============================================================================

    /// Entry function to create an escrow with validation against stored requirements.
    ///
    /// Withdraws tokens from the creator's primary FA store and creates the escrow.
    /// Validates against stored requirements and sends EscrowConfirmation to hub.
    ///
    /// # Arguments
    /// - `creator`: The signer creating the escrow (requester)
    /// - `intent_id`: 32-byte intent identifier
    /// - `token_metadata`: The fungible asset metadata object
    /// - `amount`: Amount of tokens to escrow
    public entry fun create_escrow_with_validation(
        creator: &signer,
        intent_id: vector<u8>,
        token_metadata: Object<Metadata>,
        amount: u64,
    ) acquires InflowEscrowConfig, IntentRequirementsStore, EscrowStore {
        // Withdraw tokens from creator's primary FA store
        let asset = primary_fungible_store::withdraw(creator, token_metadata, amount);

        // Call the internal function
        create_escrow_with_validation_internal(creator, intent_id, asset, token_metadata);
    }

    /// Internal function to create an escrow with validation.
    ///
    /// The user deposits tokens into escrow, and this function validates:
    /// 1. Requirements exist for the intent_id
    /// 2. Amount matches requirements
    /// 3. Token matches requirements
    /// 4. Intent has not expired
    /// 5. Escrow hasn't already been created for this intent
    ///
    /// On success, sends EscrowConfirmation to hub via GMP.
    ///
    /// # Arguments
    /// - `creator`: The signer creating the escrow (requester)
    /// - `intent_id`: 32-byte intent identifier
    /// - `asset`: Fungible asset to escrow
    /// - `token_metadata`: The fungible asset metadata object
    public fun create_escrow_with_validation_internal(
        creator: &signer,
        intent_id: vector<u8>,
        asset: FungibleAsset,
        token_metadata: Object<Metadata>,
    ) acquires InflowEscrowConfig, IntentRequirementsStore, EscrowStore {
        let creator_addr = signer::address_of(creator);

        // Verify config exists
        assert!(exists<InflowEscrowConfig>(@mvmt_intent), E_CONFIG_NOT_INITIALIZED);

        // Load requirements
        let req_store = borrow_global_mut<IntentRequirementsStore>(@mvmt_intent);
        assert!(table::contains(&req_store.requirements, intent_id), E_REQUIREMENTS_NOT_FOUND);

        let requirements = table::borrow_mut(&mut req_store.requirements, intent_id);

        // Verify escrow hasn't already been created
        assert!(!requirements.escrow_created, E_ESCROW_ALREADY_CREATED);

        // Verify not expired
        let current_time = aptos_framework::timestamp::now_seconds();
        assert!(current_time <= requirements.expiry, E_INTENT_EXPIRED);

        // Verify amount matches
        let amount = fungible_asset::amount(&asset);
        assert!(amount == requirements.amount_required, E_AMOUNT_MISMATCH);

        // Verify token matches
        let token_addr_from_metadata = address_to_bytes32(object::object_address(&token_metadata));
        assert!(token_addr_from_metadata == requirements.token_addr, E_TOKEN_MISMATCH);

        // Verify creator is the requester
        let creator_bytes = address_to_bytes32(creator_addr);
        assert!(creator_bytes == requirements.requester_addr, E_REQUESTER_MISMATCH);

        // Generate escrow ID (hash of intent_id + creator for uniqueness)
        let escrow_id = generate_escrow_id(&intent_id, creator_addr);

        // Mark requirements as having escrow created
        requirements.escrow_created = true;

        // Store escrow info
        let escrow_store = borrow_global_mut<EscrowStore>(@mvmt_intent);
        let escrow = StoredEscrow {
            escrow_id: copy escrow_id,
            creator_addr: creator_bytes,
            amount,
            token_addr: requirements.token_addr,
            solver_addr: requirements.solver_addr,
            fulfilled: false,
            released: false,
        };
        table::add(&mut escrow_store.escrows, intent_id, escrow);

        // Transfer tokens to vault
        let vault_addr = get_vault_address();
        primary_fungible_store::deposit(vault_addr, asset);

        // Emit escrow created event
        event::emit(EscrowCreated {
            intent_id: copy intent_id,
            escrow_id: copy escrow_id,
            requester: creator_addr,
            amount,
            token_addr: requirements.token_addr,
            reserved_solver: requirements.solver_addr,
            expiry: requirements.expiry,
        });

        // Send EscrowConfirmation to hub via GMP
        let config = borrow_global<InflowEscrowConfig>(@mvmt_intent);
        let confirmation = gmp_common::new_escrow_confirmation(
            copy intent_id,
            copy escrow_id,
            amount,
            requirements.token_addr,
            creator_bytes,
        );
        let payload = gmp_common::encode_escrow_confirmation(&confirmation);

        let nonce = gmp_sender::gmp_send(
            creator,
            config.hub_chain_id,
            config.hub_gmp_endpoint_addr,
            payload,
        );

        // Emit confirmation sent event
        event::emit(EscrowConfirmationSent {
            intent_id,
            escrow_id,
            amount_escrowed: amount,
            dst_chain_id: config.hub_chain_id,
        });

        // Suppress unused variable warning
        let _ = nonce;
    }

    // ============================================================================
    // INBOUND: Hub -> Connected Chain (FulfillmentProof)
    // ============================================================================

    /// Receive FulfillmentProof from the hub and auto-release escrow to solver.
    ///
    /// Called by the integrated GMP endpoint when the hub reports that a solver
    /// has fulfilled the intent on the hub. This marks the escrow as fulfilled
    /// AND immediately transfers tokens to the solver (single-step release).
    ///
    /// # Arguments
    /// - `src_chain_id`: Source chain endpoint ID (must match hub)
    /// - `remote_gmp_endpoint_addr`: Source address (must match hub GMP endpoint address)
    /// - `payload`: Raw GMP message payload (FulfillmentProof encoded)
    public fun receive_fulfillment_proof(
        src_chain_id: u32,
        remote_gmp_endpoint_addr: vector<u8>,
        payload: vector<u8>,
    ) acquires InflowEscrowConfig, EscrowStore, EscrowVault {
        // Verify config exists
        assert!(exists<InflowEscrowConfig>(@mvmt_intent), E_CONFIG_NOT_INITIALIZED);

        let config = borrow_global<InflowEscrowConfig>(@mvmt_intent);

        // Verify source chain matches hub
        assert!(src_chain_id == config.hub_chain_id, E_INVALID_SOURCE_CHAIN);

        // Verify source address matches hub GMP endpoint
        assert!(remote_gmp_endpoint_addr == config.hub_gmp_endpoint_addr, E_INVALID_SOURCE_ADDRESS);

        // Decode the message
        let msg = gmp_common::decode_fulfillment_proof(&payload);

        let intent_id = *gmp_common::fulfillment_proof_intent_id(&msg);
        let solver_addr_bytes = *gmp_common::fulfillment_proof_solver_addr(&msg);
        let amount_fulfilled = gmp_common::fulfillment_proof_amount_fulfilled(&msg);
        let timestamp = gmp_common::fulfillment_proof_timestamp(&msg);

        // Load escrow
        let escrow_store = borrow_global_mut<EscrowStore>(@mvmt_intent);
        assert!(table::contains(&escrow_store.escrows, intent_id), E_ESCROW_NOT_FOUND);

        let escrow = table::borrow_mut(&mut escrow_store.escrows, intent_id);

        // Verify not already fulfilled
        assert!(!escrow.fulfilled, E_ALREADY_FULFILLED);

        // Mark as fulfilled and released
        escrow.fulfilled = true;
        escrow.released = true;
        let amount = escrow.amount;
        let token_addr_bytes = escrow.token_addr;

        // Convert solver address from bytes32 to address
        let solver_addr = bytes32_to_address(&solver_addr_bytes);

        // Get token metadata from stored token address
        let token_addr = bytes32_to_address(&token_addr_bytes);
        let token_metadata = object::address_to_object<Metadata>(token_addr);

        // Transfer tokens from vault to solver
        let vault_signer = get_vault_signer();
        primary_fungible_store::transfer(
            &vault_signer,
            token_metadata,
            solver_addr,
            amount,
        );

        // Emit fulfillment proof received event
        event::emit(FulfillmentProofReceived {
            intent_id: copy intent_id,
            src_chain_id,
            solver_addr: solver_addr_bytes,
            amount_fulfilled,
            timestamp,
        });

        // Emit release event
        event::emit(EscrowReleased {
            intent_id,
            solver: solver_addr,
            amount,
        });
    }


    // ============================================================================
    // CANCELLATION
    // ============================================================================

    /// Cancel an expired escrow and return funds to the requester.
    ///
    /// The original requester or the admin can cancel, but only after the
    /// escrow's expiry timestamp has passed. Funds always return to the
    /// original requester regardless of who initiates the cancellation.
    ///
    /// # Arguments
    /// - `caller`: The signer — must be the original requester or admin
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Aborts
    /// - `E_CONFIG_NOT_INITIALIZED`: Module not initialized
    /// - `E_ESCROW_NOT_FOUND`: No escrow exists for this intent_id
    /// - `E_ALREADY_FULFILLED`: Escrow already released (fulfilled or cancelled)
    /// - `E_UNAUTHORIZED_CALLER`: Caller is not the original requester or admin
    /// - `E_ESCROW_NOT_EXPIRED`: Escrow has not expired yet
    public entry fun cancel_escrow(
        caller: &signer,
        intent_id: vector<u8>,
    ) acquires InflowEscrowConfig, IntentRequirementsStore, EscrowStore, EscrowVault {
        // Verify config exists
        assert!(exists<InflowEscrowConfig>(@mvmt_intent), E_CONFIG_NOT_INITIALIZED);

        // Read admin from config
        let config = borrow_global<InflowEscrowConfig>(@mvmt_intent);
        let admin = config.admin;

        // Load escrow
        let escrow_store = borrow_global_mut<EscrowStore>(@mvmt_intent);
        assert!(table::contains(&escrow_store.escrows, intent_id), E_ESCROW_NOT_FOUND);

        let escrow = table::borrow_mut(&mut escrow_store.escrows, intent_id);

        // Verify not already released (covers both fulfillment and prior cancellation)
        assert!(!escrow.released, E_ALREADY_FULFILLED);

        // Verify caller is admin (only admin can cancel expired escrows)
        let caller_addr = signer::address_of(caller);
        assert!(caller_addr == admin, E_UNAUTHORIZED_CALLER);

        // Verify escrow has expired
        let req_store = borrow_global<IntentRequirementsStore>(@mvmt_intent);
        let requirements = table::borrow(&req_store.requirements, intent_id);
        let current_time = aptos_framework::timestamp::now_seconds();
        assert!(current_time > requirements.expiry, E_ESCROW_NOT_EXPIRED);

        // Mark as released
        escrow.released = true;
        let amount = escrow.amount;
        let token_addr_bytes = escrow.token_addr;

        // Get token metadata from stored token address
        let token_addr = bytes32_to_address(&token_addr_bytes);
        let token_metadata = object::address_to_object<Metadata>(token_addr);

        // Transfer tokens from vault back to original requester (not the caller)
        let requester_addr = bytes32_to_address(&escrow.creator_addr);
        let vault_signer = get_vault_signer();
        primary_fungible_store::transfer(
            &vault_signer,
            token_metadata,
            requester_addr,
            amount,
        );

        // Emit cancellation event
        event::emit(EscrowCancelled {
            intent_id,
            requester: requester_addr,
            amount,
        });
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
    /// Check if an escrow exists for an intent.
    public fun has_escrow(intent_id: vector<u8>): bool acquires EscrowStore {
        if (!exists<EscrowStore>(@mvmt_intent)) {
            return false
        };
        let store = borrow_global<EscrowStore>(@mvmt_intent);
        table::contains(&store.escrows, intent_id)
    }

    #[view]
    /// Check if an escrow has been fulfilled.
    public fun is_fulfilled(intent_id: vector<u8>): bool acquires EscrowStore {
        if (!exists<EscrowStore>(@mvmt_intent)) {
            return false
        };
        let store = borrow_global<EscrowStore>(@mvmt_intent);
        if (!table::contains(&store.escrows, intent_id)) {
            return false
        };
        let escrow = table::borrow(&store.escrows, intent_id);
        escrow.fulfilled
    }

    #[view]
    /// Check if an escrow has been released.
    public fun is_released(intent_id: vector<u8>): bool acquires EscrowStore {
        if (!exists<EscrowStore>(@mvmt_intent)) {
            return false
        };
        let store = borrow_global<EscrowStore>(@mvmt_intent);
        if (!table::contains(&store.escrows, intent_id)) {
            return false
        };
        let escrow = table::borrow(&store.escrows, intent_id);
        escrow.released
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
    public fun get_hub_chain_id(): u32 acquires InflowEscrowConfig {
        let config = borrow_global<InflowEscrowConfig>(@mvmt_intent);
        config.hub_chain_id
    }

    #[view]
    /// Get the hub GMP endpoint address from config.
    public fun get_hub_gmp_endpoint_addr(): vector<u8> acquires InflowEscrowConfig {
        let config = borrow_global<InflowEscrowConfig>(@mvmt_intent);
        config.hub_gmp_endpoint_addr
    }

    #[view]
    /// Check if the inflow escrow GMP is initialized.
    public fun is_initialized(): bool {
        exists<InflowEscrowConfig>(@mvmt_intent)
            && exists<IntentRequirementsStore>(@mvmt_intent)
            && exists<EscrowStore>(@mvmt_intent)
    }

    // ============================================================================
    // INTERNAL HELPERS
    // ============================================================================

    /// Get the vault address.
    fun get_vault_address(): address {
        aptos_framework::account::create_resource_address(&@mvmt_intent, b"inflow_escrow_vault")
    }

    /// Get the vault signer.
    fun get_vault_signer(): signer acquires EscrowVault {
        let vault_addr = get_vault_address();
        let vault = borrow_global<EscrowVault>(vault_addr);
        aptos_framework::account::create_signer_with_capability(&vault.signer_cap)
    }

    /// Generate an escrow ID from intent_id and creator address.
    fun generate_escrow_id(intent_id: &vector<u8>, creator: address): vector<u8> {
        let combined = vector::empty<u8>();
        let i = 0;
        let len = vector::length(intent_id);
        while (i < len) {
            vector::push_back(&mut combined, *vector::borrow(intent_id, i));
            i = i + 1;
        };
        let creator_bytes = std::bcs::to_bytes(&creator);
        i = 0;
        len = vector::length(&creator_bytes);
        while (i < len) {
            vector::push_back(&mut combined, *vector::borrow(&creator_bytes, i));
            i = i + 1;
        };
        // Hash to 32 bytes — gmp_common wire format requires fixed 32-byte fields
        aptos_std::hash::sha3_256(combined)
    }

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
    fun bytes32_to_address(bytes: &vector<u8>): address {
        std::from_bcs::to_address(*bytes)
    }
}
