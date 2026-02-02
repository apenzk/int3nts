//! State definitions for the native GMP endpoint program.
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

/// Nonce tracker for inbound messages (per source chain).
/// PDA seeds: ["nonce_in", src_chain_id (as bytes)]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct InboundNonceAccount {
    /// Discriminator for account type
    pub discriminator: u8,
    /// Source chain endpoint ID
    pub src_chain_id: u32,
    /// Last processed nonce (for replay protection)
    pub last_nonce: u64,
    /// Bump seed for PDA derivation
    pub bump: u8,
}

impl InboundNonceAccount {
    pub const DISCRIMINATOR: u8 = 5;
    pub const SIZE: usize = 1 + 4 + 8 + 1; // 14 bytes

    pub fn new(src_chain_id: u32, bump: u8) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            src_chain_id,
            last_nonce: 0,
            bump,
        }
    }

    /// Check if a nonce has already been processed.
    pub fn is_replay(&self, nonce: u64) -> bool {
        nonce <= self.last_nonce
    }

    /// Update the last processed nonce.
    pub fn update_nonce(&mut self, nonce: u64) {
        if nonce > self.last_nonce {
            self.last_nonce = nonce;
        }
    }
}

/// Seeds for PDA derivation
pub mod seeds {
    pub const CONFIG_SEED: &[u8] = b"config";
    pub const RELAY_SEED: &[u8] = b"relay";
    pub const TRUSTED_REMOTE_SEED: &[u8] = b"trusted_remote";
    pub const NONCE_OUT_SEED: &[u8] = b"nonce_out";
    pub const NONCE_IN_SEED: &[u8] = b"nonce_in";
}
