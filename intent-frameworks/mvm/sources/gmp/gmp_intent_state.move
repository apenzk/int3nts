/// GMP Intent State Tracking
///
/// Tracks GMP-related state for cross-chain intents:
/// - escrow_confirmed: Set when EscrowConfirmation is received from connected chain
/// - fulfillment_proof_received: Set when FulfillmentProof is received from connected chain
///
/// This module is used by fa_intent_inflow and fa_intent_outflow to:
/// - Gate fulfillment on escrow confirmation (inflow)
/// - Auto-release tokens on fulfillment proof (outflow)
module mvmt_intent::gmp_intent_state {
    use std::error;
    use std::signer;
    use std::vector;
    use aptos_framework::event;
    use aptos_std::table::{Self, Table};

    // ============================================================================
    // ERROR CODES
    // ============================================================================

    const E_NOT_INITIALIZED: u64 = 1;
    const E_ALREADY_INITIALIZED: u64 = 2;
    const E_NOT_AUTHORIZED: u64 = 3;
    const E_INTENT_NOT_FOUND: u64 = 4;
    const E_ESCROW_NOT_CONFIRMED: u64 = 5;
    const E_ALREADY_CONFIRMED: u64 = 6;
    const E_ALREADY_FULFILLED: u64 = 7;

    // ============================================================================
    // CONSTANTS
    // ============================================================================

    /// Flow type for inflow intents (tokens locked on connected chain, desired on hub)
    const FLOW_TYPE_INFLOW: u8 = 1;
    /// Flow type for outflow intents (tokens locked on hub, desired on connected chain)
    const FLOW_TYPE_OUTFLOW: u8 = 2;

    // ============================================================================
    // STATE
    // ============================================================================

    /// Per-intent GMP state.
    struct IntentGmpState has store, copy, drop {
        /// Intent ID (32 bytes)
        intent_id: vector<u8>,
        /// Intent object address on hub chain (for looking up the intent)
        intent_addr: address,
        /// Destination chain ID for GMP messages
        dst_chain_id: u32,
        /// Flow type: 1 = inflow, 2 = outflow
        flow_type: u8,
        /// Whether escrow confirmation has been received
        escrow_confirmed: bool,
        /// Whether fulfillment proof has been received
        fulfillment_proof_received: bool,
        /// Solver's address on the connected chain (for FulfillmentProof routing)
        solver_addr_connected_chain: vector<u8>,
    }

    /// Global storage for intent GMP states.
    struct GmpStateStore has key {
        /// Maps intent_id (as bytes) -> IntentGmpState
        states: Table<vector<u8>, IntentGmpState>,
        /// Admin address
        admin: address,
    }

    // ============================================================================
    // EVENTS
    // ============================================================================

    #[event]
    /// Emitted when a new intent GMP state is created.
    struct IntentGmpStateCreated has drop, store {
        intent_id: vector<u8>,
        dst_chain_id: u32,
    }

    #[event]
    /// Emitted when escrow confirmation is recorded.
    struct EscrowConfirmed has drop, store {
        intent_id: vector<u8>,
    }

    #[event]
    /// Emitted when fulfillment proof is recorded.
    struct FulfillmentProofRecorded has drop, store {
        intent_id: vector<u8>,
    }

    // ============================================================================
    // INITIALIZATION
    // ============================================================================

    /// Initialize the GMP state store.
    /// Must be called once during deployment.
    public entry fun initialize(admin: &signer) {
        let admin_addr = signer::address_of(admin);

        assert!(
            admin_addr == @mvmt_intent,
            error::permission_denied(E_NOT_AUTHORIZED)
        );

        assert!(
            !exists<GmpStateStore>(@mvmt_intent),
            error::already_exists(E_ALREADY_INITIALIZED)
        );

        move_to(admin, GmpStateStore {
            states: table::new(),
            admin: admin_addr,
        });
    }

    /// Check if the store is initialized.
    public fun is_initialized(): bool {
        exists<GmpStateStore>(@mvmt_intent)
    }

    // ============================================================================
    // STATE MANAGEMENT
    // ============================================================================

    /// Register a new inflow intent for GMP state tracking.
    /// Called when a cross-chain inflow intent is created.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    /// - `intent_addr`: Address of the intent object on hub chain
    /// - `dst_chain_id`: Destination chain for GMP messages
    /// - `solver_addr_connected_chain`: Solver's address on the connected chain (for FulfillmentProof)
    public fun register_inflow_intent(
        intent_id: vector<u8>,
        intent_addr: address,
        dst_chain_id: u32,
        solver_addr_connected_chain: vector<u8>,
    ) acquires GmpStateStore {
        register_intent_internal(intent_id, intent_addr, dst_chain_id, FLOW_TYPE_INFLOW, solver_addr_connected_chain);
    }

