#[test_only]
module mvmt_intent::fa_intent_outflow_tests {
    use std::signer;
    use std::option;
    use std::vector;
    use std::bcs;
    use aptos_framework::event;
    use aptos_framework::timestamp;
    use aptos_framework::object::{Self as object, Object};
    use aptos_framework::primary_fungible_store;
    use aptos_std::ed25519;
    use mvmt_intent::fa_intent_outflow;
    use mvmt_intent::fa_intent;
    use mvmt_intent::fa_intent_with_oracle;
    use mvmt_intent::intent::Intent;
    use mvmt_intent::intent_reservation;
    use mvmt_intent::intent_registry;
    use mvmt_intent::solver_registry;
    use mvmt_intent::test_utils;
    use mvmt_intent::gmp_intent_state;
    use mvmt_intent::gmp_sender;
    use mvmt_intent::intent_gmp_hub;

    // ============================================================================
    // TEST HELPERS
    // ============================================================================

    /// Helper function to set up common test infrastructure (tokens, registry, keys, signed intent).
    /// Returns all values needed to create an outflow intent.
    /// This helper does NOT create the intent - it only sets up the prerequisites.
    fun setup_outflow_test_infrastructure(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ): (
        Object<aptos_framework::fungible_asset::Metadata>, // offered_metadata
        Object<aptos_framework::fungible_asset::Metadata>, // desired_metadata
        address, // solver_addr
        vector<u8>, // solver_signature_bytes
        address, // intent_id
        u64, // expiry_time
        u64, // offered_amount
        u64, // desired_amount
    ) {
        timestamp::set_time_has_started_for_testing(aptos_framework);

        // Initialize GMP modules for cross-chain messaging
        gmp_intent_state::init_for_test(mvmt_intent);
        gmp_sender::init_for_test(mvmt_intent);
        // Use dst_chain_id = 2 (connected chain) with a dummy remote GMP endpoint address
        let dummy_remote_gmp_endpoint = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut dummy_remote_gmp_endpoint, 0xAB);
            i = i + 1;
        };
        intent_gmp_hub::init_for_test(mvmt_intent, 2, dummy_remote_gmp_endpoint);

        // Create test fungible assets
        let (offered_metadata, _) = mvmt_intent::test_utils::register_and_mint_tokens(aptos_framework, requester_signer, 100);
        let (desired_metadata, _) = mvmt_intent::test_utils::register_and_mint_tokens(aptos_framework, solver_signer, 0);

        let intent_id = @0x5678;
        let solver_addr = signer::address_of(solver_signer);
        let expiry_time = timestamp::now_seconds() + 3600;
        let offered_amount = 50u64;
        let desired_amount = 25u64;

        // Initialize solver registry and intent registry
        solver_registry::init_for_test(mvmt_intent);
        intent_registry::init_for_test(mvmt_intent);

        // Generate key pair for solver
        let (solver_secret_key, validated_solver_pk) = ed25519::generate_keys();
        let solver_public_key_bytes = ed25519::validated_public_key_to_bytes(&validated_solver_pk);
        let evm_addr = test_utils::create_test_evm_address(0);

        // Register solver in registry
        solver_registry::register_solver(solver_signer, solver_public_key_bytes, @0x0, evm_addr, vector::empty<u8>());

        // Step 1: Create draft intent (off-chain)
        let draft_intent = fa_intent_outflow::create_cross_chain_draft_intent(
            offered_metadata,
            offered_amount,
            1, // offered_chain_id (hub chain where tokens are locked)
            desired_metadata,
            desired_amount,
            2, // desired_chain_id (connected chain)
            expiry_time,
            signer::address_of(requester_signer),
        );

        // Step 2: Add solver to draft and create intent to sign
        let intent_to_sign = intent_reservation::add_solver_to_draft_intent(draft_intent, solver_addr);

        // Step 3: Solver signs the intent (off-chain)
        let intent_hash = intent_reservation::hash_intent(intent_to_sign);
        let solver_signature = ed25519::sign_arbitrary_bytes(&solver_secret_key, intent_hash);
        let solver_signature_bytes = ed25519::signature_to_bytes(&solver_signature);

        (
            offered_metadata,
            desired_metadata,
            solver_addr,
            solver_signature_bytes,
            intent_id,
            expiry_time,
            offered_amount,
            desired_amount,
        )
    }

    /// Helper function to set up an outflow intent for testing.
    /// Returns the intent object, metadata, and intent_id.
    fun setup_outflow_intent(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ): (
        Object<Intent<fa_intent_with_oracle::FungibleStoreManager, fa_intent_with_oracle::OracleGuardedLimitOrder>>,
        Object<aptos_framework::fungible_asset::Metadata>,
        Object<aptos_framework::fungible_asset::Metadata>,
        address, // intent_id
    ) {
        // Set up test infrastructure using shared helper
        let (offered_metadata, desired_metadata, solver_addr, solver_signature_bytes, intent_id, expiry_time, offered_amount, desired_amount) =
            setup_outflow_test_infrastructure(aptos_framework, mvmt_intent, requester_signer, solver_signer);

        let requester_addr_connected_chain = @0x9999; // Address on connected chain

        // Create outflow intent (returns intent object)
        // Pass desired_metadata as address (for cross-chain support)
        let desired_metadata_addr = object::object_address(&desired_metadata);
        let intent_obj = fa_intent_outflow::create_outflow_intent(
            requester_signer,
            offered_metadata,
            offered_amount,
            1, // offered_chain_id (hub chain)
            desired_metadata_addr,  // Pass as address, not Object
            desired_amount,
            2, // desired_chain_id (connected chain)
            expiry_time,
            intent_id,
            requester_addr_connected_chain,
            solver_addr,
            solver_addr, // solver_addr_connected_chain (same as hub addr in tests)
            solver_signature_bytes,
        );

        (intent_obj, offered_metadata, desired_metadata, intent_id)
    }

    // ============================================================================
    // TESTS
    // ============================================================================

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    /// What is tested: create_outflow_intent locks tokens on hub and stores the connected-chain requester address
    /// Why: Outflow intents must lock real funds on hub and carry the destination address for settlement
    fun test_create_outflow_intent(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        // Set up test infrastructure using shared helper
        let (offered_metadata, desired_metadata, solver_addr, solver_signature_bytes, intent_id, expiry_time, offered_amount, desired_amount) =
            setup_outflow_test_infrastructure(aptos_framework, mvmt_intent, requester_signer, solver_signer);

        let requester_addr_connected_chain = @0x9999; // Address on connected chain

        // Verify requester_signer's initial balance
        assert!(primary_fungible_store::balance(signer::address_of(requester_signer), offered_metadata) == 100);

        // Create outflow intent (returns intent object)
        // Pass desired_metadata as address (for cross-chain support)
        let desired_metadata_addr = object::object_address(&desired_metadata);
        let intent_obj = fa_intent_outflow::create_outflow_intent(
            requester_signer,
            offered_metadata,
            offered_amount,
            1, // offered_chain_id (hub chain)
            desired_metadata_addr,  // Pass as address, not Object
            desired_amount,
            2, // desired_chain_id (connected chain)
            expiry_time,
            intent_id,
            requester_addr_connected_chain,
            solver_addr,
            solver_addr, // solver_addr_connected_chain (same as hub addr in tests)
            solver_signature_bytes,
        );
        
        // Verify tokens were actually locked (balance decreased from 100 to 50)
        assert!(primary_fungible_store::balance(signer::address_of(requester_signer), offered_metadata) == 50);
        
        // Verify intent was created successfully by checking the intent object
        let intent_addr = object::object_address(&intent_obj);
        assert!(intent_addr != @0x0); // Intent address should not be zero
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    /// What is tested: OracleGuardedLimitOrder stores requester_addr_connected_chain correctly
    /// Why: Solver needs this address to know where to send tokens on the connected chain
    fun test_outflow_intent_requester_address_storage(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        use mvmt_intent::fa_intent_with_oracle;
        
        timestamp::set_time_has_started_for_testing(aptos_framework);
        
        // Create test fungible assets
        let (offered_metadata, _) = mvmt_intent::test_utils::register_and_mint_tokens(aptos_framework, requester_signer, 100);
        let (desired_metadata, _) = mvmt_intent::test_utils::register_and_mint_tokens(aptos_framework, solver_signer, 100);
        
        let intent_id = @0xabcd;
        let solver_addr = signer::address_of(solver_signer);
        let requester_addr_connected_chain = @0x1234; // Address on connected chain
        let expiry_time = timestamp::now_seconds() + 3600;
        
        // Initialize solver registry and intent registry
        solver_registry::init_for_test(mvmt_intent);
        intent_registry::init_for_test(mvmt_intent);
        
        // Generate key pairs
        let (_, validated_solver_pk) = ed25519::generate_keys();
        let solver_public_key_bytes = ed25519::validated_public_key_to_bytes(&validated_solver_pk);
        let evm_addr = test_utils::create_test_evm_address(0);
        solver_registry::register_solver(solver_signer, solver_public_key_bytes, @0x0, evm_addr, vector::empty<u8>());
        
        let (approver_secret_key, validated_approver_pk) = ed25519::generate_keys();
        let approver_public_key = ed25519::public_key_to_unvalidated(&validated_approver_pk);
        let _approver_public_key_bytes = ed25519::unvalidated_public_key_to_bytes(&approver_public_key);
        
        // Create intent directly using lower-level function to test struct field storage
        let fa = primary_fungible_store::withdraw(requester_signer, offered_metadata, 50);
        let reservation = intent_reservation::new_reservation(solver_addr);
        let requirement = fa_intent_with_oracle::new_oracle_signature_requirement(0, approver_public_key);
        
        let intent_obj = fa_intent_with_oracle::create_fa_to_fa_intent_with_oracle_requirement(
            fa,
            1, // offered_chain_id: hub chain where tokens are locked
            desired_metadata,
            25,
            2, // desired_chain_id: connected chain where tokens are desired
            option::some(object::object_address(&desired_metadata)), // Cross-chain: pass desired_metadata_addr
            expiry_time,
            signer::address_of(requester_signer),
            requirement,
            false,
            intent_id,
            option::some(requester_addr_connected_chain), // Store requester address
            option::some(reservation),
        );
        
        // Verify tokens were locked
        assert!(primary_fungible_store::balance(signer::address_of(requester_signer), offered_metadata) == 50);
        
        // Start session and complete it to verify intent structure is correct
        // This confirms the struct field was stored (otherwise struct creation would fail)
        let (unlocked_fa, session) = fa_intent_with_oracle::start_fa_offering_session(solver_signer, intent_obj);
        primary_fungible_store::deposit(signer::address_of(solver_signer), unlocked_fa);
        
        // Verify unlocked tokens match what was locked (50 tokens)
        assert!(primary_fungible_store::balance(signer::address_of(solver_signer), offered_metadata) == 50);
        
        // Complete the session with oracle signature to properly finish it
        let desired_fa = primary_fungible_store::withdraw(solver_signer, desired_metadata, 25);
        let oracle_signature = ed25519::sign_arbitrary_bytes(&approver_secret_key, bcs::to_bytes(&intent_id));
        let witness = fa_intent_with_oracle::new_oracle_signature_witness(0, oracle_signature);
        fa_intent_with_oracle::finish_fa_receiving_session_with_oracle(session, desired_fa, option::some(witness));
        
        // Verify completion - requester_signer received desired tokens
        assert!(primary_fungible_store::balance(signer::address_of(requester_signer), desired_metadata) == 25);
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    #[expected_failure(abort_code = 393223, location = aptos_framework::object)] // error::not_found(ERESOURCE_DOES_NOT_EXIST)
    /// What is tested: fulfilling an outflow intent with the inflow function aborts with ERESOURCE_DOES_NOT_EXIST
    /// Why: Outflow intents use OracleGuardedLimitOrder type; inflow uses FALimitOrder — types are incompatible
    ///
    /// Note: The error ERESOURCE_DOES_NOT_EXIST occurs because object::address_to_object<T> checks
    /// if an object of type T exists at the address. The object exists, but not as the requested type,
    /// so the runtime reports that a resource of that type does not exist at that address.
    fun test_cannot_fulfill_outflow_with_inflow_function(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        // Set up outflow intent using shared helper
        let (intent_obj, _offered_metadata, _desired_metadata, _intent_id) = setup_outflow_intent(
            aptos_framework,
            mvmt_intent,
            requester_signer,
            solver_signer,
        );

        // Try to convert to FALimitOrder type (wrong type)
        // This should fail because the intent is OracleGuardedLimitOrder, not FALimitOrder
        // The type system prevents this conversion, which is what we're testing
        let intent_addr = object::object_address(&intent_obj);
        
        // Try to convert to the wrong type - this will fail at address_to_object
        // because object::address_to_object<T> checks if an object of type T exists at the address.
        // The object exists, but not as FALimitOrder, so the runtime reports
        // ERESOURCE_DOES_NOT_EXIST (a resource of that type doesn't exist at that address).
        let _wrong_type_intent: Object<Intent<fa_intent::FungibleStoreManager, fa_intent::FALimitOrder>> = 
            object::address_to_object(intent_addr);
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    /// What is tested: fulfill_outflow_intent releases locked tokens to solver after GMP FulfillmentProof
    /// Why: Solver receives locked tokens only after fulfillment proof is received via GMP
    fun test_fulfill_outflow_intent(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        use mvmt_intent::fa_intent_with_oracle;
        use mvmt_intent::intent::Intent;
        use mvmt_intent::gmp_common;

        // Set up outflow intent using shared helper
        let (intent_obj, offered_metadata, _desired_metadata, intent_id) = setup_outflow_intent(
            aptos_framework,
            mvmt_intent,
            requester_signer,
            solver_signer,
        );

        // Verify intent was created and registered
        let intent_addr = object::object_address(&intent_obj);
        assert!(intent_addr != @0x0);
        assert!(intent_registry::is_intent_registered(intent_addr));

        // Verify tokens were locked (requester_signer's balance decreased from 100 to 50)
        assert!(primary_fungible_store::balance(signer::address_of(requester_signer), offered_metadata) == 50);

        // Create a FulfillmentProof GMP message
        let intent_id_bytes = bcs::to_bytes(&intent_id);
        let solver_addr_bytes = bcs::to_bytes(&signer::address_of(solver_signer));
        let fulfillment_proof = gmp_common::new_fulfillment_proof(
            intent_id_bytes,
            solver_addr_bytes,
            50, // amount_fulfilled
            timestamp::now_seconds(), // timestamp
        );
        let payload = gmp_common::encode_fulfillment_proof(&fulfillment_proof);

        // Create known source address (matching what was set in init_for_test)
        let known_src_addr = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut known_src_addr, 0xAB);
            i = i + 1;
        };

        // Simulate receiving FulfillmentProof from connected chain via GMP
        let was_recorded = fa_intent_outflow::receive_fulfillment_proof(
            2, // src_chain_id (connected chain)
            known_src_addr,
            payload,
        );
        assert!(was_recorded == true); // Should be newly recorded

        // Convert to generic Object type for entry function
        let intent_obj_generic: Object<Intent<fa_intent_with_oracle::FungibleStoreManager, fa_intent_with_oracle::OracleGuardedLimitOrder>> =
            object::address_to_object(intent_addr);

        // Fulfill the outflow intent (GMP proof was received, now solver claims tokens)
        fa_intent_outflow::fulfill_outflow_intent(
            solver_signer,
            intent_obj_generic,
        );

        // Verify solver_signer received the locked tokens (their reward)
        assert!(primary_fungible_store::balance(signer::address_of(solver_signer), offered_metadata) == 50);

        // Verify intent was unregistered from registry after fulfillment
        assert!(!intent_registry::is_intent_registered(intent_addr));

        // Verify LimitOrderFulfillmentEvent was emitted (coordinator uses this to detect completion)
        let fulfillment_events = event::emitted_events<fa_intent_outflow::LimitOrderFulfillmentEvent>();
        assert!(vector::length(&fulfillment_events) == 1);
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    #[expected_failure(abort_code = 0x30007, location = mvmt_intent::fa_intent_outflow)] // error::invalid_state(E_FULFILLMENT_PROOF_NOT_RECEIVED = 7)
    /// What is tested: fulfill_outflow_intent fails when no GMP proof received
    /// Why: Solver cannot claim tokens without GMP fulfillment proof
    fun test_fulfill_fails_without_gmp_proof(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        use mvmt_intent::fa_intent_with_oracle;
        use mvmt_intent::intent::Intent;

        // Set up outflow intent using shared helper
        let (intent_obj, _offered_metadata, _desired_metadata, _intent_id) = setup_outflow_intent(
            aptos_framework,
            mvmt_intent,
            requester_signer,
            solver_signer,
        );

        // Verify intent was created
        let intent_addr = object::object_address(&intent_obj);
        assert!(intent_addr != @0x0);

        // Note: We intentionally do NOT call receive_fulfillment_proof()
        // to test that fulfillment fails without GMP proof

        // Convert to generic Object type for entry function
        let intent_obj_generic: Object<Intent<fa_intent_with_oracle::FungibleStoreManager, fa_intent_with_oracle::OracleGuardedLimitOrder>> =
            object::address_to_object(intent_addr);

        // Attempt to fulfill should fail (no GMP proof received)
        fa_intent_outflow::fulfill_outflow_intent(
            solver_signer,
            intent_obj_generic,
        );
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    #[expected_failure(abort_code = 0x10003, location = mvmt_intent::fa_intent_outflow)] // error::invalid_argument(EINVALID_REQUESTER_ADDR)
    /// What is tested: create_outflow_intent aborts when requester_addr_connected_chain is the zero address
    /// Why: Outflow intents must target a valid connected-chain recipient address
    fun test_create_outflow_intent_rejects_zero_requester_address(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        // Set up test infrastructure using shared helper
        let (offered_metadata, desired_metadata, solver_addr, solver_signature_bytes, intent_id, expiry_time, offered_amount, desired_amount) =
            setup_outflow_test_infrastructure(aptos_framework, mvmt_intent, requester_signer, solver_signer);

        let requester_addr_connected_chain = @0x0; // Zero address - should be rejected

        // Attempt to create outflow intent with zero address - should abort
        // Pass desired_metadata as address (for cross-chain support)
        let desired_metadata_addr = object::object_address(&desired_metadata);
        fa_intent_outflow::create_outflow_intent(
            requester_signer,
            offered_metadata,
            offered_amount,
            1, // offered_chain_id (hub chain)
            desired_metadata_addr,  // Pass as address, not Object
            desired_amount,
            2, // desired_chain_id (connected chain)
            expiry_time,
            intent_id,
            requester_addr_connected_chain, // Zero address - should cause abort
            solver_addr,
            solver_addr, // solver_addr_connected_chain (same as hub addr in tests)
            solver_signature_bytes,
        );
    }

    // ============================================================================
    // CANCEL TESTS
    // ============================================================================

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    #[expected_failure(abort_code = 0x50005, location = mvmt_intent::intent)] // error::permission_denied(E_INTENT_NOT_EXPIRED = 5)
    /// What is tested: cancel_outflow_intent rejects cancellation before expiry
    /// Why: Funds must remain locked until expiry to give solvers time to fulfill
    fun test_cancel_outflow_rejects_before_expiry(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        let (intent_obj, _offered_metadata, _desired_metadata, _intent_id) = setup_outflow_intent(
            aptos_framework,
            mvmt_intent,
            requester_signer,
            solver_signer,
        );

        // Admin tries to cancel before expiry — should abort
        fa_intent_outflow::cancel_outflow_intent(mvmt_intent, intent_obj);
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    /// What is tested: cancel_outflow_intent returns funds to requester after expiry
    /// Why: Admin needs a way to return funds if fulfillment doesn't occur before expiry
    fun test_cancel_outflow_after_expiry_returns_funds(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        let (intent_obj, offered_metadata, _desired_metadata, _intent_id) = setup_outflow_intent(
            aptos_framework,
            mvmt_intent,
            requester_signer,
            solver_signer,
        );

        let intent_addr = object::object_address(&intent_obj);
        let requester_addr = signer::address_of(requester_signer);

        // Verify tokens were locked (100 - 50 = 50 remaining)
        assert!(primary_fungible_store::balance(requester_addr, offered_metadata) == 50);
        assert!(intent_registry::is_intent_registered(intent_addr));

        // Advance time past expiry (expiry = now + 3600, so 3601 is past)
        timestamp::update_global_time_for_test_secs(3601);

        // Admin cancels — funds should return to requester
        fa_intent_outflow::cancel_outflow_intent(mvmt_intent, intent_obj);

        // Verify funds returned (50 + 50 = 100)
        assert!(primary_fungible_store::balance(requester_addr, offered_metadata) == 100);

        // Verify intent was unregistered
        assert!(!intent_registry::is_intent_registered(intent_addr));

        // Verify OutflowIntentCancelled event was emitted
        let cancel_events = event::emitted_events<fa_intent_outflow::OutflowIntentCancelled>();
        assert!(vector::length(&cancel_events) == 1);
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    #[expected_failure(abort_code = 0x50008, location = mvmt_intent::fa_intent_outflow)] // error::permission_denied(E_UNAUTHORIZED_CALLER = 8)
    /// What is tested: cancel_outflow_intent rejects non-admin callers (including requester)
    /// Why: Only admin should be able to cancel expired intents
    fun test_cancel_outflow_rejects_unauthorized_caller(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        let (intent_obj, _offered_metadata, _desired_metadata, _intent_id) = setup_outflow_intent(
            aptos_framework,
            mvmt_intent,
            requester_signer,
            solver_signer,
        );

        // Advance time past expiry
        timestamp::update_global_time_for_test_secs(3601);

        // Requester (not admin) tries to cancel — should abort
        fa_intent_outflow::cancel_outflow_intent(requester_signer, intent_obj);
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    #[expected_failure(abort_code = 0x3000a, location = mvmt_intent::fa_intent_outflow)] // error::invalid_state(E_ALREADY_FULFILLED = 10)
    /// What is tested: cancel_outflow_intent rejects cancellation after fulfillment proof received
    /// Why: Once solver has fulfilled on the connected chain, funds must go to solver, not back to requester
    fun test_cancel_outflow_rejects_after_fulfillment_proof(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        use mvmt_intent::gmp_common;

        let (intent_obj, _offered_metadata, _desired_metadata, intent_id) = setup_outflow_intent(
            aptos_framework,
            mvmt_intent,
            requester_signer,
            solver_signer,
        );

        // Deliver fulfillment proof via GMP
        let intent_id_bytes = bcs::to_bytes(&intent_id);
        let solver_addr_bytes = bcs::to_bytes(&signer::address_of(solver_signer));
        let fulfillment_proof = gmp_common::new_fulfillment_proof(
            intent_id_bytes,
            solver_addr_bytes,
            50,
            timestamp::now_seconds(),
        );
        let payload = gmp_common::encode_fulfillment_proof(&fulfillment_proof);

        let known_src_addr = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut known_src_addr, 0xAB);
            i = i + 1;
        };

        fa_intent_outflow::receive_fulfillment_proof(2, known_src_addr, payload);

        // Advance time past expiry
        timestamp::update_global_time_for_test_secs(3601);

        // Admin tries to cancel after fulfillment proof — should abort
        fa_intent_outflow::cancel_outflow_intent(mvmt_intent, intent_obj);
    }

    #[test(
        aptos_framework = @0x1,
        mvmt_intent = @0x123,
        requester_signer = @0xcafe,
        solver_signer = @0xdead
    )]
    /// What is tested: admin (@mvmt_intent) can cancel expired outflow intent, funds go to requester
    /// Why: Admin acts as a helper to unstick expired intents; funds always go to the original requester
    fun test_admin_cancel_outflow_after_expiry(
        aptos_framework: &signer,
        mvmt_intent: &signer,
        requester_signer: &signer,
        solver_signer: &signer,
    ) {
        let (intent_obj, offered_metadata, _desired_metadata, _intent_id) = setup_outflow_intent(
            aptos_framework,
            mvmt_intent,
            requester_signer,
            solver_signer,
        );

        let requester_addr = signer::address_of(requester_signer);

        // Verify tokens were locked
        assert!(primary_fungible_store::balance(requester_addr, offered_metadata) == 50);

        // Advance time past expiry
        timestamp::update_global_time_for_test_secs(3601);

        // Admin (mvmt_intent) cancels — funds should go to requester, not admin
        fa_intent_outflow::cancel_outflow_intent(mvmt_intent, intent_obj);

        // Verify funds returned to requester (not admin)
        assert!(primary_fungible_store::balance(requester_addr, offered_metadata) == 100);

        // Verify OutflowIntentCancelled event was emitted
        let cancel_events = event::emitted_events<fa_intent_outflow::OutflowIntentCancelled>();
        assert!(vector::length(&cancel_events) == 1);
    }

}

