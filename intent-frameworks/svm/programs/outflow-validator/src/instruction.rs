//! Instruction definitions for the outflow validator program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum OutflowInstruction {
    /// Initialize the program configuration.
    ///
    /// Accounts expected:
    /// 0. `[writable]` Config account (PDA: ["config"])
    /// 1. `[signer]` Admin/payer
    /// 2. `[]` System program
    Initialize {
        gmp_endpoint: Pubkey,
        hub_chain_id: u32,
        trusted_hub_addr: [u8; 32],
    },

    /// Receive intent requirements via GMP (lz_receive).
    /// Called by the GMP endpoint to deliver a message from the hub.
    ///
    /// Idempotency: If requirements already exist for this intent_id, the
    /// instruction succeeds but does not overwrite existing data.
    ///
    /// Accounts expected:
    /// 0. `[writable]` Requirements account (PDA: ["requirements", intent_id])
    /// 1. `[]` Config account (PDA: ["config"])
    /// 2. `[signer]` GMP endpoint or delivery authority
    /// 3. `[signer]` Payer for account creation
    /// 4. `[]` System program
    LzReceive {
        /// Source chain ID (LZ endpoint ID)
        src_chain_id: u32,
        /// Source address (hub contract)
        src_addr: [u8; 32],
        /// GMP message payload (IntentRequirements encoded)
        payload: Vec<u8>,
    },

    /// Fulfill an intent by transferring tokens to the recipient.
    /// Only the authorized solver (or any solver if solver_addr is zero) can call this.
    ///
    /// The instruction:
    /// 1. Validates the caller is the authorized solver
    /// 2. Pulls tokens from solver's token account to this program
    /// 3. Forwards tokens to the recipient
    /// 4. Marks the intent as fulfilled
    /// 5. Sends a FulfillmentProof GMP message back to the hub
    ///
    /// Accounts expected:
    /// 0. `[writable]` Requirements account (PDA: ["requirements", intent_id])
    /// 1. `[]` Config account (PDA: ["config"])
    /// 2. `[signer]` Solver
    /// 3. `[writable]` Solver token account
    /// 4. `[writable]` Recipient token account
    /// 5. `[]` Token mint
    /// 6. `[]` Token program
    /// 7. `[]` GMP endpoint program (for sending message)
    /// 8+ Additional accounts required by GMP endpoint
    FulfillIntent { intent_id: [u8; 32] },
}
