/// IntentGmp - Connected Chain Version
///
/// GMP endpoint for cross-chain message delivery and routing on connected chains.
/// Routes messages to intent_outflow_validator_impl and intent_inflow_escrow.
///
/// ## Architecture
///
/// - gmp_sender: Send functionality (lz_send)
/// - intent_gmp: Receive/routing functionality (this module)
///
/// ## Functions
///
/// - `deliver_message`: Called by relay to deliver messages to destination
/// - `set_trusted_remote`: Configure trusted source addresses per chain
///
/// For sending messages, use gmp_sender::lz_send instead.
module mvmt_intent::intent_gmp {
    use std::vector;
    use std::signer;
    use aptos_framework::event;
    use aptos_std::table::{Self, Table};
    use mvmt_intent::gmp_common;
    use mvmt_intent::intent_outflow_validator_impl;
    use mvmt_intent::intent_inflow_escrow;

    // ============================================================================
    // ERROR CODES
    // ============================================================================

    /// Caller is not an authorized relay
    const E_UNAUTHORIZED_RELAY: u64 = 1;
    /// Invalid payload format
    const E_INVALID_PAYLOAD: u64 = 3;
    /// Source address is not trusted for the given chain
    const E_UNTRUSTED_REMOTE: u64 = 4;
    /// No trusted remote configured for the source chain
    const E_NO_TRUSTED_REMOTE: u64 = 5;
    /// Caller is not the admin
    const E_UNAUTHORIZED_ADMIN: u64 = 6;
    /// Unknown message type in payload
    const E_UNKNOWN_MESSAGE_TYPE: u64 = 7;

    // ============================================================================
    // MESSAGE TYPE CONSTANTS
    // ============================================================================

    /// IntentRequirements: Hub -> Connected Chain (0x01)
    const MESSAGE_TYPE_INTENT_REQUIREMENTS: u8 = 0x01;
    /// FulfillmentProof: Hub -> Connected Chain (0x03)
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
        src_addr: vector<u8>,
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
        /// Admin address (can configure trusted remotes)
        admin: address,
        /// Trusted remote addresses per source chain (chain_id -> list of trusted 32-byte addresses)
        /// Changed from single address to vector to support multiple trusted sources per chain
        /// (e.g., both outflow-validator and intent-escrow on SVM)
        trusted_remotes: Table<u32, vector<vector<u8>>>,
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
            trusted_remotes: table::new(),
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
    /// - `src_addr`: Source address (32 bytes, the sending program)
    /// - `payload`: Message payload (encoded GMP message)
    ///
    /// # Aborts
    /// - E_UNAUTHORIZED_RELAY: If caller is not an authorized relay
    /// - E_UNTRUSTED_REMOTE: If source address is not trusted for the chain
    /// - E_INVALID_PAYLOAD: If payload is too short to extract intent_id
    public fun deliver_message(
        relay: &signer,
        src_chain_id: u32,
        src_addr: vector<u8>,
        payload: vector<u8>,
    ) acquires EndpointConfig {
        let relay_addr = signer::address_of(relay);

        // Verify relay is authorized
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);
        assert!(is_authorized_relay(&config.authorized_relays, relay_addr), E_UNAUTHORIZED_RELAY);

