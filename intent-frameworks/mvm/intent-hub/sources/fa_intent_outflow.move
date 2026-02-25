module mvmt_intent::fa_intent_outflow {
    use std::signer;
    use std::option;
    use std::error;
    use std::bcs;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::object::{Self as object, Object};
    use aptos_framework::fungible_asset::{Self as fungible_asset, FungibleAsset, Metadata};
    use mvmt_intent::fa_intent_with_oracle;
    use mvmt_intent::intent::{Self as intent, Intent};
    use mvmt_intent::intent_reservation;
    use mvmt_intent::intent_registry;
    use mvmt_intent::intent_gmp_hub;
    use mvmt_intent::gmp_intent_state;
    use mvmt_intent::gmp_common;
    use aptos_std::ed25519;
    use aptos_framework::event;
    use aptos_framework::timestamp;

    /// The solver signature is invalid and cannot be verified.
    const E_INVALID_SIGNATURE: u64 = 2;
    /// The requester address on the connected chain is invalid (zero address).
    const E_INVALID_REQUESTER_ADDR: u64 = 3;
    /// Fulfillment proof not received for this intent.
    const E_FULFILLMENT_PROOF_NOT_RECEIVED: u64 = 7;
    /// Caller is not the intent owner or admin.
    const E_UNAUTHORIZED_CALLER: u64 = 8;
    /// Fulfillment proof already received — cannot cancel.
    const E_ALREADY_FULFILLED: u64 = 10;

    #[event]
    /// Event emitted when an outflow intent is fulfilled.
    /// Mirrors fa_intent::LimitOrderFulfillmentEvent so the coordinator detects completion.
    struct LimitOrderFulfillmentEvent has store, drop {
        intent_addr: address,
        intent_id: address,
        solver: address,
        provided_metadata: Object<Metadata>,
        provided_amount: u64,
        timestamp: u64,
    }

    #[event]
    /// Event emitted when an outflow intent is cancelled after expiry.
    struct OutflowIntentCancelled has store, drop {
        intent_id: address,
        requester: address,
        amount: u64,
    }

    // ============================================================================
    // SHARED UTILITIES
    // ============================================================================

    /// Creates a draft intent for cross-chain request.
    /// This is step 1 of the reserved intent flow:
    /// 1. Requester creates draft using this function (off-chain)
    /// 2. Solver signs the draft and returns signature (off-chain)
    /// 3. Requester calls create_outflow_intent with the signature (on-chain)
    public fun create_cross_chain_draft_intent(
        offered_metadata: Object<Metadata>,
        offered_amount: u64,
        offered_chain_id: u64,
        desired_metadata: Object<Metadata>,
        desired_amount: u64,
        desired_chain_id: u64,
        expiry_time: u64,
        requester: address
    ): intent_reservation::Draftintent {
        intent_reservation::create_draft_intent(
            offered_metadata,
            offered_amount,
            offered_chain_id,
            desired_metadata,
            desired_amount,
            desired_chain_id,
            expiry_time,
            requester
        )
    }

    // ============================================================================
    // OUTFLOW REQUEST-INTENT FUNCTIONS
    // ============================================================================

    // ============================================================================
    // GMP RECEIVE HANDLER
    // ============================================================================

    /// Receive and record a FulfillmentProof via GMP.
    ///
    /// This function is called by intent_gmp when a FulfillmentProof message
    /// is received from the connected chain. It validates the message and records
    /// the proof in GMP state, enabling the solver to claim their tokens.
    ///
    /// After this is called, the solver can call `fulfill_outflow_intent`
    /// to claim the locked tokens.
    ///
    /// # Arguments
    /// - `src_chain_id`: Source chain ID of the GMP message
    /// - `src_address`: Source address on the connected chain (32 bytes)
    /// - `payload`: Raw GMP message payload containing the FulfillmentProof
    ///
    /// # Returns
    /// - true if proof was newly recorded, false if already received (idempotent)
    public fun receive_fulfillment_proof(
        src_chain_id: u32,
        src_address: vector<u8>,
        payload: vector<u8>,
    ): bool {
        // 1. Validate, decode, and record the FulfillmentProof via intent_gmp_hub.
        //    intent_gmp_hub::receive_fulfillment_proof handles state recording internally.
        let proof = intent_gmp_hub::receive_fulfillment_proof(
            src_chain_id,
            src_address,
            payload,
        );

        // 2. Return whether the proof was recorded in state
        let intent_id_bytes = *gmp_common::fulfillment_proof_intent_id(&proof);
        gmp_intent_state::is_fulfillment_proof_received(intent_id_bytes)
    }

    /// Entry function for solver to fulfill an outflow intent.
    ///
    /// Outflow intents have tokens locked on the hub chain and request tokens on the connected chain.
    /// The solver must first deliver tokens on the connected chain. Once the FulfillmentProof
    /// is received via GMP (via `receive_fulfillment_proof`), the solver can call this function
    /// to claim the locked tokens.
    ///
    /// # Arguments
    /// - `solver`: Signer of the solver claiming the tokens
    /// - `intent`: Object reference to the outflow intent
    ///
    /// # Aborts
    /// - E_FULFILLMENT_PROOF_NOT_RECEIVED: Fulfillment proof not received for this intent
    public entry fun fulfill_outflow_intent(
        solver: &signer,
        intent: Object<Intent<fa_intent_with_oracle::FungibleStoreManager, fa_intent_with_oracle::OracleGuardedLimitOrder>>,
    ) {
        let solver_addr = signer::address_of(solver);
        let intent_addr = object::object_address(&intent);

        // 1. Get intent_id from the intent argument
        // We need to start the session first to access the argument
        let (unlocked_fa, session) =
            fa_intent_with_oracle::start_fa_offering_session(solver, intent);

        // Get the intent_id from the session argument
        let argument = intent::get_argument(&session);
        let intent_id_addr = fa_intent_with_oracle::get_intent_id(argument);
        let intent_id_bytes = bcs::to_bytes(&intent_id_addr);

        // 2. Verify fulfillment proof was received via GMP
        assert!(
            gmp_intent_state::is_fulfillment_proof_received(intent_id_bytes),
            error::invalid_state(E_FULFILLMENT_PROOF_NOT_RECEIVED)
        );

        // 3. Capture token info from unlocked tokens BEFORE depositing
        let provided_metadata = fungible_asset::metadata_from_asset(&unlocked_fa);
        let provided_amount = fungible_asset::amount(&unlocked_fa);
        let payment_metadata = fungible_asset::asset_metadata(&unlocked_fa);

        // 4. Deposit unlocked tokens to solver (they get the locked tokens as reward)
        primary_fungible_store::deposit(solver_addr, unlocked_fa);

        // 5. Emit fulfillment event so coordinator/frontend detect completion
        event::emit(LimitOrderFulfillmentEvent {
            intent_addr,
            intent_id: intent_id_addr,
            solver: solver_addr,
            provided_metadata,
            provided_amount,
            timestamp: timestamp::now_seconds(),
        });

        // 6. Withdraw 0 tokens as payment (desired_amount = 0 on hub for outflow)
        let solver_payment = primary_fungible_store::withdraw(
            solver, payment_metadata, 0
        );

        // 7. Complete the intent using GMP proof flow (no oracle witness needed)
        fa_intent_with_oracle::finish_fa_receiving_session_for_gmp(
            session,
            solver_payment,
        );

        // 8. Unregister intent from the registry
        intent_registry::unregister_intent(intent_addr);

        // 9. Remove intent from GMP state tracking
        gmp_intent_state::remove_intent(intent_id_bytes);
    }

    /// Entry function to cancel an expired outflow intent and return funds to the requester.
    ///
    /// Can be called by the intent owner (requester) or the admin (@mvmt_intent).
    /// Funds always go back to the original requester regardless of who calls cancel.
    ///
    /// # Arguments
    /// - `caller`: Signer of the caller (must be intent owner or admin)
    /// - `intent`: Object reference to the outflow intent to cancel
    ///
    /// # Aborts
    /// - `E_UNAUTHORIZED_CALLER`: Caller is not the intent owner or admin
    /// - `E_INTENT_NOT_EXPIRED` (from intent.move): Intent has not expired yet
    /// - `E_ALREADY_FULFILLED`: Fulfillment proof already received for this intent
    public entry fun cancel_outflow_intent(
        caller: &signer,
        intent: Object<Intent<fa_intent_with_oracle::FungibleStoreManager, fa_intent_with_oracle::OracleGuardedLimitOrder>>,
    ) {
        let caller_addr = signer::address_of(caller);
        let intent_addr = object::object_address(&intent);

        // 1. Auth: caller must be admin (only admin can cancel expired intents)
        assert!(
            caller_addr == @mvmt_intent,
            error::permission_denied(E_UNAUTHORIZED_CALLER)
        );

        // 2. Cancel the expired intent (checks expiry internally, destructures intent)
        let (fa, requester_addr, intent_id) =
            fa_intent_with_oracle::cancel_expired_oracle_fa_intent(intent);

        // 3. Guard: fulfillment proof must NOT have been received
        let intent_id_bytes = bcs::to_bytes(&intent_id);
        assert!(
            !gmp_intent_state::is_fulfillment_proof_received(intent_id_bytes),
            error::invalid_state(E_ALREADY_FULFILLED)
        );

        // 4. Capture amount before depositing
        let amount = fungible_asset::amount(&fa);

        // 5. Return funds to the original requester (not the caller)
        primary_fungible_store::deposit(requester_addr, fa);

        // 6. Cleanup: unregister from intent registry and GMP state
        intent_registry::unregister_intent(intent_addr);
        gmp_intent_state::remove_intent(intent_id_bytes);

        // 7. Emit cancellation event
        event::emit(OutflowIntentCancelled {
            intent_id,
            requester: requester_addr,
            amount,
        });
    }

    /// Entry function to create an outflow intent.
    ///
    /// Outflow intents lock tokens on the hub chain and request tokens on a connected chain.
    /// The solver's public key is looked up from the on-chain solver registry.
    ///
    /// For argument descriptions and abort conditions, see `create_outflow_intent`.
    public entry fun create_outflow_intent_entry(
        requester_signer: &signer,
        offered_metadata: Object<Metadata>,
        offered_amount: u64,
        offered_chain_id: u64,
        desired_metadata_addr: address,
        desired_amount: u64,
        desired_chain_id: u64,
        expiry_time: u64,
        intent_id: address,
        requester_addr_connected_chain: address,
        solver: address,
        solver_addr_connected_chain: address,
        solver_signature: vector<u8>
    ) {
        let _intent_obj =
            create_outflow_intent(
                requester_signer,
                offered_metadata,
                offered_amount,
                offered_chain_id,
                desired_metadata_addr,
                desired_amount,
                desired_chain_id,
                expiry_time,
                intent_id,
                requester_addr_connected_chain,
                solver,
                solver_addr_connected_chain,
                solver_signature
            );
    }

    /// Creates an outflow intent and returns the intent object.
    ///
    /// This is the core implementation that both the entry function and tests use.
    ///
    /// # Note on parameter types:
    /// - `offered_metadata` uses `Object<Metadata>` because the offered tokens are on the hub chain,
    ///   so we can validate the object exists and is the correct type before withdrawing tokens.
    /// - `desired_metadata_addr` uses `address` because the desired tokens are on a different chain
    ///   (connected chain), so the metadata object doesn't exist on the hub chain. We can't validate
    ///   it here - validation happens on the connected chain when the solver transfers tokens.
    ///
    /// # Arguments
    /// - `requester_signer`: Signer of the requester creating the intent
    /// - `offered_metadata`: Metadata object of the token type being offered (locked on hub chain)
    /// - `offered_amount`: Amount of tokens to withdraw and lock on hub chain
    /// - `offered_chain_id`: Chain ID of the hub chain (where tokens are locked)
    /// - `desired_metadata_addr`: Address of the desired token metadata (on connected chain)
    /// - `desired_amount`: Amount of desired tokens
    /// - `desired_chain_id`: Chain ID where tokens are desired (connected chain)
    /// - `expiry_time`: Unix timestamp when intent expires
    /// - `intent_id`: Intent ID for cross-chain linking
    /// - `requester_addr_connected_chain`: Address on connected chain where solver should send tokens
    /// - `solver`: Address of the solver authorized to fulfill this intent (must be registered)
    /// - `solver_addr_connected_chain`: Solver's address on the connected chain (used in GMP message for authorization)
    /// - `solver_signature`: Ed25519 signature from the solver authorizing this intent
    ///
    /// # Returns
    /// - `Object<Intent<FungibleStoreManager, OracleGuardedLimitOrder>>`: The created intent object
    ///
    /// # Aborts
    /// - `ESOLVER_NOT_REGISTERED`: Solver is not registered in the solver registry
    /// - `E_INVALID_SIGNATURE`: Signature verification failed
    /// - `E_INVALID_REQUESTER_ADDR`: requester_addr_connected_chain is zero address (0x0)
    public fun create_outflow_intent(
        requester_signer: &signer,
        offered_metadata: Object<Metadata>,
        offered_amount: u64,
        offered_chain_id: u64,
        desired_metadata_addr: address,
        desired_amount: u64,
        desired_chain_id: u64,
        expiry_time: u64,
        intent_id: address,
        requester_addr_connected_chain: address,
        solver: address,
        solver_addr_connected_chain: address,
        solver_signature: vector<u8>
    ): Object<Intent<fa_intent_with_oracle::FungibleStoreManager, fa_intent_with_oracle::OracleGuardedLimitOrder>> {
        // Validate requester_addr_connected_chain is not zero address
        // Outflow intents require a valid address on the connected chain where the solver should send tokens
        assert!(
            requester_addr_connected_chain != @0x0,
            error::invalid_argument(E_INVALID_REQUESTER_ADDR)
        );

        // Withdraw actual tokens from requester (locked on hub chain for outflow)
        let fa: FungibleAsset =
            primary_fungible_store::withdraw(
                requester_signer, offered_metadata, offered_amount
            );

        // Get offered_metadata address for the raw intent
        let offered_metadata_addr = aptos_framework::object::object_address(&offered_metadata);

        // Verify solver signature using raw addresses (works for cross-chain where desired token doesn't exist locally)
        let intent_to_sign =
            intent_reservation::new_intent_to_sign_raw(
                offered_metadata_addr,
                offered_amount,
                offered_chain_id,
                desired_metadata_addr,
                desired_amount,
                desired_chain_id,
                expiry_time,
                signer::address_of(requester_signer),
                solver
            );

        // Use verify_and_create_reservation_from_registry_raw to look up public key from registry
        let reservation_result =
            intent_reservation::verify_and_create_reservation_from_registry_raw(
                intent_to_sign, solver_signature
            );
        // Fail if signature verification failed - cross-chain intents must be reserved
        assert!(
            option::is_some(&reservation_result),
            error::invalid_argument(E_INVALID_SIGNATURE)
        );

        // Placeholder oracle requirement — not used in GMP fulfillment flow.
        // GMP proof (receive_fulfillment_proof) is the sole authorization for outflow.
        // The OracleGuardedLimitOrder type still requires a requirement struct,
        // but finish_fa_receiving_session_for_gmp never checks it.
        // TODO: Migrate outflow to use fa_intent (FALimitOrder) like inflow does.
        //       Requires adding desired_metadata_addr_override to fa_intent::create_fa_to_fa_intent
        //       and a GMP-aware finish function, then this placeholder can be removed entirely.
        let placeholder_pk =
            ed25519::new_unvalidated_public_key_from_bytes(
                x"0000000000000000000000000000000000000000000000000000000000000000"
            );
        let requirement =
            fa_intent_with_oracle::new_oracle_signature_requirement(
                0,
                placeholder_pk
            );

        // For outflow intents on hub chain:
        // - offered_amount = actual amount locked (tokens locked on hub)
        // - desired_amount = original desired_amount (for the connected chain specified by desired_chain_id)
        // - desired_metadata = placeholder (use same as offered_metadata for payment check)
        // The payment validation will check if desired_chain_id != offered_chain_id and use 0 for payment on hub
        let placeholder_metadata = fungible_asset::asset_metadata(&fa);

        let intent_obj = fa_intent_with_oracle::create_fa_to_fa_intent_with_oracle_requirement(
            fa,
            offered_chain_id, // Chain ID where offered tokens are located (hub chain)
            placeholder_metadata, // Use same metadata as locked tokens (placeholder for payment check)
            desired_amount, // Original desired_amount (for the connected chain) - payment validation will use 0 on hub
            desired_chain_id, // Chain ID where desired tokens are located (connected chain)
            option::some(desired_metadata_addr), // Pass explicit desired_metadata_addr since tokens are on connected chain
            expiry_time,
            signer::address_of(requester_signer),
            requirement,
            false, // CRITICAL: All parts of a cross-chain intent MUST be non-revocable
            intent_id,
            option::some(requester_addr_connected_chain), // Store where solver should send tokens on connected chain
            reservation_result // Reserved for specific solver
        );

        // Register intent in the registry for dynamic account discovery
        let intent_addr = object::object_address(&intent_obj);
        intent_registry::register_intent(signer::address_of(requester_signer), intent_addr, expiry_time);

        // Convert intent_id to bytes for GMP
        let intent_id_bytes = bcs::to_bytes(&intent_id);

        // Register intent in GMP state tracking
        // For outflow, dst_chain_id is desired_chain_id (connected chain where solver delivers)
        let dst_chain_id = (desired_chain_id as u32);
        gmp_intent_state::register_outflow_intent(intent_id_bytes, intent_addr, dst_chain_id);

        // Send IntentRequirements to connected chain via GMP
        // For outflow: requester_addr is on connected chain, token is desired token on connected chain
        let requester_addr_bytes = bcs::to_bytes(&requester_addr_connected_chain);
        let token_addr_bytes = bcs::to_bytes(&desired_metadata_addr);
        let solver_addr_bytes = bcs::to_bytes(&solver_addr_connected_chain);

        let _nonce = intent_gmp_hub::send_intent_requirements(
            requester_signer,
            dst_chain_id,
            intent_id_bytes,
            requester_addr_bytes,
            desired_amount, // Amount solver must deliver on connected chain
            token_addr_bytes,
            solver_addr_bytes,
            expiry_time,
        );

        intent_obj
    }
}
