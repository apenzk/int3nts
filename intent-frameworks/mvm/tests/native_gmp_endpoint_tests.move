#[test_only]
module mvmt_intent::native_gmp_endpoint_tests {
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use mvmt_intent::native_gmp_endpoint;
    use mvmt_intent::gmp_sender;
    use mvmt_intent::gmp_common;
    use mvmt_intent::intent_gmp_hub;
    use mvmt_intent::outflow_validator_impl;
    use mvmt_intent::inflow_escrow_gmp;
    use mvmt_intent::gmp_intent_state;

    // Test addresses
    const ADMIN_ADDR: address = @0x123;
    const RELAY_ADDR: address = @0x456;
    const SOLANA_CHAIN_ID: u32 = 30168;
    const HUB_CHAIN_ID: u32 = 1;

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    fun setup_test(): signer {
        let admin = account::create_account_for_test(ADMIN_ADDR);
        // Initialize timestamp (needed by lz_send for outbox message timestamps)
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);
        // Initialize sender (for lz_send) and receiver (for deliver_message) separately
        gmp_sender::initialize(&admin);
        native_gmp_endpoint::initialize(&admin);
        intent_gmp_hub::initialize(&admin);
        gmp_intent_state::init_for_test(&admin);

        // Set trusted remote for hub (needed for receive_escrow_confirmation validation)
        intent_gmp_hub::set_trusted_remote(&admin, SOLANA_CHAIN_ID, create_test_trusted_remote());

