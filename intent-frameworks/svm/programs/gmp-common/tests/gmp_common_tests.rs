use gmp_common::messages::*;

// ============================================================================
// TEST HELPERS
// ============================================================================

const DUMMY_AMOUNT: u64 = 1_000_000;
const DUMMY_EXPIRY: u64 = 1000;
const DUMMY_TIMESTAMP: u64 = 1000;

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

// Test vectors — known inputs with hand-computed expected bytes.
// These same vectors should be used by MVM tests (Commit 6) to verify
// cross-chain encoding compatibility.

fn test_intent_id() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0xAA;
    id[31] = 0xBB;
    id
}

fn test_addr_1() -> [u8; 32] {
    let mut addr = [0u8; 32];
    addr[0] = 0x11;
    addr[31] = 0x22;
    addr
}

fn test_addr_2() -> [u8; 32] {
    let mut addr = [0u8; 32];
    addr[0] = 0x33;
    addr[31] = 0x44;
    addr
}

fn test_addr_3() -> [u8; 32] {
    let mut addr = [0u8; 32];
    addr[0] = 0x55;
    addr[31] = 0x66;
    addr
}

fn test_evm_addr() -> [u8; 32] {
    // EVM 20-byte address left-padded with 12 zero bytes
    let mut addr = [0u8; 32];
    addr[12] = 0xDE;
    addr[13] = 0xAD;
    addr[30] = 0xBE;
    addr[31] = 0xEF;
    addr
}

// ============================================================================
// INTENT REQUIREMENTS (0x01) TESTS
// ============================================================================

/// 1. Test: IntentRequirements Encoded Size
/// Verifies the encoded message is exactly 145 bytes per the wire format spec.
/// Why: A size mismatch between chains would cause the receiver to read beyond
/// the buffer or miss trailing fields, silently corrupting every message.
#[test]
fn test_intent_requirements_encode_size() {
    let msg = IntentRequirements {
        intent_id: [0u8; 32],
        requester_addr: [0u8; 32],
        amount_required: 0,
        token_addr: [0u8; 32],
        solver_addr: [0u8; 32],
        expiry: 0,
    };
    let encoded = msg.encode();
    assert_eq!(encoded.len(), INTENT_REQUIREMENTS_SIZE);
    assert_eq!(encoded.len(), 145);
}

/// 2. Test: IntentRequirements Discriminator Byte
/// Verifies the first byte is 0x01.
/// Why: The receiver reads this byte first to decide which struct to decode into.
/// A wrong discriminator causes the message to be interpreted as the wrong type.
#[test]
fn test_intent_requirements_discriminator() {
    let msg = IntentRequirements {
        intent_id: [0u8; 32],
        requester_addr: [0u8; 32],
        amount_required: 0,
        token_addr: [0u8; 32],
        solver_addr: [0u8; 32],
        expiry: 0,
    };
    let encoded = msg.encode();
    assert_eq!(encoded[0], 0x01);
}

/// 3. Test: IntentRequirements Encode/Decode Roundtrip
/// Verifies that encoding then decoding produces the original message.
/// Why: If encode and decode are not exact inverses, data is silently corrupted
/// when messages cross chains — e.g. wrong solver gets the escrow funds.
#[test]
fn test_intent_requirements_roundtrip() {
    let msg = IntentRequirements {
        intent_id: test_intent_id(),
        requester_addr: test_addr_1(),
        amount_required: DUMMY_AMOUNT,
        token_addr: test_addr_2(),
        solver_addr: test_addr_3(),
        expiry: DUMMY_EXPIRY,
    };
    let encoded = msg.encode();
    let decoded = IntentRequirements::decode(&encoded).unwrap();
    assert_eq!(decoded, msg);
}

/// 4. Test: IntentRequirements Big-Endian Amount
/// Verifies amount_required is encoded in big-endian byte order at offset 65..73.
/// Why: Rust is little-endian on most platforms. If encode accidentally uses native
/// order, Solidity (big-endian) reads a different amount, causing wrong escrow values.
#[test]
fn test_intent_requirements_big_endian_amount() {
    let msg = IntentRequirements {
        intent_id: [0u8; 32],
        requester_addr: [0u8; 32],
        amount_required: 0x0102030405060708, // sequential bytes (01..08) to assert each position confirms big-endian order
        token_addr: [0u8; 32],
        solver_addr: [0u8; 32],
        expiry: 0,
    };
    let encoded = msg.encode();
    // amount_required at offset 65..73, big-endian
    assert_eq!(encoded[65], 0x01);
    assert_eq!(encoded[66], 0x02);
    assert_eq!(encoded[67], 0x03);
    assert_eq!(encoded[68], 0x04);
    assert_eq!(encoded[69], 0x05);
    assert_eq!(encoded[70], 0x06);
    assert_eq!(encoded[71], 0x07);
    assert_eq!(encoded[72], 0x08);
}

