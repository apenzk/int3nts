/// Intent GMP Hub Interface
///
/// Interface functions for hub operations: sending requirements/proofs to connected
/// chains and receiving confirmations/proofs from them.
///
/// MVM as Hub:
/// - Sends IntentRequirements to connected chains on intent creation
/// - Receives EscrowConfirmation from connected chains
/// - Sends FulfillmentProof to connected chains
/// - Receives FulfillmentProof from connected chains
///
module mvmt_intent::intent_gmp_hub {
    use std::vector;
    use aptos_framework::event;
    use mvmt_intent::gmp_common::{
        Self,
        EscrowConfirmation,
        FulfillmentProof,
    };

    // ============================================================================
    // ERROR CODES
    // ============================================================================

    const EINVALID_SOURCE_CHAIN: u64 = 2;
    const EINVALID_SOURCE_ADDRESS: u64 = 3;
    const EINTENT_NOT_FOUND: u64 = 4;
    const EESCROW_MISMATCH: u64 = 5;
    const EALREADY_CONFIRMED: u64 = 6;
    const EALREADY_FULFILLED: u64 = 7;

    // ============================================================================
    // EVENTS
    // ============================================================================

    #[event]
    /// Emitted when IntentRequirements is sent to a connected chain.
    struct IntentRequirementsSent has drop, store {
        intent_id: vector<u8>,
        dst_chain_id: u32,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    }

    #[event]
    /// Emitted when EscrowConfirmation is received from a connected chain.
    struct EscrowConfirmationReceived has drop, store {
        intent_id: vector<u8>,
        src_chain_id: u32,
        escrow_id: vector<u8>,
        amount_escrowed: u64,
        token_addr: vector<u8>,
        creator_addr: vector<u8>,
    }

    #[event]
    /// Emitted when FulfillmentProof is sent to a connected chain.
    struct FulfillmentProofSent has drop, store {
        intent_id: vector<u8>,
        dst_chain_id: u32,
        solver_addr: vector<u8>,
        amount_fulfilled: u64,
        timestamp: u64,
    }

    #[event]
    /// Emitted when FulfillmentProof is received from a connected chain.
    struct FulfillmentProofReceived has drop, store {
        intent_id: vector<u8>,
        src_chain_id: u32,
        solver_addr: vector<u8>,
        amount_fulfilled: u64,
        timestamp: u64,
    }

    // ============================================================================
    // OUTBOUND: Hub -> Connected Chain
    // ============================================================================

    /// Send IntentRequirements to a connected chain when an intent is created.
    ///
    /// Called by the hub when a new cross-chain intent is created. The connected
    /// chain uses this to validate escrow creation matches the intent.
    ///
    /// # Arguments
    /// - `dst_chain_id`: LayerZero endpoint ID of destination chain
    /// - `intent_id`: 32-byte intent identifier
    /// - `requester_addr`: 32-byte requester address (on connected chain)
    /// - `amount_required`: Amount of tokens required in escrow
    /// - `token_addr`: 32-byte token address (on connected chain)
    /// - `solver_addr`: 32-byte solver address (on connected chain)
    /// - `expiry`: Unix timestamp when intent expires
    ///
    /// # Returns
    /// - Encoded GMP message payload (for off-chain relay)
    public fun send_intent_requirements(
        dst_chain_id: u32,
        intent_id: vector<u8>,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    ): vector<u8> {
        // Create the message
        let msg = gmp_common::new_intent_requirements(
            intent_id,
            requester_addr,
            amount_required,
            token_addr,
            solver_addr,
            expiry,
        );

        // Encode for transmission
        let payload = gmp_common::encode_intent_requirements(&msg);

        // Emit event for off-chain relay to pick up
        event::emit(IntentRequirementsSent {
            intent_id: *gmp_common::intent_requirements_intent_id(&msg),
            dst_chain_id,
            requester_addr: *gmp_common::intent_requirements_requester_addr(&msg),
            amount_required: gmp_common::intent_requirements_amount_required(&msg),
            token_addr: *gmp_common::intent_requirements_token_addr(&msg),
            solver_addr: *gmp_common::intent_requirements_solver_addr(&msg),
            expiry: gmp_common::intent_requirements_expiry(&msg),
        });

        // TODO: In production, call LayerZero endpoint to send message
        // For now, return encoded payload for off-chain relay (trusted-gmp)
        payload
    }

    /// Send FulfillmentProof to a connected chain when fulfillment is recorded.
    ///
    /// Called by the hub when a solver fulfills an intent. The connected chain
    /// uses this to release escrowed tokens to the solver.
    ///
    /// # Arguments
    /// - `dst_chain_id`: LayerZero endpoint ID of destination chain
    /// - `intent_id`: 32-byte intent identifier
    /// - `solver_addr`: 32-byte solver address (on connected chain)
    /// - `amount_fulfilled`: Amount of tokens fulfilled
    /// - `timestamp`: Unix timestamp of fulfillment
    ///
    /// # Returns
    /// - Encoded GMP message payload (for off-chain relay)
    public fun send_fulfillment_proof(
        dst_chain_id: u32,
        intent_id: vector<u8>,
        solver_addr: vector<u8>,
        amount_fulfilled: u64,
        timestamp: u64,
    ): vector<u8> {
        // Create the message
        let msg = gmp_common::new_fulfillment_proof(
            intent_id,
            solver_addr,
            amount_fulfilled,
            timestamp,
        );

        // Encode for transmission
        let payload = gmp_common::encode_fulfillment_proof(&msg);

        // Emit event for off-chain relay to pick up
        event::emit(FulfillmentProofSent {
            intent_id: *gmp_common::fulfillment_proof_intent_id(&msg),
            dst_chain_id,
            solver_addr: *gmp_common::fulfillment_proof_solver_addr(&msg),
            amount_fulfilled: gmp_common::fulfillment_proof_amount_fulfilled(&msg),
            timestamp: gmp_common::fulfillment_proof_timestamp(&msg),
        });

        // TODO: In production, call LayerZero endpoint to send message
        // For now, return encoded payload for off-chain relay (trusted-gmp)
        payload
    }

