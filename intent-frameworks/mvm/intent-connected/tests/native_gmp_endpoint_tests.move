#[test_only]
module mvmt_intent::native_gmp_endpoint_tests {
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use mvmt_intent::native_gmp_endpoint;
    use mvmt_intent::gmp_sender;
    use mvmt_intent::gmp_common;
    use mvmt_intent::outflow_validator_impl;
    use mvmt_intent::inflow_escrow_gmp;

    // Test addresses
    const ADMIN_ADDR: address = @0x123;
    const HUB_CHAIN_ID: u32 = 1;

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    fun setup_test(): signer {
        let admin = account::create_account_for_test(ADMIN_ADDR);
        // Initialize timestamp (needed by lz_send for outbox message timestamps)
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);
        // Initialize sender (for lz_send) and receiver (for deliver_message)
        gmp_sender::initialize(&admin);
        native_gmp_endpoint::initialize(&admin);
        // Initialize connected chain modules
        let trusted_hub_addr = create_test_32bytes(0x01);
        outflow_validator_impl::initialize(&admin, HUB_CHAIN_ID, copy trusted_hub_addr);
        inflow_escrow_gmp::initialize(&admin, HUB_CHAIN_ID, trusted_hub_addr);
        admin
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

    fun create_test_32bytes(fill: u8): vector<u8> {
        let bytes = vector::empty<u8>();
        let i = 0;
        while (i < 32) {
            vector::push_back(&mut bytes, fill);
            i = i + 1;
        };
        bytes
    }

    fun create_test_payload_intent_requirements(): vector<u8> {
        let intent_id = create_test_32bytes(0x11);
        let requester_addr = create_test_32bytes(0x22);
        let token_addr = create_test_32bytes(0x33);
        let solver_addr = create_test_32bytes(0x44);

        let msg = gmp_common::new_intent_requirements(
            intent_id,
            requester_addr,
            1000000,
            token_addr,
            solver_addr,
            86400, // expiry
        );
        gmp_common::encode_intent_requirements(&msg)
    }

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
        let dst_addr = create_test_32bytes(0xAB);
        let payload = vector[0x01, 0x02, 0x03];

        // Send first message
        let nonce1 = gmp_sender::lz_send(
            &admin,
            HUB_CHAIN_ID,
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
            HUB_CHAIN_ID,
            dst_addr,
            payload,
        );
        assert!(nonce2 == 2, 4);

