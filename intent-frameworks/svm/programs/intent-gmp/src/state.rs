//! State definitions for the integrated GMP endpoint program.
//!
//! ## Discriminator Pattern
//!
//! Each account type has a unique `discriminator` byte as its first field.
//! This prevents deserialization confusion when reading raw account data:
//! - ConfigAccount = 1, RelayAccount = 2, TrustedRemote = 3, etc.
//! - On read, verify discriminator matches expected type before trusting data.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Program configuration account.
/// PDA seeds: ["config"]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ConfigAccount {
    /// First byte identifies account type (1 = Config). Prevents type confusion on deserialize.
    pub discriminator: u8,
    /// The admin authority (can add/remove relays, set trusted remotes)
    pub admin: Pubkey,
    /// This chain's endpoint ID (e.g., Solana devnet = 30168)
    pub chain_id: u32,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl ConfigAccount {
    pub const DISCRIMINATOR: u8 = 1;
    pub const SIZE: usize = 1 + 32 + 4 + 1; // 38 bytes

    pub fn new(admin: Pubkey, chain_id: u32, bump: u8) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            admin,
            chain_id,
            bump,
        }
    }
}

/// Authorized relay account.
/// PDA seeds: ["relay", relay_pubkey]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct RelayAccount {
    /// Discriminator for account type
    pub discriminator: u8,
    /// The relay's public key
    pub relay: Pubkey,
    /// Whether this relay is currently authorized
    pub is_authorized: bool,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl RelayAccount {
    pub const DISCRIMINATOR: u8 = 2;
    pub const SIZE: usize = 1 + 32 + 1 + 1; // 35 bytes

    pub fn new(relay: Pubkey, bump: u8) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            relay,
            is_authorized: true,
            bump,
        }
    }
}

/// Trusted remote configuration for a source chain.
/// PDA seeds: ["trusted_remote", src_chain_id (as bytes)]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct TrustedRemoteAccount {
    /// Discriminator for account type
    pub discriminator: u8,
    /// Source chain endpoint ID
    pub src_chain_id: u32,
    /// Trusted source address (32 bytes, zero-padded if needed)
    pub trusted_addr: [u8; 32],
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl TrustedRemoteAccount {
    pub const DISCRIMINATOR: u8 = 3;
    pub const SIZE: usize = 1 + 4 + 32 + 1; // 38 bytes

    pub fn new(src_chain_id: u32, trusted_addr: [u8; 32], bump: u8) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            src_chain_id,
            trusted_addr,
            bump,
        }
    }
}

/// Nonce tracker for outbound messages (per destination chain).
/// PDA seeds: ["nonce_out", dst_chain_id (as bytes)]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct OutboundNonceAccount {
    /// Discriminator for account type
    pub discriminator: u8,
    /// Destination chain endpoint ID
    pub dst_chain_id: u32,
    /// Current nonce (incremented for each message sent)
    pub nonce: u64,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl OutboundNonceAccount {
    pub const DISCRIMINATOR: u8 = 4;
    pub const SIZE: usize = 1 + 4 + 8 + 1; // 14 bytes

    pub fn new(dst_chain_id: u32, bump: u8) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            dst_chain_id,
            nonce: 0,
            bump,
        }
    }

    pub fn increment(&mut self) -> u64 {
        let current = self.nonce;
        self.nonce = self.nonce.saturating_add(1);
        current
    }
}

/// Delivered message marker for replay protection.
/// PDA seeds: ["delivered", intent_id (32 bytes), &[msg_type]]
///
/// Replaces nonce-based replay protection — immune to program redeployments.
/// Each unique (intent_id, msg_type) pair gets its own PDA. If the account
/// exists, the message has already been delivered.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct DeliveredMessage {
    /// Discriminator for account type
    pub discriminator: u8,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl DeliveredMessage {
    pub const DISCRIMINATOR: u8 = 5;
    pub const SIZE: usize = 1 + 1; // 2 bytes

    pub fn new(bump: u8) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            bump,
        }
    }
}

/// Routing configuration for message delivery.
/// Stores program IDs that handle different message types.
/// PDA seeds: ["routing"]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct RoutingConfig {
    /// Discriminator for account type
    pub discriminator: u8,
    /// Outflow validator program (handles IntentRequirements for outflow)
    /// Zero pubkey means not configured
    pub outflow_validator: Pubkey,
    /// Intent escrow program (handles IntentRequirements for inflow)
    /// Zero pubkey means not configured
    pub intent_escrow: Pubkey,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl RoutingConfig {
    pub const DISCRIMINATOR: u8 = 6;
    pub const SIZE: usize = 1 + 32 + 32 + 1; // 66 bytes

    pub fn new(outflow_validator: Pubkey, intent_escrow: Pubkey, bump: u8) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            outflow_validator,
            intent_escrow,
            bump,
        }
    }

    /// Check if outflow_validator is configured (non-zero)
    pub fn has_outflow_validator(&self) -> bool {
        self.outflow_validator != Pubkey::default()
    }

    /// Check if intent_escrow is configured (non-zero)
    pub fn has_intent_escrow(&self) -> bool {
        self.intent_escrow != Pubkey::default()
    }
}

/// Stored outbound message for relay to read via getAccountInfo.
/// PDA seeds: ["message", dst_chain_id (as bytes), nonce (as bytes)]
///
/// TODO: These accounts accumulate forever (~0.001 SOL rent each). Add a
/// `CloseMessage` instruction that lets the relay (or admin) close the account
/// after successful delivery and reclaim the rent lamports back to the payer.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct MessageAccount {
    /// Discriminator for account type
    pub discriminator: u8,
    /// Source chain endpoint ID (this chain)
    pub src_chain_id: u32,
    /// Destination chain endpoint ID
    pub dst_chain_id: u32,
    /// Sequence number assigned by the outbound nonce counter
    pub nonce: u64,
    /// Destination address (32 bytes, zero-padded)
    pub dst_addr: [u8; 32],
    /// Source address (32 bytes — application-level sender, e.g. outflow-validator program ID)
    pub src_addr: [u8; 32],
    /// GMP message payload (variable length)
    pub payload: Vec<u8>,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl MessageAccount {
    pub const DISCRIMINATOR: u8 = 7;
    /// Fixed-size portion (excluding payload data):
    /// discriminator(1) + src_chain_id(4) + dst_chain_id(4) + nonce(8)
    /// + dst_addr(32) + src_addr(32) + payload_len_prefix(4) + bump(1) = 86
    pub const FIXED_SIZE: usize = 1 + 4 + 4 + 8 + 32 + 32 + 4 + 1;

    pub fn size(payload_len: usize) -> usize {
        Self::FIXED_SIZE + payload_len
    }

    pub fn new(
        src_chain_id: u32,
        dst_chain_id: u32,
        nonce: u64,
        dst_addr: [u8; 32],
        src_addr: [u8; 32],
        payload: Vec<u8>,
        bump: u8,
    ) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            src_chain_id,
            dst_chain_id,
            nonce,
            dst_addr,
            src_addr,
            payload,
            bump,
        }
    }
}

/// Seeds for PDA derivation
pub mod seeds {
    pub const CONFIG_SEED: &[u8] = b"config";
    pub const RELAY_SEED: &[u8] = b"relay";
    pub const TRUSTED_REMOTE_SEED: &[u8] = b"trusted_remote";
    pub const NONCE_OUT_SEED: &[u8] = b"nonce_out";
    pub const DELIVERED_SEED: &[u8] = b"delivered";
    pub const ROUTING_SEED: &[u8] = b"routing";
    pub const MESSAGE_SEED: &[u8] = b"message";
}