    /// Register a new outflow intent for GMP state tracking.
    /// Called when a cross-chain outflow intent is created.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    /// - `intent_addr`: Address of the intent object on hub chain
    /// - `dst_chain_id`: Destination chain for GMP messages
    public fun register_outflow_intent(
        intent_id: vector<u8>,
        intent_addr: address,
        dst_chain_id: u32,
    ) acquires GmpStateStore {
        // Outflow doesn't need solver_addr_connected_chain for FulfillmentProof (it's inbound)
        register_intent_internal(intent_id, intent_addr, dst_chain_id, FLOW_TYPE_OUTFLOW, vector::empty());
    }

    /// Internal function to register an intent.
    fun register_intent_internal(
        intent_id: vector<u8>,
        intent_addr: address,
        dst_chain_id: u32,
        flow_type: u8,
        solver_addr_connected_chain: vector<u8>,
    ) acquires GmpStateStore {
        let store = borrow_global_mut<GmpStateStore>(@mvmt_intent);

        // If already registered, just return (idempotent)
        if (table::contains(&store.states, intent_id)) {
            return
        };

        let state = IntentGmpState {
            intent_id: intent_id,
            intent_addr,
            dst_chain_id,
            flow_type,
            escrow_confirmed: false,
            fulfillment_proof_received: false,
            solver_addr_connected_chain,
        };

        table::add(&mut store.states, intent_id, state);

        event::emit(IntentGmpStateCreated {
            intent_id,
            dst_chain_id,
        });
    }

    /// Legacy register_intent for backwards compatibility.
    /// Uses inflow flow type and zero address for intent_addr.
    public fun register_intent(
        intent_id: vector<u8>,
        dst_chain_id: u32,
    ) acquires GmpStateStore {
        register_intent_internal(intent_id, @0x0, dst_chain_id, FLOW_TYPE_INFLOW, vector::empty());
    }

    /// Record escrow confirmation for an intent.
    /// Called when EscrowConfirmation GMP message is received.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - true if this is a new confirmation, false if already confirmed (idempotent)
    public fun confirm_escrow(intent_id: vector<u8>): bool acquires GmpStateStore {
        let store = borrow_global_mut<GmpStateStore>(@mvmt_intent);

        assert!(
            table::contains(&store.states, intent_id),
            error::not_found(E_INTENT_NOT_FOUND)
        );

        let state = table::borrow_mut(&mut store.states, intent_id);

        if (state.escrow_confirmed) {
            // Already confirmed - idempotent
            return false
        };

        state.escrow_confirmed = true;

        event::emit(EscrowConfirmed { intent_id });

        true
    }

    /// Record fulfillment proof for an intent.
    /// Called when FulfillmentProof GMP message is received.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - true if this is a new recording, false if already recorded (idempotent)
    public fun record_fulfillment_proof(intent_id: vector<u8>): bool acquires GmpStateStore {
        let store = borrow_global_mut<GmpStateStore>(@mvmt_intent);

        assert!(
            table::contains(&store.states, intent_id),
            error::not_found(E_INTENT_NOT_FOUND)
        );

        let state = table::borrow_mut(&mut store.states, intent_id);

        if (state.fulfillment_proof_received) {
            // Already recorded - idempotent
            return false
        };

        state.fulfillment_proof_received = true;

        event::emit(FulfillmentProofRecorded { intent_id });

        true
    }

    #[view]
    /// Check if escrow is confirmed for an intent.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - true if escrow is confirmed, false otherwise
    public fun is_escrow_confirmed(intent_id: vector<u8>): bool acquires GmpStateStore {
        let store = borrow_global<GmpStateStore>(@mvmt_intent);

        if (!table::contains(&store.states, intent_id)) {
            return false
        };

        let state = table::borrow(&store.states, intent_id);
        state.escrow_confirmed
    }

    #[view]
    /// Check if fulfillment proof is received for an intent.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - true if fulfillment proof is received, false otherwise
    public fun is_fulfillment_proof_received(intent_id: vector<u8>): bool acquires GmpStateStore {
        let store = borrow_global<GmpStateStore>(@mvmt_intent);

        if (!table::contains(&store.states, intent_id)) {
            return false
        };

        let state = table::borrow(&store.states, intent_id);
        state.fulfillment_proof_received
    }

