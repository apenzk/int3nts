#[test_only]
module mvmt_intent::intent_gmp_tests {
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use mvmt_intent::intent_gmp;
    use mvmt_intent::gmp_sender;
    use mvmt_intent::gmp_common;
    use mvmt_intent::intent_outflow_validator_impl;
    use mvmt_intent::intent_inflow_escrow;

    // Test addresses
    const ADMIN_ADDR: address = @0x123;
    const HUB_CHAIN_ID: u32 = 1;

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    fun setup_test(): signer {
        let admin = account::create_account_for_test(ADMIN_ADDR);
        // Initialize timestamp (needed by gmp_send for outbox message timestamps)
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);
        // Initialize sender (for gmp_send) and receiver (for deliver_message)
        gmp_sender::initialize(&admin);
        intent_gmp::initialize(&admin);
        // Initialize connected chain modules
        let hub_gmp_endpoint_addr = create_test_32bytes(0x01);
        intent_outflow_validator_impl::initialize(&admin, HUB_CHAIN_ID, copy hub_gmp_endpoint_addr);
        intent_inflow_escrow::initialize(&admin, HUB_CHAIN_ID, hub_gmp_endpoint_addr);
        admin
    }

    fun create_test_remote_gmp_endpoint(): vector<u8> {
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
    // Verifies that gmp_sender::gmp_send increments the outbound nonce correctly for each message.
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
        let nonce1 = gmp_sender::gmp_send(
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
        let nonce2 = gmp_sender::gmp_send(
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

    // 16. Test: test_deliver_message_calls_receiver
    // Verifies that deliver_message correctly calls the receiver module's gmp_receive function.
    // Why: The endpoint must route messages to the registered handler. Without this, GMP messages arrive but are never processed.
    // TODO: Implement - requires setting up a mock receiver module and verifying CPI
    // Placeholder: MVM delivery currently tested indirectly via test 24 (stores_in_both_handlers).

    // 17. Test: DeliverMessage is idempotent on replay (same intent_id + msg_type)
    // Verifies that deliver_message silently returns on duplicate (intent_id, msg_type).
    // Why: Idempotent delivery means relay retries are safe and don't cause errors.
    #[test]
    fun test_deliver_message_rejects_replay() {
        let admin = setup_test();

        // Setup remote GMP endpoint (must match the hub addr used in module initialization)
        let addr = create_test_32bytes(0x01);
        intent_gmp::set_remote_gmp_endpoint_addr(
            &admin,
            HUB_CHAIN_ID,
            copy addr,
        );

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Deliver first message - should succeed
        intent_gmp::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            copy addr,
            copy payload,
        );

        // Verify message is delivered
        let intent_id = create_test_32bytes(0x11);
        assert!(intent_gmp::is_message_delivered(copy intent_id, 0x01), 1);

        // Deliver same payload again - should return silently (idempotent)
        intent_gmp::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            addr,
            payload,
        );

        // Message is still marked as delivered
        assert!(intent_gmp::is_message_delivered(intent_id, 0x01), 2);
    }

    // 18. Test: DeliverMessage rejects unauthorized relay
    // Verifies that deliver_message aborts when called by an address not in authorized_relays.
    // Why: Only authorized relays should be able to deliver cross-chain messages.
    #[test]
    #[expected_failure(abort_code = 1, location = mvmt_intent::intent_gmp)]
    fun test_deliver_message_rejects_unauthorized_relay() {
        let _admin = setup_test();

        // Create unauthorized account
        let unauthorized = account::create_account_for_test(@0x999);

        // Setup remote GMP endpoint (using admin from setup)
        let admin = account::create_account_for_test(ADMIN_ADDR);
        let addr = create_test_remote_gmp_endpoint();
        intent_gmp::set_remote_gmp_endpoint_addr(
            &admin,
            HUB_CHAIN_ID,
            copy addr,
        );

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Try to deliver with unauthorized relay - should fail
        intent_gmp::deliver_message(
            &unauthorized,
            HUB_CHAIN_ID,
            addr,
            payload,
        );
    }

    // 19. Test: DeliverMessage with authorized relay
    // Verifies that an added relay can successfully deliver messages.
    // Why: Only authorized relays should be able to deliver messages; new relays must be added.
    #[test]
    fun test_deliver_message_authorized_relay() {
        let admin = setup_test();

        // Setup remote GMP endpoint (must match the hub addr used in module initialization)
        let addr = create_test_32bytes(0x01);
        intent_gmp::set_remote_gmp_endpoint_addr(
            &admin,
            HUB_CHAIN_ID,
            copy addr,
        );

        // Add new relay
        let new_relay = account::create_account_for_test(@0x789);
        intent_gmp::add_relay(&admin, @0x789);

        // Verify relay is authorized
        assert!(intent_gmp::is_relay_authorized(@0x789), 1);

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Deliver with new relay - should succeed
        intent_gmp::deliver_message(
            &new_relay,
            HUB_CHAIN_ID,
            addr,
            payload,
        );
    }

    // 20. Test: DeliverMessage rejects unknown remote GMP endpoint
    // Verifies that deliver_message aborts when the source address is not a known remote GMP endpoint for the chain.
    // Why: Messages from unknown sources could be malicious; only known remote GMP endpoints are accepted.
    #[test]
    #[expected_failure(abort_code = 4, location = mvmt_intent::intent_gmp)]
    fun test_deliver_message_rejects_unknown_remote_gmp_endpoint() {
        let admin = setup_test();

        // Setup remote GMP endpoint
        let addr = create_test_remote_gmp_endpoint();
        intent_gmp::set_remote_gmp_endpoint_addr(
            &admin,
            HUB_CHAIN_ID,
            addr,
        );

        // Create different (unknown) source address
        let unknown_addr = create_test_32bytes(0xFF);

        // Create valid payload
        let payload = create_test_payload_intent_requirements();

        // Try to deliver from unknown source - should fail
        intent_gmp::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            unknown_addr,
            payload,
        );
    }

    // 21. Test: DeliverMessage rejects no remote GMP endpoint configured
    // Verifies that deliver_message aborts when no remote GMP endpoint is set for the source chain.
    // Why: If no remote GMP endpoint exists, all messages from that chain should be rejected.
    #[test]
    #[expected_failure(abort_code = 5, location = mvmt_intent::intent_gmp)]
    fun test_deliver_message_rejects_no_remote_gmp_endpoint() {
        let admin = setup_test();

        // Don't set any remote GMP endpoint

        // Create payload
        let payload = create_test_payload_intent_requirements();
        let remote_gmp_endpoint_addr = create_test_32bytes(0x01);

        // Try to deliver without remote GMP endpoint configured - should fail
        intent_gmp::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            remote_gmp_endpoint_addr,
            payload,
        );
    }

    // 22. Test: SetRemoteGmpEndpointAddr rejects non-admin
    // Verifies that only the admin can configure remote GMP endpoint addresses.
    // Why: Remote GMP endpoint configuration is security-critical; must be admin-only.
    #[test]
    #[expected_failure(abort_code = 6, location = mvmt_intent::intent_gmp)]
    fun test_set_remote_gmp_endpoint_addr_unauthorized() {
        let _admin = setup_test();

        // Create non-admin account
        let non_admin = account::create_account_for_test(@0x999);

        let addr = create_test_remote_gmp_endpoint();

        // Try to set remote GMP endpoint as non-admin - should fail
        intent_gmp::set_remote_gmp_endpoint_addr(
            &non_admin,
            HUB_CHAIN_ID,
            addr,
        );
    }

    // 23. Test: DeliverMessage allows same intent_id with different msg_type
    // Verifies that (intent_id, msg_type) dedup does NOT block a different msg_type for the same intent_id.
    // Why: A single intent goes through multiple GMP phases (0x01 requirements, 0x03 fulfillment proof).
    // Approach: Deliver 0x01 first, then attempt 0x03 for the same intent_id.
    // If dedup correctly includes msg_type, 0x03 passes dedup and reaches the handler (which aborts
    // with E_ESCROW_NOT_FOUND since no escrow exists). The expected_failure proves dedup didn't block it.
    // If dedup incorrectly ignored msg_type, 0x03 would be silently skipped and the test would fail.
    #[test]
    #[expected_failure(abort_code = 11, location = mvmt_intent::intent_inflow_escrow)]
    fun test_deliver_message_different_msg_type_succeeds() {
        let admin = setup_test();

        // Setup remote GMP endpoint (must match the hub addr used in module initialization)
        let addr = create_test_32bytes(0x01);
        intent_gmp::set_remote_gmp_endpoint_addr(
            &admin,
            HUB_CHAIN_ID,
            copy addr,
        );

        // Step 1: Deliver IntentRequirements (0x01) for intent 0x11...11 - succeeds
        let payload_req = create_test_payload_intent_requirements();
        intent_gmp::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            copy addr,
            payload_req,
        );

        // Step 2: Deliver FulfillmentProof (0x03) for the SAME intent_id
        // Build payload: msg_type(1) + intent_id(32) + solver_addr(32) + amount(8) + timestamp(8)
        let payload_proof = gmp_common::encode_fulfillment_proof(
            &gmp_common::new_fulfillment_proof(
                create_test_32bytes(0x11), // same intent_id as above
                create_test_32bytes(0x44), // solver_addr
                1000000,                   // amount
                86400,                     // timestamp
            )
        );

        // This reaches the handler (dedup passes) but aborts with E_ESCROW_NOT_FOUND (11)
        // because no escrow was created. The abort proves dedup didn't falsely block 0x03.
        intent_gmp::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            addr,
            payload_proof,
        );
    }

    // 24. Test: DeliverMessage routes IntentRequirements to both handlers
    // Verifies that IntentRequirements (0x01) are routed to both intent_outflow_validator_impl and intent_inflow_escrow.
    // Why: Connected chain must process requirements in both handlers for complete flow support.
    #[test]
    fun test_deliver_intent_requirements_stores_in_both_handlers() {
        let admin = setup_test();

        // Setup remote GMP endpoint (use the hub address that modules are configured with)
        let hub_gmp_endpoint_addr = create_test_32bytes(0x01);
        intent_gmp::set_remote_gmp_endpoint_addr(
            &admin,
            HUB_CHAIN_ID,
            copy hub_gmp_endpoint_addr,
        );

        // Create IntentRequirements payload
        let intent_id = create_test_32bytes(0x11);
        let payload = create_test_payload_intent_requirements();

        // Deliver message
        intent_gmp::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            hub_gmp_endpoint_addr,
            payload,
        );

        // Verify requirements stored in both handlers
        assert!(intent_outflow_validator_impl::has_requirements(copy intent_id), 1);
        assert!(intent_inflow_escrow::has_requirements(intent_id), 2);
    }

    // 25. Test: AddRelay rejects non-admin
    // Verifies that only the admin can add relays.
    // Why: Relay management is security-critical; must be admin-only.
    #[test]
    #[expected_failure(abort_code = 6, location = mvmt_intent::intent_gmp)]
    fun test_add_relay_rejects_non_admin() {
        let _admin = setup_test();

        // Create non-admin account
        let non_admin = account::create_account_for_test(@0x999);

        // Try to add relay as non-admin - should fail
        intent_gmp::add_relay(&non_admin, @0x789);
    }

    // 26. Test: RemoveRelay rejects non-admin
    // Verifies that only the admin can remove relays.
    // Why: Relay management is security-critical; must be admin-only.
    #[test]
    #[expected_failure(abort_code = 6, location = mvmt_intent::intent_gmp)]
    fun test_remove_relay_rejects_non_admin() {
        let _admin = setup_test();

        // Create non-admin account
        let non_admin = account::create_account_for_test(@0x999);

        // Try to remove relay as non-admin - should fail
        intent_gmp::remove_relay(&non_admin, ADMIN_ADDR);
    }

    // 27. Test: DeliverMessage fails if outflow_validator not initialized
    // Verifies that IntentRequirements delivery fails if intent_outflow_validator_impl is not initialized.
    // Why: No fallbacks - all handlers must be ready to receive messages.
    #[test]
    #[expected_failure(abort_code = 0xa, location = mvmt_intent::intent_outflow_validator_impl)]
    fun test_deliver_intent_requirements_fails_without_outflow_init() {
        let admin = account::create_account_for_test(ADMIN_ADDR);
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);

        // Only initialize sender and endpoint, NOT intent_outflow_validator_impl
        gmp_sender::initialize(&admin);
        intent_gmp::initialize(&admin);

        // Setup remote GMP endpoint
        let hub_gmp_endpoint_addr = create_test_32bytes(0x01);
        intent_gmp::set_remote_gmp_endpoint_addr(
            &admin,
            HUB_CHAIN_ID,
            copy hub_gmp_endpoint_addr,
        );

        // Create IntentRequirements payload
        let payload = create_test_payload_intent_requirements();

        // Try to deliver - should fail because outflow_validator not initialized
        intent_gmp::deliver_message(
            &admin,
            HUB_CHAIN_ID,
            hub_gmp_endpoint_addr,
            payload,
        );
    }
}
