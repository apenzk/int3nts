/// GMP Sender Module
///
/// Provides the `gmp_send` function for sending cross-chain messages.
/// This module is intentionally kept separate from the receiver/routing
/// logic to avoid circular dependencies (sender/receiver split pattern).
///
/// ## Architecture
///
/// - gmp_sender: Send functionality only (this module)
/// - intent_gmp: Receive/routing functionality
/// - Application modules (outflow_validator, etc.): Import gmp_sender for sending
///
/// This separation allows application modules to send GMP messages without
/// creating import cycles with the receiver that routes messages to them.
///
/// ## Usage
///
/// Application modules call `gmp_sender::gmp_send(...)` to send messages.
///
/// ## Outbox
///
/// Messages are stored in an on-chain outbox (`Table<u64, OutboundMessage>`)
/// keyed by nonce. The GMP relay polls `get_next_nonce()` and reads new
/// messages via `get_message(nonce)` view functions. Expired messages can
/// be cleaned up via `cleanup_expired_messages`, which sweeps all expired
/// entries in a single call.
module mvmt_intent::gmp_sender {
    use std::signer;
    use aptos_framework::event;
    use aptos_framework::table::{Self, Table};
    use aptos_framework::timestamp;

    // ============================================================================
    // CONSTANTS
    // ============================================================================

    /// Message TTL in seconds (1 hour). After this, anyone can delete the message.
    const MESSAGE_TTL_SECONDS: u64 = 3600;

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

    // ============================================================================
    // STATE
    // ============================================================================

    /// An outbound message stored in the outbox for relay polling.
    struct OutboundMessage has store, copy, drop {
        /// Destination chain endpoint ID
        dst_chain_id: u32,
        /// Destination address (32 bytes)
        dst_addr: vector<u8>,
        /// Message payload
        payload: vector<u8>,
        /// Sender address
        sender: address,
        /// Timestamp when the message was created (seconds)
        timestamp: u64,
    }

    /// Sender configuration, nonce tracking, and outbox.
    struct SenderConfig has key {
        /// Next outbound nonce
        next_nonce: u64,
        /// Oldest nonce still in the outbox (cleanup cursor)
        oldest_nonce: u64,
        /// Admin address (can be used for future extensions)
        admin: address,
        /// Outbox: nonce -> message. Relay reads via view functions.
        outbox: Table<u64, OutboundMessage>,
    }

    // ============================================================================
    // INITIALIZATION
    // ============================================================================

    /// Initialize the GMP sender.
    /// Called once during deployment.
    public entry fun initialize(admin: &signer) {
        let admin_addr = signer::address_of(admin);

        move_to(admin, SenderConfig {
            next_nonce: 1,
            oldest_nonce: 1,
            admin: admin_addr,
            outbox: table::new(),
        });
    }

    /// Check if the sender is initialized.
    public fun is_initialized(): bool {
        exists<SenderConfig>(@mvmt_intent)
    }

    // ============================================================================
    // SEND
    // ============================================================================

    /// Send a cross-chain message.
    ///
    /// Stores the message in the outbox and emits a `MessageSent` event.
    /// The relay polls `get_next_nonce()` and reads messages via
    /// `get_message(nonce)`.
    ///
    /// # Arguments
    /// - `sender`: The account sending the message
    /// - `dst_chain_id`: Destination chain endpoint ID (e.g., Solana = 30168)
    /// - `dst_addr`: Destination address (32 bytes, the receiving program)
    /// - `payload`: Message payload (encoded GMP message)
    ///
    /// # Returns
    /// - Nonce assigned to this message
    public fun gmp_send(
        sender: &signer,
        dst_chain_id: u32,
        dst_addr: vector<u8>,
        payload: vector<u8>,
    ): u64 acquires SenderConfig {
        let sender_addr = signer::address_of(sender);

        // Get and increment nonce
        let config = borrow_global_mut<SenderConfig>(@mvmt_intent);
        let nonce = config.next_nonce;
        config.next_nonce = nonce + 1;

        // Store in outbox for relay polling
        let now = timestamp::now_seconds();
        table::add(&mut config.outbox, nonce, OutboundMessage {
            dst_chain_id,
            dst_addr: copy dst_addr,
            payload: copy payload,
            sender: sender_addr,
            timestamp: now,
        });

        // Emit event (kept for indexers / off-chain consumers)
        event::emit(MessageSent {
            dst_chain_id,
            dst_addr,
            payload,
            sender: sender_addr,
            nonce,
        });

        nonce
    }

    /// Entry function wrapper for gmp_send.
    public entry fun gmp_send_entry(
        sender: &signer,
        dst_chain_id: u32,
        dst_addr: vector<u8>,
        payload: vector<u8>,
    ) acquires SenderConfig {
        gmp_send(sender, dst_chain_id, dst_addr, payload);
    }

    // ============================================================================
    // CLEANUP
    // ============================================================================

    /// Sweep all expired messages from the outbox.
    /// Advances the `oldest_nonce` cursor forward, removing every message
    /// whose timestamp is past the TTL. Stops at the first non-expired
    /// message or when the cursor reaches `next_nonce`. Anyone can call this.
    public entry fun cleanup_expired_messages() acquires SenderConfig {
        let config = borrow_global_mut<SenderConfig>(@mvmt_intent);
        let now = timestamp::now_seconds();

        while (config.oldest_nonce < config.next_nonce) {
            if (!table::contains(&config.outbox, config.oldest_nonce)) {
                // Already removed (shouldn't happen, but be safe)
                config.oldest_nonce = config.oldest_nonce + 1;
                continue
            };

            let msg = table::borrow(&config.outbox, config.oldest_nonce);
            if (msg.timestamp + MESSAGE_TTL_SECONDS >= now) {
                break // First non-expired message — stop
            };

            table::remove(&mut config.outbox, config.oldest_nonce);
            config.oldest_nonce = config.oldest_nonce + 1;
        };
    }

    // ============================================================================
    // VIEW FUNCTIONS
    // ============================================================================

    #[view]
    /// Get the next outbound nonce.
    public fun get_next_nonce(): u64 acquires SenderConfig {
        borrow_global<SenderConfig>(@mvmt_intent).next_nonce
    }

    #[view]
    /// Get an outbound message by nonce.
    /// Returns (dst_chain_id, dst_addr, payload, sender).
    public fun get_message(nonce: u64): (u32, vector<u8>, vector<u8>, address) acquires SenderConfig {
        let config = borrow_global<SenderConfig>(@mvmt_intent);
        let msg = table::borrow(&config.outbox, nonce);
        (msg.dst_chain_id, msg.dst_addr, msg.payload, msg.sender)
    }

    // ============================================================================
    // TEST HELPERS
    // ============================================================================

    #[test_only]
    /// Initialize for testing.
    public fun init_for_test(admin: &signer) {
        if (!exists<SenderConfig>(@mvmt_intent)) {
            move_to(admin, SenderConfig {
                next_nonce: 1,
                oldest_nonce: 1,
                admin: signer::address_of(admin),
                outbox: table::new(),
            });
        };
    }
}
