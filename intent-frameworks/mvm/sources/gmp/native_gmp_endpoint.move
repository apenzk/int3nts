/// Native GMP Endpoint
///
/// A native GMP endpoint that can be used for local testing, CI, or production
/// with a trusted relay or DKG-based message verification.
///
/// ## Purpose
///
/// This endpoint provides a standardized interface for cross-chain messaging.
/// In production, this can be replaced by LayerZero's endpoint or used directly
/// with your own relay infrastructure.
///
/// ## Functions
///
/// - `lz_send`: Emit a MessageSent event for the relay to pick up
/// - `deliver_message`: Called by relay to deliver messages to destination
/// - `set_trusted_remote`: Configure trusted source addresses per chain
module mvmt_intent::native_gmp_endpoint {
    use std::vector;
    use std::signer;
    use aptos_framework::event;
    use aptos_std::table::{Self, Table};
    use mvmt_intent::gmp_common;
    use mvmt_intent::intent_gmp_hub;

    // ============================================================================
    // ERROR CODES
    // ============================================================================

    /// Caller is not an authorized relay
    const EUNAUTHORIZED_RELAY: u64 = 1;
    /// Message nonce already used (replay attack)
    const ENONCE_ALREADY_USED: u64 = 2;
    /// Invalid payload format
    const EINVALID_PAYLOAD: u64 = 3;
    /// Source address is not trusted for the given chain
    const EUNTRUSTED_REMOTE: u64 = 4;
    /// No trusted remote configured for the source chain
    const ENO_TRUSTED_REMOTE: u64 = 5;
    /// Caller is not the admin
    const EUNAUTHORIZED_ADMIN: u64 = 6;
    /// Unknown message type in payload
    const EUNKNOWN_MESSAGE_TYPE: u64 = 7;

    // ============================================================================
    // MESSAGE TYPE CONSTANTS
    // ============================================================================

    /// IntentRequirements: Hub -> Connected Chain (0x01)
    const MESSAGE_TYPE_INTENT_REQUIREMENTS: u8 = 0x01;
    /// EscrowConfirmation: Connected Chain -> Hub (0x02)
    const MESSAGE_TYPE_ESCROW_CONFIRMATION: u8 = 0x02;
    /// FulfillmentProof: Either direction (0x03)
    const MESSAGE_TYPE_FULFILLMENT_PROOF: u8 = 0x03;

    // ============================================================================
    // EVENTS
    // ============================================================================

    #[event]
    /// Emitted when a message is sent to another chain.
    /// The GMP relay monitors these events and delivers them to the destination.
    struct MessageSent has drop, store {
        /// Destination chain endpoint ID (e.g., Solana = 30168)
        dst_chain_id: u32,
        /// Destination address (32 bytes, the receiving program)
        dst_addr: vector<u8>,
        /// Message payload (encoded GMP message)
        payload: vector<u8>,
        /// Sender address
        sender: address,
        /// Sequence number for ordering
        nonce: u64,
    }

    #[event]
    /// Emitted when a message is delivered from another chain.
    struct MessageDelivered has drop, store {
        /// Source chain endpoint ID
        src_chain_id: u32,
        /// Source address (32 bytes, the sending program)
        src_addr: vector<u8>,
        /// Message payload (encoded GMP message)
        payload: vector<u8>,
        /// Nonce from source chain
        nonce: u64,
    }

    // ============================================================================
    // STATE
    // ============================================================================

    /// Global endpoint configuration.
    struct EndpointConfig has key {
        /// Authorized relay addresses (can call deliver_message)
        authorized_relays: vector<address>,
        /// Next outbound nonce
        next_nonce: u64,
        /// Admin address (can configure trusted remotes)
        admin: address,
        /// Trusted remote addresses per source chain (chain_id -> trusted_addr)
        trusted_remotes: Table<u32, vector<u8>>,
        /// Inbound nonces per source chain (chain_id -> last_nonce)
        inbound_nonces: Table<u32, u64>,
    }

    // ============================================================================
    // INITIALIZATION
    // ============================================================================

    /// Initialize the native GMP endpoint.
    /// Called once during deployment.
    public entry fun initialize(admin: &signer) {
        let admin_addr = signer::address_of(admin);

        // Create config with admin as initial authorized relay
        let authorized_relays = vector::empty<address>();
        vector::push_back(&mut authorized_relays, admin_addr);

        move_to(admin, EndpointConfig {
            authorized_relays,
            next_nonce: 1,
            admin: admin_addr,
            trusted_remotes: table::new(),
            inbound_nonces: table::new(),
        });
    }

    // ============================================================================
    // OUTBOUND: Send message to another chain
    // ============================================================================

