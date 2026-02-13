/// IntentGmp - Hub Version
///
/// GMP endpoint for cross-chain message delivery and routing on the hub chain.
/// Routes messages to intent_gmp_hub only.
///
/// ## Architecture
///
/// - gmp_sender: Send functionality (gmp_send)
/// - intent_gmp: Receive/routing functionality (this module)
///
/// ## Functions
///
/// - `deliver_message`: Called by relay to deliver messages to destination
/// - `set_remote_gmp_endpoint_addr`: Configure remote GMP endpoint addresses per chain
///
/// For sending messages, use gmp_sender::gmp_send instead.
module mvmt_intent::intent_gmp {
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
    const E_UNAUTHORIZED_RELAY: u64 = 1;
    /// Message nonce already used (replay attack)
    const E_NONCE_ALREADY_USED: u64 = 2;
    /// Invalid payload format
    const E_INVALID_PAYLOAD: u64 = 3;
    /// Source address is not a known remote GMP endpoint for the given chain
    const E_UNKNOWN_REMOTE_GMP_ENDPOINT: u64 = 4;
    /// No remote GMP endpoint configured for the source chain
    const E_NO_REMOTE_GMP_ENDPOINT: u64 = 5;
    /// Caller is not the admin
    const E_UNAUTHORIZED_ADMIN: u64 = 6;
    /// Unknown message type in payload
    const E_UNKNOWN_MESSAGE_TYPE: u64 = 7;

    // ============================================================================
    // MESSAGE TYPE CONSTANTS
    // ============================================================================

    /// EscrowConfirmation: Connected Chain -> Hub (0x02)
    const MESSAGE_TYPE_ESCROW_CONFIRMATION: u8 = 0x02;
    /// FulfillmentProof: Either direction (0x03)
    const MESSAGE_TYPE_FULFILLMENT_PROOF: u8 = 0x03;

    // ============================================================================
    // EVENTS
    // ============================================================================

    #[event]
    /// Emitted when a message is delivered from another chain.
    struct MessageDelivered has drop, store {
        /// Source chain endpoint ID
        src_chain_id: u32,
        /// Source address (32 bytes, the sending program)
        remote_gmp_endpoint_addr: vector<u8>,
        /// Message payload (encoded GMP message)
        payload: vector<u8>,
        /// Intent ID extracted from payload (bytes 1..33)
        intent_id: vector<u8>,
    }

    // ============================================================================
    // STATE
    // ============================================================================

    /// Global endpoint configuration for message delivery.
    struct EndpointConfig has key {
        /// Authorized relay addresses (can call deliver_message)
        authorized_relays: vector<address>,
        /// Admin address (can configure remote GMP endpoints)
        admin: address,
        /// Remote GMP endpoint addresses per source chain (chain_id -> list of 32-byte addresses)
        /// Changed from single address to vector to support multiple sources per chain
        /// (e.g., both outflow-validator and intent-escrow on SVM)
        remote_gmp_endpoint_addrs: Table<u32, vector<vector<u8>>>,
        /// Delivered messages: key is intent_id (32 bytes) ++ msg_type (1 byte) = 33 bytes.
        /// Replaces sequential nonce tracking — immune to module redeployments.
        delivered_messages: Table<vector<u8>, bool>,
    }

    // ============================================================================
    // INITIALIZATION
    // ============================================================================

