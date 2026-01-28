module mvmt_intent::fa_intent_outflow {
    use std::signer;
    use std::option;
    use std::error;
    use aptos_framework::primary_fungible_store;
    use aptos_framework::object::Object;
    use aptos_framework::fungible_asset::{Self as fungible_asset, FungibleAsset, Metadata};
    use mvmt_intent::fa_intent_with_oracle;
    use mvmt_intent::intent::Intent;
    use mvmt_intent::intent_reservation;
    use mvmt_intent::intent_registry;
    use aptos_std::ed25519;

    /// The solver signature is invalid and cannot be verified.
    const EINVALID_SIGNATURE: u64 = 2;
    /// The requester address on the connected chain is invalid (zero address).
    const EINVALID_REQUESTER_ADDR: u64 = 3;
    /// The approver (trusted-gmp) config has not been initialized.
    const EAPPROVER_NOT_INITIALIZED: u64 = 4;
    /// Only the module publisher can initialize/update the approver config.
    const ENOT_AUTHORIZED: u64 = 5;

    // ============================================================================
    // APPROVER CONFIG (Global trusted-gmp approver public key)
    // ============================================================================

    /// Global configuration storing the trusted-gmp approver's public key.
    struct ApproverConfig has key {
        /// The Ed25519 public key of the trusted-gmp (32 bytes)
        public_key: vector<u8>,
    }

    /// Initialize the approver configuration with the trusted-gmp public key.
    /// Can only be called by the module publisher (@mvmt_intent).
    ///
    /// # Arguments
    /// - `admin`: Must be the module publisher
    /// - `approver_public_key`: 32-byte Ed25519 public key of the trusted-gmp
    public entry fun initialize_approver(
        admin: &signer,
        approver_public_key: vector<u8>
    ) acquires ApproverConfig {
        let admin_addr = signer::address_of(admin);
        assert!(admin_addr == @mvmt_intent, error::permission_denied(ENOT_AUTHORIZED));
        
        if (exists<ApproverConfig>(@mvmt_intent)) {
            // Update existing config
            let config = borrow_global_mut<ApproverConfig>(@mvmt_intent);
            config.public_key = approver_public_key;
        } else {
            // Create new config
            move_to(admin, ApproverConfig {
                public_key: approver_public_key,
            });
        }
    }

    /// Get the configured approver public key.
    /// Aborts if approver config has not been initialized.
    fun get_approver_public_key(): vector<u8> acquires ApproverConfig {
        assert!(exists<ApproverConfig>(@mvmt_intent), error::not_found(EAPPROVER_NOT_INITIALIZED));
        let config = borrow_global<ApproverConfig>(@mvmt_intent);
        config.public_key
    }

    #[view]
    /// View function to check if approver is configured and get the public key.
    public fun get_approver_config(): vector<u8> acquires ApproverConfig {
        get_approver_public_key()
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

    /// Entry function for solver to fulfill an outflow intent.
    ///
    /// Outflow intents have tokens locked on the hub chain and request tokens on the connected chain.
    /// The solver must first transfer tokens on the connected chain, then the approver approves that transaction.
    /// The solver receives the locked tokens from the hub as reward, and provides 0 tokens as payment
    /// (since desired_amount = 0 on hub for outflow intents).
    /// Approver signature is required - it proves the solver transferred tokens on the connected chain.
    ///
    /// # Arguments
    /// - `solver`: Signer fulfilling the intent
    /// - `intent`: Object reference to the outflow intent to fulfill (OracleGuardedLimitOrder)
    /// - `approver_signature_bytes`: Approver's Ed25519 signature as bytes (signs the intent_id, proves connected chain transfer)
    public entry fun fulfill_outflow_intent(
        solver: &signer,
        intent: Object<Intent<fa_intent_with_oracle::FungibleStoreManager, fa_intent_with_oracle::OracleGuardedLimitOrder>>,
        approver_signature_bytes: vector<u8>
    ) {
        let solver_addr = signer::address_of(solver);
        let intent_addr = aptos_framework::object::object_address(&intent);

        // 1. Start the session (unlocks actual tokens that were locked on hub - these are the solver's reward)
        let (unlocked_fa, session) =
            fa_intent_with_oracle::start_fa_offering_session(solver, intent);

        // 2. Infer payment metadata from the unlocked tokens BEFORE depositing (FungibleAsset doesn't have copy)
        // For outflow, desired_metadata matches offered_metadata (placeholder), so we use unlocked tokens' metadata
        let payment_metadata = fungible_asset::asset_metadata(&unlocked_fa);

        // 3. Deposit unlocked tokens to solver (they get the locked tokens as payment for their work)
        primary_fungible_store::deposit(solver_addr, unlocked_fa);

        // 4. Withdraw 0 tokens as payment (desired_amount = 0 on hub for outflow)
        // The actual desired tokens are on the connected chain, which the solver already transferred
        let solver_payment = primary_fungible_store::withdraw(
            solver, payment_metadata, 0
        );

        // 5. Convert signature bytes to ed25519::Signature
        let approver_signature =
            ed25519::new_signature_from_bytes(approver_signature_bytes);

        // 6. Create approver witness - signature itself is the approval
        // The intent_id is stored in the session argument and will be used automatically
        // by finish_fa_receiving_session_with_oracle for signature verification
        let witness =
            fa_intent_with_oracle::new_oracle_signature_witness(
                0, // reported_value: signature verification is what matters, this is just metadata
                approver_signature
            );

        // 7. Complete the intent with approver signature (proves connected chain transfer happened)
        // The finish function will verify the signature against the intent_id stored in the argument
        // Payment amount is 0, which matches desired_amount = 0 for outflow intents
        fa_intent_with_oracle::finish_fa_receiving_session_with_oracle(
            session,
            solver_payment,
            option::some(witness)
        );

        // 8. Unregister intent from the registry
        intent_registry::unregister_intent(intent_addr);
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
    /// - `approver_public_key`: Public key of the trusted-gmp (approver) that will approve the connected chain transaction (32 bytes)
    /// - `solver`: Address of the solver authorized to fulfill this intent (must be registered)
    /// - `solver_signature`: Ed25519 signature from the solver authorizing this intent
    ///
    /// # Returns
    /// - `Object<Intent<FungibleStoreManager, OracleGuardedLimitOrder>>`: The created intent object
    ///
    /// # Aborts
    /// - `ESOLVER_NOT_REGISTERED`: Solver is not registered in the solver registry
    /// - `EINVALID_SIGNATURE`: Signature verification failed
    /// - `EINVALID_REQUESTER_ADDR`: requester_addr_connected_chain is zero address (0x0)
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
        approver_public_key: vector<u8>, // 32 bytes
        solver: address,
        solver_signature: vector<u8>
    ): Object<Intent<fa_intent_with_oracle::FungibleStoreManager, fa_intent_with_oracle::OracleGuardedLimitOrder>> {
        // Validate requester_addr_connected_chain is not zero address
        // Outflow intents require a valid address on the connected chain where the solver should send tokens
        assert!(
            requester_addr_connected_chain != @0x0,
            error::invalid_argument(EINVALID_REQUESTER_ADDR)
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
            error::invalid_argument(EINVALID_SIGNATURE)
        );

        // Build ed25519::UnvalidatedPublicKey from bytes
        let approver_pk =
            ed25519::new_unvalidated_public_key_from_bytes(approver_public_key);

        // Create oracle requirement: signature itself is the approval, min_reported_value is 0
        // (the signature verification is what matters, not the reported_value)
        let requirement =
            fa_intent_with_oracle::new_oracle_signature_requirement(
                0, // min_reported_value: signature verification is what matters, this check always passes
                approver_pk
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
            // Ensures consistent safety guarantees for approvers across chains
            intent_id,
            option::some(requester_addr_connected_chain), // Store where solver should send tokens on connected chain
            reservation_result // Reserved for specific solver
        );

        // Register intent in the registry for dynamic account discovery
        let intent_addr = aptos_framework::object::object_address(&intent_obj);
        intent_registry::register_intent(signer::address_of(requester_signer), intent_addr, expiry_time);

        intent_obj
    }

    /// Entry function to create an outflow intent.
    ///
    /// Reads the approver public key from the module's ApproverConfig.
    /// The approver config must be initialized via `initialize_approver` before calling this function.
    ///
    /// # Arguments
    /// - `requester_signer`: The account creating the intent (tokens will be withdrawn from this account)
    /// - `offered_metadata`: Metadata of the token being offered (locked on hub chain)
    /// - `offered_amount`: Amount of tokens to lock
    /// - `offered_chain_id`: Chain ID where offered tokens are located (hub chain)
    /// - `desired_metadata_addr`: Address of the desired token metadata (on connected chain)
    /// - `desired_amount`: Amount of tokens desired on connected chain
    /// - `desired_chain_id`: Chain ID where desired tokens are located (connected chain)
    /// - `expiry_time`: Unix timestamp when the intent expires
    /// - `intent_id`: Unique identifier for cross-chain correlation
    /// - `requester_addr_connected_chain`: Requester's address on the connected chain
    /// - `solver`: Address of the solver who will fulfill the intent
    /// - `solver_signature`: Solver's signature approving the intent
    ///
    /// # Aborts
    /// - `EAPPROVER_NOT_INITIALIZED`: Approver config has not been set
    /// - `EINVALID_SIGNATURE`: Solver signature verification failed
    /// - `EINVALID_REQUESTER_ADDR`: requester_addr_connected_chain is zero address
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
        solver_signature: vector<u8>
    ) acquires ApproverConfig {
        // Read approver public key from stored config
        let approver_public_key = get_approver_public_key();
        
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
                approver_public_key,
                solver,
                solver_signature
            );
    }
}
