//! SVM types shared across intent services
//!
//! These types are used by the coordinator, integrated-gmp, and solver
//! for querying Solana escrow accounts and parsing program data.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::Deserialize;
use solana_program::pubkey::Pubkey;

// ============================================================================
// ESCROW ACCOUNT STRUCTURES
// ============================================================================

/// On-chain escrow account data (Borsh-serialized by the intent_inflow_escrow program)
#[derive(BorshDeserialize, BorshSerialize, Debug, Clone)]
pub struct EscrowAccount {
    pub discriminator: [u8; 8],
    pub requester: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub is_claimed: bool,
    pub expiry: i64,
    pub reserved_solver: Pubkey,
    pub intent_id: [u8; 32],
    pub bump: u8,
}

/// Escrow account paired with its on-chain address
#[derive(Debug, Clone)]
pub struct EscrowWithPubkey {
    pub pubkey: Pubkey,
    pub escrow: EscrowAccount,
}

/// Simplified escrow event with hex-encoded IDs
#[derive(Debug, Clone)]
pub struct EscrowEvent {
    pub intent_id: String,
    pub escrow_id: String,
}

// ============================================================================
// JSON-RPC TYPES (internal)
// ============================================================================

#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcResponse<T> {
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct JsonRpcError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProgramAccountResult {
    pub pubkey: String,
    pub account: RpcAccount,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RpcAccount {
    pub data: (String, String),
}

#[derive(Debug, Deserialize)]
pub(crate) struct AccountInfoResult {
    pub value: Option<AccountInfoValue>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AccountInfoValue {
    pub data: (String, String),
}

#[derive(Debug, Deserialize)]
pub(crate) struct BalanceResult {
    pub value: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenBalanceResult {
    pub value: TokenAmount,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenAmount {
    pub amount: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SignatureInfo {
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TransactionResult {
    pub meta: Option<TransactionMeta>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct TransactionMeta {
    #[serde(rename = "logMessages")]
    pub log_messages: Option<Vec<String>>,
}
