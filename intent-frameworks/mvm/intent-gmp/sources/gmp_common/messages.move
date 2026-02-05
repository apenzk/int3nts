/// GMP message encoding/decoding per the wire format specification.
///
/// All messages use fixed-width fields, big-endian integers, and 32-byte addresses.
/// No serialization library — plain bytes readable by Move, Rust, and Solidity.
module mvmt_intent::gmp_common {
    use std::vector;

    // ============================================================================
    // ERROR CODES
    // ============================================================================

    const E_INVALID_MESSAGE_TYPE: u64 = 1;
    const E_INVALID_LENGTH: u64 = 2;
    const E_UNKNOWN_MESSAGE_TYPE: u64 = 3;

    // ============================================================================
    // CONSTANTS
    // ============================================================================

    const MESSAGE_TYPE_INTENT_REQUIREMENTS: u8 = 0x01;
    const MESSAGE_TYPE_ESCROW_CONFIRMATION: u8 = 0x02;
    const MESSAGE_TYPE_FULFILLMENT_PROOF: u8 = 0x03;

    const INTENT_REQUIREMENTS_SIZE: u64 = 145;
    const ESCROW_CONFIRMATION_SIZE: u64 = 137;
    const FULFILLMENT_PROOF_SIZE: u64 = 81;

    // ============================================================================
    // STRUCTS
    // ============================================================================

    /// Hub -> Connected chain. Sent on intent creation to tell the connected chain
    /// what requirements must be met.
    struct IntentRequirements has copy, drop {
        intent_id: vector<u8>,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    }

    /// Connected chain -> Hub. Confirms an escrow was created matching the intent
    /// requirements. The hub gates solver fulfillment on this confirmation.
    struct EscrowConfirmation has copy, drop {
        intent_id: vector<u8>,
        escrow_id: vector<u8>,
        amount_escrowed: u64,
        token_addr: vector<u8>,
        creator_addr: vector<u8>,
    }

    /// Either direction. Proves a solver fulfilled the intent, triggering token
    /// release on the other chain.
    struct FulfillmentProof has copy, drop {
        intent_id: vector<u8>,
        solver_addr: vector<u8>,
        amount_fulfilled: u64,
        timestamp: u64,
    }

    // ============================================================================
    // CONSTRUCTORS
    // ============================================================================

    public fun new_intent_requirements(
        intent_id: vector<u8>,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    ): IntentRequirements {
        IntentRequirements {
            intent_id,
            requester_addr,
            amount_required,
            token_addr,
            solver_addr,
            expiry,
        }
    }

    public fun new_escrow_confirmation(
        intent_id: vector<u8>,
        escrow_id: vector<u8>,
        amount_escrowed: u64,
        token_addr: vector<u8>,
        creator_addr: vector<u8>,
    ): EscrowConfirmation {
        EscrowConfirmation {
            intent_id,
            escrow_id,
            amount_escrowed,
            token_addr,
            creator_addr,
        }
    }

    public fun new_fulfillment_proof(
        intent_id: vector<u8>,
        solver_addr: vector<u8>,
        amount_fulfilled: u64,
        timestamp: u64,
    ): FulfillmentProof {
        FulfillmentProof {
            intent_id,
            solver_addr,
            amount_fulfilled,
            timestamp,
        }
    }

    // ============================================================================
    // ACCESSORS
    // ============================================================================

    public fun intent_requirements_intent_id(msg: &IntentRequirements): &vector<u8> { &msg.intent_id }
    public fun intent_requirements_requester_addr(msg: &IntentRequirements): &vector<u8> { &msg.requester_addr }
    public fun intent_requirements_amount_required(msg: &IntentRequirements): u64 { msg.amount_required }
    public fun intent_requirements_token_addr(msg: &IntentRequirements): &vector<u8> { &msg.token_addr }
    public fun intent_requirements_solver_addr(msg: &IntentRequirements): &vector<u8> { &msg.solver_addr }
    public fun intent_requirements_expiry(msg: &IntentRequirements): u64 { msg.expiry }

