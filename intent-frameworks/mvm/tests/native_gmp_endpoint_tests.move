#[test_only]
module mvmt_intent::native_gmp_endpoint_tests {
    use std::vector;
    use aptos_framework::account;
    use mvmt_intent::native_gmp_endpoint;
    use mvmt_intent::gmp_sender;
    use mvmt_intent::gmp_common;
    use mvmt_intent::intent_gmp_hub;

    // Test addresses
    const ADMIN_ADDR: address = @0x123;
    const RELAY_ADDR: address = @0x456;
    const SOLANA_CHAIN_ID: u32 = 30168;

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    fun setup_test(): signer {
        let admin = account::create_account_for_test(ADMIN_ADDR);
        // Initialize sender (for lz_send) and receiver (for deliver_message) separately
        gmp_sender::initialize(&admin);
        native_gmp_endpoint::initialize(&admin);
        intent_gmp_hub::initialize(&admin);

        // Set trusted remote for hub (needed for receive_escrow_confirmation validation)
        intent_gmp_hub::set_trusted_remote(&admin, SOLANA_CHAIN_ID, create_test_trusted_remote());

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

    // ============================================================================
    // STATE SERIALIZATION TESTS (N/A for Move)
    // ============================================================================
    //
    // 6. test_config_account_serialization - N/A
    //    Why: SVM tests Borsh serialization of ConfigAccount state, but Move stores state
    //    in typed resources - the VM handles serialization automatically.
    //
    // 7. test_relay_account_serialization - N/A
    //    Why: SVM tests Borsh serialization of RelayAccount state, but Move stores state
    //    in typed resources - the VM handles serialization automatically.
    //
    // 8. test_trusted_remote_account_serialization - N/A
    //    Why: SVM tests Borsh serialization of TrustedRemoteAccount state, but Move stores
    //    state in typed resources - the VM handles serialization automatically.

    // ============================================================================
    // NONCE TRACKING TESTS (N/A for Move)
    // ============================================================================
    //
    // 9. test_outbound_nonce_account - N/A
    //    Why: SVM unit tests OutboundNonceAccount::increment() in isolation, but Move
    //    doesn't support isolated struct method tests - covered by integration tests 13-15.
    //
    // 10. test_inbound_nonce_account_replay_detection - N/A
    //     Why: SVM unit tests InboundNonceAccount::is_replay() in isolation, but Move
    //     doesn't support isolated struct method tests - covered by integration tests 15, 21.

    // ============================================================================
    // ERROR CONVERSION TESTS (N/A for Move)
    // ============================================================================
    //
    // 11. test_error_conversion - N/A
    //     Why: SVM tests GmpError to ProgramError conversion, but Move errors are abort
    //     codes directly - no conversion layer exists.
    //
    // 12. test_error_codes_unique - N/A
    //     Why: SVM verifies all error variants have unique codes, but Move abort codes are
    //     module constants - uniqueness is enforced at compile time.

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    // 13. Test: Send updates nonce state
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

    // 14. Test: DeliverMessage calls receiver
    // Verifies that deliver_message routes to the destination module handler after validation.
    // Why: Message routing is the core delivery mechanism; messages must reach their handlers.
    #[test]
    fun test_deliver_message_calls_receiver() {
        let admin = setup_test();

        // Setup trusted remote
        let trusted_addr = create_test_trusted_remote();
        native_gmp_endpoint::set_trusted_remote(
            &admin,
            SOLANA_CHAIN_ID,
            copy trusted_addr,
        );

        // Verify trusted remote is set
        assert!(native_gmp_endpoint::has_trusted_remote(SOLANA_CHAIN_ID), 1);
        let stored_addr = native_gmp_endpoint::get_trusted_remote(SOLANA_CHAIN_ID);
        assert!(stored_addr == trusted_addr, 2);

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
        assert!(inbound_nonce == 1, 3);
    }

    // 15. Test: DeliverMessage rejects replay
    // Verifies that delivering a message with an already-used nonce fails.
    // Why: Replay protection prevents attackers from re-submitting old messages.
    #[test]
    #[expected_failure(abort_code = 2)] // ENONCE_ALREADY_USED
    fun test_deliver_message_rejects_replay() {
        let admin = setup_test();

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

    // 16. Test: Unauthorized relay rejected
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

    // 17. Test: Authorized relay succeeds
    // Verifies that explicitly authorized relays can deliver messages.
    // Why: The relay authorization system must correctly grant access to approved relays.
    #[test]
    fun test_deliver_message_authorized_relay() {
        let admin = setup_test();
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

    // 18. Test: Untrusted remote address rejected
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

    // 19. Test: No trusted remote configured
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

    // 20. Test: Non-admin cannot set trusted remote
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

    // 21. Test: Lower nonce rejected
    // Verifies that delivering a message with a nonce lower than the last processed fails.
    // Why: Strictly increasing nonces prevent out-of-order message processing attacks.
    #[test]
    #[expected_failure(abort_code = 2)] // ENONCE_ALREADY_USED
    fun test_deliver_message_rejects_lower_nonce() {
        let admin = setup_test();

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
}