/// 5. Test: IntentRequirements Big-Endian Expiry
/// Verifies expiry is encoded in big-endian byte order at offset 137..145.
/// Why: Wrong endianness on expiry could cause escrows to expire at the wrong time
/// or never expire, locking funds permanently.
#[test]
fn test_intent_requirements_big_endian_expiry() {
    let msg = IntentRequirements {
        intent_id: [0u8; 32],
        requester_addr: [0u8; 32],
        amount_required: 0,
        token_addr: [0u8; 32],
        solver_addr: [0u8; 32],
        expiry: 0xAABBCCDD00112233, // distinct byte pairs to assert each position confirms big-endian order
    };
    let encoded = msg.encode();
    // expiry at offset 137..145, big-endian
    assert_eq!(encoded[137], 0xAA);
    assert_eq!(encoded[138], 0xBB);
    assert_eq!(encoded[139], 0xCC);
    assert_eq!(encoded[140], 0xDD);
    assert_eq!(encoded[141], 0x00);
    assert_eq!(encoded[142], 0x11);
    assert_eq!(encoded[143], 0x22);
    assert_eq!(encoded[144], 0x33);
}

/// 6. Test: IntentRequirements Field Offsets
/// Verifies each field starts at the correct byte offset per the wire format spec.
/// Why: All chains decode by slicing at fixed offsets. If any field starts at the
/// wrong byte, that field and every field after it reads wrong data.
#[test]
fn test_intent_requirements_field_offsets() {
    let msg = IntentRequirements {
        intent_id: test_intent_id(),
        requester_addr: test_addr_1(),
        amount_required: DUMMY_AMOUNT,
        token_addr: test_addr_2(),
        solver_addr: test_addr_3(),
        expiry: DUMMY_EXPIRY,
    };
    let encoded = msg.encode();

    // Offset 0: discriminator
    assert_eq!(encoded[0], 0x01);
    // Offset 1: intent_id[0]
    assert_eq!(encoded[1], 0xAA);
    // Offset 32: intent_id[31]
    assert_eq!(encoded[32], 0xBB);
    // Offset 33: requester_addr[0]
    assert_eq!(encoded[33], 0x11);
    // Offset 64: requester_addr[31]
    assert_eq!(encoded[64], 0x22);
    // Offset 73: token_addr[0]
    assert_eq!(encoded[73], 0x33);
    // Offset 104: token_addr[31]
    assert_eq!(encoded[104], 0x44);
    // Offset 105: solver_addr[0]
    assert_eq!(encoded[105], 0x55);
    // Offset 136: solver_addr[31]
    assert_eq!(encoded[136], 0x66);
}

/// 7. Test: IntentRequirements EVM Address Encoding
/// Verifies that a 20-byte EVM address left-padded to 32 bytes encodes correctly.
/// Why: EVM addresses are 20 bytes left-padded to 32. If padding bytes are corrupted,
/// the EVM contract won't recognize the address and funds go to a wrong account.
#[test]
fn test_intent_requirements_evm_address() {
    let msg = IntentRequirements {
        intent_id: [0u8; 32],
        requester_addr: test_evm_addr(),
        amount_required: 0,
        token_addr: [0u8; 32],
        solver_addr: [0u8; 32],
        expiry: 0,
    };
    let encoded = msg.encode();
    // EVM address: 12 zero bytes then the 20-byte address
    for i in 33..45 {
        assert_eq!(encoded[i], 0x00, "padding byte at offset {} should be 0", i);
    }
    assert_eq!(encoded[45], 0xDE);
    assert_eq!(encoded[46], 0xAD);
}

// ============================================================================
// ESCROW CONFIRMATION (0x02) TESTS
// ============================================================================

/// 8. Test: EscrowConfirmation Encoded Size
/// Verifies the encoded message is exactly 137 bytes per the wire format spec.
/// Why: The hub decodes this message to confirm an escrow was created on the
/// connected chain. A wrong size means the hub reads garbage and may release
/// funds for an escrow that doesn't exist.
#[test]
fn test_escrow_confirmation_encode_size() {
    let msg = EscrowConfirmation {
        intent_id: [0u8; 32],
        escrow_id: [0u8; 32],
        amount_escrowed: 0,
        token_addr: [0u8; 32],
        creator_addr: [0u8; 32],
    };
    let encoded = msg.encode();
    assert_eq!(encoded.len(), ESCROW_CONFIRMATION_SIZE);
    assert_eq!(encoded.len(), 137);
}

/// 9. Test: EscrowConfirmation Discriminator Byte
/// Verifies the first byte is 0x02.
/// Why: The hub uses this byte to route the message to the escrow confirmation
/// handler. A wrong value (e.g. 0x01) would route it to the wrong handler.
#[test]
fn test_escrow_confirmation_discriminator() {
    let msg = EscrowConfirmation {
        intent_id: [0u8; 32],
        escrow_id: [0u8; 32],
        amount_escrowed: 0,
        token_addr: [0u8; 32],
        creator_addr: [0u8; 32],
    };
    let encoded = msg.encode();
    assert_eq!(encoded[0], 0x02);
}

