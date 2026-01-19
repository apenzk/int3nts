//! Instruction definitions

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum EscrowInstruction {
    /// Initialize the escrow program with verifier pubkey
    ///
    /// Accounts expected:
    /// 0. `[writable]` State account (PDA)
    /// 1. `[signer]` Payer
    /// 2. `[]` System program
    Initialize { verifier: Pubkey },

    /// Create a new escrow and deposit funds atomically
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
    CreateEscrow {
        intent_id: [u8; 32],
        amount: u64,
        expiry_duration: Option<i64>,
    },

    /// Claim escrow funds with verifier signature
    ///
    /// Accounts expected:
    /// 0. `[writable]` Escrow account (PDA)
    /// 1. `[]` State account (PDA)
    /// 2. `[writable]` Escrow vault (PDA)
    /// 3. `[writable]` Solver token account
    /// 4. `[]` Instructions sysvar
    /// 5. `[]` Token program
    Claim {
        intent_id: [u8; 32],
        signature: [u8; 64],
    },

    /// Cancel escrow and return funds to requester (only after expiry)
    ///
    /// Accounts expected:
    /// 0. `[writable]` Escrow account (PDA)
    /// 1. `[writable, signer]` Requester
    /// 2. `[writable]` Escrow vault (PDA)
    /// 3. `[writable]` Requester token account
    /// 4. `[]` Token program
    Cancel { intent_id: [u8; 32] },
}