    /// Initialize the integrated GMP endpoint (receiver).
    /// Called once during deployment.
    /// Note: For sending, also initialize gmp_sender separately.
    public entry fun initialize(admin: &signer) {
        let admin_addr = signer::address_of(admin);

        // Create config with admin as initial authorized relay
        let authorized_relays = vector::empty<address>();
        vector::push_back(&mut authorized_relays, admin_addr);

        move_to(admin, EndpointConfig {
            authorized_relays,
            admin: admin_addr,
            remote_gmp_endpoint_addrs: table::new(),
            delivered_messages: table::new(),
        });
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
    /// Deduplication uses (intent_id, msg_type) extracted from the payload,
    /// making delivery immune to module redeployments (unlike sequential nonces).
    /// Idempotent: delivering the same message twice is a silent no-op.
    ///
    /// # Arguments
    /// - `relay`: The authorized relay account (must be in authorized_relays list)
    /// - `src_chain_id`: Source chain endpoint ID
    /// - `remote_gmp_endpoint_addr`: Source address (32 bytes, the sending program)
    /// - `payload`: Message payload (encoded GMP message)
    ///
    /// # Aborts
    /// - E_UNAUTHORIZED_RELAY: If caller is not an authorized relay
    /// - E_UNKNOWN_REMOTE_GMP_ENDPOINT: If source address is not a known remote GMP endpoint for the chain
    /// - E_INVALID_PAYLOAD: If payload is too short to extract intent_id
    public fun deliver_message(
        relay: &signer,
        src_chain_id: u32,
        remote_gmp_endpoint_addr: vector<u8>,
        payload: vector<u8>,
    ) acquires EndpointConfig {
        let relay_addr = signer::address_of(relay);

        // Verify relay is authorized
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);
        assert!(is_authorized_relay(&config.authorized_relays, relay_addr), E_UNAUTHORIZED_RELAY);

        // Verify remote GMP endpoint: source address must be in the list of known addresses for this chain
        assert!(table::contains(&config.remote_gmp_endpoint_addrs, src_chain_id), E_NO_REMOTE_GMP_ENDPOINT);
        let addrs = table::borrow(&config.remote_gmp_endpoint_addrs, src_chain_id);
        assert!(is_address(addrs, &remote_gmp_endpoint_addr), E_UNKNOWN_REMOTE_GMP_ENDPOINT);

        // Replay protection: deduplicate by (intent_id, msg_type)
        // All GMP messages have: msg_type (1 byte) + intent_id (32 bytes) at the start
        assert!(vector::length(&payload) >= 33, E_INVALID_PAYLOAD);
        let dedup_key = build_dedup_key(&payload);

        if (table::contains(&config.delivered_messages, dedup_key)) {
            // Already delivered — idempotent, return silently
            return
        };
        table::add(&mut config.delivered_messages, dedup_key, true);

        // Extract intent_id for event (bytes 1..33 of payload)
        let intent_id = slice(payload, 1, 32);

        // Emit delivery event (copy payload before routing consumes it)
        event::emit(MessageDelivered {
            src_chain_id,
            remote_gmp_endpoint_addr: copy remote_gmp_endpoint_addr,
            payload: copy payload,
            intent_id,
        });

        // Route message to destination module based on payload type
        route_message(src_chain_id, remote_gmp_endpoint_addr, payload);
    }

    /// Route a GMP message to the appropriate handler based on payload type.
    ///
    /// Hub receives:
    /// - 0x02 (EscrowConfirmation): Connected Chain -> Hub
    /// - 0x03 (FulfillmentProof): Connected Chain -> Hub (for outflow intents)
    ///
    /// No fallbacks - if message type is unexpected, abort.
    fun route_message(
        src_chain_id: u32,
        remote_gmp_endpoint_addr: vector<u8>,
        payload: vector<u8>,
    ) {
        let msg_type = gmp_common::peek_message_type(&payload);

        if (msg_type == MESSAGE_TYPE_ESCROW_CONFIRMATION) {
            // Hub receives escrow confirmations from connected chains
            intent_gmp_hub::receive_escrow_confirmation(src_chain_id, remote_gmp_endpoint_addr, payload);
        } else if (msg_type == MESSAGE_TYPE_FULFILLMENT_PROOF) {
            // Hub receives fulfillment proofs from connected chains (for outflow intents)
            intent_gmp_hub::receive_fulfillment_proof(src_chain_id, remote_gmp_endpoint_addr, payload);
        } else {
            // Hub should NOT receive IntentRequirements (0x01) - it sends them
            abort E_UNKNOWN_MESSAGE_TYPE
        };
    }

    /// Entry function wrapper for deliver_message.
    public entry fun deliver_message_entry(
        relay: &signer,
        src_chain_id: u32,
        remote_gmp_endpoint_addr: vector<u8>,
        payload: vector<u8>,
    ) acquires EndpointConfig {
        deliver_message(relay, src_chain_id, remote_gmp_endpoint_addr, payload);
    }

    // ============================================================================
    // ADMIN FUNCTIONS
    // ============================================================================