    // ============================================================================
    // INBOUND: Connected Chain -> Hub
    // ============================================================================

    /// Receive and process EscrowConfirmation from a connected chain.
    ///
    /// Called by trusted-gmp relay when a connected chain confirms escrow creation.
    /// The hub validates the confirmation matches the original intent requirements.
    ///
    /// # Arguments
    /// - `src_chain_id`: LayerZero endpoint ID of source chain
    /// - `src_address`: 32-byte source address (connected chain's program)
    /// - `payload`: Raw GMP message payload
    ///
    /// # Returns
    /// - Decoded EscrowConfirmation struct
    ///
    /// # Aborts
    /// - EINVALID_SOURCE_CHAIN: If source chain is not trusted
    /// - EINTENT_NOT_FOUND: If intent_id doesn't exist
    /// - EESCROW_MISMATCH: If confirmation doesn't match intent requirements
    /// - EALREADY_CONFIRMED: If escrow was already confirmed
    public fun receive_escrow_confirmation(
        src_chain_id: u32,
        _src_address: vector<u8>,
        payload: vector<u8>,
    ): EscrowConfirmation {
        // Decode the message
        let msg = gmp_common::decode_escrow_confirmation(&payload);

        // Emit event for tracking
        event::emit(EscrowConfirmationReceived {
            intent_id: *gmp_common::escrow_confirmation_intent_id(&msg),
            src_chain_id,
            escrow_id: *gmp_common::escrow_confirmation_escrow_id(&msg),
            amount_escrowed: gmp_common::escrow_confirmation_amount_escrowed(&msg),
            token_addr: *gmp_common::escrow_confirmation_token_addr(&msg),
            creator_addr: *gmp_common::escrow_confirmation_creator_addr(&msg),
        });

        // TODO: Validate source chain and address are trusted
        // TODO: Look up intent by intent_id and validate confirmation matches
        // TODO: Mark intent as having confirmed escrow

        msg
    }

    /// Receive and process FulfillmentProof from a connected chain.
    ///
    /// Called by trusted-gmp relay when a connected chain reports fulfillment.
    /// The hub uses this to trigger release of desired tokens to the solver.
    ///
    /// # Arguments
    /// - `src_chain_id`: LayerZero endpoint ID of source chain
    /// - `src_address`: 32-byte source address (connected chain's program)
    /// - `payload`: Raw GMP message payload
    ///
    /// # Returns
    /// - Decoded FulfillmentProof struct
    ///
    /// # Aborts
    /// - EINVALID_SOURCE_CHAIN: If source chain is not trusted
    /// - EINTENT_NOT_FOUND: If intent_id doesn't exist
    /// - EALREADY_FULFILLED: If intent was already fulfilled
    public fun receive_fulfillment_proof(
        src_chain_id: u32,
        _src_address: vector<u8>,
        payload: vector<u8>,
    ): FulfillmentProof {
        // Decode the message
        let msg = gmp_common::decode_fulfillment_proof(&payload);

        // Emit event for tracking
        event::emit(FulfillmentProofReceived {
            intent_id: *gmp_common::fulfillment_proof_intent_id(&msg),
            src_chain_id,
            solver_addr: *gmp_common::fulfillment_proof_solver_addr(&msg),
            amount_fulfilled: gmp_common::fulfillment_proof_amount_fulfilled(&msg),
            timestamp: gmp_common::fulfillment_proof_timestamp(&msg),
        });

        // TODO: Validate source chain and address are trusted
        // TODO: Look up intent by intent_id
        // TODO: Trigger release of escrowed tokens to solver

        msg
    }

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    /// Convert an address to a 32-byte vector for GMP message encoding.
    public fun address_to_bytes32(addr: address): vector<u8> {
        let bytes = std::bcs::to_bytes(&addr);
        // BCS encodes address as 32 bytes on Aptos/Movement
        bytes
    }

    /// Create a 32-byte zero-padded vector from a shorter byte array.
    /// Pads on the left (big-endian style).
    public fun bytes_to_bytes32(input: vector<u8>): vector<u8> {
        let len = vector::length(&input);
        if (len >= 32) {
            // If already 32+ bytes, return first 32
            let result = vector::empty<u8>();
            let i = 0;
            while (i < 32) {
                vector::push_back(&mut result, *vector::borrow(&input, i));
                i = i + 1;
            };
            result
        } else {
            // Pad with zeros on the left
            let result = vector::empty<u8>();
            let padding = 32 - len;
            let i = 0;
            while (i < padding) {
                vector::push_back(&mut result, 0);
                i = i + 1;
            };
            i = 0;
            while (i < len) {
                vector::push_back(&mut result, *vector::borrow(&input, i));
                i = i + 1;
            };
            result
        }
    }
}