/// 10. Test: EscrowConfirmation Encode/Decode Roundtrip
/// Verifies that encoding then decoding produces the original message.
/// Why: The connected chain encodes this and the hub decodes it. If they disagree,
/// the hub could confirm the wrong escrow or credit the wrong creator.
#[test]
fn test_escrow_confirmation_roundtrip() {
    let msg = EscrowConfirmation {
        intent_id: test_intent_id(),
        escrow_id: test_addr_1(),
        amount_escrowed: DUMMY_AMOUNT,
        token_addr: test_addr_2(),
        creator_addr: test_addr_3(),
    };
    let encoded = msg.encode();
    let decoded = EscrowConfirmation::decode(&encoded).unwrap();
    assert_eq!(decoded, msg);
}

/// 11. Test: EscrowConfirmation Big-Endian Amount
/// Verifies amount_escrowed is encoded in big-endian byte order at offset 65..73.
/// Why: The hub checks that escrow amount matches the intent requirement. Wrong
/// endianness means the amounts never match even when they should.
#[test]
fn test_escrow_confirmation_big_endian_amount() {
    let msg = EscrowConfirmation {
        intent_id: [0u8; 32],
        escrow_id: [0u8; 32],
        amount_escrowed: 0x0A0B0C0D0E0F1011, // sequential bytes (0A..11) to assert each position confirms big-endian order
        token_addr: [0u8; 32],
        creator_addr: [0u8; 32],
    };
    let encoded = msg.encode();
    // amount_escrowed at offset 65..73
    assert_eq!(encoded[65], 0x0A);
    assert_eq!(encoded[66], 0x0B);
    assert_eq!(encoded[67], 0x0C);
    assert_eq!(encoded[68], 0x0D);
    assert_eq!(encoded[69], 0x0E);
    assert_eq!(encoded[70], 0x0F);
    assert_eq!(encoded[71], 0x10);
    assert_eq!(encoded[72], 0x11);
}

/// 12. Test: EscrowConfirmation Field Offsets
/// Verifies each field starts at the correct byte offset per the wire format spec.
/// Why: The hub reads escrow_id, token_addr, and creator_addr by slicing at fixed
/// offsets. A shifted offset means the hub associates the wrong creator with the escrow.
#[test]
fn test_escrow_confirmation_field_offsets() {
    let msg = EscrowConfirmation {
        intent_id: test_intent_id(),
        escrow_id: test_addr_1(),
        amount_escrowed: DUMMY_AMOUNT,
        token_addr: test_addr_2(),
        creator_addr: test_addr_3(),
    };
    let encoded = msg.encode();

    assert_eq!(encoded[0], 0x02);
    assert_eq!(encoded[1], 0xAA);   // intent_id[0]
    assert_eq!(encoded[32], 0xBB);  // intent_id[31]
    assert_eq!(encoded[33], 0x11);  // escrow_id[0]
    assert_eq!(encoded[64], 0x22);  // escrow_id[31]
    assert_eq!(encoded[73], 0x33);  // token_addr[0]
    assert_eq!(encoded[104], 0x44); // token_addr[31]
    assert_eq!(encoded[105], 0x55); // creator_addr[0]
    assert_eq!(encoded[136], 0x66); // creator_addr[31]
}

// ============================================================================
// FULFILLMENT PROOF (0x03) TESTS
// ============================================================================

/// 13. Test: FulfillmentProof Encoded Size
/// Verifies the encoded message is exactly 81 bytes per the wire format spec.
/// Why: The connected chain decodes this to release escrowed funds. A wrong size
/// means the escrow contract reads garbage and funds stay locked.
#[test]
fn test_fulfillment_proof_encode_size() {
    let msg = FulfillmentProof {
        intent_id: [0u8; 32],
        solver_addr: [0u8; 32],
        amount_fulfilled: 0,
        timestamp: 0,
    };
    let encoded = msg.encode();
    assert_eq!(encoded.len(), FULFILLMENT_PROOF_SIZE);
    assert_eq!(encoded.len(), 81);
}

/// 14. Test: FulfillmentProof Discriminator Byte
/// Verifies the first byte is 0x03.
/// Why: The connected chain uses this byte to route to the fulfillment handler.
/// A wrong value would route it to the wrong handler or reject the message entirely.
#[test]
fn test_fulfillment_proof_discriminator() {
    let msg = FulfillmentProof {
        intent_id: [0u8; 32],
        solver_addr: [0u8; 32],
        amount_fulfilled: 0,
        timestamp: 0,
    };
    let encoded = msg.encode();
    assert_eq!(encoded[0], 0x03);
}

/// 15. Test: FulfillmentProof Encode/Decode Roundtrip
/// Verifies that encoding then decoding produces the original message.
/// Why: The hub encodes this and the connected chain decodes it to release funds.
/// A mismatch means the escrow releases to the wrong solver or wrong amount.
#[test]
fn test_fulfillment_proof_roundtrip() {
    let msg = FulfillmentProof {
        intent_id: test_intent_id(),
        solver_addr: test_addr_1(),
        amount_fulfilled: DUMMY_AMOUNT,
        timestamp: DUMMY_TIMESTAMP,
    };
    let encoded = msg.encode();
    let decoded = FulfillmentProof::decode(&encoded).unwrap();
    assert_eq!(decoded, msg);
}

