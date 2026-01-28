#[test_only]
module mvmt_intent::gmp_common_tests {
    use std::vector;
    use mvmt_intent::gmp_messages;

    // ============================================================================
    // TEST CONSTANTS
    // ============================================================================

    const DUMMY_AMOUNT: u64 = 1000000;
    const DUMMY_EXPIRY: u64 = 1000;
    const DUMMY_TIMESTAMP: u64 = 1000;

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

    fun repeat(byte: u8, n: u64): vector<u8> {
        let v = vector::empty<u8>();
        let i = 0;
        while (i < n) {
            vector::push_back(&mut v, byte);
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

    fun test_addr_1(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0x11;
        *vector::borrow_mut(&mut v, 31) = 0x22;
        v
    }

    fun test_addr_2(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0x33;
        *vector::borrow_mut(&mut v, 31) = 0x44;
        v
    }

    fun test_addr_3(): vector<u8> {
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 0) = 0x55;
        *vector::borrow_mut(&mut v, 31) = 0x66;
        v
    }

    fun test_evm_addr(): vector<u8> {
        // EVM 20-byte address left-padded with 12 zero bytes
        let v = zeros(32);
        *vector::borrow_mut(&mut v, 12) = 0xDE;
        *vector::borrow_mut(&mut v, 13) = 0xAD;
        *vector::borrow_mut(&mut v, 30) = 0xBE;
        *vector::borrow_mut(&mut v, 31) = 0xEF;
        v
    }

    fun set_be_u64(v: &mut vector<u8>, offset: u64, val: u64) {
        *vector::borrow_mut(v, offset)     = ((val >> 56) & 0xFF as u8);
        *vector::borrow_mut(v, offset + 1) = ((val >> 48) & 0xFF as u8);
        *vector::borrow_mut(v, offset + 2) = ((val >> 40) & 0xFF as u8);
        *vector::borrow_mut(v, offset + 3) = ((val >> 32) & 0xFF as u8);
        *vector::borrow_mut(v, offset + 4) = ((val >> 24) & 0xFF as u8);
        *vector::borrow_mut(v, offset + 5) = ((val >> 16) & 0xFF as u8);
        *vector::borrow_mut(v, offset + 6) = ((val >> 8) & 0xFF as u8);
        *vector::borrow_mut(v, offset + 7) = ((val & 0xFF) as u8);
    }

    // ============================================================================
    // INTENT REQUIREMENTS (0x01) TESTS
    // ============================================================================

    //1. Test: IntentRequirements Encoded Size
    //Why: A size mismatch between chains would cause the receiver to read beyond
    //the buffer or miss trailing fields, silently corrupting every message.
    #[test]
    fun test_intent_requirements_encode_size() {
        let msg = gmp_messages::new_intent_requirements(
            zeros(32), zeros(32), 0, zeros(32), zeros(32), 0,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        assert!(vector::length(&encoded) == gmp_messages::intent_requirements_size(), 1);
        assert!(vector::length(&encoded) == 145, 2);
    }

    //2. Test: IntentRequirements Discriminator Byte
    //Why: The receiver reads this byte first to decide which struct to decode into.
    //A wrong discriminator causes the message to be interpreted as the wrong type.
    #[test]
    fun test_intent_requirements_discriminator() {
        let msg = gmp_messages::new_intent_requirements(
            zeros(32), zeros(32), 0, zeros(32), zeros(32), 0,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        assert!(*vector::borrow(&encoded, 0) == 0x01, 1);
    }

    //3. Test: IntentRequirements Encode/Decode Roundtrip
    //Why: If encode and decode are not exact inverses, data is silently corrupted
    //when messages cross chains — e.g. wrong solver gets the escrow funds.
    #[test]
    fun test_intent_requirements_roundtrip() {
        let msg = gmp_messages::new_intent_requirements(
            test_intent_id(), test_addr_1(), DUMMY_AMOUNT,
            test_addr_2(), test_addr_3(), DUMMY_EXPIRY,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        let decoded = gmp_messages::decode_intent_requirements(&encoded);
        assert!(decoded == msg, 1);
    }

    //4. Test: IntentRequirements Big-Endian Amount
    //Why: Move is little-endian internally. If encode accidentally uses native
    //order, Solidity (big-endian) reads a different amount, causing wrong escrow values.
    #[test]
    fun test_intent_requirements_big_endian_amount() {
        let msg = gmp_messages::new_intent_requirements(
            zeros(32), zeros(32),
            0x0102030405060708, // sequential bytes (01..08) to assert big-endian order
            zeros(32), zeros(32), 0,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        // amount_required at offset 65..73, big-endian
        assert!(*vector::borrow(&encoded, 65) == 0x01, 1);
        assert!(*vector::borrow(&encoded, 66) == 0x02, 2);
        assert!(*vector::borrow(&encoded, 67) == 0x03, 3);
        assert!(*vector::borrow(&encoded, 68) == 0x04, 4);
        assert!(*vector::borrow(&encoded, 69) == 0x05, 5);
        assert!(*vector::borrow(&encoded, 70) == 0x06, 6);
        assert!(*vector::borrow(&encoded, 71) == 0x07, 7);
        assert!(*vector::borrow(&encoded, 72) == 0x08, 8);
    }

    //5. Test: IntentRequirements Big-Endian Expiry
    //Why: Wrong endianness on expiry could cause escrows to expire at the wrong time
    //or never expire, locking funds permanently.
    #[test]
    fun test_intent_requirements_big_endian_expiry() {
        let msg = gmp_messages::new_intent_requirements(
            zeros(32), zeros(32), 0, zeros(32), zeros(32),
            0xAABBCCDD00112233, // distinct byte pairs to assert big-endian order
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        // expiry at offset 137..145, big-endian
        assert!(*vector::borrow(&encoded, 137) == 0xAA, 1);
        assert!(*vector::borrow(&encoded, 138) == 0xBB, 2);
        assert!(*vector::borrow(&encoded, 139) == 0xCC, 3);
        assert!(*vector::borrow(&encoded, 140) == 0xDD, 4);
        assert!(*vector::borrow(&encoded, 141) == 0x00, 5);
        assert!(*vector::borrow(&encoded, 142) == 0x11, 6);
        assert!(*vector::borrow(&encoded, 143) == 0x22, 7);
        assert!(*vector::borrow(&encoded, 144) == 0x33, 8);
    }

    //6. Test: IntentRequirements Field Offsets
    //Why: All chains decode by slicing at fixed offsets. If any field starts at the
    //wrong byte, that field and every field after it reads wrong data.
    #[test]
    fun test_intent_requirements_field_offsets() {
        let msg = gmp_messages::new_intent_requirements(
            test_intent_id(), test_addr_1(), DUMMY_AMOUNT,
            test_addr_2(), test_addr_3(), DUMMY_EXPIRY,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);

        // Offset 0: discriminator
        assert!(*vector::borrow(&encoded, 0) == 0x01, 1);
        // Offset 1: intent_id[0]
        assert!(*vector::borrow(&encoded, 1) == 0xAA, 2);
        // Offset 32: intent_id[31]
        assert!(*vector::borrow(&encoded, 32) == 0xBB, 3);
        // Offset 33: requester_addr[0]
        assert!(*vector::borrow(&encoded, 33) == 0x11, 4);
        // Offset 64: requester_addr[31]
        assert!(*vector::borrow(&encoded, 64) == 0x22, 5);
        // Offset 73: token_addr[0]
        assert!(*vector::borrow(&encoded, 73) == 0x33, 6);
        // Offset 104: token_addr[31]
        assert!(*vector::borrow(&encoded, 104) == 0x44, 7);
        // Offset 105: solver_addr[0]
        assert!(*vector::borrow(&encoded, 105) == 0x55, 8);
        // Offset 136: solver_addr[31]
        assert!(*vector::borrow(&encoded, 136) == 0x66, 9);
    }

    //7. Test: IntentRequirements EVM Address Encoding
    //Why: EVM addresses are 20 bytes left-padded to 32. If padding bytes are corrupted,
    //the EVM contract won't recognize the address and funds go to a wrong account.
    #[test]
    fun test_intent_requirements_evm_address() {
        let msg = gmp_messages::new_intent_requirements(
            zeros(32), test_evm_addr(), 0, zeros(32), zeros(32), 0,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        // EVM address: 12 zero bytes then the 20-byte address
        let i = 33;
        while (i < 45) {
            assert!(*vector::borrow(&encoded, i) == 0x00, 100 + i);
            i = i + 1;
        };
        assert!(*vector::borrow(&encoded, 45) == 0xDE, 1);
        assert!(*vector::borrow(&encoded, 46) == 0xAD, 2);
    }

    // ============================================================================
    // ESCROW CONFIRMATION (0x02) TESTS
    // ============================================================================

    //8. Test: EscrowConfirmation Encoded Size
    //Why: The hub decodes this message to confirm an escrow was created on the
    //connected chain. A wrong size means the hub reads garbage and may release
    //funds for an escrow that doesn't exist.
    #[test]
    fun test_escrow_confirmation_encode_size() {
        let msg = gmp_messages::new_escrow_confirmation(
            zeros(32), zeros(32), 0, zeros(32), zeros(32),
        );
        let encoded = gmp_messages::encode_escrow_confirmation(&msg);
        assert!(vector::length(&encoded) == gmp_messages::escrow_confirmation_size(), 1);
        assert!(vector::length(&encoded) == 137, 2);
    }

    //9. Test: EscrowConfirmation Discriminator Byte
    //Why: The hub uses this byte to route the message to the escrow confirmation
    //handler. A wrong value would route it to the wrong handler.
    #[test]
    fun test_escrow_confirmation_discriminator() {
        let msg = gmp_messages::new_escrow_confirmation(
            zeros(32), zeros(32), 0, zeros(32), zeros(32),
        );
        let encoded = gmp_messages::encode_escrow_confirmation(&msg);
        assert!(*vector::borrow(&encoded, 0) == 0x02, 1);
    }

    //10. Test: EscrowConfirmation Encode/Decode Roundtrip
    //Why: The connected chain encodes this and the hub decodes it. If they disagree,
    //the hub could confirm the wrong escrow or credit the wrong creator.
    #[test]
    fun test_escrow_confirmation_roundtrip() {
        let msg = gmp_messages::new_escrow_confirmation(
            test_intent_id(), test_addr_1(), DUMMY_AMOUNT,
            test_addr_2(), test_addr_3(),
        );
        let encoded = gmp_messages::encode_escrow_confirmation(&msg);
        let decoded = gmp_messages::decode_escrow_confirmation(&encoded);
        assert!(decoded == msg, 1);
    }

    //11. Test: EscrowConfirmation Big-Endian Amount
    //Why: The hub checks that escrow amount matches the intent requirement. Wrong
    //endianness means the amounts never match even when they should.
    #[test]
    fun test_escrow_confirmation_big_endian_amount() {
        let msg = gmp_messages::new_escrow_confirmation(
            zeros(32), zeros(32),
            0x0A0B0C0D0E0F1011, // sequential bytes (0A..11) to assert big-endian order
            zeros(32), zeros(32),
        );
        let encoded = gmp_messages::encode_escrow_confirmation(&msg);
        // amount_escrowed at offset 65..73
        assert!(*vector::borrow(&encoded, 65) == 0x0A, 1);
        assert!(*vector::borrow(&encoded, 66) == 0x0B, 2);
        assert!(*vector::borrow(&encoded, 67) == 0x0C, 3);
        assert!(*vector::borrow(&encoded, 68) == 0x0D, 4);
        assert!(*vector::borrow(&encoded, 69) == 0x0E, 5);
        assert!(*vector::borrow(&encoded, 70) == 0x0F, 6);
        assert!(*vector::borrow(&encoded, 71) == 0x10, 7);
        assert!(*vector::borrow(&encoded, 72) == 0x11, 8);
    }

    //12. Test: EscrowConfirmation Field Offsets
    //Why: The hub reads escrow_id, token_addr, and creator_addr by slicing at fixed
    //offsets. A shifted offset means the hub associates the wrong creator with the escrow.
    #[test]
    fun test_escrow_confirmation_field_offsets() {
        let msg = gmp_messages::new_escrow_confirmation(
            test_intent_id(), test_addr_1(), DUMMY_AMOUNT,
            test_addr_2(), test_addr_3(),
        );
        let encoded = gmp_messages::encode_escrow_confirmation(&msg);

        assert!(*vector::borrow(&encoded, 0) == 0x02, 1);
        assert!(*vector::borrow(&encoded, 1) == 0xAA, 2);   // intent_id[0]
        assert!(*vector::borrow(&encoded, 32) == 0xBB, 3);  // intent_id[31]
        assert!(*vector::borrow(&encoded, 33) == 0x11, 4);  // escrow_id[0]
        assert!(*vector::borrow(&encoded, 64) == 0x22, 5);  // escrow_id[31]
        assert!(*vector::borrow(&encoded, 73) == 0x33, 6);  // token_addr[0]
        assert!(*vector::borrow(&encoded, 104) == 0x44, 7); // token_addr[31]
        assert!(*vector::borrow(&encoded, 105) == 0x55, 8); // creator_addr[0]
        assert!(*vector::borrow(&encoded, 136) == 0x66, 9); // creator_addr[31]
    }

    // ============================================================================
    // FULFILLMENT PROOF (0x03) TESTS
    // ============================================================================

    //13. Test: FulfillmentProof Encoded Size
    //Why: The connected chain decodes this to release escrowed funds. A wrong size
    //means the escrow contract reads garbage and funds stay locked.
    #[test]
    fun test_fulfillment_proof_encode_size() {
        let msg = gmp_messages::new_fulfillment_proof(
            zeros(32), zeros(32), 0, 0,
        );
        let encoded = gmp_messages::encode_fulfillment_proof(&msg);
        assert!(vector::length(&encoded) == gmp_messages::fulfillment_proof_size(), 1);
        assert!(vector::length(&encoded) == 81, 2);
    }

    //14. Test: FulfillmentProof Discriminator Byte
    //Why: The connected chain uses this byte to route to the fulfillment handler.
    //A wrong value would route it to the wrong handler or reject the message entirely.
    #[test]
    fun test_fulfillment_proof_discriminator() {
        let msg = gmp_messages::new_fulfillment_proof(
            zeros(32), zeros(32), 0, 0,
        );
        let encoded = gmp_messages::encode_fulfillment_proof(&msg);
        assert!(*vector::borrow(&encoded, 0) == 0x03, 1);
    }

    //15. Test: FulfillmentProof Encode/Decode Roundtrip
    //Why: The hub encodes this and the connected chain decodes it to release funds.
    //A mismatch means the escrow releases to the wrong solver or wrong amount.
    #[test]
    fun test_fulfillment_proof_roundtrip() {
        let msg = gmp_messages::new_fulfillment_proof(
            test_intent_id(), test_addr_1(), DUMMY_AMOUNT, DUMMY_TIMESTAMP,
        );
        let encoded = gmp_messages::encode_fulfillment_proof(&msg);
        let decoded = gmp_messages::decode_fulfillment_proof(&encoded);
        assert!(decoded == msg, 1);
    }

    //16. Test: FulfillmentProof Big-Endian Fields
    //Why: The escrow contract checks amount_fulfilled against the locked amount.
    //Wrong endianness means the check fails and funds stay locked forever.
    #[test]
    fun test_fulfillment_proof_big_endian_fields() {
        let msg = gmp_messages::new_fulfillment_proof(
            zeros(32), zeros(32),
            0x0102030405060708, // sequential bytes (01..08) to assert big-endian order
            0xAABBCCDD00112233, // distinct byte pairs to assert big-endian order
        );
        let encoded = gmp_messages::encode_fulfillment_proof(&msg);
        // amount_fulfilled at offset 65..73
        assert!(*vector::borrow(&encoded, 65) == 0x01, 1);
        assert!(*vector::borrow(&encoded, 72) == 0x08, 2);
        // timestamp at offset 73..81
        assert!(*vector::borrow(&encoded, 73) == 0xAA, 3);
        assert!(*vector::borrow(&encoded, 80) == 0x33, 4);
    }

    //17. Test: FulfillmentProof Field Offsets
    //Why: The escrow contract reads solver_addr by offset to verify the solver.
    //A shifted offset means a different address is read and the wrong party gets funds.
    #[test]
    fun test_fulfillment_proof_field_offsets() {
        let msg = gmp_messages::new_fulfillment_proof(
            test_intent_id(), test_addr_1(), DUMMY_AMOUNT, DUMMY_TIMESTAMP,
        );
        let encoded = gmp_messages::encode_fulfillment_proof(&msg);

        assert!(*vector::borrow(&encoded, 0) == 0x03, 1);
        assert!(*vector::borrow(&encoded, 1) == 0xAA, 2);   // intent_id[0]
        assert!(*vector::borrow(&encoded, 32) == 0xBB, 3);  // intent_id[31]
        assert!(*vector::borrow(&encoded, 33) == 0x11, 4);  // solver_addr[0]
        assert!(*vector::borrow(&encoded, 64) == 0x22, 5);  // solver_addr[31]
    }

    // ============================================================================
    // PEEK MESSAGE TYPE TESTS
    // ============================================================================

    //18. Test: Peek IntentRequirements Type
    //Why: The lzReceive handler calls peek first to decide which decode path to take.
    //A wrong peek result routes the message to the wrong handler.
    #[test]
    fun test_peek_intent_requirements() {
        let msg = gmp_messages::new_intent_requirements(
            zeros(32), zeros(32), 0, zeros(32), zeros(32), 0,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        assert!(gmp_messages::peek_message_type(&encoded) == 0x01, 1);
    }

    //19. Test: Peek EscrowConfirmation Type
    //Why: Each discriminator value must map to the correct type.
    //Tests 18-20 together ensure all three types are correctly identified.
    #[test]
    fun test_peek_escrow_confirmation() {
        let msg = gmp_messages::new_escrow_confirmation(
            zeros(32), zeros(32), 0, zeros(32), zeros(32),
        );
        let encoded = gmp_messages::encode_escrow_confirmation(&msg);
        assert!(gmp_messages::peek_message_type(&encoded) == 0x02, 1);
    }

    //20. Test: Peek FulfillmentProof Type
    //Why: Each discriminator value must map to the correct type.
    //Tests 18-20 together ensure all three types are correctly identified.
    #[test]
    fun test_peek_fulfillment_proof() {
        let msg = gmp_messages::new_fulfillment_proof(
            zeros(32), zeros(32), 0, 0,
        );
        let encoded = gmp_messages::encode_fulfillment_proof(&msg);
        assert!(gmp_messages::peek_message_type(&encoded) == 0x03, 1);
    }

    // ============================================================================
    // ERROR CONDITION TESTS
    // ============================================================================

    //21. Test: Reject Wrong Discriminator
    //Why: Without this check, a buffer encoded as EscrowConfirmation could be
    //decoded as IntentRequirements, silently producing garbage fields.
    #[test]
    #[expected_failure(abort_code = 1, location = mvmt_intent::gmp_messages)]
    fun test_reject_wrong_discriminator() {
        let msg = gmp_messages::new_intent_requirements(
            zeros(32), zeros(32), 0, zeros(32), zeros(32), 0,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        // Change discriminator to EscrowConfirmation
        *vector::borrow_mut(&mut encoded, 0) = 0x02;
        gmp_messages::decode_intent_requirements(&encoded);
    }

    //22. Test: Reject Wrong Length
    //Why: A truncated buffer could cause out-of-bounds reads. A padded buffer could
    //contain trailing garbage. Both must be rejected to prevent silent corruption.
    #[test]
    #[expected_failure(abort_code = 2, location = mvmt_intent::gmp_messages)]
    fun test_reject_wrong_length() {
        let data = repeat(0x01, 10);
        gmp_messages::decode_intent_requirements(&data);
    }

    //23. Test: Reject Empty Buffer
    //Why: GMP messages could arrive empty due to network errors or malicious senders.
    //Decode must abort, not panic or read uninitialized memory.
    #[test]
    #[expected_failure(abort_code = 2, location = mvmt_intent::gmp_messages)]
    fun test_reject_empty_buffer() {
        let empty = vector::empty<u8>();
        gmp_messages::decode_intent_requirements(&empty);
    }

    //24. Test: Peek Rejects Empty Buffer
    //Why: peek is called before decode, so it's the first line of defense. It must
    //not index out of bounds on empty input from the network.
    #[test]
    #[expected_failure(abort_code = 2, location = mvmt_intent::gmp_messages)]
    fun test_peek_reject_empty_buffer() {
        let empty = vector::empty<u8>();
        gmp_messages::peek_message_type(&empty);
    }

    //25. Test: Peek Rejects Unknown Type
    //Why: If a new message type is added on one chain but not another, the receiver
    //must reject it cleanly rather than silently misinterpreting the payload.
    #[test]
    #[expected_failure(abort_code = 3, location = mvmt_intent::gmp_messages)]
    fun test_peek_reject_unknown_type() {
        let data = vector::empty<u8>();
        vector::push_back(&mut data, 0xFF);
        gmp_messages::peek_message_type(&data);
    }

    //26. Test: Reject Wrong Discriminator for EscrowConfirmation
    //Why: Each message type has its own decode function. If EscrowConfirmation accepts
    //a 0x01 buffer, it would silently misinterpret IntentRequirements fields.
    #[test]
    #[expected_failure(abort_code = 1, location = mvmt_intent::gmp_messages)]
    fun test_reject_wrong_discriminator_escrow_confirmation() {
        let data = zeros(137);
        *vector::borrow_mut(&mut data, 0) = 0x01; // should be 0x02
        gmp_messages::decode_escrow_confirmation(&data);
    }

    //27. Test: Reject Wrong Discriminator for FulfillmentProof
    //Why: Same reasoning as test 26 — each decode function must enforce its own
    //discriminator to prevent cross-type misinterpretation.
    #[test]
    #[expected_failure(abort_code = 1, location = mvmt_intent::gmp_messages)]
    fun test_reject_wrong_discriminator_fulfillment_proof() {
        let data = zeros(81);
        *vector::borrow_mut(&mut data, 0) = 0x01; // should be 0x03
        gmp_messages::decode_fulfillment_proof(&data);
    }

    //28. Test: Reject Wrong Length for EscrowConfirmation
    //Why: Each message type has a different expected size. A bug in the
    //EscrowConfirmation length check is independent of the IntentRequirements one.
    #[test]
    #[expected_failure(abort_code = 2, location = mvmt_intent::gmp_messages)]
    fun test_reject_wrong_length_escrow_confirmation() {
        let data = repeat(0x02, 10);
        gmp_messages::decode_escrow_confirmation(&data);
    }

    //29. Test: Reject Wrong Length for FulfillmentProof
    //Why: Each message type checks its own expected size independently. A bug in one
    //length check doesn't imply the others are correct.
    #[test]
    #[expected_failure(abort_code = 2, location = mvmt_intent::gmp_messages)]
    fun test_reject_wrong_length_fulfillment_proof() {
        let data = repeat(0x03, 10);
        gmp_messages::decode_fulfillment_proof(&data);
    }

    //30. Test: Reject Off-By-One Length
    //Why: Off-by-one is the most likely length check bug. Testing exact_size-1
    //catches this where a wildly wrong size like 10 might not.
    #[test]
    #[expected_failure(abort_code = 2, location = mvmt_intent::gmp_messages)]
    fun test_reject_off_by_one_length() {
        // IntentRequirements: 145 bytes, try 144
        let data = repeat(0x01, 144);
        gmp_messages::decode_intent_requirements(&data);
    }

    // ============================================================================
    // KNOWN BYTE SEQUENCE TESTS
    // ============================================================================

    //31. Test: Decode Known IntentRequirements Bytes
    //Why: Roundtrip tests use encode+decode together, so a bug that is symmetric
    //in both functions would be invisible. This test decodes a hand-built buffer
    //to catch bugs that roundtrip alone cannot.
    #[test]
    fun test_decode_known_intent_requirements_bytes() {
        let data = zeros(145);
        *vector::borrow_mut(&mut data, 0) = 0x01;    // discriminator
        *vector::borrow_mut(&mut data, 1) = 0xFF;    // intent_id[0]
        *vector::borrow_mut(&mut data, 32) = 0xEE;   // intent_id[31]
        *vector::borrow_mut(&mut data, 33) = 0xDD;   // requester_addr[0]
        set_be_u64(&mut data, 65, DUMMY_AMOUNT);
        *vector::borrow_mut(&mut data, 73) = 0xCC;   // token_addr[0]
        *vector::borrow_mut(&mut data, 105) = 0xBB;  // solver_addr[0]
        set_be_u64(&mut data, 137, DUMMY_EXPIRY);

        let msg = gmp_messages::decode_intent_requirements(&data);
        assert!(*vector::borrow(gmp_messages::intent_requirements_intent_id(&msg), 0) == 0xFF, 1);
        assert!(*vector::borrow(gmp_messages::intent_requirements_intent_id(&msg), 31) == 0xEE, 2);
        assert!(*vector::borrow(gmp_messages::intent_requirements_requester_addr(&msg), 0) == 0xDD, 3);
        assert!(gmp_messages::intent_requirements_amount_required(&msg) == DUMMY_AMOUNT, 4);
        assert!(*vector::borrow(gmp_messages::intent_requirements_token_addr(&msg), 0) == 0xCC, 5);
        assert!(*vector::borrow(gmp_messages::intent_requirements_solver_addr(&msg), 0) == 0xBB, 6);
        assert!(gmp_messages::intent_requirements_expiry(&msg) == DUMMY_EXPIRY, 7);
    }

    //32. Test: Decode Known EscrowConfirmation Bytes
    //Why: Same reasoning as test 31 — decoding a hand-built buffer catches bugs
    //that are symmetric in encode and decode.
    #[test]
    fun test_decode_known_escrow_confirmation_bytes() {
        let data = zeros(137);
        *vector::borrow_mut(&mut data, 0) = 0x02;
        *vector::borrow_mut(&mut data, 1) = 0xAA;    // intent_id[0]
        *vector::borrow_mut(&mut data, 33) = 0xBB;   // escrow_id[0]
        set_be_u64(&mut data, 65, DUMMY_AMOUNT);
        *vector::borrow_mut(&mut data, 73) = 0xCC;   // token_addr[0]
        *vector::borrow_mut(&mut data, 105) = 0xDD;  // creator_addr[0]

        let msg = gmp_messages::decode_escrow_confirmation(&data);
        assert!(*vector::borrow(gmp_messages::escrow_confirmation_intent_id(&msg), 0) == 0xAA, 1);
        assert!(*vector::borrow(gmp_messages::escrow_confirmation_escrow_id(&msg), 0) == 0xBB, 2);
        assert!(gmp_messages::escrow_confirmation_amount_escrowed(&msg) == DUMMY_AMOUNT, 3);
        assert!(*vector::borrow(gmp_messages::escrow_confirmation_token_addr(&msg), 0) == 0xCC, 4);
        assert!(*vector::borrow(gmp_messages::escrow_confirmation_creator_addr(&msg), 0) == 0xDD, 5);
    }

    //33. Test: Decode Known FulfillmentProof Bytes
    //Why: Same reasoning as test 31 — decoding a hand-built buffer catches bugs
    //that are symmetric in encode and decode.
    #[test]
    fun test_decode_known_fulfillment_proof_bytes() {
        let data = zeros(81);
        *vector::borrow_mut(&mut data, 0) = 0x03;
        *vector::borrow_mut(&mut data, 1) = 0xAA;    // intent_id[0]
        *vector::borrow_mut(&mut data, 33) = 0xBB;   // solver_addr[0]
        set_be_u64(&mut data, 65, DUMMY_AMOUNT);
        set_be_u64(&mut data, 73, DUMMY_TIMESTAMP);

        let msg = gmp_messages::decode_fulfillment_proof(&data);
        assert!(*vector::borrow(gmp_messages::fulfillment_proof_intent_id(&msg), 0) == 0xAA, 1);
        assert!(*vector::borrow(gmp_messages::fulfillment_proof_solver_addr(&msg), 0) == 0xBB, 2);
        assert!(gmp_messages::fulfillment_proof_amount_fulfilled(&msg) == DUMMY_AMOUNT, 3);
        assert!(gmp_messages::fulfillment_proof_timestamp(&msg) == DUMMY_TIMESTAMP, 4);
    }

    // ============================================================================
    // BOUNDARY CONDITION TESTS
    // ============================================================================

    //34. Test: Max u64 Amount Roundtrip
    //Why: u64::MAX (0xFFFFFFFFFFFFFFFF) is all 0xFF bytes in big-endian. This
    //catches sign-extension bugs or off-by-one errors at the maximum boundary.
    #[test]
    fun test_max_u64_amount_roundtrip() {
        let max_u64: u64 = 18446744073709551615;
        let ff32 = repeat(0xFF, 32);
        let msg = gmp_messages::new_intent_requirements(
            copy ff32, copy ff32, max_u64, copy ff32, copy ff32, max_u64,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        let decoded = gmp_messages::decode_intent_requirements(&encoded);
        assert!(gmp_messages::intent_requirements_amount_required(&decoded) == max_u64, 1);
        assert!(gmp_messages::intent_requirements_expiry(&decoded) == max_u64, 2);
    }

    //35. Test: Zero Solver Address Means Any Solver
    //Why: bytes32(0) is a sentinel meaning "any solver may fulfill this intent".
    //If it doesn't roundtrip, the open-solver feature silently breaks and intents
    //become unfulfillable.
    #[test]
    fun test_zero_solver_addr_means_any() {
        let msg = gmp_messages::new_intent_requirements(
            test_intent_id(), test_addr_1(), DUMMY_AMOUNT,
            test_addr_2(), zeros(32), DUMMY_EXPIRY,
        );
        let encoded = gmp_messages::encode_intent_requirements(&msg);
        let decoded = gmp_messages::decode_intent_requirements(&encoded);
        assert!(*gmp_messages::intent_requirements_solver_addr(&decoded) == zeros(32), 1);
    }
}
