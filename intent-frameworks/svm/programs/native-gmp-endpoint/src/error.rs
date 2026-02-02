//! Error definitions for the native GMP endpoint program.

use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum GmpError {
    #[error("Invalid instruction data")]
    InvalidInstructionData,

    #[error("Account not initialized")]
    AccountNotInitialized,

    #[error("Account already initialized")]
    AccountAlreadyInitialized,

    #[error("Invalid account discriminator")]
    InvalidDiscriminator,

    #[error("Invalid PDA")]
    InvalidPda,

    #[error("Unauthorized: caller is not admin")]
    UnauthorizedAdmin,

    #[error("Unauthorized: caller is not an authorized relay")]
    UnauthorizedRelay,

    #[error("Untrusted remote: source chain or address not configured")]
    UntrustedRemote,

    #[error("Replay detected: nonce already processed")]
    ReplayDetected,

    #[error("Invalid nonce: expected sequential nonce")]
    InvalidNonce,

    #[error("Destination program not provided")]
    MissingDestinationProgram,

    #[error("CPI to destination program failed")]
    CpiDeliveryFailed,

    #[error("Invalid account owner")]
    InvalidAccountOwner,

    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
}

impl From<GmpError> for ProgramError {
    fn from(e: GmpError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
