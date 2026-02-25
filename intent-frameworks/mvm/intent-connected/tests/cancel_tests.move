#[test_only]
module mvmt_intent::cancel_tests {
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

    /// Initialize all modules needed for cancel tests.
    fun init_modules(admin: &signer) {
        let hub_addr = create_test_hub_addr();
        intent_inflow_escrow::initialize(admin, HUB_CHAIN_ID, copy hub_addr);
        intent_gmp::initialize(admin);
        intent_gmp::add_relay(admin, ADMIN_ADDR);
        intent_gmp::set_remote_gmp_endpoint_addr(admin, HUB_CHAIN_ID, hub_addr);
        gmp_sender::initialize(admin);
    }

    /// Store requirements via the GMP inbound path.
    fun store_requirements(
        intent_id: vector<u8>,
        requester_addr: vector<u8>,
        amount: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    ) {
        let requirements = gmp_common::new_intent_requirements(
            intent_id,
            requester_addr,
            amount,
            token_addr,
            solver_addr,
            expiry,
        );
        let payload = gmp_common::encode_intent_requirements(&requirements);
        let hub_addr = create_test_hub_addr();
        intent_inflow_escrow::receive_intent_requirements(
            HUB_CHAIN_ID,
            hub_addr,
            payload,
        );
    }

    /// Create a FulfillmentProof GMP payload.
    fun create_test_fulfillment_proof_payload(
        intent_id: vector<u8>,
        solver_addr: vector<u8>,
        amount: u64,
        timestamp: u64,
    ): vector<u8> {
        let proof = gmp_common::new_fulfillment_proof(
            intent_id,
            solver_addr,
            amount,
            timestamp,
        );
        gmp_common::encode_fulfillment_proof(&proof)
    }

    // ============================================================================
    // CANCEL ESCROW TESTS
    // ============================================================================

    // 1. Test: Should revert if escrow has not expired yet
    // Verifies that admin cannot cancel escrows before the expiry timestamp.
    // Why: Funds must remain locked until expiry to give the solver time to fulfill.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 15, location = mvmt_intent::intent_inflow_escrow)] // E_ESCROW_NOT_EXPIRED
    fun test_cancel_rejects_before_expiry(
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

        // Admin tries to cancel before expiry (current time 1000, expiry 2000)
        intent_inflow_escrow::cancel_escrow(admin, intent_id);
    }

    // 2. Test: Should allow admin to cancel and return funds to requester after expiry
    // Verifies that admin can cancel escrows after expiry and funds return to requester.
    // Why: Admin needs a way to return funds if the solver never fulfills.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    fun test_cancel_after_expiry_returns_funds(
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

        // Verify requester balance dropped by escrow amount
        let balance_after_escrow = primary_fungible_store::balance(REQUESTER_ADDR, token_metadata);
        assert!(balance_after_escrow == 50, 1); // 100 - 50 = 50

        // Advance time past expiry
        timestamp::update_global_time_for_test_secs(2001);

        // Admin cancels escrow, funds return to requester
        intent_inflow_escrow::cancel_escrow(admin, copy intent_id);

        // Verify funds returned to requester (not admin)
        let balance_after_cancel = primary_fungible_store::balance(REQUESTER_ADDR, token_metadata);
        assert!(balance_after_cancel == 100, 2); // 50 + 50 = 100

        // Verify escrow marked as released
        assert!(intent_inflow_escrow::is_released(intent_id), 3);
    }

    // 3. Test: Should revert if caller is not admin
    // Verifies that only admin can cancel the escrow — requester cannot.
    // Why: Security requirement — only admin can cancel expired escrows.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 17, location = mvmt_intent::intent_inflow_escrow)] // E_UNAUTHORIZED_CALLER
    fun test_cancel_rejects_non_admin(
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

        // Advance time past expiry
        timestamp::update_global_time_for_test_secs(2001);

        // Requester tries to cancel — should fail (only admin can cancel)
        intent_inflow_escrow::cancel_escrow(requester, intent_id);
    }

    // 4. Test: Should revert if already claimed
    // Verifies that cancelling an already-fulfilled escrow is rejected.
    // Why: Once funds are released to the solver, they cannot also be returned to the requester.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 12, location = mvmt_intent::intent_inflow_escrow)] // E_ALREADY_FULFILLED
    fun test_cancel_rejects_already_fulfilled(
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

        // Create solver account (needed for auto-release transfer)
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
            address_to_bytes32(SOLVER_ADDR),
            expiry,
        );

        intent_inflow_escrow::create_escrow_with_validation(
            requester,
            copy intent_id,
            token_metadata,
            amount,
        );

        // Fulfill via GMP (auto-releases to solver)
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

        // Advance time past expiry
        timestamp::update_global_time_for_test_secs(2001);

        // Admin tries to cancel after fulfillment — should fail
        intent_inflow_escrow::cancel_escrow(admin, intent_id);
    }

    // 5. Test: Should revert if escrow does not exist
    // Verifies that cancelling a non-existent escrow is rejected.
    // Why: Prevents cancellation of non-existent escrows and ensures proper error handling.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent)]
    #[expected_failure(abort_code = 11, location = mvmt_intent::intent_inflow_escrow)] // E_ESCROW_NOT_FOUND
    fun test_cancel_rejects_nonexistent_escrow(
        aptos_framework: &signer,
        admin: &signer,
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(1000);

        // Initialize modules
        init_modules(admin);

        // Admin tries to cancel a non-existent escrow
        let fake_intent_id = create_test_intent_id();
        intent_inflow_escrow::cancel_escrow(admin, fake_intent_id);
    }

    // 6. Test: Should revert if already cancelled
    // Verifies that cancelling an already-cancelled escrow is rejected (double-cancel prevention).
    // Why: Prevents double-refund by ensuring released escrows cannot be cancelled again.
    #[test(aptos_framework = @0x1, admin = @mvmt_intent, token_creator = @0xABC, requester = @0x789)]
    #[expected_failure(abort_code = 12, location = mvmt_intent::intent_inflow_escrow)] // E_ALREADY_FULFILLED (released)
    fun test_cancel_rejects_already_cancelled(
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

        // Advance time past expiry
        timestamp::update_global_time_for_test_secs(2001);

        // First cancel succeeds
        intent_inflow_escrow::cancel_escrow(admin, copy intent_id);

        // Second cancel should fail — escrow already released
        intent_inflow_escrow::cancel_escrow(admin, intent_id);
    }
}
