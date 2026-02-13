#[test_only]
module mvmt_intent::intent_inflow_escrow_tests {
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use aptos_framework::object::{Self};
    use aptos_framework::primary_fungible_store;
    use mvmt_intent::intent_inflow_escrow;
    use mvmt_intent::intent_gmp;
    use mvmt_intent::gmp_sender;
    use mvmt_intent::gmp_common;
    use mvmt_intent::test_utils;

    // Test addresses
    const ADMIN_ADDR: address = @mvmt_intent;
    const SOLVER_ADDR: address = @0x456;
    const REQUESTER_ADDR: address = @0x789;
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

    fun create_test_requirements_payload(
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

    fun create_test_fulfillment_proof_payload(
        intent_id: vector<u8>,
        solver_addr: vector<u8>,
        amount_fulfilled: u64,
        timestamp: u64,
    ): vector<u8> {
        let msg = gmp_common::new_fulfillment_proof(
            intent_id,
            solver_addr,
            amount_fulfilled,
            timestamp,
        );
        gmp_common::encode_fulfillment_proof(&msg)
    }

    /// Initialize GMP modules and inflow escrow.
    fun init_modules(admin: &signer) {
        gmp_sender::initialize(admin);
        intent_gmp::initialize(admin);
        let hub_gmp_endpoint_addr = create_test_hub_addr();
        intent_inflow_escrow::initialize(admin, HUB_CHAIN_ID, hub_gmp_endpoint_addr);
    }

    /// Store intent requirements for a given intent_id.
    fun store_requirements(
        intent_id: vector<u8>,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    ) {
        let payload = create_test_requirements_payload(
            intent_id,
            requester_addr,
            amount_required,
            token_addr,
            solver_addr,
            expiry,
        );

        let remote_gmp_endpoint_addr = create_test_hub_addr();

        intent_inflow_escrow::receive_intent_requirements(
            HUB_CHAIN_ID,
            remote_gmp_endpoint_addr,
            payload,
        );
    }

    // ============================================================================
    // INITIALIZATION TESTS
    // ============================================================================

    // 1. Test: Initialize creates config
    // Verifies that initialize correctly sets up the inflow escrow GMP configuration with hub chain ID and hub GMP endpoint address.
    // Why: Proper initialization is required before any GMP operations can succeed. Without correct config, all cross-chain messages will be rejected.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_initialize_creates_config(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        // Verify initialization
        assert!(intent_inflow_escrow::is_initialized(), 1);
        assert!(intent_inflow_escrow::get_hub_chain_id() == HUB_CHAIN_ID, 2);

        let stored_hub_addr = intent_inflow_escrow::get_hub_gmp_endpoint_addr();
        let expected_hub_addr = create_test_hub_addr();
        assert!(stored_hub_addr == expected_hub_addr, 3);
    }

    // 2. Test: Initialize rejects double initialization
    // Verifies that trying to initialize twice fails with an error.
    // Why: Config must only be set once to prevent admin takeover or config manipulation attacks.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    #[expected_failure] // Already exists
    fun test_initialize_rejects_double_init(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        // Second init should fail
        let hub_gmp_endpoint_addr = create_test_hub_addr();
        intent_inflow_escrow::initialize(admin, HUB_CHAIN_ID, hub_gmp_endpoint_addr);
    }

    // ============================================================================
    // RECEIVE INTENT REQUIREMENTS TESTS
    // ============================================================================

    // 3. Test: Receive requirements stores requirements
    // Verifies that receiving intent requirements from the hub correctly stores them for later validation during escrow creation.
    // Why: Requirements must be stored before escrow creation so that amount, token, and solver can be validated against hub expectations.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_receive_requirements_stores_requirements(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        let intent_id = create_test_intent_id();
        let requester_addr = address_to_bytes32(REQUESTER_ADDR);
        let token_addr = create_zero_bytes32();
        let solver_addr = address_to_bytes32(SOLVER_ADDR);
        let amount = 1000000u64;
        let expiry = 2000u64;

        // Receive requirements
        store_requirements(
            copy intent_id,
            requester_addr,
            amount,
            token_addr,
            solver_addr,
            expiry,
        );

