//! Error types

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone)]
pub enum EscrowError {
    #[error("Escrow already claimed")]
    EscrowAlreadyClaimed,

    #[error("Escrow does not exist")]
    EscrowDoesNotExist,

    #[error("No deposit")]
    NoDeposit,

    #[error("Unauthorized requester")]
    UnauthorizedRequester,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Unauthorized verifier")]
    UnauthorizedVerifier,

    #[error("Escrow expired")]
    EscrowExpired,

    #[error("Escrow not expired yet")]
    EscrowNotExpiredYet,

    #[error("Invalid amount")]
    InvalidAmount,

    #[error("Invalid solver")]
    InvalidSolver,

    #[error("Invalid instruction data")]
    InvalidInstructionData,

    #[error("Account not initialized")]
    AccountNotInitialized,

    #[error("Invalid PDA")]
    InvalidPDA,

    #[error("Invalid account owner")]
    InvalidAccountOwner,

    #[error("Escrow already exists")]
    EscrowAlreadyExists,
}

impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
