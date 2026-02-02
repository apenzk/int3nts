#[test_only]
module mvmt_intent::outflow_validator_tests {
    use std::vector;
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use aptos_framework::fungible_asset::Metadata;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::primary_fungible_store;
    use mvmt_intent::outflow_validator_impl;
    use mvmt_intent::native_gmp_endpoint;
    use mvmt_intent::gmp_sender;
    use mvmt_intent::gmp_common;
    use mvmt_intent::test_utils;

    // Test addresses
    const ADMIN_ADDR: address = @mvmt_intent;
    const SOLVER_ADDR: address = @0x456;
    const RECIPIENT_ADDR: address = @0x789;
    const HUB_CHAIN_ID: u32 = 30106; // Movement hub chain ID

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    fun create_test_hub_addr(): vector<u8> {
        let addr = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut addr, ((i + 1) as u8));
            i = i + 1;
        };
        addr
    }

    fun create_test_intent_id(): vector<u8> {
        let id = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut id, 0x11);
            i = i + 1;
        };
        id
    }

    fun create_test_intent_id_2(): vector<u8> {
        let id = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut id, 0x22);
            i = i + 1;
        };
        id
    }

    fun address_to_bytes32(addr: address): vector<u8> {
        std::bcs::to_bytes(&addr)
    }

    fun create_zero_bytes32(): vector<u8> {
        let result = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut result, 0);
            i = i + 1;
        };
        result
    }

    fun create_test_payload(
        intent_id: vector<u8>,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    ): vector<u8> {
        let msg = gmp_common::new_intent_requirements(
            intent_id,
            requester_addr,
            amount_required,
            token_addr,
            solver_addr,
            expiry,
        );
        gmp_common::encode_intent_requirements(&msg)
    }

    /// Initialize GMP modules and outflow validator.
    fun init_modules(admin: &signer) {
        gmp_sender::initialize(admin);
        native_gmp_endpoint::initialize(admin);
        let trusted_hub_addr = create_test_hub_addr();
        outflow_validator_impl::initialize(admin, HUB_CHAIN_ID, trusted_hub_addr);
    }

    /// Store intent requirements for a given intent_id with the specified parameters.
    fun store_requirements(
        intent_id: vector<u8>,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    ) {
        let payload = create_test_payload(
            intent_id,
            requester_addr,
            amount_required,
            token_addr,
            solver_addr,
            expiry,
        );

        let src_addr = create_test_hub_addr();

        outflow_validator_impl::receive_intent_requirements(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );
    }

    // ============================================================================
    // INITIALIZATION TESTS
    // ============================================================================

    /// 1. Test: Initialize creates config
    /// Verifies that initialize correctly sets up the outflow validator configuration.
    /// Why: Proper initialization is required before any other operations can succeed.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_initialize_creates_config(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        // Verify initialization
        assert!(outflow_validator_impl::is_initialized(), 1);
        assert!(outflow_validator_impl::get_hub_chain_id() == HUB_CHAIN_ID, 2);

        let stored_hub_addr = outflow_validator_impl::get_trusted_hub_addr();
        let expected_hub_addr = create_test_hub_addr();
        assert!(stored_hub_addr == expected_hub_addr, 3);
    }

    /// 2. Test: Double initialization fails
    /// Verifies that trying to initialize twice fails.
    /// Why: Config must only be set once to prevent admin takeover.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    #[expected_failure] // Already exists
    fun test_initialize_rejects_double_init(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        // Second init should fail
        let trusted_hub_addr = create_test_hub_addr();
        outflow_validator_impl::initialize(admin, HUB_CHAIN_ID, trusted_hub_addr);
    }

    // ============================================================================
    // LZ_RECEIVE TESTS
    // ============================================================================

    /// 3. Test: Receive stores intent requirements
    /// Verifies that receive_intent_requirements correctly stores requirements.
    /// Why: Storing requirements is essential for validating fulfillments later.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_receive_stores_requirements(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        let intent_id = create_test_intent_id();
        let requester_addr = address_to_bytes32(RECIPIENT_ADDR);
        let token_addr = create_zero_bytes32(); // Placeholder token
        let solver_addr = address_to_bytes32(SOLVER_ADDR);
        let amount = 1000000u64;
        let expiry = 2000u64; // Future timestamp

        let payload = create_test_payload(
            copy intent_id,
            requester_addr,
            amount,
            token_addr,
            solver_addr,
            expiry,
        );

        let src_addr = create_test_hub_addr();

        // Receive requirements
        outflow_validator_impl::receive_intent_requirements(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );

        // Verify requirements stored
        assert!(outflow_validator_impl::has_requirements(copy intent_id), 1);
        assert!(!outflow_validator_impl::is_fulfilled(copy intent_id), 2);
        assert!(outflow_validator_impl::get_amount_required(intent_id) == amount, 3);
    }

    /// 4. Test: Receive is idempotent (duplicate ignored)
    /// Verifies that receiving the same requirements twice doesn't fail or overwrite.
    /// Why: Idempotency prevents issues with duplicate GMP message delivery.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_receive_idempotent(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        let intent_id = create_test_intent_id();
        let requester_addr = address_to_bytes32(RECIPIENT_ADDR);
        let token_addr = create_zero_bytes32();
        let solver_addr = address_to_bytes32(SOLVER_ADDR);
        let amount = 1000000u64;
        let expiry = 2000u64;

        let payload = create_test_payload(
            copy intent_id,
            requester_addr,
            amount,
            token_addr,
            solver_addr,
            expiry,
        );

        let src_addr = create_test_hub_addr();

        // First receive
        outflow_validator_impl::receive_intent_requirements(
            HUB_CHAIN_ID,
            copy src_addr,
            copy payload,
        );

        // Second receive (should succeed without error - idempotent)
        outflow_validator_impl::receive_intent_requirements(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );

        // Verify still stored correctly (not duplicated or overwritten)
        assert!(outflow_validator_impl::has_requirements(intent_id), 1);
    }

    /// 5. Test: Receive rejects untrusted source
    /// Verifies that requirements from non-trusted hub chains/addresses are rejected.
    /// Why: Source verification prevents spoofed messages from attackers.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    #[expected_failure(abort_code = 2, location = mvmt_intent::outflow_validator_impl)] // EINVALID_SOURCE_CHAIN
    fun test_receive_rejects_untrusted_source(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        let intent_id = create_test_intent_id();
        let requester_addr = address_to_bytes32(RECIPIENT_ADDR);
        let token_addr = create_zero_bytes32();
        let solver_addr = address_to_bytes32(SOLVER_ADDR);
        let amount = 1000000u64;
        let expiry = 2000u64;

        let payload = create_test_payload(
            intent_id,
            requester_addr,
            amount,
            token_addr,
            solver_addr,
            expiry,
        );

        let src_addr = create_test_hub_addr();

        // Use wrong chain ID
        outflow_validator_impl::receive_intent_requirements(
            99999u32, // Wrong chain ID
            src_addr,
            payload,
        );
    }

    /// 6. Test: Receive rejects invalid payload
    /// Verifies that malformed GMP payloads are rejected.
    /// Why: Prevents processing of corrupted or malicious messages.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    #[expected_failure] // gmp_common decode will fail
    fun test_receive_rejects_invalid_payload(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        let src_addr = create_test_hub_addr();

        // Invalid payload - too short to be valid IntentRequirements
        let invalid_payload = vector[0x01, 0x02, 0x03];

        outflow_validator_impl::receive_intent_requirements(
            HUB_CHAIN_ID,
            src_addr,
            invalid_payload,
        );
    }

    // ============================================================================
    // FULFILL INTENT TESTS
    // ============================================================================

    /// 7. Test: Fulfill intent rejects already fulfilled
    /// Verifies that double fulfillment is rejected.
    /// Why: Prevents solver from claiming payment twice.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, solver = @0x456)]
    #[expected_failure(abort_code = 6, location = mvmt_intent::outflow_validator_impl)] // EALREADY_FULFILLED
    fun test_fulfill_intent_rejects_already_fulfilled(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        solver: &signer,
    ) {
        // Setup FA token via test_utils - mints to token_creator (max supply = 100)
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to solver
        primary_fungible_store::transfer(token_creator, token_metadata, SOLVER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 30u64; // Use smaller amount within max supply
        let expiry = 2000u64; // Future timestamp (current is 1000)

        // Create recipient account
        account::create_account_for_test(RECIPIENT_ADDR);

        // Store requirements with zero solver (any solver allowed)
        store_requirements(
            copy intent_id,
            address_to_bytes32(RECIPIENT_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(), // Any solver
            expiry,
        );

        // First fulfillment should succeed
        outflow_validator_impl::fulfill_intent(solver, copy intent_id, token_metadata);

        // Second fulfillment should fail
        outflow_validator_impl::fulfill_intent(solver, intent_id, token_metadata);
    }

    /// 8. Test: Fulfill intent rejects expired intent
    /// Verifies that expired intents cannot be fulfilled.
    /// Why: Protects solver from fulfilling intents user no longer wants.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, solver = @0x456)]
    #[expected_failure(abort_code = 7, location = mvmt_intent::outflow_validator_impl)] // EINTENT_EXPIRED
    fun test_fulfill_intent_rejects_expired(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        solver: &signer,
    ) {
        // Setup FA token (max supply = 100)
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to solver
        primary_fungible_store::transfer(token_creator, token_metadata, SOLVER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 500u64; // Past timestamp (current is 1000)

        // Create recipient account
        account::create_account_for_test(RECIPIENT_ADDR);

        // Store requirements with expired timestamp
        store_requirements(
            copy intent_id,
            address_to_bytes32(RECIPIENT_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(),
            expiry,
        );

        // Should fail due to expiry
        outflow_validator_impl::fulfill_intent(solver, intent_id, token_metadata);
    }

    /// 9. Test: Fulfill intent rejects unauthorized solver
    /// Verifies that only the authorized solver can fulfill.
    /// Why: Ensures intent creator's solver preference is respected.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, solver = @0x456)]
    #[expected_failure(abort_code = 8, location = mvmt_intent::outflow_validator_impl)] // EUNAUTHORIZED_SOLVER
    fun test_fulfill_intent_rejects_unauthorized_solver(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        solver: &signer,
    ) {
        // Setup FA token (max supply = 100)
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to solver
        primary_fungible_store::transfer(token_creator, token_metadata, SOLVER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Create a different authorized solver address
        let authorized_solver_addr = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut authorized_solver_addr, 0xAA);
            i = i + 1;
        };

        // Create recipient account
        account::create_account_for_test(RECIPIENT_ADDR);

        // Store requirements with specific authorized solver (not our solver)
        store_requirements(
            copy intent_id,
            address_to_bytes32(RECIPIENT_ADDR),
            amount,
            token_addr,
            authorized_solver_addr, // Different solver required
            expiry,
        );

        // Should fail because solver is not authorized
        outflow_validator_impl::fulfill_intent(solver, intent_id, token_metadata);
    }

    /// 10. Test: Fulfill intent rejects token mismatch
    /// Verifies that wrong token mint is rejected.
    /// Why: Prevents solver from fulfilling with different token.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, solver = @0x456)]
    #[expected_failure(abort_code = 9, location = mvmt_intent::outflow_validator_impl)] // ETOKEN_MISMATCH
    fun test_fulfill_intent_rejects_token_mismatch(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        solver: &signer,
    ) {
        // Setup FA token (max supply = 100)
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to solver
        primary_fungible_store::transfer(token_creator, token_metadata, SOLVER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let amount = 50u64;
        let expiry = 2000u64;

        // Create a different token address (not matching our actual token)
        let wrong_token_addr = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut wrong_token_addr, 0xBB);
            i = i + 1;
        };

        // Create recipient account
        account::create_account_for_test(RECIPIENT_ADDR);

        // Store requirements expecting a different token
        store_requirements(
            copy intent_id,
            address_to_bytes32(RECIPIENT_ADDR),
            amount,
            wrong_token_addr, // Wrong token address
            create_zero_bytes32(),
            expiry,
        );

        // Should fail because token doesn't match
        outflow_validator_impl::fulfill_intent(solver, intent_id, token_metadata);
    }

    /// 11. Test: Fulfill intent rejects requirements not found
    /// Verifies that fulfilling unknown intent_id fails.
    /// Why: Prevents fulfillment of intents that were never created.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, solver = @0x456)]
    #[expected_failure(abort_code = 5, location = mvmt_intent::outflow_validator_impl)] // EREQUIREMENTS_NOT_FOUND
    fun test_fulfill_intent_rejects_requirements_not_found(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        solver: &signer,
    ) {
        // Setup FA token (max supply = 100)
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to solver
        primary_fungible_store::transfer(token_creator, token_metadata, SOLVER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        // Use an intent_id that was never created
        let unknown_intent_id = create_test_intent_id_2();

        // Should fail because requirements don't exist
        outflow_validator_impl::fulfill_intent(solver, unknown_intent_id, token_metadata);
    }

    /// 12. Test: Fulfill intent validates recipient
    /// Verifies that tokens go to the correct recipient as stored in requirements.
    /// Why: Ensures funds are delivered to the intended recipient.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, solver = @0x456)]
    fun test_fulfill_intent_rejects_recipient_mismatch(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        solver: &signer,
    ) {
        // Setup FA token (max supply = 100)
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to solver
        primary_fungible_store::transfer(token_creator, token_metadata, SOLVER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Create recipient account
        account::create_account_for_test(RECIPIENT_ADDR);

        // Store requirements with specific recipient
        store_requirements(
            copy intent_id,
            address_to_bytes32(RECIPIENT_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(),
            expiry,
        );

        // Fulfill should succeed and tokens go to RECIPIENT_ADDR
        outflow_validator_impl::fulfill_intent(solver, copy intent_id, token_metadata);

        // Verify recipient received tokens
        let recipient_balance = primary_fungible_store::balance(RECIPIENT_ADDR, token_metadata);
        assert!(recipient_balance == amount, 1);

        // Verify solver's balance decreased
        let solver_balance = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);
        assert!(solver_balance == 100 - amount, 2);
    }

    /// 13. Test: Fulfill intent succeeds with valid inputs
    /// Verifies the happy path: tokens transferred, state updated, GMP message sent.
    /// Why: Ensures the core fulfillment flow works end-to-end.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, solver = @0x456)]
    fun test_fulfill_intent_succeeds(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        solver: &signer,
    ) {
        // Setup FA token (max supply = 100)
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to solver
        primary_fungible_store::transfer(token_creator, token_metadata, SOLVER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Create recipient account
        account::create_account_for_test(RECIPIENT_ADDR);

        // Store requirements
        store_requirements(
            copy intent_id,
            address_to_bytes32(RECIPIENT_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(), // Any solver allowed
            expiry,
        );

        // Verify initial state
        assert!(outflow_validator_impl::has_requirements(copy intent_id), 1);
        assert!(!outflow_validator_impl::is_fulfilled(copy intent_id), 2);

        let initial_solver_balance = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);
        let initial_recipient_balance = primary_fungible_store::balance(RECIPIENT_ADDR, token_metadata);

        // Fulfill the intent
        outflow_validator_impl::fulfill_intent(solver, copy intent_id, token_metadata);

        // Verify fulfillment state
        assert!(outflow_validator_impl::is_fulfilled(copy intent_id), 3);

        // Verify token balances changed correctly
        let final_solver_balance = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);
        let final_recipient_balance = primary_fungible_store::balance(RECIPIENT_ADDR, token_metadata);

        assert!(final_solver_balance == initial_solver_balance - amount, 4);
        assert!(final_recipient_balance == initial_recipient_balance + amount, 5);

        // Verify GMP nonce was incremented (message was sent)
        let nonce = gmp_sender::get_next_nonce();
        assert!(nonce == 2, 6); // Should be 2 after one send (started at 1)
    }
}