        // Verify requirements stored
        assert!(intent_inflow_escrow::has_requirements(copy intent_id), 1);
        assert!(intent_inflow_escrow::get_amount_required(intent_id) == amount, 2);
    }

    // 4. Test: Receive requirements is idempotent
    // Verifies that receiving the same requirements message twice doesn't fail or overwrite existing data.
    // Why: GMP messages may be delivered multiple times due to retries. Idempotency prevents duplicate message errors and ensures reliable delivery.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_receive_requirements_idempotent(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        let intent_id = create_test_intent_id();
        let requester_addr = address_to_bytes32(REQUESTER_ADDR);
        let token_addr = create_zero_bytes32();
        let solver_addr = address_to_bytes32(SOLVER_ADDR);
        let amount = 1000000u64;
        let expiry = 2000u64;

        let payload = create_test_requirements_payload(
            copy intent_id,
            requester_addr,
            amount,
            token_addr,
            solver_addr,
            expiry,
        );

        let remote_gmp_endpoint_addr = create_test_hub_addr();

        // First receive
        intent_inflow_escrow::receive_intent_requirements(
            HUB_CHAIN_ID,
            copy remote_gmp_endpoint_addr,
            copy payload,
        );

        // Second receive (should succeed without error - idempotent)
        intent_inflow_escrow::receive_intent_requirements(
            HUB_CHAIN_ID,
            remote_gmp_endpoint_addr,
            payload,
        );

        // Verify still stored correctly
        assert!(intent_inflow_escrow::has_requirements(intent_id), 1);
    }

    // 5. Test: Receive requirements rejects unknown source
    // Verifies that requirements from non-hub chain IDs or addresses are rejected.
    // Why: Source verification prevents spoofed messages from malicious actors who could inject fake requirements and steal escrowed funds.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    #[expected_failure(abort_code = 2, location = mvmt_intent::intent_inflow_escrow)] // EINVALID_SOURCE_CHAIN
    fun test_receive_requirements_rejects_unknown_source(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        let intent_id = create_test_intent_id();
        let requester_addr = address_to_bytes32(REQUESTER_ADDR);
        let token_addr = create_zero_bytes32();
        let solver_addr = address_to_bytes32(SOLVER_ADDR);
        let amount = 1000000u64;
        let expiry = 2000u64;

        let payload = create_test_requirements_payload(
            intent_id,
            requester_addr,
            amount,
            token_addr,
            solver_addr,
            expiry,
        );

        let remote_gmp_endpoint_addr = create_test_hub_addr();

        // Use wrong chain ID
        intent_inflow_escrow::receive_intent_requirements(
            99999u32, // Wrong chain ID
            remote_gmp_endpoint_addr,
            payload,
        );
    }

    // ============================================================================
    // RECEIVE FULFILLMENT PROOF TESTS
    // ============================================================================

    // 6. Test: Receive fulfillment proof marks fulfilled (MVM: manual release)
    // Verifies that receiving a fulfillment proof from the hub marks the escrow as fulfilled and ready for release.
    // Why: Fulfillment proof from hub confirms the solver delivered tokens on the hub side, allowing safe release of escrowed tokens.
    // Note: MVM uses manual release (see tests 16-19). SVM auto-releases in this test.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    fun test_receive_fulfillment_proof_marks_fulfilled(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Store requirements and create escrow
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Verify not fulfilled yet
        assert!(!intent_inflow_escrow::is_fulfilled(copy intent_id), 1);

        // Send fulfillment proof from hub
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let remote_gmp_endpoint_addr = create_test_hub_addr();

        intent_inflow_escrow::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            remote_gmp_endpoint_addr,
            payload,
        );

        // Verify now fulfilled
        assert!(intent_inflow_escrow::is_fulfilled(intent_id), 2);
    }

    // 7. Test: Receive fulfillment proof rejects unknown source
    // Verifies that fulfillment proofs from non-hub chain IDs are rejected.
    // Why: Only the hub can send fulfillment proofs. Accepting proofs from unknown sources would allow attackers to steal escrowed funds.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 2, location = mvmt_intent::intent_inflow_escrow)] // EINVALID_SOURCE_CHAIN
    fun test_receive_fulfillment_rejects_unknown_source(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Store requirements and create escrow
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Try to send fulfillment proof with wrong chain ID
        let payload = create_test_fulfillment_proof_payload(
            intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let remote_gmp_endpoint_addr = create_test_hub_addr();

        intent_inflow_escrow::receive_fulfillment_proof(
            99999u32, // Wrong chain ID
            remote_gmp_endpoint_addr,
            payload,
        );
    }

    // 8. Test: Receive fulfillment proof rejects already fulfilled
    // Verifies that receiving a fulfillment proof twice is rejected.
    // Why: Prevents replay attacks and double-spending.
    // Note: With auto-release, the first proof also releases, so no manual release_escrow call needed.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 12, location = mvmt_intent::intent_inflow_escrow)] // E_ALREADY_FULFILLED
    fun test_receive_fulfillment_proof_rejects_already_fulfilled(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Create solver account
        account::create_account_for_test(SOLVER_ADDR);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Store requirements
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(), // Any solver allowed
            expiry,
        );

        // Create escrow
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // First fulfillment proof (should succeed, auto-release to solver)
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let remote_gmp_endpoint_addr = create_test_hub_addr();

        intent_inflow_escrow::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            copy remote_gmp_endpoint_addr,
            copy payload,
        );

        // Second fulfillment proof (should fail - already fulfilled)
        intent_inflow_escrow::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            remote_gmp_endpoint_addr,
            payload,
        );
    }

    // ============================================================================
    // CREATE ESCROW TESTS
    // ============================================================================

    // 9. Test: Create escrow validates against requirements
    // Verifies that creating an escrow validates amount and token against previously received requirements from the hub.
    // Why: Validation ensures the escrow matches hub expectations, and sends confirmation back to hub for coordination.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    fun test_create_escrow_validates_requirements(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token via test_utils
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Store requirements
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        // Create escrow
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Verify escrow created
        assert!(intent_inflow_escrow::has_escrow(intent_id), 1);

        // Verify GMP message was sent (nonce incremented)
        let nonce = gmp_sender::get_next_nonce();
        assert!(nonce == 2, 2); // Started at 1, now 2
    }

    // 10. Test: Create escrow rejects amount mismatch
    // Verifies that creating an escrow with a different amount than required is rejected.
    // Why: Amount validation prevents users from creating under/over-funded escrows that don't match hub expectations.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 8, location = mvmt_intent::intent_inflow_escrow)] // EAMOUNT_MISMATCH
    fun test_create_escrow_rejects_amount_mismatch(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let required_amount = 50u64;
        let expiry = 2000u64;

        // Store requirements with 50 tokens required
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            required_amount,
            token_addr,
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        // Try to create escrow with wrong amount (30 instead of 50)
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            intent_id,
            token_metadata,
            30u64, // Wrong amount
        );
    }

    // 11. Test: Create escrow rejects token mismatch
    // Verifies that creating an escrow with a different token than required is rejected.
    // Why: Token validation prevents users from locking wrong tokens that can't be used to fulfill the intent on the hub.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 9, location = mvmt_intent::intent_inflow_escrow)] // ETOKEN_MISMATCH
    fun test_create_escrow_rejects_token_mismatch(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

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

        // Store requirements expecting a different token
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            wrong_token_addr,
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        // Try to create escrow with different token
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            intent_id,
            token_metadata,
            amount,
        );
    }

    // 12. Test: Create escrow sends EscrowConfirmation
    // Verifies that EscrowConfirmation GMP message is sent to hub on escrow creation.
    // Why: Hub needs confirmation to proceed with intent processing.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    fun test_create_escrow_sends_escrow_confirmation(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Verify initial nonce
        let initial_nonce = gmp_sender::get_next_nonce();
        assert!(initial_nonce == 1, 1); // Nonce starts at 1

        // Store requirements
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        // Create escrow - this should send EscrowConfirmation via GMP
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Verify GMP message was sent by checking nonce was incremented
        let final_nonce = gmp_sender::get_next_nonce();
        assert!(final_nonce == 2, 2); // Nonce incremented after sending message

        // Verify escrow was created
        assert!(intent_inflow_escrow::has_escrow(intent_id), 3);
    }

    // 13. Test: Full inflow GMP workflow (with auto-release)
    // Verifies complete flow: requirements → escrow → fulfillment proof (auto-release).
    // Why: Integration test for the entire inflow GMP flow.
    // Note: With auto-release, step 4 (manual release_escrow) is no longer needed.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    fun test_full_inflow_gmp_workflow(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Create solver account
        account::create_account_for_test(SOLVER_ADDR);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Record initial balances
        let initial_requester_balance = primary_fungible_store::balance(REQUESTER_ADDR, token_metadata);
        let initial_solver_balance = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);

        // ========================================
        // Step 1: Hub sends requirements via GMP
        // ========================================
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(), // Any solver allowed
            expiry,
        );

        // Verify: Requirements stored
        assert!(intent_inflow_escrow::has_requirements(copy intent_id), 1);
        assert!(!intent_inflow_escrow::has_escrow(copy intent_id), 2);

        // ========================================
        // Step 2: User creates escrow (locks tokens)
        // ========================================
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Verify: Escrow created, tokens locked
        assert!(intent_inflow_escrow::has_escrow(copy intent_id), 3);
        let requester_balance = primary_fungible_store::balance(REQUESTER_ADDR, token_metadata);
        assert!(requester_balance == initial_requester_balance - amount, 4);

        // Verify: Not fulfilled yet
        assert!(!intent_inflow_escrow::is_fulfilled(copy intent_id), 5);

        // ========================================
        // Step 3: Solver fulfills on hub, hub sends proof via GMP
        // (Auto-release happens here - tokens transferred to solver)
        // ========================================
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );
        let remote_gmp_endpoint_addr = create_test_hub_addr();

        intent_inflow_escrow::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            remote_gmp_endpoint_addr,
            payload,
        );

        // Verify: Marked as fulfilled AND released (auto-release)
        assert!(intent_inflow_escrow::is_fulfilled(copy intent_id), 6);
        assert!(intent_inflow_escrow::is_released(intent_id), 7);

        // Verify: Solver received tokens (auto-released, no manual release_escrow needed)
        let solver_balance = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);
        assert!(solver_balance == initial_solver_balance + amount, 8);
    }

    // 14. Test: Create escrow rejects no requirements (MVM-specific)
    // Verifies that creating an escrow without receiving requirements first is rejected.
    // Why: Requirements must exist before escrow creation to ensure the hub has coordinated this intent and validation is possible.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 5, location = mvmt_intent::intent_inflow_escrow)] // EREQUIREMENTS_NOT_FOUND
    fun test_create_escrow_rejects_no_requirements(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let amount = 50u64;

        // Don't store requirements - try to create escrow directly
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            intent_id,
            token_metadata,
            amount,
        );
    }

    // 15. Test: Create escrow rejects double creation (MVM-specific)
    // Verifies that creating an escrow twice for the same intent_id is rejected.
    // Why: One intent should have exactly one escrow. Double creation could lead to double-spending or fund locking issues.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 6, location = mvmt_intent::intent_inflow_escrow)] // EESCROW_ALREADY_CREATED
    fun test_create_escrow_rejects_double_create(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token (max supply = 100)
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 30u64; // Use smaller amount to allow two attempts within 100 max
        let expiry = 2000u64;

        // Store requirements
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        // First escrow should succeed
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Second escrow should fail (even if requester has more tokens)
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            intent_id,
            token_metadata,
            amount,
        );
    }

    // ============================================================================
    // AUTO-RELEASE ESCROW TESTS (single-step release on FulfillmentProof)
    // ============================================================================

    // 16. Test: Auto-release on fulfillment proof receipt
    // Verifies that receive_fulfillment_proof automatically transfers escrowed tokens to the solver.
    // Why: This is the final step in the inflow intent lifecycle. Auto-release eliminates the need for a separate release call.
    // Note: MVM now auto-releases like SVM (see test 6). No manual release_escrow call needed.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    fun test_auto_release_on_fulfillment_proof(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Create solver account
        account::create_account_for_test(SOLVER_ADDR);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Store requirements (zero solver = any solver allowed)
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(), // Any solver allowed
            expiry,
        );

        // Create escrow
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Get solver balance before fulfillment proof
        let solver_balance_before = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);

        // Send fulfillment proof from hub - this should auto-release to solver
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let remote_gmp_endpoint_addr = create_test_hub_addr();

        intent_inflow_escrow::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            remote_gmp_endpoint_addr,
            payload,
        );

        // Verify solver received tokens (auto-released, no manual release_escrow call needed)
        let solver_balance_after = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);
        assert!(solver_balance_after == solver_balance_before + amount, 1);

        // Verify escrow marked as released
        assert!(intent_inflow_escrow::is_released(intent_id), 2);

        // Verify escrow also marked as fulfilled
        assert!(intent_inflow_escrow::is_fulfilled(intent_id), 3);
    }

    // 17. test_release_escrow_rejects_without_fulfillment - N/A
    //     Why: MVM now auto-releases tokens on fulfillment proof receipt (see test 16).
    //     There is no separate release_escrow call, so there is no way to call release
    //     without fulfillment. The fulfillment proof IS the release trigger.

    // 18. test_release_escrow_rejects_unauthorized_solver - N/A
    //     Why: MVM now auto-releases tokens on fulfillment proof receipt (see test 16).
    //     Solver validation happens at proof receipt time - the solver address comes from
    //     the hub's signed GMP message, which is inherently authorized.

    // 19. Test: Duplicate fulfillment proof is rejected (double release prevention)
    // Verifies that receiving the same FulfillmentProof twice fails.
    // Why: Prevents replay attacks where a duplicate GMP message could cause double-release.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 12, location = mvmt_intent::intent_inflow_escrow)] // E_ALREADY_FULFILLED
    fun test_duplicate_fulfillment_proof_rejected(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Create solver account
        account::create_account_for_test(SOLVER_ADDR);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Store requirements
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(),
            expiry,
        );

        // Create escrow
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // First fulfillment proof - should succeed and auto-release
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let remote_gmp_endpoint_addr = create_test_hub_addr();

        intent_inflow_escrow::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            copy remote_gmp_endpoint_addr,
            copy payload,
        );

        // Second fulfillment proof (duplicate) - should fail with E_ALREADY_FULFILLED
        intent_inflow_escrow::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            remote_gmp_endpoint_addr,
            payload,
        );
    }

    // ============================================================================
    // SVM-SPECIFIC TESTS (N/A for MVM)
    // ============================================================================
    //
    // 20. test_generic_gmp_receive_routes_requirements - N/A
    //     Why: SVM tests generic GmpReceive instruction (variant index 1) routing based
    //     on message type. MVM receives messages through direct function calls to
    //     receive_intent_requirements/receive_fulfillment_proof, not via a generic
    //     GmpReceive dispatcher. Routing is handled by the integrated GMP endpoint calling
    //     the appropriate module function directly.
    //
    // 21. test_generic_gmp_receive_routes_fulfillment_proof - N/A
    //     Why: Same as test 20. MVM's integrated GMP endpoint routes messages directly to
    //     the appropriate handler functions rather than through a generic dispatcher
    //     instruction that peeks at message type and routes accordingly.
    //
    // 22. test_generic_gmp_receive_rejects_unknown_message_type - N/A
    //     Why: Same as tests 20-21. MVM doesn't have a generic GmpReceive instruction
    //     that needs to reject unknown message types - each message type is handled
    //     by its own entry function in the destination module.

    // ============================================================================
    // ESCROW CONFIRMATION WIRE FORMAT TESTS
    // ============================================================================

    // 18. Test: EscrowConfirmation payload produced by create_escrow_with_validation
    // decodes without error on the hub side.
    // Verifies the full encode → outbox → decode round-trip for EscrowConfirmation.
    // Why: The escrow_id and all other fields must be exactly 32 bytes so the
    // gmp_common wire format remains fixed-width. A mismatch causes E_INVALID_LENGTH
    // on the receiving chain, silently breaking the cross-chain flow.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    fun test_escrow_confirmation_payload_decodes_correctly(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
    ) {
        // Setup FA token
        let (token_metadata, _mint_ref) = test_utils::register_and_mint_tokens(
            aptos_framework,
            token_creator,
            100,
        );
        timestamp::update_global_time_for_test_secs(1000);

        // Transfer tokens to requester
        primary_fungible_store::transfer(token_creator, token_metadata, REQUESTER_ADDR, 100);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Store requirements
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        // Create escrow — this writes EscrowConfirmation to the outbox
        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Read the payload from the outbox (nonce 1)
        let (_dst_chain_id, _dst_addr, payload, _sender) = gmp_sender::get_message(1);

        // This MUST NOT abort. If escrow_id were not 32 bytes the decode would
        // fail with E_INVALID_LENGTH, catching the bug at test time.
        let confirmation = gmp_common::decode_escrow_confirmation(&payload);

        // Verify the decoded fields match what was escrowed
        assert!(*gmp_common::escrow_confirmation_intent_id(&confirmation) == intent_id, 1);
        assert!(gmp_common::escrow_confirmation_amount_escrowed(&confirmation) == amount, 2);
        assert!(vector::length(gmp_common::escrow_confirmation_escrow_id(&confirmation)) == 32, 3);
    }
}