        admin
    }

    /// Register the test intent (intent_id = 0x11 * 32) in gmp_intent_state
    /// so that EscrowConfirmation delivery can call confirm_escrow.
    fun register_test_escrow_intent() {
        let intent_id = create_test_32bytes(0x11);
        gmp_intent_state::register_inflow_intent(intent_id, @0x0, SOLANA_CHAIN_ID, x"0000000000000000000000000000000000000000000000000000000000000000");
    }

    fun create_test_trusted_remote(): vector<u8> {
        let addr = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut addr, ((i + 1) as u8));
            i = i + 1;
        };
        addr
    }

    fun create_test_payload_escrow_confirmation(): vector<u8> {
        let intent_id = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut intent_id, 0x11);
            i = i + 1;
        };

        let escrow_id = vector::empty<u8>();
        i = 0;
        while (i < 32) {
            vector::push_back(&mut escrow_id, 0x22);
            i = i + 1;
        };

        let token_addr = vector::empty<u8>();
        i = 0;
        while (i < 32) {
            vector::push_back(&mut token_addr, 0x33);
            i = i + 1;
        };

        let creator_addr = vector::empty<u8>();
        i = 0;
        while (i < 32) {
            vector::push_back(&mut creator_addr, 0x44);
            i = i + 1;
        };

        let msg = gmp_common::new_escrow_confirmation(
            intent_id,
            escrow_id,
            1000000,
            token_addr,
            creator_addr,
        );
        gmp_common::encode_escrow_confirmation(&msg)
    }

    // ============================================================================
    // INSTRUCTION SERIALIZATION TESTS (N/A for Move)
    // ============================================================================
    //
    // 1. test_send_instruction_serialization - N/A
    //    Why: SVM tests Borsh encode/decode roundtrip for Send instruction, but Move
    //    function calls are typed by the VM - no manual instruction serialization needed.
    //
    // 2. test_deliver_message_instruction_serialization - N/A
    //    Why: SVM tests Borsh encode/decode roundtrip for DeliverMessage instruction, but
    //    Move function calls are typed by the VM - no manual instruction serialization needed.
    //
    // 3. test_initialize_instruction_serialization - N/A
    //    Why: SVM tests Borsh encode/decode roundtrip for Initialize instruction, but Move
    //    function calls are typed by the VM - no manual instruction serialization needed.
    //
    // 4. test_add_relay_instruction_serialization - N/A
    //    Why: SVM tests Borsh encode/decode roundtrip for AddRelay instruction, but Move
    //    function calls are typed by the VM - no manual instruction serialization needed.
    //
    // 5. test_set_trusted_remote_instruction_serialization - N/A
    //    Why: SVM tests Borsh encode/decode roundtrip for SetTrustedRemote instruction, but
    //    Move function calls are typed by the VM - no manual instruction serialization needed.
    //
    // 6. test_set_routing_instruction_serialization - N/A
    //    Why: SVM tests Borsh encode/decode roundtrip for SetRouting instruction, but Move
    //    function calls are typed by the VM - no manual instruction serialization needed.
    //
    // 7. test_routing_config_serialization - N/A
    //    Why: SVM tests Borsh serialization of RoutingConfig state, but Move stores state
    //    in typed resources - the VM handles serialization automatically.

    // ============================================================================
    // STATE SERIALIZATION TESTS (N/A for Move)
    // ============================================================================
    //
    // 8. test_config_account_serialization - N/A
    //    Why: SVM tests Borsh serialization of ConfigAccount state, but Move stores state
    //    in typed resources - the VM handles serialization automatically.
    //
    // 9. test_relay_account_serialization - N/A
    //    Why: SVM tests Borsh serialization of RelayAccount state, but Move stores state
    //    in typed resources - the VM handles serialization automatically.
    //
    // 10. test_trusted_remote_account_serialization - N/A
    //     Why: SVM tests Borsh serialization of TrustedRemoteAccount state, but Move stores
    //     state in typed resources - the VM handles serialization automatically.

    // ============================================================================
    // NONCE TRACKING TESTS (N/A for Move)
    // ============================================================================
    //
    // 11. test_outbound_nonce_account - N/A
    //     Why: SVM unit tests OutboundNonceAccount::increment() in isolation, but Move
    //     doesn't support isolated struct method tests - covered by integration tests 15-17.
    //
    // 12. test_inbound_nonce_account_replay_detection - N/A
    //     Why: SVM unit tests InboundNonceAccount::is_replay() in isolation, but Move
    //     doesn't support isolated struct method tests - covered by integration tests 17, 23.

    // ============================================================================
    // ERROR CONVERSION TESTS (N/A for Move)
    // ============================================================================
    //
    // 13. test_error_conversion - N/A
    //     Why: SVM tests GmpError to ProgramError conversion, but Move errors are abort
    //     codes directly - no conversion layer exists.
    //
    // 14. test_error_codes_unique - N/A
    //     Why: SVM verifies all error variants have unique codes, but Move abort codes are
    //     module constants - uniqueness is enforced at compile time.

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    // 15. Test: Send updates nonce state
    // Verifies that gmp_sender::lz_send increments the outbound nonce correctly for each message.
    // Why: Nonce tracking prevents message reordering and provides unique message IDs.
    #[test]
    fun test_send_updates_nonce_state() {
        let admin = setup_test();

        // Initial nonce should be 1
        let initial_nonce = gmp_sender::get_next_nonce();
        assert!(initial_nonce == 1, 1);

        // Create destination address and payload
        let dst_addr = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut dst_addr, 0xAB);
            i = i + 1;
        };
        let payload = vector[0x01, 0x02, 0x03];

        // Send first message (using gmp_sender, following LZ pattern)
        let nonce1 = gmp_sender::lz_send(
            &admin,
            SOLANA_CHAIN_ID,
            copy dst_addr,
            copy payload,
        );
        assert!(nonce1 == 1, 2);

        // Nonce should now be 2
        let after_first = gmp_sender::get_next_nonce();
        assert!(after_first == 2, 3);

        // Send second message
        let nonce2 = gmp_sender::lz_send(
            &admin,
            SOLANA_CHAIN_ID,
            copy dst_addr,
            payload,
        );
        assert!(nonce2 == 2, 4);

        // Nonce should now be 3
        let after_second = gmp_sender::get_next_nonce();
        assert!(after_second == 3, 5);
    }

    // 16. Test: DeliverMessage calls receiver
    // Verifies that deliver_message routes to the destination module handler after validation.
    // Why: Message routing is the core delivery mechanism; messages must reach their handlers.
    #[test]
    fun test_deliver_message_calls_receiver() {
        let admin = setup_test();
        register_test_escrow_intent();

        // Setup trusted remote
        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            SOLANA_CHAIN_ID,
            copy trusted_addr,
        );

        // Verify trusted remote is set
        assert!(native_gmp_endpoint::has_trusted_remote(SOLANA_CHAIN_ID), 1);
        let stored_addrs = native_gmp_endpoint::get_trusted_remote(SOLANA_CHAIN_ID);
        assert!(std::vector::length(&stored_addrs) == 1, 2);
        assert!(*std::vector::borrow(&stored_addrs, 0) == trusted_addr, 3);

        // Create valid payload (EscrowConfirmation)
        let payload = create_test_payload_escrow_confirmation();

        // Deliver message - should succeed and route to intent_gmp_hub
        native_gmp_endpoint::deliver_message(
            &admin, // admin is authorized relay by default
            SOLANA_CHAIN_ID,
            trusted_addr,
            payload,
            1, // nonce
        );

        // Verify inbound nonce was updated
        let inbound_nonce = native_gmp_endpoint::get_inbound_nonce(SOLANA_CHAIN_ID);
        assert!(inbound_nonce == 1, 4);
    }

    // 17. Test: DeliverMessage rejects replay
    // Verifies that delivering a message with an already-used nonce fails.
    // Why: Replay protection prevents attackers from re-submitting old messages.
    #[test]
    #[expected_failure(abort_code = 2)] // ENONCE_ALREADY_USED
    fun test_deliver_message_rejects_replay() {
        let admin = setup_test();
        register_test_escrow_intent();

        // Setup trusted remote
        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            SOLANA_CHAIN_ID,
            copy trusted_addr,
        );

        let payload = create_test_payload_escrow_confirmation();

        // First delivery should succeed
        native_gmp_endpoint::deliver_message(
            &admin, // admin is authorized relay by default
            SOLANA_CHAIN_ID,
            copy trusted_addr,
            copy payload,
            1, // nonce
        );

        // Second delivery with same nonce should fail (replay attack)
        native_gmp_endpoint::deliver_message(
            &admin,
            SOLANA_CHAIN_ID,
            trusted_addr,
            payload,
            1, // same nonce - replay!
        );
    }

    // ============================================================================
    // RELAY AUTHORIZATION TESTS
    // ============================================================================

    // 18. Test: Unauthorized relay rejected
    // Verifies that only authorized relays can deliver messages.
    // Why: Relay authorization prevents malicious actors from injecting fake messages.
    #[test]
    #[expected_failure(abort_code = 1)] // EUNAUTHORIZED_RELAY
    fun test_deliver_message_rejects_unauthorized_relay() {
        let admin = setup_test();
        let unauthorized = account::create_account_for_test(RELAY_ADDR);

        // Setup trusted remote
        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            SOLANA_CHAIN_ID,
            copy trusted_addr,
        );

        let payload = create_test_payload_escrow_confirmation();

        // Unauthorized relay should fail
        native_gmp_endpoint::deliver_message(
            &unauthorized, // not authorized
            SOLANA_CHAIN_ID,
            trusted_addr,
            payload,
            1, // nonce
        );
    }

    // 19. Test: Authorized relay succeeds
    // Verifies that explicitly authorized relays can deliver messages.
    // Why: The relay authorization system must correctly grant access to approved relays.
    #[test]
    fun test_deliver_message_authorized_relay() {
        let admin = setup_test();
        register_test_escrow_intent();
        let relay = account::create_account_for_test(RELAY_ADDR);

        // Add relay as authorized
        native_gmp_endpoint::add_authorized_relay(&admin, RELAY_ADDR);
        assert!(native_gmp_endpoint::is_relay_authorized(RELAY_ADDR), 1);

        // Setup trusted remote
        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            SOLANA_CHAIN_ID,
            copy trusted_addr,
        );

        let payload = create_test_payload_escrow_confirmation();

        // Authorized relay should succeed
        native_gmp_endpoint::deliver_message(
            &relay, // explicitly authorized
            SOLANA_CHAIN_ID,
            trusted_addr,
            payload,
            1, // nonce
        );
    }

    // ============================================================================
    // TRUSTED REMOTE VERIFICATION TESTS
    // ============================================================================

    // 20. Test: Untrusted remote address rejected
    // Verifies that messages from non-trusted source addresses are rejected.
    // Why: Trusted remote verification prevents spoofed cross-chain messages.
    #[test]
    #[expected_failure(abort_code = 4)] // EUNTRUSTED_REMOTE
    fun test_deliver_message_rejects_untrusted_remote() {
        let admin = setup_test();

        // Setup trusted remote
        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            SOLANA_CHAIN_ID,
            copy trusted_addr,
        );

        // Create a different (untrusted) address
        let untrusted_addr = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut untrusted_addr, 0xFF);
            i = i + 1;
        };

        let payload = create_test_payload_escrow_confirmation();

        // Untrusted address should fail
        native_gmp_endpoint::deliver_message(
            &admin, // admin is authorized relay by default
            SOLANA_CHAIN_ID,
            untrusted_addr, // not the trusted address
            payload,
            1, // nonce
        );
    }

    // 21. Test: No trusted remote configured
    // Verifies that messages fail when no trusted remote is configured for the source chain.
    // Why: Missing configuration must be caught early to prevent security holes.
    #[test]
    #[expected_failure(abort_code = 5)] // ENO_TRUSTED_REMOTE
    fun test_deliver_message_rejects_no_trusted_remote() {
        let admin = setup_test();
        // Don't configure any trusted remote

        let src_addr = create_test_trusted_remote();
        let payload = create_test_payload_escrow_confirmation();

        // Should fail because no trusted remote is configured
        native_gmp_endpoint::deliver_message(
            &admin, // admin is authorized relay by default
            SOLANA_CHAIN_ID, // no trusted remote configured for this chain
            src_addr,
            payload,
            1, // nonce
        );
    }

    // ============================================================================
    // ADMIN FUNCTION TESTS
    // ============================================================================

    // 22. Test: Non-admin cannot set trusted remote
    // Verifies that only the admin can configure trusted remote addresses.
    // Why: Admin-only access prevents unauthorized trust configuration changes.
    #[test]
    #[expected_failure(abort_code = 6)] // EUNAUTHORIZED_ADMIN
    fun test_set_trusted_remote_unauthorized() {
        let _admin = setup_test();
        let non_admin = account::create_account_for_test(RELAY_ADDR);

        let trusted_addr = create_test_trusted_remote();

        // Non-admin should fail
        native_gmp_endpoint::set_trusted_remote(
            &non_admin,
            SOLANA_CHAIN_ID,
            trusted_addr,
        );
    }

    // 23. Test: Lower nonce rejected
    // Verifies that delivering a message with a nonce lower than the last processed fails.
    // Why: Strictly increasing nonces prevent out-of-order message processing attacks.
    #[test]
    #[expected_failure(abort_code = 2)] // ENONCE_ALREADY_USED
    fun test_deliver_message_rejects_lower_nonce() {
        let admin = setup_test();
        register_test_escrow_intent();

        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            SOLANA_CHAIN_ID,
            copy trusted_addr,
        );

        let payload = create_test_payload_escrow_confirmation();

        // Deliver with nonce 5
        native_gmp_endpoint::deliver_message(
            &admin, // admin is authorized relay by default
            SOLANA_CHAIN_ID,
            copy trusted_addr,
            copy payload,
            5, // nonce
        );

        // Try to deliver with lower nonce 3 - should fail
        native_gmp_endpoint::deliver_message(
            &admin,
            SOLANA_CHAIN_ID,
            trusted_addr,
            payload,
            3, // lower than 5 - should fail
        );
    }

    // ============================================================================
    // CONNECTED CHAIN ROUTING INTEGRATION TESTS
    // ============================================================================

    /// Setup for connected chain tests: initializes ALL modules that route_message
    /// dispatches to, simulating a fully configured connected chain deployment.
    fun setup_test_connected_chain(): signer {
        let admin = account::create_account_for_test(ADMIN_ADDR);
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);

        // Core GMP infrastructure
        gmp_sender::initialize(&admin);
        native_gmp_endpoint::initialize(&admin);

        // Hub module (always initialized on both hub and connected chains)
        intent_gmp_hub::initialize(&admin);

        // Connected chain modules (the ones that receive IntentRequirements)
        let trusted_hub_addr = create_test_trusted_remote();
        outflow_validator_impl::initialize(&admin, HUB_CHAIN_ID, copy trusted_hub_addr);
        inflow_escrow_gmp::initialize(&admin, HUB_CHAIN_ID, copy trusted_hub_addr);

        // Configure native_gmp_endpoint to trust the hub chain
        native_gmp_endpoint::set_trusted_remote(&admin, HUB_CHAIN_ID, trusted_hub_addr);

        admin
    }

    fun create_test_32bytes(fill: u8): vector<u8> {
        let v = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut v, fill);
            i = i + 1;
        };
        v
    }

    fun create_test_intent_requirements_payload(): vector<u8> {
        let msg = gmp_common::new_intent_requirements(
            create_test_32bytes(0xAA), // intent_id
            create_test_32bytes(0xBB), // requester_addr
            1000000,                   // amount_required
            create_test_32bytes(0xCC), // token_addr
            create_test_32bytes(0xDD), // solver_addr
            999999,                    // expiry
        );
        gmp_common::encode_intent_requirements(&msg)
    }

    // 24. Test: IntentRequirements delivery stores requirements in both handlers
    // Verifies the full connected chain delivery flow:
    //   deliver_message → route_message → outflow_validator_impl + inflow_escrow_gmp
    // Then asserts has_requirements() returns true on both stores.
    // Why: This integration test catches missing module initialization or routing errors
    // that unit tests on individual modules cannot detect.
    #[test]
    fun test_deliver_intent_requirements_stores_in_both_handlers() {
        let admin = setup_test_connected_chain();

        let payload = create_test_intent_requirements_payload();
        let trusted_addr = create_test_trusted_remote();
        let intent_id = create_test_32bytes(0xAA);

        // Deliver IntentRequirements through native_gmp_endpoint (simulating relay)
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            trusted_addr,
            payload,
            1, // nonce
        );

        // Verify requirements stored in BOTH connected chain handlers
        assert!(outflow_validator_impl::has_requirements(copy intent_id), 1);
        assert!(inflow_escrow_gmp::has_requirements(intent_id), 2);
    }

    // 25. Test: IntentRequirements delivery aborts if outflow_validator_impl not initialized
    // Verifies that deliver_message aborts when a required handler module is not initialized.
    // Why: All modules in the routing path must be initialized before message delivery.
    // A missing config must cause a hard failure, not silent data loss.
    #[test]
    #[expected_failure(abort_code = 0xa, location = mvmt_intent::outflow_validator_impl)]
    fun test_deliver_intent_requirements_fails_without_outflow_init() {
        let admin = account::create_account_for_test(ADMIN_ADDR);
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);

        // Initialize everything EXCEPT outflow_validator_impl
        gmp_sender::initialize(&admin);
        native_gmp_endpoint::initialize(&admin);
        intent_gmp_hub::initialize(&admin);

        let trusted_hub_addr = create_test_trusted_remote();
        // Skip: outflow_validator_impl::initialize (intentionally omitted)
        inflow_escrow_gmp::initialize(&admin, HUB_CHAIN_ID, copy trusted_hub_addr);
        native_gmp_endpoint::set_trusted_remote(&admin, HUB_CHAIN_ID, trusted_hub_addr);

        let payload = create_test_intent_requirements_payload();
        let trusted_addr = create_test_trusted_remote();

        // This should abort with E_CONFIG_NOT_INITIALIZED (0xa) from outflow_validator_impl
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            trusted_addr,
            payload,
            1,
        );
    }
}
