//! Account state definitions

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Global escrow state containing the authorized approver
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct EscrowState {
    /// Discriminator for account type
    pub discriminator: [u8; 8],
    /// Authorized approver public key that can approve releases
    pub approver: Pubkey,
}

impl EscrowState {
    pub const DISCRIMINATOR: [u8; 8] = [0x45, 0x53, 0x43, 0x52, 0x4f, 0x57, 0x53, 0x54]; // "ESCROWST"
    pub const LEN: usize = 8 + 32; // discriminator + approver pubkey

    pub fn new(approver: Pubkey) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            approver,
        }
    }
}

/// Escrow data structure (matches EVM Escrow struct)
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct Escrow {
    /// Discriminator for account type
    pub discriminator: [u8; 8],
    /// Requester who deposited funds
    pub requester: Pubkey,
    /// SPL token mint address
    pub token_mint: Pubkey,
    /// Amount deposited
    pub amount: u64,
    /// Whether funds have been claimed
    pub is_claimed: bool,
    /// Expiry timestamp (contract-defined)
    pub expiry: i64,
    /// Solver address that receives funds when escrow is claimed
    pub reserved_solver: Pubkey,
    /// Unique intent identifier (32 bytes)
    pub intent_id: [u8; 32],
    /// PDA bump seed
    pub bump: u8,
}

impl Escrow {
    pub const DISCRIMINATOR: [u8; 8] = [0x45, 0x53, 0x43, 0x52, 0x4f, 0x57, 0x44, 0x41]; // "ESCROWDA"
    pub const LEN: usize = 8 + 32 + 32 + 8 + 1 + 8 + 32 + 32 + 1; // 154 bytes

    pub fn new(
        requester: Pubkey,
        token_mint: Pubkey,
        amount: u64,
        expiry: i64,
        reserved_solver: Pubkey,
        intent_id: [u8; 32],
        bump: u8,
    ) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            requester,
            token_mint,
            amount,
            is_claimed: false,
            expiry,
            reserved_solver,
            intent_id,
            bump,
        }
    }
}

/// Seeds for PDA derivation
pub mod seeds {
    pub const STATE_SEED: &[u8] = b"state";
    pub const ESCROW_SEED: &[u8] = b"escrow";
    pub const VAULT_SEED: &[u8] = b"vault";
}
