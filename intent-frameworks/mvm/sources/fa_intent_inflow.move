module mvmt_intent::fa_intent_inflow {
    use std::signer;
    use std::option;
    use std::error;
    use std::bcs;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::object::{Self as object, Object};
    use aptos_framework::fungible_asset::{FungibleAsset, Metadata};
    use aptos_framework::timestamp;
    use mvmt_intent::fa_intent::{Self, FungibleStoreManager, FungibleAssetLimitOrder};
    use mvmt_intent::intent::{Self as intent, Intent};
    use mvmt_intent::intent_reservation;
    use mvmt_intent::intent_registry;
    use mvmt_intent::intent_gmp_hub;
    use mvmt_intent::gmp_intent_state;

    /// The solver signature is invalid and cannot be verified.
    const E_INVALID_SIGNATURE: u64 = 2;
    /// Escrow has not been confirmed for this intent.
    const E_ESCROW_NOT_CONFIRMED: u64 = 6;

    // ============================================================================
    // SHARED UTILITIES
    // ============================================================================

    /// Creates a draft intent for cross-chain request.
    /// This is step 1 of the reserved intent flow:
    /// 1. Requester creates draft using this function (off-chain)
    /// 2. Solver signs the draft and returns signature (off-chain)
    /// 3. Requester calls create_inflow_intent with the signature (on-chain)
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
    // INFLOW REQUEST-INTENT FUNCTIONS
    // ============================================================================

    /// Entry function for solver to fulfill an inflow intent.
    ///
    /// Inflow intents have tokens locked on the connected chain (in escrow) and request tokens on the hub.
    /// The solver provides the desired tokens to the requester on the hub chain.
    /// No approver signature is required for inflow intents.
    ///
    /// For cross-chain intents, this function:
    /// 1. Verifies escrow confirmation was received from the connected chain
    /// 2. Fulfills the intent by transferring tokens to requester
    /// 3. Sends FulfillmentProof to the connected chain to release escrow
    ///
    /// # Arguments
    /// - `solver`: Signer fulfilling the intent
    /// - `intent`: Object reference to the inflow intent to fulfill (FungibleAssetLimitOrder)
    /// - `payment_amount`: Amount of tokens to provide
    public entry fun fulfill_inflow_intent(
        solver: &signer,
        intent: Object<Intent<FungibleStoreManager, FungibleAssetLimitOrder>>,
        payment_amount: u64
    ) {
        let intent_addr = object::object_address(&intent);
        let solver_addr = signer::address_of(solver);

        // 1. Start the session (this unlocks 0 tokens, but creates the session)
        let (unlocked_fa, session) = fa_intent::start_fa_offering_session(
            solver, intent
        );

        // Deposit the unlocked tokens (which are 0 for inflow intents)
        primary_fungible_store::deposit(solver_addr, unlocked_fa);

        // 2. Infer desired metadata from the intent's stored argument
        let argument = intent::get_argument(&session);
        let desired_metadata = fa_intent::get_desired_metadata(argument);

        // Get intent_id for GMP (stored in the argument)
        let intent_id_opt = fa_intent::get_intent_id(argument);
        let is_cross_chain = option::is_some(&intent_id_opt);

        // 3. For cross-chain intents, check escrow confirmation before proceeding
        let intent_id_bytes = if (is_cross_chain) {
            let intent_id_addr = *option::borrow(&intent_id_opt);
            let id_bytes = bcs::to_bytes(&intent_id_addr);
            // Check escrow is confirmed
            assert!(
                gmp_intent_state::is_escrow_confirmed(id_bytes),
                error::invalid_state(E_ESCROW_NOT_CONFIRMED)
            );
            id_bytes
        } else {
            std::vector::empty<u8>()
        };

        // 4. Withdraw the desired tokens from solver's account
        let payment_fa =
            primary_fungible_store::withdraw(solver, desired_metadata, payment_amount);

        // 5. Finish the session by providing the payment tokens and emit fulfillment event
        fa_intent::finish_fa_receiving_session_with_event(
            session,
            payment_fa,
            intent_addr,
            solver_addr
        );

        // 6. Unregister intent from the registry
        intent_registry::unregister_intent(intent_addr);

        // 7. For cross-chain intents, send FulfillmentProof to connected chain and clean up state
        if (is_cross_chain) {
            // Get dst_chain_id from GMP state
            let dst_chain_id = gmp_intent_state::get_dst_chain_id(intent_id_bytes);

            // Convert solver address to 32-byte vector
            let solver_addr_bytes = bcs::to_bytes(&solver_addr);

            // Send fulfillment proof to connected chain
            let _nonce = intent_gmp_hub::send_fulfillment_proof(
                solver,
                dst_chain_id,
                intent_id_bytes,
                solver_addr_bytes,
                payment_amount,
                timestamp::now_seconds(),
            );

            // Remove intent from GMP state tracking
            gmp_intent_state::remove_intent(intent_id_bytes);
        };
    }

    /// Creates an inflow intent and returns the intent object.
    ///
    /// This is the core implementation that both the entry function and tests use.
    ///
    /// # Note on parameter types:
    /// - `offered_metadata_addr` uses `address` because the offered tokens are on a different chain
    ///   (connected chain), so the metadata object doesn't exist on the hub chain. We can't validate
    ///   it here - validation happens on the connected chain where the escrow was created.
    /// - `desired_metadata` uses `Object<Metadata>` because the desired tokens are on the hub chain,
    ///   so we can validate the object exists and is the correct type.
    ///
    /// # Arguments
    /// - `account`: Signer of the requester creating the intent
    /// - `offered_metadata_addr`: Address of the token metadata being offered (locked on connected chain)
    /// - `offered_amount`: Amount of tokens offered (locked in escrow on connected chain)
    /// - `offered_chain_id`: Chain ID where the escrow is created (connected chain)
    /// - `desired_metadata`: Metadata object of the desired token type (on hub chain)
    /// - `desired_amount`: Amount of desired tokens
    /// - `desired_chain_id`: Chain ID of the hub chain (where this intent is created)
    /// - `expiry_time`: Unix timestamp when intent expires
    /// - `intent_id`: Intent ID for cross-chain linking
    /// - `solver`: Address of the solver authorized to fulfill this intent (must be registered)
    /// - `solver_addr_connected_chain`: Solver's address on the connected chain (used in GMP message for authorization)
    /// - `solver_signature`: Ed25519 signature from the solver authorizing this intent
    /// - `requester_addr_connected_chain`: Requester's address on the connected chain (for escrow lookup)
    ///
    /// # Returns
    /// - `Object<Intent<FungibleStoreManager, FungibleAssetLimitOrder>>`: The created intent object
    ///
    /// # Aborts
    /// - `ESOLVER_NOT_REGISTERED`: Solver is not registered in the solver registry
    /// - `E_INVALID_SIGNATURE`: Signature verification failed
    public fun create_inflow_intent(
        account: &signer,
        offered_metadata_addr: address,
        offered_amount: u64,
        offered_chain_id: u64,
        desired_metadata: Object<Metadata>,
        desired_amount: u64,
        desired_chain_id: u64,
        expiry_time: u64,
        intent_id: address,
        solver: address,
        solver_addr_connected_chain: address,
        solver_signature: vector<u8>,
        requester_addr_connected_chain: address
    ): Object<Intent<FungibleStoreManager, FungibleAssetLimitOrder>> {
        // Withdraw 0 tokens of DESIRED type (not offered type).
        // Why: The offered token metadata is on the connected chain, so the Object doesn't exist here.
        // We use desired_metadata (which exists on hub) to create a placeholder FungibleAsset.
        // No actual tokens are locked on hub for inflow - they're locked on connected chain.
        let fa: FungibleAsset =
            primary_fungible_store::withdraw(account, desired_metadata, 0);

        // Get desired_metadata address for the raw intent
        let desired_metadata_addr = object::object_address(&desired_metadata);

        // Verify solver signature using raw addresses (works for cross-chain where offered token doesn't exist locally)
        let intent_to_sign =
            intent_reservation::new_intent_to_sign_raw(
                offered_metadata_addr,
                offered_amount,
                offered_chain_id,
                desired_metadata_addr,
                desired_amount,
                desired_chain_id,
                expiry_time,
                signer::address_of(account),
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

        let intent_obj = fa_intent::create_fa_to_fa_intent(
            fa,
            offered_chain_id, // where escrow is created
            option::some(offered_amount), // Pass explicit offered_amount since tokens are locked on connected chain
            option::some(offered_metadata_addr), // Pass explicit offered_metadata_addr since tokens are on connected chain
            desired_metadata,
            desired_amount,
            desired_chain_id, // hub chain where this intent is created
            expiry_time,
            signer::address_of(account),
            reservation_result, // Reserved for specific solver
            false, // CRITICAL: All parts of a cross-chain intent MUST be non-revocable (including the hub intent)
            // Ensures consistent safety guarantees for approvers across chains
            option::some(intent_id), // Store the cross-chain intent_id for fulfillment event
            option::some(requester_addr_connected_chain) // Store requester address on connected chain for escrow lookup
        );

        // Register intent in the registry for dynamic account discovery
        let intent_addr = object::object_address(&intent_obj);
        intent_registry::register_intent(signer::address_of(account), intent_addr, expiry_time);

        // Convert intent_id to bytes for GMP
        let intent_id_bytes = bcs::to_bytes(&intent_id);

        // Register intent in GMP state tracking
        // Cast offered_chain_id from u64 to u32 for GMP (chain IDs fit in u32)
        let dst_chain_id = (offered_chain_id as u32);
        gmp_intent_state::register_inflow_intent(intent_id_bytes, intent_addr, dst_chain_id);

        // Send IntentRequirements to connected chain via GMP
        // Convert addresses to 32-byte vectors for GMP message
        let requester_addr_bytes = bcs::to_bytes(&requester_addr_connected_chain);
        let token_addr_bytes = bcs::to_bytes(&offered_metadata_addr);
        let solver_addr_bytes = bcs::to_bytes(&solver_addr_connected_chain);

        let _nonce = intent_gmp_hub::send_intent_requirements(
            account,
            dst_chain_id,
            intent_id_bytes,
            requester_addr_bytes,
            offered_amount,
            token_addr_bytes,
            solver_addr_bytes,
            expiry_time,
        );

        intent_obj
    }

    /// Entry function to create an inflow intent.
    ///
    /// Inflow intents have tokens locked on the connected chain (in escrow) and request tokens on the hub.
    /// The solver's public key is looked up from the on-chain solver registry.
    ///
    /// For argument descriptions and abort conditions, see `create_inflow_intent`.
    public entry fun create_inflow_intent_entry(
        account: &signer,
        offered_metadata_addr: address,
        offered_amount: u64,
        offered_chain_id: u64,
        desired_metadata: Object<Metadata>,
        desired_amount: u64,
        desired_chain_id: u64,
        expiry_time: u64,
        intent_id: address,
        solver: address,
        solver_addr_connected_chain: address,
        solver_signature: vector<u8>,
        requester_addr_connected_chain: address
    ) {
        let _intent_obj =
            create_inflow_intent(
                account,
                offered_metadata_addr,
                offered_amount,
                offered_chain_id,
                desired_metadata,
                desired_amount,
                desired_chain_id,
                expiry_time,
                intent_id,
                solver,
                solver_addr_connected_chain,
                solver_signature,
                requester_addr_connected_chain
            );
    }
}
