#[test_only]
module mvmt_intent::intent_gmp_tests {
    use std::vector;
    use mvmt_intent::intent_gmp_hub;
    use mvmt_intent::outflow_validator;
    use mvmt_intent::gmp_common;

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

    /// 1. Test: send_intent_requirements returns valid encoded payload
    /// Verifies that the function returns a properly encoded IntentRequirements message.
    #[test]
    fun test_send_intent_requirements_returns_encoded_payload() {
        let payload = intent_gmp_hub::send_intent_requirements(
            DUMMY_CHAIN_ID,
            test_intent_id(),
            test_requester_addr(),
            DUMMY_AMOUNT,
            test_token_addr(),
            test_solver_addr(),
            DUMMY_EXPIRY,
        );

        // Verify payload length matches IntentRequirements size (145 bytes)
        assert!(vector::length(&payload) == gmp_common::intent_requirements_size(), 1);

        // Verify discriminator byte
        assert!(*vector::borrow(&payload, 0) == 0x01, 2);
    }

    /// 2. Test: send_intent_requirements payload can be decoded back
    /// Verifies roundtrip: encode via send function, decode via gmp_common.
    #[test]
    fun test_send_intent_requirements_roundtrip() {
        let intent_id = test_intent_id();
        let requester = test_requester_addr();
        let token = test_token_addr();
        let solver = test_solver_addr();

        let payload = intent_gmp_hub::send_intent_requirements(
            DUMMY_CHAIN_ID,
            intent_id,
            requester,
            DUMMY_AMOUNT,
            token,
            solver,
            DUMMY_EXPIRY,
        );

        // Decode and verify fields
        let decoded = gmp_common::decode_intent_requirements(&payload);
        assert!(*gmp_common::intent_requirements_intent_id(&decoded) == intent_id, 1);
        assert!(*gmp_common::intent_requirements_requester_addr(&decoded) == requester, 2);
        assert!(gmp_common::intent_requirements_amount_required(&decoded) == DUMMY_AMOUNT, 3);
        assert!(*gmp_common::intent_requirements_token_addr(&decoded) == token, 4);
        assert!(*gmp_common::intent_requirements_solver_addr(&decoded) == solver, 5);
        assert!(gmp_common::intent_requirements_expiry(&decoded) == DUMMY_EXPIRY, 6);
    }

    // ============================================================================
    // SEND FULFILLMENT PROOF TESTS
    // ============================================================================

    /// 3. Test: send_fulfillment_proof returns valid encoded payload
    /// Verifies that the function returns a properly encoded FulfillmentProof message.
    #[test]
    fun test_send_fulfillment_proof_returns_encoded_payload() {
        let payload = intent_gmp_hub::send_fulfillment_proof(
            DUMMY_CHAIN_ID,
            test_intent_id(),
            test_solver_addr(),
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );

        // Verify payload length matches FulfillmentProof size (81 bytes)
        assert!(vector::length(&payload) == gmp_common::fulfillment_proof_size(), 1);

        // Verify discriminator byte
        assert!(*vector::borrow(&payload, 0) == 0x03, 2);
    }

    /// 4. Test: send_fulfillment_proof payload can be decoded back
    /// Verifies roundtrip: encode via send function, decode via gmp_common.
    #[test]
    fun test_send_fulfillment_proof_roundtrip() {
        let intent_id = test_intent_id();
        let solver = test_solver_addr();

        let payload = intent_gmp_hub::send_fulfillment_proof(
            DUMMY_CHAIN_ID,
            intent_id,
            solver,
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );

        // Decode and verify fields
        let decoded = gmp_common::decode_fulfillment_proof(&payload);
        assert!(*gmp_common::fulfillment_proof_intent_id(&decoded) == intent_id, 1);
        assert!(*gmp_common::fulfillment_proof_solver_addr(&decoded) == solver, 2);
        assert!(gmp_common::fulfillment_proof_amount_fulfilled(&decoded) == DUMMY_AMOUNT, 3);
        assert!(gmp_common::fulfillment_proof_timestamp(&decoded) == DUMMY_TIMESTAMP, 4);
    }

    // ============================================================================
    // RECEIVE ESCROW CONFIRMATION TESTS
    // ============================================================================

    /// 5. Test: receive_escrow_confirmation decodes valid payload
    /// Verifies that the function correctly decodes an EscrowConfirmation message.
    #[test]
    fun test_receive_escrow_confirmation_decodes_payload() {
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
            test_src_address(),
            payload,
        );