    /// Send a cross-chain message.
    ///
    /// Emits a `MessageSent` event that the GMP relay monitors.
    /// The relay picks up the event and calls `deliver_message` on the
    /// destination chain.
    ///
    /// # Arguments
    /// - `sender`: The account sending the message
    /// - `dst_chain_id`: Destination chain endpoint ID (e.g., Solana = 30168)
    /// - `dst_addr`: Destination address (32 bytes, the receiving program)
    /// - `payload`: Message payload (encoded GMP message)
    ///
    /// # Returns
    /// - Nonce assigned to this message
    public fun lz_send(
        sender: &signer,
        dst_chain_id: u32,
        dst_addr: vector<u8>,
        payload: vector<u8>,
    ): u64 acquires EndpointConfig {
        let sender_addr = signer::address_of(sender);

        // Get and increment nonce
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);
        let nonce = config.next_nonce;
        config.next_nonce = nonce + 1;

        // Emit event for relay to pick up
        event::emit(MessageSent {
            dst_chain_id,
            dst_addr,
            payload,
            sender: sender_addr,
            nonce,
        });

        nonce
    }

    /// Entry function wrapper for lz_send.
    public entry fun lz_send_entry(
        sender: &signer,
        dst_chain_id: u32,
        dst_addr: vector<u8>,
        payload: vector<u8>,
    ) acquires EndpointConfig {
        lz_send(sender, dst_chain_id, dst_addr, payload);
    }

    // ============================================================================
    // INBOUND: Deliver message from another chain
    // ============================================================================

    /// Deliver a cross-chain message from another chain.
    ///
    /// Called by the GMP relay after observing a `MessageSent` event
    /// on the source chain. The relay decodes the event, constructs this
    /// call, and submits it to the destination chain.
    ///
    /// # Arguments
    /// - `relay`: The authorized relay account (must be in authorized_relays list)
    /// - `src_chain_id`: Source chain endpoint ID
    /// - `src_addr`: Source address (32 bytes, the sending program)
    /// - `payload`: Message payload (encoded GMP message)
    /// - `nonce`: Nonce from source chain (for ordering/replay protection)
    ///
    /// # Aborts
    /// - EUNAUTHORIZED_RELAY: If caller is not an authorized relay
    /// - EUNTRUSTED_REMOTE: If source address is not trusted for the chain
    /// - ENONCE_ALREADY_USED: If nonce has already been processed (replay)
    public fun deliver_message(
        relay: &signer,
        src_chain_id: u32,
        src_addr: vector<u8>,
        payload: vector<u8>,
        nonce: u64,
    ) acquires EndpointConfig {
        let relay_addr = signer::address_of(relay);

        // Verify relay is authorized
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);
        assert!(is_authorized_relay(&config.authorized_relays, relay_addr), EUNAUTHORIZED_RELAY);

        // Verify trusted remote: source address must be trusted for this chain
        assert!(table::contains(&config.trusted_remotes, src_chain_id), ENO_TRUSTED_REMOTE);
        let trusted_addr = table::borrow(&config.trusted_remotes, src_chain_id);
        assert!(&src_addr == trusted_addr, EUNTRUSTED_REMOTE);

        // Replay protection: check and update inbound nonce
        if (table::contains(&config.inbound_nonces, src_chain_id)) {
            let last_nonce = *table::borrow(&config.inbound_nonces, src_chain_id);
            assert!(nonce > last_nonce, ENONCE_ALREADY_USED);
            *table::borrow_mut(&mut config.inbound_nonces, src_chain_id) = nonce;
        } else {
            // First message from this chain
            table::add(&mut config.inbound_nonces, src_chain_id, nonce);
        };

        // Emit delivery event (copy payload before routing consumes it)
        event::emit(MessageDelivered {
            src_chain_id,
            src_addr: copy src_addr,
            payload: copy payload,
            nonce,
        });

        // Route message to destination module based on payload type
        route_message(src_chain_id, src_addr, payload);
    }

    /// Route a GMP message to the appropriate handler based on payload type.
    ///
    /// Message types:
    /// - 0x02 (EscrowConfirmation): Route to intent_gmp_hub::receive_escrow_confirmation
    /// - 0x03 (FulfillmentProof): Route to intent_gmp_hub::receive_fulfillment_proof
    ///
    /// Note: 0x01 (IntentRequirements) is outbound-only from hub, not expected inbound.
    fun route_message(
        src_chain_id: u32,
        src_addr: vector<u8>,
        payload: vector<u8>,
    ) {
        // Peek at message type (first byte)
        let msg_type = gmp_common::peek_message_type(&payload);

        if (msg_type == MESSAGE_TYPE_ESCROW_CONFIRMATION) {
            // Connected chain confirms escrow was created
            intent_gmp_hub::receive_escrow_confirmation(src_chain_id, src_addr, payload);
        } else if (msg_type == MESSAGE_TYPE_FULFILLMENT_PROOF) {
            // Connected chain reports fulfillment (for outflow intents)
            intent_gmp_hub::receive_fulfillment_proof(src_chain_id, src_addr, payload);
        } else {
            // IntentRequirements (0x01) is outbound-only, shouldn't be received
            abort EUNKNOWN_MESSAGE_TYPE
        };
    }

    /// Entry function wrapper for deliver_message.
    public entry fun deliver_message_entry(
        relay: &signer,
        src_chain_id: u32,
        src_addr: vector<u8>,
        payload: vector<u8>,
        nonce: u64,
    ) acquires EndpointConfig {
        deliver_message(relay, src_chain_id, src_addr, payload, nonce);
    }

    // ============================================================================
    // ADMIN FUNCTIONS
    // ============================================================================

    /// Set a trusted remote address for a source chain.
    /// Only the admin can call this function.
    ///
    /// # Arguments
    /// - `admin`: The admin signer
    /// - `src_chain_id`: Source chain endpoint ID (e.g., Solana = 30168)
    /// - `trusted_addr`: Trusted source address (32 bytes)
    public entry fun set_trusted_remote(
        admin: &signer,
        src_chain_id: u32,
        trusted_addr: vector<u8>,
    ) acquires EndpointConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);

        // Verify caller is admin
        assert!(config.admin == admin_addr, EUNAUTHORIZED_ADMIN);

        // Store or update trusted remote
        if (table::contains(&config.trusted_remotes, src_chain_id)) {
            *table::borrow_mut(&mut config.trusted_remotes, src_chain_id) = trusted_addr;
        } else {
            table::add(&mut config.trusted_remotes, src_chain_id, trusted_addr);
        };
    }

    /// Add an authorized relay address.
    public entry fun add_authorized_relay(
        admin: &signer,
        relay_addr: address,
    ) acquires EndpointConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);

        // Only existing authorized relays can add new ones
        assert!(is_authorized_relay(&config.authorized_relays, admin_addr), EUNAUTHORIZED_RELAY);

        // Add if not already present
        if (!is_authorized_relay(&config.authorized_relays, relay_addr)) {
            vector::push_back(&mut config.authorized_relays, relay_addr);
        };
    }

    /// Remove an authorized relay address.
    public entry fun remove_authorized_relay(
        admin: &signer,
        relay_addr: address,
    ) acquires EndpointConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);

        // Only existing authorized relays can remove
        assert!(is_authorized_relay(&config.authorized_relays, admin_addr), EUNAUTHORIZED_RELAY);

        // Find and remove
        let (found, index) = vector::index_of(&config.authorized_relays, &relay_addr);
        if (found) {
            vector::remove(&mut config.authorized_relays, index);
        };
    }

    // ============================================================================
    // VIEW FUNCTIONS
    // ============================================================================

    #[view]
    /// Get the next outbound nonce.
    public fun get_next_nonce(): u64 acquires EndpointConfig {
        borrow_global<EndpointConfig>(@mvmt_intent).next_nonce
    }

    #[view]
    /// Check if an address is an authorized relay.
    public fun is_relay_authorized(addr: address): bool acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        is_authorized_relay(&config.authorized_relays, addr)
    }

    #[view]
    /// Get the trusted remote address for a source chain.
    /// Returns empty vector if no trusted remote is configured.
    public fun get_trusted_remote(src_chain_id: u32): vector<u8> acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        if (table::contains(&config.trusted_remotes, src_chain_id)) {
            *table::borrow(&config.trusted_remotes, src_chain_id)
        } else {
            vector::empty()
        }
    }

    #[view]
    /// Get the last processed inbound nonce for a source chain.
    /// Returns 0 if no messages have been received from this chain.
    public fun get_inbound_nonce(src_chain_id: u32): u64 acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        if (table::contains(&config.inbound_nonces, src_chain_id)) {
            *table::borrow(&config.inbound_nonces, src_chain_id)
        } else {
            0
        }
    }

    #[view]
    /// Check if a source chain has a trusted remote configured.
    public fun has_trusted_remote(src_chain_id: u32): bool acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        table::contains(&config.trusted_remotes, src_chain_id)
    }

    // ============================================================================
    // INTERNAL HELPERS
    // ============================================================================

    /// Check if an address is in the authorized relays list.
    fun is_authorized_relay(relays: &vector<address>, addr: address): bool {
        let len = vector::length(relays);
        let i = 0;
        while (i < len) {
            if (*vector::borrow(relays, i) == addr) {
                return true
            };
            i = i + 1;
        };
        false
    }
}
