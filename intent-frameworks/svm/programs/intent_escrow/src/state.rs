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

/// Stored intent requirements received via GMP from the hub
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct StoredIntentRequirements {
    /// Discriminator for account type
    pub discriminator: [u8; 8],
    /// Unique intent identifier
    pub intent_id: [u8; 32],
    /// Requester address (32-byte canonical form)
    pub requester_addr: [u8; 32],
    /// Required escrow amount
    pub amount_required: u64,
    /// Token address (32-byte canonical form)
    pub token_addr: [u8; 32],
    /// Authorized solver address (32-byte canonical form, zeros = any)
    pub solver_addr: [u8; 32],
    /// Expiry timestamp
    pub expiry: u64,
    /// Whether an escrow has been created for these requirements
    pub escrow_created: bool,
    /// Whether fulfillment proof has been received
    pub fulfilled: bool,
    /// PDA bump seed
    pub bump: u8,
}

impl StoredIntentRequirements {
    pub const DISCRIMINATOR: [u8; 8] = [0x49, 0x4e, 0x54, 0x52, 0x45, 0x51, 0x53, 0x54]; // "INTREQST"
    pub const LEN: usize = 8 + 32 + 32 + 8 + 32 + 32 + 8 + 1 + 1 + 1; // 155 bytes

    pub fn new(
        intent_id: [u8; 32],
        requester_addr: [u8; 32],
        amount_required: u64,
        token_addr: [u8; 32],
        solver_addr: [u8; 32],
        expiry: u64,
        bump: u8,
    ) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            intent_id,
            requester_addr,
            amount_required,
            token_addr,
            solver_addr,
            expiry,
            escrow_created: false,
            fulfilled: false,
            bump,
        }
    }
}

/// GMP configuration for cross-chain messaging.
/// Stores trusted hub address and GMP endpoint for source validation.
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct GmpConfig {
    /// Discriminator for account type
    pub discriminator: [u8; 8],
    /// Admin who can update config
    pub admin: Pubkey,
    /// The hub chain ID (LZ endpoint ID, e.g., Movement = 30106)
    pub hub_chain_id: u32,
    /// The trusted hub address (32 bytes, for GMP message verification)
    pub trusted_hub_addr: [u8; 32],
    /// The native GMP endpoint program ID (for CPI)
    pub gmp_endpoint: Pubkey,
    /// PDA bump seed
    pub bump: u8,
}

impl GmpConfig {
    pub const DISCRIMINATOR: [u8; 8] = [0x47, 0x4d, 0x50, 0x43, 0x4f, 0x4e, 0x46, 0x47]; // "GMPCONFG"
    pub const LEN: usize = 8 + 32 + 4 + 32 + 32 + 1; // 109 bytes

    pub fn new(
        admin: Pubkey,
        hub_chain_id: u32,
        trusted_hub_addr: [u8; 32],
        gmp_endpoint: Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            discriminator: Self::DISCRIMINATOR,
            admin,
            hub_chain_id,
            trusted_hub_addr,
            gmp_endpoint,
            bump,
        }
    }
}

/// Seeds for PDA derivation
pub mod seeds {
    pub const STATE_SEED: &[u8] = b"state";
    pub const ESCROW_SEED: &[u8] = b"escrow";
    pub const VAULT_SEED: &[u8] = b"vault";
    pub const REQUIREMENTS_SEED: &[u8] = b"requirements";
    pub const GMP_CONFIG_SEED: &[u8] = b"gmp_config";
}