/// 16. Test: FulfillmentProof Big-Endian Fields
/// Verifies amount_fulfilled and timestamp are encoded in big-endian byte order.
/// Why: The escrow contract checks amount_fulfilled against the locked amount.
/// Wrong endianness means the check fails and funds stay locked forever.
#[test]
fn test_fulfillment_proof_big_endian_fields() {
    let msg = FulfillmentProof {
        intent_id: [0u8; 32],
        solver_addr: [0u8; 32],
        amount_fulfilled: 0x0102030405060708, // sequential bytes (01..08) to assert each position confirms big-endian order
        timestamp: 0xAABBCCDD00112233,       // distinct byte pairs to assert each position confirms big-endian order
    };
    let encoded = msg.encode();
    // amount_fulfilled at offset 65..73
    assert_eq!(encoded[65], 0x01);
    assert_eq!(encoded[72], 0x08);
    // timestamp at offset 73..81
    assert_eq!(encoded[73], 0xAA);
    assert_eq!(encoded[80], 0x33);
}

/// 17. Test: FulfillmentProof Field Offsets
/// Verifies each field starts at the correct byte offset per the wire format spec.
/// Why: The escrow contract reads solver_addr by offset to verify the solver.
/// A shifted offset means a different address is read and the wrong party gets funds.
#[test]
fn test_fulfillment_proof_field_offsets() {
    let msg = FulfillmentProof {
        intent_id: test_intent_id(),
        solver_addr: test_addr_1(),
        amount_fulfilled: DUMMY_AMOUNT,
        timestamp: DUMMY_TIMESTAMP,
    };
    let encoded = msg.encode();

    assert_eq!(encoded[0], 0x03);
    assert_eq!(encoded[1], 0xAA);   // intent_id[0]
    assert_eq!(encoded[32], 0xBB);  // intent_id[31]
    assert_eq!(encoded[33], 0x11);  // solver_addr[0]
    assert_eq!(encoded[64], 0x22);  // solver_addr[31]
}

// ============================================================================
// PEEK MESSAGE TYPE TESTS
// ============================================================================

/// 18. Test: Peek IntentRequirements Type
/// Verifies peek_message_type returns IntentRequirements for a 0x01 message.
/// Why: The lzReceive handler calls peek first to decide which decode path to take.
/// A wrong peek result routes the message to the wrong handler.
#[test]
fn test_peek_intent_requirements() {
    let msg = IntentRequirements {
        intent_id: [0u8; 32],
        requester_addr: [0u8; 32],
        amount_required: 0,
        token_addr: [0u8; 32],
        solver_addr: [0u8; 32],
        expiry: 0,
    };
    let encoded = msg.encode();
    assert_eq!(
        peek_message_type(&encoded).unwrap(),
        GmpMessageType::IntentRequirements
    );
}

/// 19. Test: Peek EscrowConfirmation Type
/// Verifies peek_message_type returns EscrowConfirmation for a 0x02 message.
/// Why: Same as test 18 — each discriminator value must map to the correct type.
/// Tests 18-20 together ensure all three types are correctly identified.
#[test]
fn test_peek_escrow_confirmation() {
    let msg = EscrowConfirmation {
        intent_id: [0u8; 32],
        escrow_id: [0u8; 32],
        amount_escrowed: 0,
        token_addr: [0u8; 32],
        creator_addr: [0u8; 32],
    };
    let encoded = msg.encode();
    assert_eq!(
        peek_message_type(&encoded).unwrap(),
        GmpMessageType::EscrowConfirmation
    );
}

/// 20. Test: Peek FulfillmentProof Type
/// Verifies peek_message_type returns FulfillmentProof for a 0x03 message.
/// Why: Same as test 18 — each discriminator value must map to the correct type.
/// Tests 18-20 together ensure all three types are correctly identified.
#[test]
fn test_peek_fulfillment_proof() {
    let msg = FulfillmentProof {
        intent_id: [0u8; 32],
        solver_addr: [0u8; 32],
        amount_fulfilled: 0,
        timestamp: 0,
    };
    let encoded = msg.encode();
    assert_eq!(
        peek_message_type(&encoded).unwrap(),
        GmpMessageType::FulfillmentProof
    );
}

// ============================================================================
// ERROR CONDITION TESTS
// ============================================================================

/// 21. Test: Reject Wrong Discriminator
/// Verifies decode rejects a message with the wrong discriminator byte.
/// Why: Without this check, a buffer encoded as EscrowConfirmation could be
/// decoded as IntentRequirements, silently producing garbage fields.
#[test]
fn test_reject_wrong_discriminator() {
    let msg = IntentRequirements {
        intent_id: [0u8; 32],
        requester_addr: [0u8; 32],
        amount_required: 0,
        token_addr: [0u8; 32],
        solver_addr: [0u8; 32],
        expiry: 0,
    };
    let mut encoded = msg.encode();
    // Change discriminator to EscrowConfirmation
    encoded[0] = 0x02;
    let result = IntentRequirements::decode(&encoded);
    assert!(result.is_err(), "Should reject wrong discriminator");
    match result.unwrap_err() {
        GmpError::InvalidMessageType { expected, got } => {
            assert_eq!(expected, 0x01);
            assert_eq!(got, 0x02);
        }
        _ => panic!("expected InvalidMessageType error"),
    }
}