        // Verify fields
        assert!(*gmp_common::escrow_confirmation_intent_id(&decoded) == test_intent_id(), 1);
        assert!(*gmp_common::escrow_confirmation_escrow_id(&decoded) == test_escrow_id(), 2);
        assert!(gmp_common::escrow_confirmation_amount_escrowed(&decoded) == DUMMY_AMOUNT, 3);
        assert!(*gmp_common::escrow_confirmation_token_addr(&decoded) == test_token_addr(), 4);
        assert!(*gmp_common::escrow_confirmation_creator_addr(&decoded) == test_creator_addr(), 5);
    }

    // ============================================================================
    // RECEIVE FULFILLMENT PROOF TESTS
    // ============================================================================

    /// 6. Test: receive_fulfillment_proof decodes valid payload
    /// Verifies that the function correctly decodes a FulfillmentProof message.
    #[test]
    fun test_receive_fulfillment_proof_decodes_payload() {
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
            test_src_address(),
            payload,
        );

        // Verify fields
        assert!(*gmp_common::fulfillment_proof_intent_id(&decoded) == test_intent_id(), 1);
        assert!(*gmp_common::fulfillment_proof_solver_addr(&decoded) == test_solver_addr(), 2);
        assert!(gmp_common::fulfillment_proof_amount_fulfilled(&decoded) == DUMMY_AMOUNT, 3);
        assert!(gmp_common::fulfillment_proof_timestamp(&decoded) == DUMMY_TIMESTAMP, 4);
    }

    // ============================================================================
    // HELPER FUNCTION TESTS
    // ============================================================================

    /// 8. Test: bytes_to_bytes32 pads short input
    /// Verifies that inputs shorter than 32 bytes are left-padded with zeros.
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

    /// 9. Test: bytes_to_bytes32 truncates long input
    /// Verifies that inputs longer than 32 bytes are truncated to first 32.
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

    /// 10. Test: bytes_to_bytes32 returns exact 32 bytes unchanged
    /// Verifies that 32-byte inputs are returned unchanged.
    #[test]
    fun test_bytes_to_bytes32_exact_length() {
        let exact = test_intent_id(); // Already 32 bytes

        let result = intent_gmp_hub::bytes_to_bytes32(exact);

        assert!(vector::length(&result) == 32, 1);
        assert!(result == test_intent_id(), 2);
    }

    /// 11. Test: bytes_to_bytes32 handles empty input
    /// Verifies that empty input results in 32 zero bytes.
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
    // RECEIVE INTENT REQUIREMENTS TESTS (MVM as connected chain)
    // ============================================================================

    /// 7. Test: receive_intent_requirements decodes valid payload
    /// Verifies that the function correctly decodes an IntentRequirements message.
    /// This mirrors SVM's LzReceiveRequirements test.
    #[test]
    fun test_receive_intent_requirements_decodes_payload() {
        // Create a valid IntentRequirements payload
        let msg = gmp_common::new_intent_requirements(
            test_intent_id(),
            test_requester_addr(),
            DUMMY_AMOUNT,
            test_token_addr(),
            test_solver_addr(),
            DUMMY_EXPIRY,
        );
        let payload = gmp_common::encode_intent_requirements(&msg);

        // Receive and decode
        let decoded = outflow_validator::receive_intent_requirements(
            DUMMY_CHAIN_ID,
            test_src_address(),
            payload,
        );

        // Verify fields
        assert!(*gmp_common::intent_requirements_intent_id(&decoded) == test_intent_id(), 1);
        assert!(*gmp_common::intent_requirements_requester_addr(&decoded) == test_requester_addr(), 2);
        assert!(gmp_common::intent_requirements_amount_required(&decoded) == DUMMY_AMOUNT, 3);
        assert!(*gmp_common::intent_requirements_token_addr(&decoded) == test_token_addr(), 4);
        assert!(*gmp_common::intent_requirements_solver_addr(&decoded) == test_solver_addr(), 5);
        assert!(gmp_common::intent_requirements_expiry(&decoded) == DUMMY_EXPIRY, 6);
    }

    // ============================================================================
    // INTEGRATION TESTS
    // ============================================================================

    /// 12. Test: Full send-receive roundtrip for IntentRequirements
    /// Simulates the full flow: hub sends requirements, connected chain receives.
    #[test]
    fun test_intent_requirements_full_flow() {
        let intent_id = test_intent_id();
        let requester = test_requester_addr();
        let token = test_token_addr();
        let solver = test_solver_addr();

        // Hub sends requirements
        let payload = intent_gmp_hub::send_intent_requirements(
            DUMMY_CHAIN_ID,
            intent_id,
            requester,
            DUMMY_AMOUNT,
            token,
            solver,
            DUMMY_EXPIRY,
        );

        // Payload can be decoded by gmp_common (simulating connected chain)
        let decoded = gmp_common::decode_intent_requirements(&payload);

        // Verify all fields match
        assert!(*gmp_common::intent_requirements_intent_id(&decoded) == intent_id, 1);
        assert!(*gmp_common::intent_requirements_requester_addr(&decoded) == requester, 2);
        assert!(gmp_common::intent_requirements_amount_required(&decoded) == DUMMY_AMOUNT, 3);
        assert!(*gmp_common::intent_requirements_token_addr(&decoded) == token, 4);
        assert!(*gmp_common::intent_requirements_solver_addr(&decoded) == solver, 5);
        assert!(gmp_common::intent_requirements_expiry(&decoded) == DUMMY_EXPIRY, 6);
    }

    /// 13. Test: Full send-receive roundtrip for FulfillmentProof
    /// Simulates the full flow: hub sends proof, connected chain receives and decodes.
    #[test]
    fun test_fulfillment_proof_full_flow() {
        let intent_id = test_intent_id();
        let solver = test_solver_addr();

        // Hub sends fulfillment proof
        let payload = intent_gmp_hub::send_fulfillment_proof(
            DUMMY_CHAIN_ID,
            intent_id,
            solver,
            DUMMY_AMOUNT,
            DUMMY_TIMESTAMP,
        );

        // Connected chain receives and decodes
        let decoded = intent_gmp_hub::receive_fulfillment_proof(
            DUMMY_CHAIN_ID,
            test_src_address(),
            payload,
        );

        // Verify all fields match
        assert!(*gmp_common::fulfillment_proof_intent_id(&decoded) == intent_id, 1);
        assert!(*gmp_common::fulfillment_proof_solver_addr(&decoded) == solver, 2);
        assert!(gmp_common::fulfillment_proof_amount_fulfilled(&decoded) == DUMMY_AMOUNT, 3);
        assert!(gmp_common::fulfillment_proof_timestamp(&decoded) == DUMMY_TIMESTAMP, 4);
    }
}
