#[test_only]
module mvmt_intent::inflow_escrow_gmp_tests {
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use aptos_framework::object::{Self};
    use aptos_framework::primary_fungible_store;
    use mvmt_intent::inflow_escrow_gmp;
    use mvmt_intent::native_gmp_endpoint;
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
        native_gmp_endpoint::initialize(admin);
        let trusted_hub_addr = create_test_hub_addr();
        inflow_escrow_gmp::initialize(admin, HUB_CHAIN_ID, trusted_hub_addr);
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

        let src_addr = create_test_hub_addr();

        inflow_escrow_gmp::receive_intent_requirements(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );
    }

    // ============================================================================
    // INITIALIZATION TESTS
    // ============================================================================

    // 1. Test: Initialize creates config
    // Verifies that initialize correctly sets up the inflow escrow GMP configuration with hub chain ID and trusted hub address.
    // Why: Proper initialization is required before any GMP operations can succeed. Without correct config, all cross-chain messages will be rejected.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    fun test_initialize_creates_config(aptos_framework: &signer, admin: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        init_modules(admin);

        // Verify initialization
        assert!(inflow_escrow_gmp::is_initialized(), 1);
        assert!(inflow_escrow_gmp::get_hub_chain_id() == HUB_CHAIN_ID, 2);

        let stored_hub_addr = inflow_escrow_gmp::get_trusted_hub_addr();
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
        let trusted_hub_addr = create_test_hub_addr();
        inflow_escrow_gmp::initialize(admin, HUB_CHAIN_ID, trusted_hub_addr);
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
        assert!(inflow_escrow_gmp::has_requirements(copy intent_id), 1);
        assert!(inflow_escrow_gmp::get_amount_required(intent_id) == amount, 2);
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

        let src_addr = create_test_hub_addr();

        // First receive
        inflow_escrow_gmp::receive_intent_requirements(
            HUB_CHAIN_ID,
            copy src_addr,
            copy payload,
        );

        // Second receive (should succeed without error - idempotent)
        inflow_escrow_gmp::receive_intent_requirements(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );

        // Verify still stored correctly
        assert!(inflow_escrow_gmp::has_requirements(intent_id), 1);
    }

    // 5. Test: Receive requirements rejects untrusted source
    // Verifies that requirements from non-trusted chain IDs or addresses are rejected.
    // Why: Source verification prevents spoofed messages from malicious actors who could inject fake requirements and steal escrowed funds.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    #[expected_failure(abort_code = 2, location = mvmt_intent::inflow_escrow_gmp)] // EINVALID_SOURCE_CHAIN
    fun test_receive_requirements_rejects_untrusted_source(aptos_framework: &signer, admin: &signer) {
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

        let src_addr = create_test_hub_addr();

        // Use wrong chain ID
        inflow_escrow_gmp::receive_intent_requirements(
            99999u32, // Wrong chain ID
            src_addr,
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

        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Verify not fulfilled yet
        assert!(!inflow_escrow_gmp::is_fulfilled(copy intent_id), 1);

        // Send fulfillment proof from hub
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let src_addr = create_test_hub_addr();

        inflow_escrow_gmp::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );

        // Verify now fulfilled
        assert!(inflow_escrow_gmp::is_fulfilled(intent_id), 2);
    }

    // 7. Test: Receive fulfillment proof rejects untrusted source
    // Verifies that fulfillment proofs from non-trusted chain IDs are rejected.
    // Why: Only the trusted hub can send fulfillment proofs. Accepting proofs from untrusted sources would allow attackers to steal escrowed funds.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 2, location = mvmt_intent::inflow_escrow_gmp)] // EINVALID_SOURCE_CHAIN
    fun test_receive_fulfillment_rejects_untrusted_source(
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

        inflow_escrow_gmp::create_escrow_with_validation(
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

        let src_addr = create_test_hub_addr();

        inflow_escrow_gmp::receive_fulfillment_proof(
            99999u32, // Wrong chain ID
            src_addr,
            payload,
        );
    }

    // 8. Test: Receive fulfillment proof rejects already fulfilled
    // Verifies that receiving a fulfillment proof twice is rejected.
    // Why: Prevents replay attacks and double-spending.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789, solver = @0x456)]
    #[expected_failure(abort_code = 12, location = mvmt_intent::inflow_escrow_gmp)] // EALREADY_FULFILLED
    fun test_receive_fulfillment_proof_rejects_already_fulfilled(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
        solver: &signer,
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
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // First fulfillment proof (should succeed and mark fulfilled)
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let src_addr = create_test_hub_addr();

        inflow_escrow_gmp::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            copy src_addr,
            copy payload,
        );

        // Release escrow (so solver gets tokens)
        inflow_escrow_gmp::release_escrow(
            solver,
            copy intent_id,
            token_metadata,
        );

        // Second fulfillment proof (should fail - already fulfilled)
        inflow_escrow_gmp::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            src_addr,
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
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Verify escrow created
        assert!(inflow_escrow_gmp::has_escrow(intent_id), 1);

        // Verify GMP message was sent (nonce incremented)
        let nonce = gmp_sender::get_next_nonce();
        assert!(nonce == 2, 2); // Started at 1, now 2
    }

    // 10. Test: Create escrow rejects amount mismatch
    // Verifies that creating an escrow with a different amount than required is rejected.
    // Why: Amount validation prevents users from creating under/over-funded escrows that don't match hub expectations.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 8, location = mvmt_intent::inflow_escrow_gmp)] // EAMOUNT_MISMATCH
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
        inflow_escrow_gmp::create_escrow_with_validation(
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
    #[expected_failure(abort_code = 9, location = mvmt_intent::inflow_escrow_gmp)] // ETOKEN_MISMATCH
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
        inflow_escrow_gmp::create_escrow_with_validation(
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
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Verify GMP message was sent by checking nonce was incremented
        let final_nonce = gmp_sender::get_next_nonce();
        assert!(final_nonce == 2, 2); // Nonce incremented after sending message

        // Verify escrow was created
        assert!(inflow_escrow_gmp::has_escrow(intent_id), 3);
    }

    // 13. Test: Full inflow GMP workflow
    // Verifies complete flow: requirements → escrow → fulfillment proof → release.
    // Why: Integration test for the entire inflow GMP flow.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789, solver = @0x456)]
    fun test_full_inflow_gmp_workflow(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
        solver: &signer,
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
        assert!(inflow_escrow_gmp::has_requirements(copy intent_id), 1);
        assert!(!inflow_escrow_gmp::has_escrow(copy intent_id), 2);

        // ========================================
        // Step 2: User creates escrow (locks tokens)
        // ========================================
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Verify: Escrow created, tokens locked
        assert!(inflow_escrow_gmp::has_escrow(copy intent_id), 3);
        let requester_balance = primary_fungible_store::balance(REQUESTER_ADDR, token_metadata);
        assert!(requester_balance == initial_requester_balance - amount, 4);

        // Verify: Not fulfilled yet
        assert!(!inflow_escrow_gmp::is_fulfilled(copy intent_id), 5);

        // ========================================
        // Step 3: Solver fulfills on hub, hub sends proof via GMP
        // ========================================
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );
        let src_addr = create_test_hub_addr();

        inflow_escrow_gmp::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );

        // Verify: Marked as fulfilled
        assert!(inflow_escrow_gmp::is_fulfilled(copy intent_id), 6);
        assert!(!inflow_escrow_gmp::is_released(copy intent_id), 7);

        // ========================================
        // Step 4: Solver releases escrow (MVM manual release)
        // ========================================
        inflow_escrow_gmp::release_escrow(
            solver,
            copy intent_id,
            token_metadata,
        );

        // Verify: Escrow released to solver
        assert!(inflow_escrow_gmp::is_released(intent_id), 8);
        let solver_balance = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);
        assert!(solver_balance == initial_solver_balance + amount, 9);
    }

    // 14. Test: Create escrow rejects no requirements (MVM-specific)
    // Verifies that creating an escrow without receiving requirements first is rejected.
    // Why: Requirements must exist before escrow creation to ensure the hub has coordinated this intent and validation is possible.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 5, location = mvmt_intent::inflow_escrow_gmp)] // EREQUIREMENTS_NOT_FOUND
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
        inflow_escrow_gmp::create_escrow_with_validation(
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
    #[expected_failure(abort_code = 6, location = mvmt_intent::inflow_escrow_gmp)] // EESCROW_ALREADY_CREATED
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
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Second escrow should fail (even if requester has more tokens)
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            intent_id,
            token_metadata,
            amount,
        );
    }

    // ============================================================================
    // RELEASE ESCROW TESTS (MVM-specific manual release)
    // ============================================================================

    // 16. Test: Release escrow succeeds after fulfillment (MVM-specific)
    // Verifies that the solver can successfully claim escrowed tokens after receiving a fulfillment proof from the hub.
    // Why: This is the final step in the inflow intent lifecycle. The solver must receive payment after fulfilling the intent on the hub.
    // Note: MVM requires manual release call. SVM auto-releases in test 6.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789, solver = @0x456)]
    fun test_release_escrow_succeeds_after_fulfillment(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
        solver: &signer,
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
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Send fulfillment proof from hub
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let src_addr = create_test_hub_addr();

        inflow_escrow_gmp::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );

        // Get solver balance before release
        let solver_balance_before = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);

        // Release escrow
        inflow_escrow_gmp::release_escrow(
            solver,
            copy intent_id,
            token_metadata,
        );

        // Verify solver received tokens
        let solver_balance_after = primary_fungible_store::balance(SOLVER_ADDR, token_metadata);
        assert!(solver_balance_after == solver_balance_before + amount, 1);

        // Verify escrow marked as released
        assert!(inflow_escrow_gmp::is_released(intent_id), 2);
    }

    // 17. Test: Release escrow rejects without fulfillment (MVM-specific)
    // Verifies that attempting to release an escrow before receiving a fulfillment proof is rejected.
    // Why: Tokens must not be released until the hub confirms the solver fulfilled the intent. Early release allows theft without fulfillment.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789, solver = @0x456)]
    #[expected_failure(abort_code = 13, location = mvmt_intent::inflow_escrow_gmp)] // ENOT_FULFILLED
    fun test_release_escrow_rejects_without_fulfillment(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
        solver: &signer,
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

        // Store requirements and create escrow
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            create_zero_bytes32(),
            expiry,
        );

        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Try to release without fulfillment proof - should fail
        inflow_escrow_gmp::release_escrow(
            solver,
            intent_id,
            token_metadata,
        );
    }

    // 18. Test: Release escrow rejects unauthorized solver (MVM-specific)
    // Verifies that only the solver specified in requirements can release the escrow.
    // Why: Prevents unauthorized actors from stealing escrowed funds intended for a specific solver.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789, unauthorized = @0xDEAD)]
    #[expected_failure(abort_code = 14, location = mvmt_intent::inflow_escrow_gmp)] // EUNAUTHORIZED_SOLVER
    fun test_release_escrow_rejects_unauthorized_solver(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
        unauthorized: &signer,
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

        // Create unauthorized account
        account::create_account_for_test(@0xDEAD);

        // Initialize modules
        init_modules(admin);

        let intent_id = create_test_intent_id();
        let token_addr = address_to_bytes32(object::object_address(&token_metadata));
        let amount = 50u64;
        let expiry = 2000u64;

        // Store requirements with specific solver (not @0xDEAD)
        store_requirements(
            copy intent_id,
            address_to_bytes32(REQUESTER_ADDR),
            amount,
            token_addr,
            address_to_bytes32(SOLVER_ADDR), // Specific solver required
            expiry,
        );

        // Create escrow
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Send fulfillment proof
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let src_addr = create_test_hub_addr();

        inflow_escrow_gmp::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );

        // Try to release with unauthorized solver - should fail
        inflow_escrow_gmp::release_escrow(
            unauthorized,
            intent_id,
            token_metadata,
        );
    }

    // 19. Test: Release escrow rejects double release (MVM-specific)
    // Verifies that attempting to release the same escrow twice is rejected.
    // Why: Prevents double-spending where the solver could claim the same escrowed tokens multiple times.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789, solver = @0x456)]
    #[expected_failure(abort_code = 15, location = mvmt_intent::inflow_escrow_gmp)] // EESCROW_ALREADY_RELEASED
    fun test_release_escrow_rejects_double_release(
        aptos_framework: &signer,
        admin: &signer,
        token_creator: &signer,
        requester: &signer,
        solver: &signer,
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
        inflow_escrow_gmp::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Send fulfillment proof
        let payload = create_test_fulfillment_proof_payload(
            copy intent_id,
            address_to_bytes32(SOLVER_ADDR),
            amount,
            1500u64,
        );

        let src_addr = create_test_hub_addr();

        inflow_escrow_gmp::receive_fulfillment_proof(
            HUB_CHAIN_ID,
            src_addr,
            payload,
        );

        // First release should succeed
        inflow_escrow_gmp::release_escrow(
            solver,
            copy intent_id,
            token_metadata,
        );

        // Second release should fail
        inflow_escrow_gmp::release_escrow(
            solver,
            intent_id,
            token_metadata,
        );
    }
}