/// 22. Test: Reject Wrong Length
/// Verifies decode rejects a buffer that is not the exact expected size.
/// Why: A truncated buffer could cause out-of-bounds reads. A padded buffer could
/// contain trailing garbage. Both must be rejected to prevent silent corruption.
#[test]
fn test_reject_wrong_length() {
    let result = IntentRequirements::decode(&[0x01; 10]);
    assert!(result.is_err(), "Should reject wrong length");
    match result.unwrap_err() {
        GmpError::InvalidLength { expected, got } => {
            assert_eq!(expected, 145);
            assert_eq!(got, 10);
        }
        _ => panic!("expected InvalidLength error"),
    }
}

/// 23. Test: Reject Empty Buffer
/// Verifies all three message types reject an empty buffer.
/// Why: GMP messages could arrive empty due to network errors or malicious senders.
/// Decode must return an error, not panic or read uninitialized memory.
#[test]
fn test_reject_empty_buffer() {
    let result = IntentRequirements::decode(&[]);
    assert!(result.is_err(), "IntentRequirements should reject empty buffer");

    let result = EscrowConfirmation::decode(&[]);
    assert!(result.is_err(), "EscrowConfirmation should reject empty buffer");

    let result = FulfillmentProof::decode(&[]);
    assert!(result.is_err(), "FulfillmentProof should reject empty buffer");
}

/// 24. Test: Peek Rejects Empty Buffer
/// Verifies peek_message_type rejects an empty buffer.
/// Why: peek is called before decode, so it's the first line of defense. It must
/// not index out of bounds on empty input from the network.
#[test]
fn test_peek_reject_empty_buffer() {
    let result = peek_message_type(&[]);
    assert!(result.is_err(), "Should reject empty buffer");
    match result.unwrap_err() {
        GmpError::InvalidLength { expected, got } => {
            assert_eq!(expected, 1);
            assert_eq!(got, 0);
        }
        _ => panic!("expected InvalidLength error"),
    }
}

/// 25. Test: Peek Rejects Unknown Type
/// Verifies peek_message_type rejects an unknown discriminator byte.
/// Why: If a new message type is added on one chain but not another, the receiver
/// must reject it cleanly rather than silently misinterpreting the payload.
#[test]
fn test_peek_reject_unknown_type() {
    let result = peek_message_type(&[0xFF]);
    assert!(result.is_err(), "Should reject unknown type 0xFF");
    match result.unwrap_err() {
        GmpError::UnknownMessageType(t) => assert_eq!(t, 0xFF),
        _ => panic!("expected UnknownMessageType error"),
    }
}

/// 26. Test: Reject Wrong Discriminator for EscrowConfirmation
/// Verifies EscrowConfirmation::decode rejects a buffer with discriminator 0x01.
/// Why: Each message type has its own decode function. If EscrowConfirmation accepts
/// a 0x01 buffer, it would silently misinterpret IntentRequirements fields.
#[test]
fn test_reject_wrong_discriminator_escrow_confirmation() {
    let mut data = [0u8; 137];
    data[0] = 0x01; // IntentRequirements discriminator, not 0x02
    let result = EscrowConfirmation::decode(&data);
    assert!(result.is_err(), "Should reject wrong discriminator");
    match result.unwrap_err() {
        GmpError::InvalidMessageType { expected, got } => {
            assert_eq!(expected, 0x02);
            assert_eq!(got, 0x01);
        }
        _ => panic!("expected InvalidMessageType error"),
    }
}

/// 27. Test: Reject Wrong Discriminator for FulfillmentProof
/// Verifies FulfillmentProof::decode rejects a buffer with discriminator 0x01.
/// Why: Same reasoning as test 26 — each decode function must enforce its own
/// discriminator to prevent cross-type misinterpretation.
#[test]
fn test_reject_wrong_discriminator_fulfillment_proof() {
    let mut data = [0u8; 81];
    data[0] = 0x01; // IntentRequirements discriminator, not 0x03
    let result = FulfillmentProof::decode(&data);
    assert!(result.is_err(), "Should reject wrong discriminator");
    match result.unwrap_err() {
        GmpError::InvalidMessageType { expected, got } => {
            assert_eq!(expected, 0x03);
            assert_eq!(got, 0x01);
        }
        _ => panic!("expected InvalidMessageType error"),
    }
}

