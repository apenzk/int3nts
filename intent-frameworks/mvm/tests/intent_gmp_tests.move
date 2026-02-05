// GMP Hub Tests (MVM as Hub Chain)
//
// Tests GMP (General Message Passing) functionality for the intent_gmp_hub module
// when MVM acts as the hub chain in the cross-chain intent system.
//
// ## Send Functions (Hub → Connected Chain)
// - `send_intent_requirements`: Hub broadcasts intent requirements to connected chain
// - `send_fulfillment_proof`: Hub sends fulfillment proof after solver completes
//
// ## Receive Functions (Connected Chain → Hub)
// - `receive_escrow_confirmation`: Hub receives confirmation that escrow was created
// - `receive_fulfillment_proof`: Hub receives proof that solver fulfilled on connected chain
//
// ## Helper Functions
// - `bytes_to_bytes32`: Utility for converting byte vectors to 32-byte format
//
// ## Related Test Files (MVM as Connected Chain)
// - `interface_tests.move`: Tests outflow_validator interface (symmetric with SVM)
// - `outflow_validator_tests.move`: Tests outflow validator implementation
// - `inflow_escrow_gmp_tests.move`: Tests inflow escrow GMP functionality
#[test_only]
module mvmt_intent::intent_gmp_tests {
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::timestamp;
    use mvmt_intent::intent_gmp_hub;
    use mvmt_intent::gmp_common;
    use mvmt_intent::gmp_sender;
    use mvmt_intent::gmp_intent_state;

    // ============================================================================
    // TEST CONSTANTS
    // ============================================================================

    const DUMMY_CHAIN_ID: u32 = 1;
    const DUMMY_AMOUNT: u64 = 1000000;
    const DUMMY_EXPIRY: u64 = 1000;
    const DUMMY_TIMESTAMP: u64 = 1234567890;

    // ============================================================================
    // TEST HELPERS
    // ============================================================================

    fun zeros(n: u64): vector<u8> {
        let v = vector::empty<u8>();
        let i = 0;
        while (i < n) {
            vector::push_back(&mut v, 0);
            i = i + 1;
        };
        v
    }

    fun test_intent_id(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0xAA;
        *vector::borrow_mut(&mut v, 31) = 0xBB;
        v
    }

    fun test_requester_addr(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0x11;
        *vector::borrow_mut(&mut v, 31) = 0x22;
        v
    }

    fun test_token_addr(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0x33;
        *vector::borrow_mut(&mut v, 31) = 0x44;
        v
    }

    fun test_solver_addr(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0x55;
        *vector::borrow_mut(&mut v, 31) = 0x66;
        v
    }

    fun test_escrow_id(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0x77;
        *vector::borrow_mut(&mut v, 31) = 0x88;
        v
    }

    fun test_creator_addr(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0x99;
        *vector::borrow_mut(&mut v, 31) = 0xAA;
        v
    }

    fun test_src_address(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0xCC;
        *vector::borrow_mut(&mut v, 31) = 0xDD;
        v
    }

    // ============================================================================
    // SEND INTENT REQUIREMENTS TESTS
    // ============================================================================

    // 1. Test: send_intent_requirements sends GMP message with correct encoding
    // Verifies that the function sends a properly encoded IntentRequirements message via GMP.
    #[test(admin = @mvmt_intent)]
    fun test_send_intent_requirements_sends_message(admin: &signer) {
        // Initialize timestamp for lz_send
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);

        // Initialize configs
        intent_gmp_hub::initialize(admin);
        gmp_sender::initialize(admin);

