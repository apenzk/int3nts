//! State definitions for the outflow validator program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Stored intent requirements received via GMP from the hub.
/// PDA seeds: ["requirements", intent_id]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct IntentRequirementsAccount {
    /// Discriminator for account type
    pub discriminator: u8,
    /// The intent ID this requirement is for
    pub intent_id: [u8; 32],
    /// The recipient address on this chain
    pub recipient_addr: Pubkey,
    /// The amount required to be transferred
    pub amount_required: u64,
    /// The token mint address
    pub token_mint: Pubkey,
    /// The authorized solver address (zero = any solver)
    pub authorized_solver: Pubkey,
    /// Expiry timestamp (Unix seconds)
    pub expiry: u64,
    /// Whether this intent has been fulfilled
    pub fulfilled: bool,
}

impl IntentRequirementsAccount {
    pub const DISCRIMINATOR: u8 = 1;
    pub const SIZE: usize = 1 + 32 + 32 + 8 + 32 + 32 + 8 + 1; // 146 bytes

    pub fn new(
        intent_id: [u8; 32],
        recipient_addr: Pubkey,
        amount_required: u64,
        token_mint: Pubkey,
        authorized_solver: Pubkey,
        expiry: u64,
    ) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            intent_id,
            recipient_addr,
            amount_required,
            token_mint,
            authorized_solver,
            expiry,
            fulfilled: false,
        }
    }
}

/// Program configuration account.
/// PDA seeds: ["config"]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct ConfigAccount {
    /// Discriminator for account type
    pub discriminator: u8,
    /// The admin authority
    pub admin: Pubkey,
    /// The trusted GMP endpoint program ID
    pub gmp_endpoint: Pubkey,
    /// The hub chain ID (LayerZero endpoint ID)
    pub hub_chain_id: u32,
    /// The trusted hub address (for GMP message verification)
    pub trusted_hub_addr: [u8; 32],
}

impl ConfigAccount {
    pub const DISCRIMINATOR: u8 = 2;
    pub const SIZE: usize = 1 + 32 + 32 + 4 + 32; // 101 bytes

    pub fn new(admin: Pubkey, gmp_endpoint: Pubkey, hub_chain_id: u32, trusted_hub_addr: [u8; 32]) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            admin,
            gmp_endpoint,
            hub_chain_id,
            trusted_hub_addr,
        }
    }
}