    public fun escrow_confirmation_intent_id(msg: &EscrowConfirmation): &vector<u8> { &msg.intent_id }
    public fun escrow_confirmation_escrow_id(msg: &EscrowConfirmation): &vector<u8> { &msg.escrow_id }
    public fun escrow_confirmation_amount_escrowed(msg: &EscrowConfirmation): u64 { msg.amount_escrowed }
    public fun escrow_confirmation_token_addr(msg: &EscrowConfirmation): &vector<u8> { &msg.token_addr }
    public fun escrow_confirmation_creator_addr(msg: &EscrowConfirmation): &vector<u8> { &msg.creator_addr }

    public fun fulfillment_proof_intent_id(msg: &FulfillmentProof): &vector<u8> { &msg.intent_id }
    public fun fulfillment_proof_solver_addr(msg: &FulfillmentProof): &vector<u8> { &msg.solver_addr }
    public fun fulfillment_proof_amount_fulfilled(msg: &FulfillmentProof): u64 { msg.amount_fulfilled }
    public fun fulfillment_proof_timestamp(msg: &FulfillmentProof): u64 { msg.timestamp }

    // ============================================================================
    // SIZE ACCESSORS (for tests and external callers)
    // ============================================================================

    public fun intent_requirements_size(): u64 { INTENT_REQUIREMENTS_SIZE }
    public fun escrow_confirmation_size(): u64 { ESCROW_CONFIRMATION_SIZE }
    public fun fulfillment_proof_size(): u64 { FULFILLMENT_PROOF_SIZE }

    // ============================================================================
    // ENCODE
    // ============================================================================

    public fun encode_intent_requirements(msg: &IntentRequirements): vector<u8> {
        let buf = vector::empty<u8>();
        vector::push_back(&mut buf, MESSAGE_TYPE_INTENT_REQUIREMENTS);
        push_bytes(&mut buf, &msg.intent_id);
        push_bytes(&mut buf, &msg.requester_addr);
        push_be_u64(&mut buf, msg.amount_required);
        push_bytes(&mut buf, &msg.token_addr);
        push_bytes(&mut buf, &msg.solver_addr);
        push_be_u64(&mut buf, msg.expiry);
        buf
    }

    public fun encode_escrow_confirmation(msg: &EscrowConfirmation): vector<u8> {
        let buf = vector::empty<u8>();
        vector::push_back(&mut buf, MESSAGE_TYPE_ESCROW_CONFIRMATION);
        push_bytes(&mut buf, &msg.intent_id);
        push_bytes(&mut buf, &msg.escrow_id);
        push_be_u64(&mut buf, msg.amount_escrowed);
        push_bytes(&mut buf, &msg.token_addr);
        push_bytes(&mut buf, &msg.creator_addr);
        buf
    }

    public fun encode_fulfillment_proof(msg: &FulfillmentProof): vector<u8> {
        let buf = vector::empty<u8>();
        vector::push_back(&mut buf, MESSAGE_TYPE_FULFILLMENT_PROOF);
        push_bytes(&mut buf, &msg.intent_id);
        push_bytes(&mut buf, &msg.solver_addr);
        push_be_u64(&mut buf, msg.amount_fulfilled);
        push_be_u64(&mut buf, msg.timestamp);
        buf
    }

    // ============================================================================
    // DECODE
    // ============================================================================

    /// Decode IntentRequirements from raw bytes. Aborts on wrong length or type.
    public fun decode_intent_requirements(data: &vector<u8>): IntentRequirements {
        let len = vector::length(data);
        assert!(len == INTENT_REQUIREMENTS_SIZE, E_INVALID_LENGTH);
        assert!(*vector::borrow(data, 0) == MESSAGE_TYPE_INTENT_REQUIREMENTS, E_INVALID_MESSAGE_TYPE);

        IntentRequirements {
            intent_id: slice_bytes(data, 1, 32),
            requester_addr: slice_bytes(data, 33, 32),
            amount_required: read_be_u64(data, 65),
            token_addr: slice_bytes(data, 73, 32),
            solver_addr: slice_bytes(data, 105, 32),
            expiry: read_be_u64(data, 137),
        }
    }

