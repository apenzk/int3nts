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

    #[error("Unauthorized caller")]
    UnauthorizedCaller,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Unauthorized approver")]
    UnauthorizedApprover,

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
    InvalidPda,

    #[error("Invalid account owner")]
    InvalidAccountOwner,

    #[error("Escrow already exists")]
    EscrowAlreadyExists,

    // GMP-related errors
    #[error("Invalid GMP message")]
    InvalidGmpMessage,

    #[error("Intent requirements not found")]
    RequirementsNotFound,

    #[error("Intent requirements already exist")]
    RequirementsAlreadyExist,

    #[error("Amount mismatch with requirements")]
    AmountMismatch,

    #[error("Token mismatch with requirements")]
    TokenMismatch,

    #[error("Escrow already created for this intent")]
    EscrowAlreadyCreated,

    #[error("Already fulfilled")]
    AlreadyFulfilled,

    #[error("Unauthorized GMP source")]
    UnauthorizedGmpSource,

    #[error("Intent has expired")]
    IntentExpired,
}

impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
