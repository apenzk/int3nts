/// Outflow Validator (MVM as Connected Chain)
///
/// Handles the connected chain side of outflow intents. Mirrors SVM's
/// outflow-validator program structure for test alignment.
///
/// MVM as Connected Chain:
/// - Receives IntentRequirements from hub
/// - Validates escrow creation against requirements
/// - (Future) Sends fulfillment proof back to hub
///
module mvmt_intent::outflow_validator {
    use aptos_framework::event;
    use mvmt_intent::gmp_common::{
        Self,
        IntentRequirements,
    };

    // ============================================================================
    // ERROR CODES
    // ============================================================================

    const E_INVALID_SOURCE_CHAIN: u64 = 1;
    const E_INVALID_SOURCE_ADDRESS: u64 = 2;
    const E_REQUIREMENTS_ALREADY_STORED: u64 = 3;
    const E_REQUIREMENTS_NOT_FOUND: u64 = 4;
    const E_ALREADY_FULFILLED: u64 = 5;

    // ============================================================================
    // EVENTS
    // ============================================================================

    #[event]
    /// Emitted when IntentRequirements is received from hub.
    struct IntentRequirementsReceived has drop, store {
        intent_id: vector<u8>,
        src_chain_id: u32,
        requester_addr: vector<u8>,
        amount_required: u64,
        token_addr: vector<u8>,
        solver_addr: vector<u8>,
        expiry: u64,
    }

    // ============================================================================
    // INBOUND: Hub -> Connected Chain
    // ============================================================================

    /// Receive and process IntentRequirements from hub.
    ///
    /// Called by trusted-gmp relay when hub sends intent requirements.
    /// The connected chain stores these requirements and validates escrow creation against them.
    ///
    /// # Arguments
    /// - `src_chain_id`: LayerZero endpoint ID of source chain (hub)
    /// - `src_address`: 32-byte source address (hub's program)
    /// - `payload`: Raw GMP message payload
    ///
    /// # Returns
    /// - Decoded IntentRequirements struct
    ///
    /// # Aborts
    /// - EINVALID_SOURCE_CHAIN: If source chain is not the trusted hub
    /// - EINVALID_SOURCE_ADDRESS: If source address is not the trusted hub program
    public fun receive_intent_requirements(
        src_chain_id: u32,
        _src_address: vector<u8>,
        payload: vector<u8>,
    ): IntentRequirements {
        // Decode the message
        let msg = gmp_common::decode_intent_requirements(&payload);

        // Emit event for tracking
        event::emit(IntentRequirementsReceived {
            intent_id: *gmp_common::intent_requirements_intent_id(&msg),
            src_chain_id,
            requester_addr: *gmp_common::intent_requirements_requester_addr(&msg),
            amount_required: gmp_common::intent_requirements_amount_required(&msg),
            token_addr: *gmp_common::intent_requirements_token_addr(&msg),
            solver_addr: *gmp_common::intent_requirements_solver_addr(&msg),
            expiry: gmp_common::intent_requirements_expiry(&msg),
        });

        // TODO: Validate source chain and address are the trusted hub
        // TODO: Store requirements for later validation during escrow creation

        msg
    }

    // ============================================================================
    // FULFILLMENT (Future)
    // ============================================================================

    // TODO: Add fulfill_intent() function that:
    // 1. Validates solver is authorized
    // 2. Sends FulfillmentProof back to hub
}