    /// Decode EscrowConfirmation from raw bytes. Aborts on wrong length or type.
    public fun decode_escrow_confirmation(data: &vector<u8>): EscrowConfirmation {
        let len = vector::length(data);
        assert!(len == ESCROW_CONFIRMATION_SIZE, E_INVALID_LENGTH);
        assert!(*vector::borrow(data, 0) == MESSAGE_TYPE_ESCROW_CONFIRMATION, E_INVALID_MESSAGE_TYPE);

        EscrowConfirmation {
            intent_id: slice_bytes(data, 1, 32),
            escrow_id: slice_bytes(data, 33, 32),
            amount_escrowed: read_be_u64(data, 65),
            token_addr: slice_bytes(data, 73, 32),
            creator_addr: slice_bytes(data, 105, 32),
        }
    }

    /// Decode FulfillmentProof from raw bytes. Aborts on wrong length or type.
    public fun decode_fulfillment_proof(data: &vector<u8>): FulfillmentProof {
        let len = vector::length(data);
        assert!(len == FULFILLMENT_PROOF_SIZE, E_INVALID_LENGTH);
        assert!(*vector::borrow(data, 0) == MESSAGE_TYPE_FULFILLMENT_PROOF, E_INVALID_MESSAGE_TYPE);

        FulfillmentProof {
            intent_id: slice_bytes(data, 1, 32),
            solver_addr: slice_bytes(data, 33, 32),
            amount_fulfilled: read_be_u64(data, 65),
            timestamp: read_be_u64(data, 73),
        }
    }

    /// Read the message type byte without fully decoding. Aborts if empty or unknown.
    public fun peek_message_type(data: &vector<u8>): u8 {
        assert!(vector::length(data) > 0, E_INVALID_LENGTH);
        let msg_type = *vector::borrow(data, 0);
        assert!(
            msg_type == MESSAGE_TYPE_INTENT_REQUIREMENTS
                || msg_type == MESSAGE_TYPE_ESCROW_CONFIRMATION
                || msg_type == MESSAGE_TYPE_FULFILLMENT_PROOF,
            E_UNKNOWN_MESSAGE_TYPE,
        );
        msg_type
    }

    // ============================================================================
    // INTERNAL HELPERS
    // ============================================================================

    /// Append all bytes from src to buf.
    fun push_bytes(buf: &mut vector<u8>, src: &vector<u8>) {
        let i = 0;
        let len = vector::length(src);
        while (i < len) {
            vector::push_back(buf, *vector::borrow(src, i));
            i = i + 1;
        };
    }

    /// Append a u64 as 8 big-endian bytes.
    fun push_be_u64(buf: &mut vector<u8>, val: u64) {
        vector::push_back(buf, ((val >> 56) & 0xFF as u8));
        vector::push_back(buf, ((val >> 48) & 0xFF as u8));
        vector::push_back(buf, ((val >> 40) & 0xFF as u8));
        vector::push_back(buf, ((val >> 32) & 0xFF as u8));
        vector::push_back(buf, ((val >> 24) & 0xFF as u8));
        vector::push_back(buf, ((val >> 16) & 0xFF as u8));
        vector::push_back(buf, ((val >> 8) & 0xFF as u8));
        vector::push_back(buf, ((val & 0xFF) as u8));
    }

    /// Read 8 big-endian bytes from data starting at offset, return u64.
    fun read_be_u64(data: &vector<u8>, offset: u64): u64 {
        let val: u64 = 0;
        val = val | ((*vector::borrow(data, offset) as u64) << 56);
        val = val | ((*vector::borrow(data, offset + 1) as u64) << 48);
        val = val | ((*vector::borrow(data, offset + 2) as u64) << 40);
        val = val | ((*vector::borrow(data, offset + 3) as u64) << 32);
        val = val | ((*vector::borrow(data, offset + 4) as u64) << 24);
        val = val | ((*vector::borrow(data, offset + 5) as u64) << 16);
        val = val | ((*vector::borrow(data, offset + 6) as u64) << 8);
        val = val | (*vector::borrow(data, offset + 7) as u64);
        val
    }

    /// Extract a sub-vector of `len` bytes starting at `start`.
    fun slice_bytes(data: &vector<u8>, start: u64, len: u64): vector<u8> {
        let result = vector::empty<u8>();
        let i = 0;
        while (i < len) {
            vector::push_back(&mut result, *vector::borrow(data, start + i));
            i = i + 1;
        };
        result
    }
}
