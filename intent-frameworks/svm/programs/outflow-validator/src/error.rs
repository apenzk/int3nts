//! Error types for the outflow validator program.

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutflowError {
    #[error("Invalid GMP message")]
    InvalidGmpMessage,

    #[error("Intent requirements not found")]
    RequirementsNotFound,

    #[error("Intent requirements already exist")]
    RequirementsAlreadyExist,

    #[error("Unauthorized solver")]
    UnauthorizedSolver,

    #[error("Amount mismatch")]
    AmountMismatch,

    #[error("Token mismatch")]
    TokenMismatch,

    #[error("Recipient mismatch")]
    RecipientMismatch,

    #[error("Intent already fulfilled")]
    AlreadyFulfilled,

    #[error("Intent expired")]
    IntentExpired,

    #[error("Invalid account owner")]
    InvalidAccountOwner,

    #[error("Invalid PDA")]
    InvalidPda,
}

impl From<OutflowError> for ProgramError {
    fn from(e: OutflowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