/// 28. Test: Reject Wrong Length for EscrowConfirmation
/// Verifies EscrowConfirmation::decode rejects a buffer that is not 137 bytes.
/// Why: Same reasoning as test 22 — each message type has a different expected
/// size. A bug in the EscrowConfirmation length check is independent of the
/// IntentRequirements length check.
#[test]
fn test_reject_wrong_length_escrow_confirmation() {
    let result = EscrowConfirmation::decode(&[0x02; 10]);
    assert!(result.is_err(), "Should reject wrong length");
    match result.unwrap_err() {
        GmpError::InvalidLength { expected, got } => {
            assert_eq!(expected, 137);
            assert_eq!(got, 10);
        }
        _ => panic!("expected InvalidLength error"),
    }
}

/// 29. Test: Reject Wrong Length for FulfillmentProof
/// Verifies FulfillmentProof::decode rejects a buffer that is not 81 bytes.
/// Why: Same reasoning as test 22 — each message type checks its own expected size
/// independently. A bug in one length check doesn't imply the others are correct.
#[test]
fn test_reject_wrong_length_fulfillment_proof() {
    let result = FulfillmentProof::decode(&[0x03; 10]);
    assert!(result.is_err(), "Should reject wrong length");
    match result.unwrap_err() {
        GmpError::InvalidLength { expected, got } => {
            assert_eq!(expected, 81);
            assert_eq!(got, 10);
        }
        _ => panic!("expected InvalidLength error"),
    }
}

/// 30. Test: Reject Off-By-One Length
/// Verifies all three types reject buffers that are one byte too short or too long.
/// Why: Off-by-one is the most likely length check bug (e.g. `<` instead of `!=`).
/// Testing exact_size-1 and exact_size+1 catches this where a wildly wrong size
/// like 10 might not.
#[test]
fn test_reject_off_by_one_length() {
    // IntentRequirements: 145 bytes
    let result = IntentRequirements::decode(&[0x01; 144]);
    assert!(result.is_err(), "IntentRequirements should reject 144 bytes");
    let result = IntentRequirements::decode(&[0x01; 146]);
    assert!(result.is_err(), "IntentRequirements should reject 146 bytes");

    // EscrowConfirmation: 137 bytes
    let result = EscrowConfirmation::decode(&[0x02; 136]);
    assert!(result.is_err(), "EscrowConfirmation should reject 136 bytes");
    let result = EscrowConfirmation::decode(&[0x02; 138]);
    assert!(result.is_err(), "EscrowConfirmation should reject 138 bytes");

    // FulfillmentProof: 81 bytes
    let result = FulfillmentProof::decode(&[0x03; 80]);
    assert!(result.is_err(), "FulfillmentProof should reject 80 bytes");
    let result = FulfillmentProof::decode(&[0x03; 82]);
    assert!(result.is_err(), "FulfillmentProof should reject 82 bytes");
}

// ============================================================================
// KNOWN BYTE SEQUENCE TESTS
// ============================================================================

/// 31. Test: Decode Known IntentRequirements Bytes
/// Decodes a hand-constructed 145-byte buffer and verifies each field.
/// Why: Roundtrip tests (test 3) use encode+decode together, so a bug that is
/// symmetric in both functions would be invisible. This test decodes a hand-built
/// buffer to catch bugs that roundtrip alone cannot.
#[test]
fn test_decode_known_intent_requirements_bytes() {
    let mut data = [0u8; 145];
    data[0] = 0x01;                          // discriminator
    data[1] = 0xFF;                          // intent_id[0]
    data[32] = 0xEE;                         // intent_id[31]
    data[33] = 0xDD;                         // requester_addr[0]
    data[65..73].copy_from_slice(&DUMMY_AMOUNT.to_be_bytes());
    data[73] = 0xCC;                         // token_addr[0]
    data[105] = 0xBB;                        // solver_addr[0]
    data[137..145].copy_from_slice(&DUMMY_EXPIRY.to_be_bytes());

    let msg = IntentRequirements::decode(&data).unwrap();
    assert_eq!(msg.intent_id[0], 0xFF);
    assert_eq!(msg.intent_id[31], 0xEE);
    assert_eq!(msg.requester_addr[0], 0xDD);
    assert_eq!(msg.amount_required, DUMMY_AMOUNT);
    assert_eq!(msg.token_addr[0], 0xCC);
    assert_eq!(msg.solver_addr[0], 0xBB);
    assert_eq!(msg.expiry, DUMMY_EXPIRY);
}

/// 32. Test: Decode Known EscrowConfirmation Bytes
/// Decodes a hand-constructed 137-byte buffer and verifies each field.
/// Why: Same reasoning as test 31 — decoding a hand-built buffer catches bugs
/// that are symmetric in encode and decode.
#[test]
fn test_decode_known_escrow_confirmation_bytes() {
    let mut data = [0u8; 137];
    data[0] = 0x02;
    data[1] = 0xAA;                           // intent_id[0]
    data[33] = 0xBB;                          // escrow_id[0]
    data[65..73].copy_from_slice(&DUMMY_AMOUNT.to_be_bytes());
    data[73] = 0xCC;                          // token_addr[0]
    data[105] = 0xDD;                         // creator_addr[0]

    let msg = EscrowConfirmation::decode(&data).unwrap();
    assert_eq!(msg.intent_id[0], 0xAA);
    assert_eq!(msg.escrow_id[0], 0xBB);
    assert_eq!(msg.amount_escrowed, DUMMY_AMOUNT);
    assert_eq!(msg.token_addr[0], 0xCC);
    assert_eq!(msg.creator_addr[0], 0xDD);
}