        // Verify trusted remote: source address must be in the list of trusted addresses for this chain
        assert!(table::contains(&config.trusted_remotes, src_chain_id), E_NO_TRUSTED_REMOTE);
        let trusted_addrs = table::borrow(&config.trusted_remotes, src_chain_id);
        assert!(is_trusted_address(trusted_addrs, &src_addr), E_UNTRUSTED_REMOTE);

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
            src_addr: copy src_addr,
            payload: copy payload,
            intent_id,
        });

        // Route message to destination module based on payload type
        route_message(src_chain_id, src_addr, payload);
    }

    /// Route a GMP message to the appropriate handler based on payload type.
    ///
    /// Connected chain receives:
    /// - 0x01 (IntentRequirements): Hub -> Connected Chain
    ///   Routes to intent_outflow_validator_impl and inflow_escrow
    /// - 0x03 (FulfillmentProof): Hub -> Connected Chain (for inflow escrow release)
    ///   Routes to inflow_escrow
    ///
    /// No fallbacks - if message type is unexpected, abort.
    fun route_message(
        src_chain_id: u32,
        src_addr: vector<u8>,
        payload: vector<u8>,
    ) {
        let msg_type = gmp_common::peek_message_type(&payload);

        if (msg_type == MESSAGE_TYPE_INTENT_REQUIREMENTS) {
            // Connected chain: both outflow + inflow handlers receive requirements
            intent_outflow_validator_impl::receive_intent_requirements(
                src_chain_id, copy src_addr, copy payload
            );
            intent_inflow_escrow::receive_intent_requirements(
                src_chain_id, src_addr, payload
            );
        } else if (msg_type == MESSAGE_TYPE_FULFILLMENT_PROOF) {
            // Connected chain receives fulfillment proofs from hub (for inflow escrow release)
            intent_inflow_escrow::receive_fulfillment_proof(
                src_chain_id, src_addr, payload
            );
        } else {
            // Connected chain should NOT receive EscrowConfirmation (0x02) - it sends them
            abort E_UNKNOWN_MESSAGE_TYPE
        };
    }

    /// Entry function wrapper for deliver_message.
    public entry fun deliver_message_entry(
        relay: &signer,
        src_chain_id: u32,
        src_addr: vector<u8>,
        payload: vector<u8>,
    ) acquires EndpointConfig {
        deliver_message(relay, src_chain_id, src_addr, payload);
    }

    // ============================================================================
    // ADMIN FUNCTIONS
    // ============================================================================

    /// Set a trusted remote address for a source chain.
    /// This replaces all existing trusted addresses for the chain with a single address.
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
        assert!(config.admin == admin_addr, E_UNAUTHORIZED_ADMIN);

        // Create a new vector with the single address
        let addrs = vector::empty<vector<u8>>();
        vector::push_back(&mut addrs, trusted_addr);

        // Store or update trusted remotes
        if (table::contains(&config.trusted_remotes, src_chain_id)) {
            *table::borrow_mut(&mut config.trusted_remotes, src_chain_id) = addrs;
        } else {
            table::add(&mut config.trusted_remotes, src_chain_id, addrs);
        };
    }

    /// Add a trusted remote address for a source chain without replacing existing ones.
    /// Only the admin can call this function.
    ///
    /// # Arguments
    /// - `admin`: The admin signer
    /// - `src_chain_id`: Source chain endpoint ID (e.g., Solana = 30168)
    /// - `trusted_addr`: Trusted source address (32 bytes) to add
    public entry fun add_trusted_remote(
        admin: &signer,
        src_chain_id: u32,
        trusted_addr: vector<u8>,
    ) acquires EndpointConfig {
        let admin_addr = signer::address_of(admin);
        let config = borrow_global_mut<EndpointConfig>(@mvmt_intent);

        // Verify caller is admin
        assert!(config.admin == admin_addr, E_UNAUTHORIZED_ADMIN);

        // Add to existing set or create new entry
        if (table::contains(&config.trusted_remotes, src_chain_id)) {
            let addrs = table::borrow_mut(&mut config.trusted_remotes, src_chain_id);
            // Only add if not already present
            if (!is_trusted_address(addrs, &trusted_addr)) {
                vector::push_back(addrs, trusted_addr);
            };
        } else {
            let addrs = vector::empty<vector<u8>>();
            vector::push_back(&mut addrs, trusted_addr);
            table::add(&mut config.trusted_remotes, src_chain_id, addrs);
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
    /// Get the trusted remote addresses for a source chain.
    /// Returns empty vector if no trusted remote is configured.
    public fun get_trusted_remote(src_chain_id: u32): vector<vector<u8>> acquires EndpointConfig {
        let config = borrow_global<EndpointConfig>(@mvmt_intent);
        if (table::contains(&config.trusted_remotes, src_chain_id)) {
            *table::borrow(&config.trusted_remotes, src_chain_id)
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

    /// Check if an address (32 bytes) is in the trusted addresses list.
    fun is_trusted_address(addrs: &vector<vector<u8>>, addr: &vector<u8>): bool {
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
