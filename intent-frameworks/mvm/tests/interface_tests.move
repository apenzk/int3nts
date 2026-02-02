#[test_only]
/// Tests for the outflow_validator module (MVM as connected chain).
module mvmt_intent::interface_tests {
    use std::vector;
    use mvmt_intent::outflow_validator;
    use mvmt_intent::gmp_common;

    // ============================================================================
    // TEST CONSTANTS
    // ============================================================================

    const DUMMY_CHAIN_ID: u32 = 1;
    const DUMMY_AMOUNT: u64 = 1000000;
    const DUMMY_EXPIRY: u64 = 1000;

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

    fun test_src_address(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0xCC;
        *vector::borrow_mut(&mut v, 31) = 0xDD;
        v
    }

    // ============================================================================
    // INTERFACE TESTS (mirrors SVM interface_tests.rs)
    // ============================================================================

    /// 2. Test: Receive instruction roundtrip (receive_intent_requirements)
    /// Verifies that the function correctly decodes an IntentRequirements message.
    /// Mirrors SVM's test_receive_instruction_roundtrip.
    #[test]
    fun test_receive_instruction_roundtrip() {
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
}