/// 33. Test: Decode Known FulfillmentProof Bytes
/// Decodes a hand-constructed 81-byte buffer and verifies each field.
/// Why: Same reasoning as test 31 — decoding a hand-built buffer catches bugs
/// that are symmetric in encode and decode.
#[test]
fn test_decode_known_fulfillment_proof_bytes() {
    let mut data = [0u8; 81];
    data[0] = 0x03;
    data[1] = 0xAA;                           // intent_id[0]
    data[33] = 0xBB;                          // solver_addr[0]
    data[65..73].copy_from_slice(&DUMMY_AMOUNT.to_be_bytes());
    data[73..81].copy_from_slice(&DUMMY_TIMESTAMP.to_be_bytes());

    let msg = FulfillmentProof::decode(&data).unwrap();
    assert_eq!(msg.intent_id[0], 0xAA);
    assert_eq!(msg.solver_addr[0], 0xBB);
    assert_eq!(msg.amount_fulfilled, DUMMY_AMOUNT);
    assert_eq!(msg.timestamp, DUMMY_TIMESTAMP);
}

// ============================================================================
// BOUNDARY CONDITION TESTS
// ============================================================================

/// 34. Test: Max u64 Amount Roundtrip
/// Verifies u64::MAX encodes and decodes correctly for amount and expiry fields.
/// Why: u64::MAX (0xFFFFFFFFFFFFFFFF) is all 0xFF bytes in big-endian. This
/// catches sign-extension bugs or off-by-one errors at the maximum boundary.
#[test]
fn test_max_u64_amount_roundtrip() {
    let msg = IntentRequirements {
        intent_id: [0xFF; 32],
        requester_addr: [0xFF; 32],
        amount_required: u64::MAX,
        token_addr: [0xFF; 32],
        solver_addr: [0xFF; 32],
        expiry: u64::MAX,
    };
    let decoded = IntentRequirements::decode(&msg.encode()).unwrap();
    assert_eq!(decoded.amount_required, u64::MAX);
    assert_eq!(decoded.expiry, u64::MAX);
}

/// 35. Test: Zero Solver Address Means Any Solver
/// Verifies that bytes32(0) solver_addr roundtrips correctly.
/// Why: bytes32(0) is a sentinel meaning "any solver may fulfill this intent".
/// If it doesn't roundtrip, the open-solver feature silently breaks and intents
/// become unfulfillable.
#[test]
fn test_zero_solver_addr_means_any() {
    let msg = IntentRequirements {
        intent_id: test_intent_id(),
        requester_addr: test_addr_1(),
        amount_required: DUMMY_AMOUNT,
        token_addr: test_addr_2(),
        solver_addr: [0u8; 32],
        expiry: DUMMY_EXPIRY,
    };
    let decoded = IntentRequirements::decode(&msg.encode()).unwrap();
    assert_eq!(decoded.solver_addr, [0u8; 32]);
}

// ============================================================================
// CROSS-CHAIN ENCODING COMPATIBILITY TESTS
// ============================================================================
// These tests verify that SVM encoding matches the expected bytes defined in
// intent-frameworks/common/testing/gmp-encoding-test-vectors.json. The same bytes must be
// produced by MVM to ensure cross-chain compatibility.

/// 36. Test: Cross-chain IntentRequirements Encoding
/// Verifies that encoding produces bytes identical to gmp-encoding-test-vectors.json.
/// Why: Cross-chain GMP requires byte-exact encoding. If SVM produces different bytes
/// than MVM, messages cannot be decoded correctly on the receiving chain.
#[test]
fn test_cross_chain_encoding_intent_requirements() {
    let msg = IntentRequirements {
        intent_id: test_intent_id(),
        requester_addr: test_addr_1(),
        amount_required: DUMMY_AMOUNT,
        token_addr: test_addr_2(),
        solver_addr: test_addr_3(),
        expiry: DUMMY_EXPIRY,
    };
    let encoded = msg.encode();
    let hex = bytes_to_hex(&encoded);

    // Expected from gmp-encoding-test-vectors.json "intent_requirements_standard"
    // 01 + intent_id(32) + requester(32) + amount(8) + token(32) + solver(32) + expiry(8) = 145 bytes
    let expected = "01aa000000000000000000000000000000000000000000000000000000000000bb110000000000000000000000000000000000000000000000000000000000002200000000000f42403300000000000000000000000000000000000000000000000000000000000044550000000000000000000000000000000000000000000000000000000000006600000000000003e8";

    assert_eq!(
        hex, expected,
        "IntentRequirements encoding mismatch!\nGot:      {}\nExpected: {}",
        hex, expected
    );
    println!("IntentRequirements encoding matches expected: {} bytes", encoded.len());
}

