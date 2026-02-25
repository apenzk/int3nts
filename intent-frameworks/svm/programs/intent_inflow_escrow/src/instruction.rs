//! Instruction definitions

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum EscrowInstruction {
    /// Initialize the escrow program with approver pubkey
    ///
    /// Accounts expected:
    /// 0. `[writable]` State account (PDA)
    /// 1. `[signer]` Payer
    /// 2. `[]` System program
    Initialize { approver: Pubkey },

    /// Generic GmpReceive for GMP message delivery (variant index 1).
    /// Routes to GmpReceiveRequirements or GmpReceiveFulfillmentProof based on message type.
    ///
    /// This must be at index 1 to match the GMP endpoint's CPI format which uses
    /// variant index 1 for all destination programs.
    ///
    /// Accounts expected (for IntentRequirements - message type 0x01):
    /// 0. `[writable]` Requirements account (PDA)
    /// 1. `[]` GMP config account (PDA)
    /// 2. `[signer]` GMP endpoint or relay (trusted caller)
    /// 3. `[signer]` Payer
    /// 4. `[]` System program
    ///
    /// Accounts expected (for FulfillmentProof - message type 0x03):
    /// 0. `[writable]` Requirements account (PDA)
    /// 1. `[writable]` Escrow account (PDA)
    /// 2. `[writable]` Escrow vault (PDA)
    /// 3. `[writable]` Solver token account
    /// 4. `[]` GMP config account (PDA)
    /// 5. `[signer]` GMP endpoint or relay (trusted caller)
    /// 6. `[]` Token program
    GmpReceive {
        /// Source chain ID
        src_chain_id: u32,
        /// Source address (hub GMP endpoint address)
        remote_gmp_endpoint_addr: [u8; 32],
        /// GMP payload (message type in first byte determines routing)
        payload: Vec<u8>,
    },

    /// Set or update GMP configuration for cross-chain messaging
    ///
    /// Accounts expected:
    /// 0. `[writable]` GMP config account (PDA)
    /// 1. `[signer]` Admin (must match state approver or be initial setup)
    /// 2. `[]` System program
    SetGmpConfig {
        /// The hub chain ID (LZ endpoint ID)
        hub_chain_id: u32,
        /// The hub GMP endpoint address (32 bytes)
        hub_gmp_endpoint_addr: [u8; 32],
        /// The integrated GMP endpoint program ID
        gmp_endpoint: Pubkey,
    },

    /// Create a new escrow and deposit funds atomically.
    /// Validates escrow against stored GMP requirements and sends EscrowConfirmation to hub.
    /// Expiry is always taken from the hub-provided requirements (no local default).
    ///
    /// Accounts expected:
    /// 0. `[writable]` Escrow account (PDA)
    /// 1. `[writable, signer]` Requester
    /// 2. `[]` Token mint
    /// 3. `[writable]` Requester token account
    /// 4. `[writable]` Escrow vault (PDA)
    /// 5. `[]` Reserved solver
    /// 6. `[]` Token program
    /// 7. `[]` System program
    /// 8. `[]` Rent sysvar
    /// 9. `[writable]` Requirements account (PDA) - validates against GMP requirements
    /// 10. `[optional]` GMP config account (PDA) - required if sending EscrowConfirmation
    /// 11. `[optional]` GMP endpoint program - required if sending EscrowConfirmation
    /// 12+ `[optional]` Additional accounts for GMP endpoint CPI
    CreateEscrow {
        intent_id: [u8; 32],
        amount: u64,
    },

    /// Claim escrow funds (GMP mode - no signature required)
    ///
    /// In GMP mode, the fulfillment proof from the hub authorizes the release.
    /// This instruction is called after GmpReceiveFulfillmentProof marks the
    /// requirements as fulfilled.
    ///
    /// Accounts expected:
    /// 0. `[writable]` Escrow account (PDA)
    /// 1. `[]` Requirements account (PDA)
    /// 2. `[writable]` Escrow vault (PDA)
    /// 3. `[writable]` Solver token account
    /// 4. `[]` Token program
    Claim { intent_id: [u8; 32] },

    /// Cancel escrow and return funds to requester (only after expiry)
    ///
    /// Only the admin can cancel. Funds always return to the original requester.
    ///
    /// Accounts expected:
    /// 0. `[writable]` Escrow account (PDA)
    /// 1. `[writable, signer]` Admin
    /// 2. `[writable]` Escrow vault (PDA)
    /// 3. `[writable]` Requester token account
    /// 4. `[]` Token program
    /// 5. `[]` GMP config account (PDA)
    Cancel { intent_id: [u8; 32] },

    /// Receive intent requirements from hub via GMP
    ///
    /// Implements idempotency: if requirements already exist, silently succeeds.
    ///
    /// Accounts expected:
    /// 0. `[writable]` Requirements account (PDA)
    /// 1. `[]` GMP config account (PDA)
    /// 2. `[signer]` GMP endpoint or relay (trusted caller)
    /// 3. `[signer]` Payer
    /// 4. `[]` System program
    GmpReceiveRequirements {
        /// Source chain ID
        src_chain_id: u32,
        /// Source address (hub GMP endpoint address)
        remote_gmp_endpoint_addr: [u8; 32],
        /// GMP payload (IntentRequirements message)
        payload: Vec<u8>,
    },

    /// Receive fulfillment proof from hub via GMP (auto-releases escrow)
    ///
    /// Accounts expected:
    /// 0. `[writable]` Requirements account (PDA)
    /// 1. `[writable]` Escrow account (PDA)
    /// 2. `[writable]` Escrow vault (PDA)
    /// 3. `[writable]` Solver token account
    /// 4. `[]` GMP config account (PDA)
    /// 5. `[signer]` GMP endpoint or relay (trusted caller)
    /// 6. `[]` Token program
    GmpReceiveFulfillmentProof {
        /// Source chain ID
        src_chain_id: u32,
        /// Source address (hub GMP endpoint address)
        remote_gmp_endpoint_addr: [u8; 32],
        /// GMP payload (FulfillmentProof message)
        payload: Vec<u8>,
    },
}
