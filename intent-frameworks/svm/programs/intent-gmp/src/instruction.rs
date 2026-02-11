//! Instruction definitions for the integrated GMP endpoint program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum NativeGmpInstruction {
    /// Initialize the GMP endpoint configuration.
    ///
    /// Accounts expected:
    /// 0. `[writable]` Config account (PDA: ["config"])
    /// 1. `[signer]` Admin (becomes the config admin)
    /// 2. `[signer]` Payer
    /// 3. `[]` System program
    Initialize {
        /// This chain's endpoint ID
        chain_id: u32,
    },

    /// Add an authorized relay.
    ///
    /// Accounts expected:
    /// 0. `[]` Config account (PDA: ["config"])
    /// 1. `[writable]` Relay account (PDA: ["relay", relay_pubkey])
    /// 2. `[signer]` Admin
    /// 3. `[signer]` Payer
    /// 4. `[]` System program
    AddRelay {
        /// The relay public key to authorize
        relay: Pubkey,
    },

    /// Remove an authorized relay.
    ///
    /// Accounts expected:
    /// 0. `[]` Config account (PDA: ["config"])
    /// 1. `[writable]` Relay account (PDA: ["relay", relay_pubkey])
    /// 2. `[signer]` Admin
    RemoveRelay {
        /// The relay public key to deauthorize
        relay: Pubkey,
    },

    /// Set a trusted remote address for a source chain.
    ///
    /// Accounts expected:
    /// 0. `[]` Config account (PDA: ["config"])
    /// 1. `[writable]` Trusted remote account (PDA: ["trusted_remote", src_chain_id])
    /// 2. `[signer]` Admin
    /// 3. `[signer]` Payer
    /// 4. `[]` System program
    SetTrustedRemote {
        /// Source chain endpoint ID
        src_chain_id: u32,
        /// Trusted source address (32 bytes)
        trusted_addr: [u8; 32],
    },

    /// Set routing configuration for message delivery.
    ///
    /// Configures which programs handle different message types.
    /// Similar to MVM's route_message, this enables routing IntentRequirements
    /// to both outflow-validator AND intent-escrow for inflow support.
    ///
    /// Accounts expected:
    /// 0. `[]` Config account (PDA: ["config"])
    /// 1. `[writable]` Routing config account (PDA: ["routing"])
    /// 2. `[signer]` Admin
    /// 3. `[signer]` Payer
    /// 4. `[]` System program
    SetRouting {
        /// Outflow validator program ID (zero = not configured)
        outflow_validator: Pubkey,
        /// Intent escrow program ID (zero = not configured)
        intent_escrow: Pubkey,
    },

    /// Send a cross-chain message.
    ///
    /// Emits a `MessageSent` event that the GMP relay monitors.
    /// The relay picks up the event and calls `DeliverMessage` on the
    /// destination chain.
    ///
    /// Accounts expected:
    /// 0. `[]` Config account (PDA: ["config"])
    /// 1. `[writable]` Outbound nonce account (PDA: ["nonce_out", dst_chain_id])
    /// 2. `[signer]` Sender (the program/user sending the message, for authorization)
    /// 3. `[signer]` Payer (pays for account creation)
    /// 4. `[]` System program
    /// 5. `[writable]` Message account (PDA: ["message", dst_chain_id, nonce])
    ///    Caller derives this from the current nonce value in the nonce_out account.
    Send {
        /// Destination chain endpoint ID (e.g., Movement = 30325)
        dst_chain_id: u32,
        /// Destination address (32 bytes, the receiving program/module)
        dst_addr: [u8; 32],
        /// Source address to include in the message (32 bytes).
        /// This is the application-level sender (e.g., outflow-validator program ID).
        /// The sender account is still required for authorization.
        src_addr: [u8; 32],
        /// Message payload (encoded GMP message)
        payload: Vec<u8>,
    },

    /// Deliver a cross-chain message to a destination program.
    ///
    /// Called by the GMP relay after observing a `MessageSent` event
    /// on the source chain. The relay decodes the event, constructs this
    /// instruction, and submits it to the destination chain.
    ///
    /// Deduplication uses (intent_id, msg_type) extracted from the payload,
    /// making delivery immune to program redeployments (unlike sequential nonces).
    ///
    /// Message routing (similar to MVM's route_message):
    /// - IntentRequirements (0x01): Routes to BOTH outflow_validator AND intent_escrow (if routing configured)
    /// - Other message types: Single destination (destination_program_1 account)
    ///
    /// Accounts expected:
    /// 0. `[]` Config account (PDA: ["config"])
    /// 1. `[]` Relay account (PDA: ["relay", relay_pubkey])
    /// 2. `[]` Trusted remote account (PDA: ["trusted_remote", src_chain_id])
    /// 3. `[writable]` Delivered message account (PDA: ["delivered", intent_id, &[msg_type]])
    /// 4. `[signer]` Relay (must be authorized)
    /// 5. `[signer]` Payer (for delivered message account creation)
    /// 6. `[]` System program
    /// 7. `[]` Routing config account (PDA: ["routing"]) - pass any account if routing not configured
    /// 8. `[]` Destination program 1 (outflow_validator for routing, or single destination)
    /// 9. `[]` Destination program 2 (intent_escrow for routing, or any account if not routing)
    /// 10+. Additional accounts required by destination program(s)
    DeliverMessage {
        /// Source chain endpoint ID
        src_chain_id: u32,
        /// Source address (32 bytes, the sending program/module)
        src_addr: [u8; 32],
        /// Message payload (encoded GMP message)
        payload: Vec<u8>,
    },
}