        // Nonce should now be 3
        let after_second = gmp_sender::get_next_nonce();
        assert!(after_second == 3, 5);
    }

    // 17. Test: DeliverMessage rejects replay
    // Verifies that deliver_message rejects messages with a nonce <= the last processed nonce.
    // Why: Replay protection prevents attackers from re-submitting old messages.
    #[test]
    #[expected_failure(abort_code = 2, location = mvmt_intent::native_gmp_endpoint)]
    fun test_deliver_message_rejects_replay() {
        let admin = setup_test();

        // Setup trusted remote (must match the hub addr used in module initialization)
        let trusted_addr = create_test_32bytes(0x01);
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            HUB_CHAIN_ID,
            copy trusted_addr,
        );

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Deliver first message with nonce 1 - should succeed
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            copy trusted_addr,
            copy payload,
            1,
        );

        // Try to deliver same nonce again - should fail
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            trusted_addr,
            payload,
            1,
        );
    }

    // 18. Test: DeliverMessage rejects unauthorized relay
    // Verifies that deliver_message aborts when called by an address not in authorized_relays.
    // Why: Only authorized relays should be able to deliver cross-chain messages.
    #[test]
    #[expected_failure(abort_code = 1, location = mvmt_intent::native_gmp_endpoint)]
    fun test_deliver_message_rejects_unauthorized_relay() {
        let _admin = setup_test();

        // Create unauthorized account
        let unauthorized = account::create_account_for_test(@0x999);

        // Setup trusted remote (using admin from setup)
        let admin = account::create_account_for_test(ADMIN_ADDR);
        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            HUB_CHAIN_ID,
            copy trusted_addr,
        );

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Try to deliver with unauthorized relay - should fail
        native_gmp_endpoint::deliver_message(
            &unauthorized,
            HUB_CHAIN_ID,
            trusted_addr,
            payload,
            1,
        );
    }

    // 19. Test: DeliverMessage with authorized relay
    // Verifies that an added relay can successfully deliver messages.
    // Why: Only authorized relays should be able to deliver messages; new relays must be added.
    #[test]
    fun test_deliver_message_authorized_relay() {
        let admin = setup_test();

        // Setup trusted remote (must match the hub addr used in module initialization)
        let trusted_addr = create_test_32bytes(0x01);
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            HUB_CHAIN_ID,
            copy trusted_addr,
        );

        // Add new relay
        let new_relay = account::create_account_for_test(@0x789);
        native_gmp_endpoint::add_authorized_relay(&admin, @0x789);

        // Verify relay is authorized
        assert!(native_gmp_endpoint::is_relay_authorized(@0x789), 1);

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Deliver with new relay - should succeed
        native_gmp_endpoint::deliver_message(
            &new_relay,
            HUB_CHAIN_ID,
            trusted_addr,
            payload,
            1,
        );
    }

    // 20. Test: DeliverMessage rejects untrusted remote
    // Verifies that deliver_message aborts when the source address is not trusted for the chain.
    // Why: Messages from untrusted sources could be malicious; only trusted sources are accepted.
    #[test]
    #[expected_failure(abort_code = 4, location = mvmt_intent::native_gmp_endpoint)]
    fun test_deliver_message_rejects_untrusted_remote() {
        let admin = setup_test();

        // Setup trusted remote
        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            HUB_CHAIN_ID,
            trusted_addr,
        );

        // Create different (untrusted) source address
        let untrusted_addr = create_test_32bytes(0xFF);

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Try to deliver from untrusted source - should fail
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            untrusted_addr,
            payload,
            1,
        );
    }

    // 21. Test: DeliverMessage rejects no trusted remote configured
    // Verifies that deliver_message aborts when no trusted remote is set for the source chain.
    // Why: If no trusted remote exists, all messages from that chain should be rejected.
    #[test]
    #[expected_failure(abort_code = 5, location = mvmt_intent::native_gmp_endpoint)]
    fun test_deliver_message_rejects_no_trusted_remote() {
        let admin = setup_test();

        // Don't set any trusted remote

        // Create payload
        let payload = create_test_payload_intent_requirements();
        let src_addr = create_test_32bytes(0x01);

        // Try to deliver without trusted remote configured - should fail
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            src_addr,
            payload,
            1,
        );
    }

    // 22. Test: SetTrustedRemote rejects non-admin
    // Verifies that only the admin can configure trusted remotes.
    // Why: Trusted remote configuration is security-critical; must be admin-only.
    #[test]
    #[expected_failure(abort_code = 6, location = mvmt_intent::native_gmp_endpoint)]
    fun test_set_trusted_remote_unauthorized() {
        let _admin = setup_test();

        // Create non-admin account
        let non_admin = account::create_account_for_test(@0x999);

        let trusted_addr = create_test_trusted_remote();

        // Try to set trusted remote as non-admin - should fail
        native_gmp_endpoint::set_trusted_remote(
            &non_admin,
            HUB_CHAIN_ID,
            trusted_addr,
        );
    }

    // 23. Test: DeliverMessage rejects lower nonce
    // Verifies that deliver_message rejects messages with nonce less than the last processed.
    // Why: Ensures strict ordering - can't go backwards in nonce sequence.
    #[test]
    #[expected_failure(abort_code = 2, location = mvmt_intent::native_gmp_endpoint)]
    fun test_deliver_message_rejects_lower_nonce() {
        let admin = setup_test();

        // Setup trusted remote (must match the hub addr used in module initialization)
        let trusted_addr = create_test_32bytes(0x01);
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            HUB_CHAIN_ID,
            copy trusted_addr,
        );

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Deliver with nonce 5
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            copy trusted_addr,
            copy payload,
            5,
        );

        // Try to deliver with nonce 3 (lower than 5) - should fail
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            trusted_addr,
            payload,
            3,
        );
    }

    // 24. Test: DeliverMessage routes IntentRequirements to both handlers
    // Verifies that IntentRequirements (0x01) are routed to both outflow_validator_impl and inflow_escrow_gmp.
    // Why: Connected chain must process requirements in both handlers for complete flow support.
    #[test]
    fun test_deliver_intent_requirements_stores_in_both_handlers() {
        let admin = setup_test();

        // Setup trusted remote (use the hub address that modules are configured with)
        let trusted_hub_addr = create_test_32bytes(0x01);
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            HUB_CHAIN_ID,
            copy trusted_hub_addr,
        );

        // Create IntentRequirements payload
        let intent_id = create_test_32bytes(0x11);
        let payload = create_test_payload_intent_requirements();

        // Deliver message
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            trusted_hub_addr,
            payload,
            1,
        );

        // Verify requirements stored in both handlers
        assert!(outflow_validator_impl::has_requirements(copy intent_id), 1);
        assert!(inflow_escrow_gmp::has_requirements(intent_id), 2);
    }

    // 25. Test: DeliverMessage fails if outflow_validator not initialized
    // Verifies that IntentRequirements delivery fails if outflow_validator_impl is not initialized.
    // Why: No fallbacks - all handlers must be ready to receive messages.
    #[test]
    #[expected_failure(abort_code = 0xa, location = mvmt_intent::outflow_validator_impl)]
    fun test_deliver_intent_requirements_fails_without_outflow_init() {
        let admin = account::create_account_for_test(ADMIN_ADDR);
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);

        // Only initialize sender and endpoint, NOT outflow_validator_impl
        gmp_sender::initialize(&admin);
        native_gmp_endpoint::initialize(&admin);

        // Setup trusted remote
        let trusted_hub_addr = create_test_32bytes(0x01);
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            HUB_CHAIN_ID,
            copy trusted_hub_addr,
        );

        // Create IntentRequirements payload
        let payload = create_test_payload_intent_requirements();

        // Try to deliver - should fail because outflow_validator not initialized
        native_gmp_endpoint::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            trusted_hub_addr,
            payload,
            1,
        );
    }
}