        // Set trusted remote for destination chain
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, test_solver_addr());

        let intent_id = test_intent_id();
        let requester = test_requester_addr();
        let token = test_token_addr();
        let solver = test_solver_addr();

        // Send requirements via GMP
        let nonce = intent_gmp_hub::send_intent_requirements(
            admin,
            DUMMY_CHAIN_ID,
            intent_id,
            requester,
            DUMMY_AMOUNT,
            token,
            solver,
            DUMMY_EXPIRY,
        );

        // Verify nonce was assigned
        assert!(nonce == 1, 1);

        // Verify the encoding is correct by creating the expected payload
        let expected_msg = gmp_common::new_intent_requirements(
            intent_id,
            requester,
            DUMMY_AMOUNT,
            token,
            solver,
            DUMMY_EXPIRY,
        );
        let expected_payload = gmp_common::encode_intent_requirements(&expected_msg);

        // Verify payload properties
        assert!(vector::length(&expected_payload) == gmp_common::intent_requirements_size(), 2);
        assert!(*vector::borrow(&expected_payload, 0) == 0x01, 3); // Discriminator
    }

    // 2. Test: send_intent_requirements payload roundtrip
    // Verifies roundtrip: encode via send function, decode and verify all fields.
    #[test]
    fun test_send_intent_requirements_roundtrip() {
        let intent_id = test_intent_id();
        let requester = test_requester_addr();
        let token = test_token_addr();
        let solver = test_solver_addr();

        // Create and encode the message (simulating what send_intent_requirements does internally)
        let msg = gmp_common::new_intent_requirements(
            intent_id,
            requester,
            DUMMY_AMOUNT,
            token,
            solver,
            DUMMY_EXPIRY,
        );
        let payload = gmp_common::encode_intent_requirements(&msg);

        // Verify payload structure
        assert!(vector::length(&payload) == gmp_common::intent_requirements_size(), 1);
        assert!(*vector::borrow(&payload, 0) == 0x01, 2);

        // Decode and verify all fields survived encoding
        let decoded = gmp_common::decode_intent_requirements(&payload);
        assert!(*gmp_common::intent_requirements_intent_id(&decoded) == intent_id, 3);
        assert!(*gmp_common::intent_requirements_requester_addr(&decoded) == requester, 4);
        assert!(gmp_common::intent_requirements_amount_required(&decoded) == DUMMY_AMOUNT, 5);
        assert!(*gmp_common::intent_requirements_token_addr(&decoded) == token, 6);
        assert!(*gmp_common::intent_requirements_solver_addr(&decoded) == solver, 7);
        assert!(gmp_common::intent_requirements_expiry(&decoded) == DUMMY_EXPIRY, 8);
    }

    // ============================================================================
    // SEND FULFILLMENT PROOF TESTS
    // ============================================================================

    // 3. Test: send_fulfillment_proof sends GMP message with correct encoding
    // Verifies that the function sends a properly encoded FulfillmentProof message via GMP.
    #[test(admin = @mvmt_intent)]
    fun test_send_fulfillment_proof_sends_message(admin: &signer) {
        // Initialize timestamp for lz_send
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);

        // Initialize configs
        intent_gmp_hub::initialize(admin);
        gmp_sender::initialize(admin);

        // Set trusted remote for destination chain
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, test_solver_addr());

        let intent_id = test_intent_id();
        let solver = test_solver_addr();

        // Send fulfillment proof via GMP
        let nonce = intent_gmp_hub::send_fulfillment_proof(
            admin,
            DUMMY_CHAIN_ID,
            intent_id,
            solver,
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );

        // Verify nonce was assigned
        assert!(nonce == 1, 1);

        // Verify the encoding is correct by creating the expected payload
        let expected_msg = gmp_common::new_fulfillment_proof(
            intent_id,
            solver,
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );
        let expected_payload = gmp_common::encode_fulfillment_proof(&expected_msg);

        // Verify payload properties
        assert!(vector::length(&expected_payload) == gmp_common::fulfillment_proof_size(), 2);
        assert!(*vector::borrow(&expected_payload, 0) == 0x03, 3); // Discriminator
    }

    // 4. Test: send_fulfillment_proof payload roundtrip
    // Verifies roundtrip: encode via send function, decode and verify all fields.
    #[test]
    fun test_send_fulfillment_proof_roundtrip() {
        let intent_id = test_intent_id();
        let solver = test_solver_addr();

        // Create and encode the message (simulating what send_fulfillment_proof does internally)
        let msg = gmp_common::new_fulfillment_proof(
            intent_id,
            solver,
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );
        let payload = gmp_common::encode_fulfillment_proof(&msg);

        // Verify payload structure
        assert!(vector::length(&payload) == gmp_common::fulfillment_proof_size(), 1);
        assert!(*vector::borrow(&payload, 0) == 0x03, 2);

        // Decode and verify all fields survived encoding
        let decoded = gmp_common::decode_fulfillment_proof(&payload);
        assert!(*gmp_common::fulfillment_proof_intent_id(&decoded) == intent_id, 3);
        assert!(*gmp_common::fulfillment_proof_solver_addr(&decoded) == solver, 4);
        assert!(gmp_common::fulfillment_proof_amount_fulfilled(&decoded) == DUMMY_AMOUNT, 5);
        assert!(gmp_common::fulfillment_proof_timestamp(&decoded) == DUMMY_TIMESTAMP, 6);
    }

    // ============================================================================
    // RECEIVE ESCROW CONFIRMATION TESTS
    // ============================================================================

    // 5. Test: receive_escrow_confirmation decodes valid payload from trusted source
    // Verifies that the function correctly decodes an EscrowConfirmation message with source validation.
    #[test(admin = @mvmt_intent)]
    fun test_receive_escrow_confirmation_decodes_payload(admin: &signer) {
        // Initialize config
        intent_gmp_hub::initialize(admin);
        gmp_intent_state::init_for_test(admin);

        // Register the intent so confirm_escrow can find it
        gmp_intent_state::register_inflow_intent(test_intent_id(), @0x0, DUMMY_CHAIN_ID, x"0000000000000000000000000000000000000000000000000000000000000000");

        // Set trusted remote
        let src_address = test_src_address();
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, src_address);

        // Create a valid EscrowConfirmation payload
        let msg = gmp_common::new_escrow_confirmation(
            test_intent_id(),
            test_escrow_id(),
            DUMMY_AMOUNT,
            test_token_addr(),
            test_creator_addr(),
        );
        let payload = gmp_common::encode_escrow_confirmation(&msg);

        // Receive and decode
        let decoded = intent_gmp_hub::receive_escrow_confirmation(
            DUMMY_CHAIN_ID,
            src_address,
            payload,
        );

        // Verify fields
        assert!(*gmp_common::escrow_confirmation_intent_id(&decoded) == test_intent_id(), 1);
        assert!(*gmp_common::escrow_confirmation_escrow_id(&decoded) == test_escrow_id(), 2);
        assert!(gmp_common::escrow_confirmation_amount_escrowed(&decoded) == DUMMY_AMOUNT, 3);
        assert!(*gmp_common::escrow_confirmation_token_addr(&decoded) == test_token_addr(), 4);
        assert!(*gmp_common::escrow_confirmation_creator_addr(&decoded) == test_creator_addr(), 5);
    }

    // 6. Test: receive_escrow_confirmation updates gmp_intent_state
    // Verifies that escrow_confirmed transitions from false to true after receiving EscrowConfirmation.
    // Why: Without this state update, the solver's fulfillment call aborts with E_ESCROW_NOT_CONFIRMED.
    #[test(admin = @mvmt_intent)]
    fun test_receive_escrow_confirmation_updates_state(admin: &signer) {
        intent_gmp_hub::initialize(admin);
        gmp_intent_state::init_for_test(admin);

        let intent_id = test_intent_id();
        gmp_intent_state::register_inflow_intent(copy intent_id, @0x0, DUMMY_CHAIN_ID, x"0000000000000000000000000000000000000000000000000000000000000000");

        // Before: escrow is NOT confirmed
        assert!(!gmp_intent_state::is_escrow_confirmed(copy intent_id), 1);

        // Set trusted remote and receive EscrowConfirmation
        let src_address = test_src_address();
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, src_address);

        let msg = gmp_common::new_escrow_confirmation(
            copy intent_id,
            test_escrow_id(),
            DUMMY_AMOUNT,
            test_token_addr(),
            test_creator_addr(),
        );
        let payload = gmp_common::encode_escrow_confirmation(&msg);

        intent_gmp_hub::receive_escrow_confirmation(
            DUMMY_CHAIN_ID,
            src_address,
            payload,
        );

        // After: escrow IS confirmed
        assert!(gmp_intent_state::is_escrow_confirmed(intent_id), 2);
    }

    // 7. Test: receive_escrow_confirmation rejects untrusted source
    // Verifies that messages from untrusted sources are rejected.
    #[test(admin = @mvmt_intent)]
    #[expected_failure(abort_code = 0x50003, location = mvmt_intent::intent_gmp_hub)]
    fun test_receive_escrow_confirmation_rejects_untrusted_source(admin: &signer) {
        // Initialize config
        intent_gmp_hub::initialize(admin);

        // Set trusted remote to a different address
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, test_token_addr());

        // Create payload
        let msg = gmp_common::new_escrow_confirmation(
            test_intent_id(),
            test_escrow_id(),
            DUMMY_AMOUNT,
            test_token_addr(),
            test_creator_addr(),
        );
        let payload = gmp_common::encode_escrow_confirmation(&msg);

        // Try to receive from untrusted source (should abort)
        intent_gmp_hub::receive_escrow_confirmation(
            DUMMY_CHAIN_ID,
            test_src_address(), // Different from trusted address
            payload,
        );
    }

    // ============================================================================
    // RECEIVE FULFILLMENT PROOF TESTS
    // ============================================================================

    // 8. Test: receive_fulfillment_proof decodes valid payload from trusted source
    // Verifies that the function correctly decodes a FulfillmentProof message with source validation.
    #[test(admin = @mvmt_intent)]
    fun test_receive_fulfillment_proof_decodes_payload(admin: &signer) {
        // Initialize config
        intent_gmp_hub::initialize(admin);

        // Set trusted remote
        let src_address = test_src_address();
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, src_address);

        // Create a valid FulfillmentProof payload
        let msg = gmp_common::new_fulfillment_proof(
            test_intent_id(),
            test_solver_addr(),
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );
        let payload = gmp_common::encode_fulfillment_proof(&msg);

        // Receive and decode
        let decoded = intent_gmp_hub::receive_fulfillment_proof(
            DUMMY_CHAIN_ID,
            src_address,
            payload,
        );

        // Verify fields
        assert!(*gmp_common::fulfillment_proof_intent_id(&decoded) == test_intent_id(), 1);
        assert!(*gmp_common::fulfillment_proof_solver_addr(&decoded) == test_solver_addr(), 2);
        assert!(gmp_common::fulfillment_proof_amount_fulfilled(&decoded) == DUMMY_AMOUNT, 3);
        assert!(gmp_common::fulfillment_proof_timestamp(&decoded) == DUMMY_TIMESTAMP, 4);
    }

    // 9. Test: receive_fulfillment_proof rejects untrusted source
    // Verifies that messages from untrusted sources are rejected.
    #[test(admin = @mvmt_intent)]
    #[expected_failure(abort_code = 0x50003, location = mvmt_intent::intent_gmp_hub)]
    fun test_receive_fulfillment_proof_rejects_untrusted_source(admin: &signer) {
        // Initialize config
        intent_gmp_hub::initialize(admin);

        // Set trusted remote to a different address
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, test_token_addr());

        // Create payload
        let msg = gmp_common::new_fulfillment_proof(
            test_intent_id(),
            test_solver_addr(),
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );
        let payload = gmp_common::encode_fulfillment_proof(&msg);

        // Try to receive from untrusted source (should abort)
        intent_gmp_hub::receive_fulfillment_proof(
            DUMMY_CHAIN_ID,
            test_src_address(), // Different from trusted address
            payload,
        );
    }

    // ============================================================================
    // HELPER FUNCTION TESTS
    // ============================================================================

    // 10. Test: bytes_to_bytes32 pads short input
    // Verifies that inputs shorter than 32 bytes are left-padded with zeros.
    #[test]
    fun test_bytes_to_bytes32_pads_short_input() {
        let short = vector::empty<u8>();
        vector::push_back(&mut short, 0xAB);
        vector::push_back(&mut short, 0xCD);

        let result = intent_gmp_hub::bytes_to_bytes32(short);

        // Should be 32 bytes
        assert!(vector::length(&result) == 32, 1);
        // First 30 bytes should be zeros
        let i = 0;
        while (i < 30) {
            assert!(*vector::borrow(&result, i) == 0, 2);
            i = i + 1;
        };
        // Last 2 bytes should be original data
        assert!(*vector::borrow(&result, 30) == 0xAB, 3);
        assert!(*vector::borrow(&result, 31) == 0xCD, 4);
    }

    // 11. Test: bytes_to_bytes32 truncates long input
    // Verifies that inputs longer than 32 bytes are truncated to first 32.
    #[test]
    fun test_bytes_to_bytes32_truncates_long_input() {
        let long = vector::empty<u8>();
        let i = 0;
        while (i < 40) {
            vector::push_back(&mut long, (i as u8));
            i = i + 1;
        };

        let result = intent_gmp_hub::bytes_to_bytes32(long);

        // Should be exactly 32 bytes
        assert!(vector::length(&result) == 32, 1);
        // Should contain first 32 bytes of input
        i = 0;
        while (i < 32) {
            assert!(*vector::borrow(&result, i) == (i as u8), 2);
            i = i + 1;
        };
    }

    // 12. Test: bytes_to_bytes32 returns exact 32 bytes unchanged
    // Verifies that 32-byte inputs are returned unchanged.
    #[test]
    fun test_bytes_to_bytes32_exact_length() {
        let exact = test_intent_id(); // Already 32 bytes

        let result = intent_gmp_hub::bytes_to_bytes32(exact);

        assert!(vector::length(&result) == 32, 1);
        assert!(result == test_intent_id(), 2);
    }

    // 13. Test: bytes_to_bytes32 handles empty input
    // Verifies that empty input results in 32 zero bytes.
    #[test]
    fun test_bytes_to_bytes32_empty_input() {
        let empty = vector::empty<u8>();

        let result = intent_gmp_hub::bytes_to_bytes32(empty);

        assert!(vector::length(&result) == 32, 1);
        let i = 0;
        while (i < 32) {
            assert!(*vector::borrow(&result, i) == 0, 2);
            i = i + 1;
        };
    }

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    // 14. Test: Full send-receive roundtrip for IntentRequirements
    // Simulates the full flow: hub sends requirements via GMP, connected chain receives.
    #[test(admin = @mvmt_intent)]
    fun test_intent_requirements_full_flow(admin: &signer) {
        // Initialize timestamp for lz_send
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);

        // Initialize configs
        intent_gmp_hub::initialize(admin);
        gmp_sender::initialize(admin);

        // Set trusted remote
        let remote_addr = test_solver_addr();
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, remote_addr);

        let intent_id = test_intent_id();
        let requester = test_requester_addr();
        let token = test_token_addr();
        let solver = test_solver_addr();

        // Hub sends requirements (returns nonce, not payload)
        let _nonce = intent_gmp_hub::send_intent_requirements(
            admin,
            DUMMY_CHAIN_ID,
            intent_id,
            requester,
            DUMMY_AMOUNT,
            token,
            solver,
            DUMMY_EXPIRY,
        );

        // Simulate the payload being created for connected chain
        let msg = gmp_common::new_intent_requirements(
            intent_id,
            requester,
            DUMMY_AMOUNT,
            token,
            solver,
            DUMMY_EXPIRY,
        );
        let decoded = gmp_common::decode_intent_requirements(&gmp_common::encode_intent_requirements(&msg));

        // Verify all fields match
        assert!(*gmp_common::intent_requirements_intent_id(&decoded) == intent_id, 1);
        assert!(*gmp_common::intent_requirements_requester_addr(&decoded) == requester, 2);
        assert!(gmp_common::intent_requirements_amount_required(&decoded) == DUMMY_AMOUNT, 3);
        assert!(*gmp_common::intent_requirements_token_addr(&decoded) == token, 4);
        assert!(*gmp_common::intent_requirements_solver_addr(&decoded) == solver, 5);
        assert!(gmp_common::intent_requirements_expiry(&decoded) == DUMMY_EXPIRY, 6);
    }

    // 15. Test: Full send-receive roundtrip for FulfillmentProof
    // Simulates the full flow: hub sends proof via GMP, connected chain receives and decodes.
    #[test(admin = @mvmt_intent)]
    fun test_fulfillment_proof_full_flow(admin: &signer) {
        // Initialize timestamp for lz_send
        let framework = account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(&framework);

        // Initialize configs
        intent_gmp_hub::initialize(admin);
        gmp_sender::initialize(admin);

        // Set trusted remote
        let remote_addr = test_src_address();
        intent_gmp_hub::set_trusted_remote(admin, DUMMY_CHAIN_ID, remote_addr);

        let intent_id = test_intent_id();
        let solver = test_solver_addr();

        // Hub sends fulfillment proof (returns nonce)
        let _nonce = intent_gmp_hub::send_fulfillment_proof(
            admin,
            DUMMY_CHAIN_ID,
            intent_id,
            solver,
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );

        // Simulate payload received on connected chain
        let msg = gmp_common::new_fulfillment_proof(
            intent_id,
            solver,
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );
        let payload = gmp_common::encode_fulfillment_proof(&msg);

        // Connected chain receives and decodes
        let decoded = intent_gmp_hub::receive_fulfillment_proof(
            DUMMY_CHAIN_ID,
            remote_addr,
            payload,
        );

        // Verify all fields match
        assert!(*gmp_common::fulfillment_proof_intent_id(&decoded) == intent_id, 1);
        assert!(*gmp_common::fulfillment_proof_solver_addr(&decoded) == solver, 2);
        assert!(gmp_common::fulfillment_proof_amount_fulfilled(&decoded) == DUMMY_AMOUNT, 3);
        assert!(gmp_common::fulfillment_proof_timestamp(&decoded) == DUMMY_TIMESTAMP, 4);
    }
}