    /// Set a remote GMP endpoint address for a source chain.
    /// This replaces all existing remote GMP endpoint addresses for the chain with a single address.
    /// Only the admin can call this function.
    ///
    /// # Arguments
    /// - `admin`: The admin signer
    /// - `src_chain_id`: Source chain endpoint ID (e.g., Solana = 30168)
    /// - `addr`: Remote GMP endpoint address (32 bytes)
    public entry fun set_remote_gmp_endpoint_addr(
        admin: &signer,
        src_chain_id: u32,
        addr: vector<u8>,
    ) acquires EndpointConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);

        // Verify caller is admin
        assert!(config.admin == admin_addr, E_UNAUTHORIZED_ADMIN);

        // Create a new vector with the single address
        let addrs = vector::empty<vector<u8>>();
        vector::push_back(&mut addrs, addr);

        // Store or update remote GMP endpoint addresses
        if (table::contains(&config.remote_gmp_endpoint_addrs, src_chain_id)) {
            *table::borrow_mut(&mut config.remote_gmp_endpoint_addrs, src_chain_id) = addrs;
        } else {
            table::add(&mut config.remote_gmp_endpoint_addrs, src_chain_id, addrs);
        };
    }

    /// Add a remote GMP endpoint address for a source chain without replacing existing ones.
    /// Only the admin can call this function.
    ///
    /// # Arguments
    /// - `admin`: The admin signer
    /// - `src_chain_id`: Source chain endpoint ID (e.g., Solana = 30168)
    /// - `addr`: Remote GMP endpoint address (32 bytes) to add
    public entry fun add_remote_gmp_endpoint_addr(
        admin: &signer,
        src_chain_id: u32,
        addr: vector<u8>,
    ) acquires EndpointConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);

        // Verify caller is admin
        assert!(config.admin == admin_addr, E_UNAUTHORIZED_ADMIN);

        // Add to existing set or create new entry
        if (table::contains(&config.remote_gmp_endpoint_addrs, src_chain_id)) {
            let addrs = table::borrow_mut(&mut config.remote_gmp_endpoint_addrs, src_chain_id);
            // Only add if not already present
            if (!is_address(addrs, &addr)) {
                vector::push_back(addrs, addr);
            };
        } else {
            let addrs = vector::empty<vector<u8>>();
            vector::push_back(&mut addrs, addr);
            table::add(&mut config.remote_gmp_endpoint_addrs, src_chain_id, addrs);
        };
    }

    /// Add a relay address. Only the admin can call this.
    public entry fun add_relay(
        admin: &signer,
        relay_addr: address,
    ) acquires EndpointConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);

        // Only admin can add relays
        assert!(config.admin == admin_addr, E_UNAUTHORIZED_ADMIN);

        // Add if not already present
        if (!is_authorized_relay(&config.authorized_relays, relay_addr)) {
            vector::push_back(&mut config.authorized_relays, relay_addr);
        };
    }

    /// Remove a relay address. Only the admin can call this.
    public entry fun remove_relay(
        admin: &signer,
        relay_addr: address,
    ) acquires EndpointConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);

        // Only admin can remove relays
        assert!(config.admin == admin_addr, E_UNAUTHORIZED_ADMIN);

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
    /// Check if an address is an authorized relay.
    public fun is_relay_authorized(addr: address): bool acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        is_authorized_relay(&config.authorized_relays, addr)
    }

    #[view]
    /// Get the remote GMP endpoint addresses for a source chain.
    /// Returns empty vector if no remote GMP endpoint is configured.
    public fun get_remote_gmp_endpoint_addrs(src_chain_id: u32): vector<vector<u8>> acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        if (table::contains(&config.remote_gmp_endpoint_addrs, src_chain_id)) {
            *table::borrow(&config.remote_gmp_endpoint_addrs, src_chain_id)
        } else {
            vector::empty<vector<u8>>()
        }
    }

    #[view]
    /// Check if a specific message has already been delivered.
    /// Uses (intent_id, msg_type) as the dedup key.
    public fun is_message_delivered(intent_id: vector<u8>, msg_type: u8): bool acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        let key = copy intent_id;
        vector::push_back(&mut key, msg_type);
        table::contains(&config.delivered_messages, key)
    }

    #[view]
    /// Check if a source chain has a remote GMP endpoint configured.
    public fun has_remote_gmp_endpoint(src_chain_id: u32): bool acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        table::contains(&config.remote_gmp_endpoint_addrs, src_chain_id)
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

    /// Check if an address (32 bytes) is in the remote GMP endpoint addresses list.
    fun is_address(addrs: &vector<vector<u8>>, addr: &vector<u8>): bool {
        let len = vector::length(addrs);
        let i = 0;
        while (i < len) {
            if (vector::borrow(addrs, i) == addr) {
                return true
            };
            i = i + 1;
        };
        false
    }

    /// Build dedup key from payload: intent_id (bytes 1..33) ++ msg_type (byte 0).
    /// Result is 33 bytes: [intent_id(32)] [msg_type(1)].
    fun build_dedup_key(payload: &vector<u8>): vector<u8> {
        let key = slice(*payload, 1, 32);
        vector::push_back(&mut key, *vector::borrow(payload, 0));
        key
    }

    /// Extract a sub-vector of `len` bytes starting at `start`.
    fun slice(data: vector<u8>, start: u64, len: u64): vector<u8> {
        let result = vector::empty<u8>();
        let i = 0;
        while (i < len) {
            vector::push_back(&mut result, *vector::borrow(&data, start + i));
            i = i + 1;
        };
        result
    }
}