    /// Get the destination chain ID for an intent.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - Destination chain ID
    ///
    /// # Aborts
    /// - E_INTENT_NOT_FOUND if intent is not registered
    public fun get_dst_chain_id(intent_id: vector<u8>): u32 acquires GmpStateStore {
        let store = borrow_global<GmpStateStore>(@mvmt_intent);

        assert!(
            table::contains(&store.states, intent_id),
            error::not_found(E_INTENT_NOT_FOUND)
        );

        let state = table::borrow(&store.states, intent_id);
        state.dst_chain_id
    }

    /// Get the intent address for an intent.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - Intent object address on hub chain
    ///
    /// # Aborts
    /// - E_INTENT_NOT_FOUND if intent is not registered
    public fun get_intent_addr(intent_id: vector<u8>): address acquires GmpStateStore {
        let store = borrow_global<GmpStateStore>(@mvmt_intent);

        assert!(
            table::contains(&store.states, intent_id),
            error::not_found(E_INTENT_NOT_FOUND)
        );

        let state = table::borrow(&store.states, intent_id);
        state.intent_addr
    }

    /// Get the solver's connected chain address for an intent.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - Solver's address on the connected chain (32 bytes)
    ///
    /// # Aborts
    /// - E_INTENT_NOT_FOUND if intent is not registered
    public fun get_solver_addr_connected_chain(intent_id: vector<u8>): vector<u8> acquires GmpStateStore {
        let store = borrow_global<GmpStateStore>(@mvmt_intent);

        assert!(
            table::contains(&store.states, intent_id),
            error::not_found(E_INTENT_NOT_FOUND)
        );

        let state = table::borrow(&store.states, intent_id);
        state.solver_addr_connected_chain
    }

    /// Check if an intent is an outflow intent.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - true if outflow, false if inflow or not found
    public fun is_outflow_intent(intent_id: vector<u8>): bool acquires GmpStateStore {
        let store = borrow_global<GmpStateStore>(@mvmt_intent);

        if (!table::contains(&store.states, intent_id)) {
            return false
        };

        let state = table::borrow(&store.states, intent_id);
        state.flow_type == FLOW_TYPE_OUTFLOW
    }

    /// Check if an intent exists in the GMP state.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    ///
    /// # Returns
    /// - true if intent exists, false otherwise
    public fun intent_exists(intent_id: vector<u8>): bool acquires GmpStateStore {
        let store = borrow_global<GmpStateStore>(@mvmt_intent);
        table::contains(&store.states, intent_id)
    }

    /// Assert that escrow is confirmed for an intent.
    /// Aborts with E_ESCROW_NOT_CONFIRMED if not confirmed.
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    public fun assert_escrow_confirmed(intent_id: vector<u8>) acquires GmpStateStore {
        assert!(
            is_escrow_confirmed(intent_id),
            error::invalid_state(E_ESCROW_NOT_CONFIRMED)
        );
    }

    /// Remove an intent from state tracking.
    /// Called after intent is fully completed (fulfilled or expired).
    ///
    /// # Arguments
    /// - `intent_id`: 32-byte intent identifier
    public fun remove_intent(intent_id: vector<u8>) acquires GmpStateStore {
        let store = borrow_global_mut<GmpStateStore>(@mvmt_intent);

        if (table::contains(&store.states, intent_id)) {
            table::remove(&mut store.states, intent_id);
        };
    }

    // ============================================================================
    // VIEW FUNCTIONS
    // ============================================================================

    #[view]
    /// Get the full GMP state for an intent.
    public fun get_intent_state(intent_id: vector<u8>): (bool, bool, bool) acquires GmpStateStore {
        let store = borrow_global<GmpStateStore>(@mvmt_intent);

        if (!table::contains(&store.states, intent_id)) {
            return (false, false, false) // (exists, escrow_confirmed, fulfillment_proof_received)
        };

        let state = table::borrow(&store.states, intent_id);
        (true, state.escrow_confirmed, state.fulfillment_proof_received)
    }

    // ============================================================================
    // TEST HELPERS
    // ============================================================================

    #[test_only]
    /// Initialize for testing.
    public fun init_for_test(admin: &signer) {
        if (!exists<GmpStateStore>(@mvmt_intent)) {
            move_to(admin, GmpStateStore {
                states: table::new(),
                admin: signer::address_of(admin),
            });
        };
    }
}