/// 37. Test: Cross-chain EscrowConfirmation Encoding
/// Verifies that encoding produces bytes identical to gmp-encoding-test-vectors.json.
/// Why: Cross-chain GMP requires byte-exact encoding. If SVM produces different bytes
/// than MVM, messages cannot be decoded correctly on the receiving chain.
#[test]
fn test_cross_chain_encoding_escrow_confirmation() {
    let msg = EscrowConfirmation {
        intent_id: test_intent_id(),
        escrow_id: test_addr_1(),
        amount_escrowed: DUMMY_AMOUNT,
        token_addr: test_addr_2(),
        creator_addr: test_addr_3(),
    };
    let encoded = msg.encode();
    let hex = bytes_to_hex(&encoded);

    // Expected from gmp-encoding-test-vectors.json "escrow_confirmation_standard"
    // 02 + intent_id(32) + escrow_id(32) + amount(8) + token(32) + creator(32) = 137 bytes
    let expected = "02aa000000000000000000000000000000000000000000000000000000000000bb110000000000000000000000000000000000000000000000000000000000002200000000000f424033000000000000000000000000000000000000000000000000000000000000445500000000000000000000000000000000000000000000000000000000000066";

    assert_eq!(
        hex, expected,
        "EscrowConfirmation encoding mismatch!\nGot:      {}\nExpected: {}",
        hex, expected
    );
    println!("EscrowConfirmation encoding matches expected: {} bytes", encoded.len());
}

/// 38. Test: Cross-chain FulfillmentProof Encoding
/// Verifies that encoding produces bytes identical to gmp-encoding-test-vectors.json.
/// Why: Cross-chain GMP requires byte-exact encoding. If SVM produces different bytes
/// than MVM, messages cannot be decoded correctly on the receiving chain.
#[test]
fn test_cross_chain_encoding_fulfillment_proof() {
    let msg = FulfillmentProof {
        intent_id: test_intent_id(),
        solver_addr: test_addr_1(),
        amount_fulfilled: DUMMY_AMOUNT,
        timestamp: DUMMY_TIMESTAMP,
    };
    let encoded = msg.encode();
    let hex = bytes_to_hex(&encoded);

    // Expected from gmp-encoding-test-vectors.json "fulfillment_proof_standard"
    // 03 + intent_id(32) + solver(32) + amount(8) + timestamp(8) = 81 bytes
    let expected = "03aa000000000000000000000000000000000000000000000000000000000000bb110000000000000000000000000000000000000000000000000000000000002200000000000f424000000000000003e8";

    assert_eq!(
        hex, expected,
        "FulfillmentProof encoding mismatch!\nGot:      {}\nExpected: {}",
        hex, expected
    );
    println!("FulfillmentProof encoding matches expected: {} bytes", encoded.len());
}

/// 39. Test: Cross-chain IntentRequirements Zeros Encoding
/// Verifies that all-zero values encode correctly across chains.
/// Why: Boundary test for zero values. Ensures no special-casing or off-by-one errors
/// when all fields are at their minimum value.
#[test]
fn test_cross_chain_encoding_intent_requirements_zeros() {
    let msg = IntentRequirements {
        intent_id: [0u8; 32],
        requester_addr: [0u8; 32],
        amount_required: 0,
        token_addr: [0u8; 32],
        solver_addr: [0u8; 32],
        expiry: 0,
    };
    let encoded = msg.encode();
    let hex = bytes_to_hex(&encoded);

    // Expected from gmp-encoding-test-vectors.json "intent_requirements_zeros"
    // 01 + 144 zero bytes = 145 bytes = 290 hex chars
    let expected = "01000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

    assert_eq!(
        hex, expected,
        "IntentRequirements zeros encoding mismatch!\nGot:      {}\nExpected: {}",
        hex, expected
    );
    println!("IntentRequirements zeros encoding matches expected: {} bytes", encoded.len());
}

/// 40. Test: Cross-chain IntentRequirements Max Values Encoding
/// Verifies that maximum u64 values encode correctly across chains.
/// Why: Boundary test for max values. Ensures no overflow, sign-extension, or
/// truncation errors when all fields are at their maximum value.
#[test]
fn test_cross_chain_encoding_intent_requirements_max() {
    let msg = IntentRequirements {
        intent_id: [0xFF; 32],
        requester_addr: [0xFF; 32],
        amount_required: u64::MAX,
        token_addr: [0xFF; 32],
        solver_addr: [0xFF; 32],
        expiry: u64::MAX,
    };
    let encoded = msg.encode();
    let hex = bytes_to_hex(&encoded);

    // Expected from gmp-encoding-test-vectors.json "intent_requirements_max_values"
    // 01 + 144 0xFF bytes = 145 bytes = 290 hex chars
    let expected = "01ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

    assert_eq!(
        hex, expected,
        "IntentRequirements max values encoding mismatch!\nGot:      {}\nExpected: {}",
        hex, expected
    );
    println!("IntentRequirements max values encoding matches expected: {} bytes", encoded.len());
}
