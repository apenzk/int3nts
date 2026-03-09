//! EVM JSON-RPC types shared across intent services
//!
//! These types are used by the coordinator, integrated-gmp, and solver
//! for communicating with EVM-compatible blockchain nodes.

use serde::{Deserialize, Serialize};

/// EVM JSON-RPC request wrapper
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<serde_json::Value>,
    pub id: u64,
}

/// EVM JSON-RPC response wrapper
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse<T> {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
    #[allow(dead_code)]
    pub id: u64,
}

/// JSON-RPC error object
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

/// EVM event log entry from eth_getLogs
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvmLog {
    /// Address of the contract that emitted the event
    pub address: String,
    /// Array of topics (indexed event parameters)
    pub topics: Vec<String>,
    /// Event data (non-indexed parameters)
    pub data: String,
    /// Block number (JSON-RPC uses camelCase: blockNumber)
    #[serde(rename = "blockNumber")]
    pub block_number: String,
    /// Transaction hash (JSON-RPC uses camelCase: transactionHash)
    #[serde(rename = "transactionHash")]
    pub transaction_hash: String,
    /// Log index (JSON-RPC uses camelCase: logIndex)
    #[serde(rename = "logIndex")]
    pub log_index: String,
}

/// EscrowCreated event data parsed from EVM logs
///
/// Event signature: EscrowCreated(bytes32 indexed intentId, bytes32 escrowId, address indexed requester, uint64 amount, address indexed token, bytes32 reservedSolver, uint64 expiry)
/// topics[0] = event signature hash
/// topics[1] = intentId (bytes32)
/// topics[2] = requester (address, padded to 32 bytes)
/// topics[3] = token (address, padded to 32 bytes)
/// data = abi.encode(escrowId, amount, reservedSolver, expiry) = 256 hex chars
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscrowCreatedEvent {
    /// Intent ID (indexed topic[1], bytes32)
    pub intent_id: String,
    /// Escrow ID (from data, bytes32)
    pub escrow_id: String,
    /// Requester address (indexed topic[2], address)
    pub requester_addr: String,
    /// Amount escrowed (from data, uint64)
    pub amount: u64,
    /// Token contract address (indexed topic[3], address)
    pub token_addr: String,
    /// Reserved solver address (from data, bytes32)
    pub reserved_solver: String,
    /// Expiry timestamp (from data, uint64)
    pub expiry: u64,
    /// Block number
    pub block_number: String,
    /// Transaction hash
    pub transaction_hash: String,
}

/// EVM transaction details from JSON-RPC
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvmTransaction {
    /// Transaction hash
    #[serde(rename = "hash")]
    pub hash: String,
    /// Block number (hex string)
    #[serde(rename = "blockNumber")]
    pub block_number: Option<String>,
    /// Transaction index in block (hex string)
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<String>,
    /// From address (sender)
    #[serde(rename = "from")]
    pub from: String,
    /// To address (recipient/contract)
    #[serde(rename = "to")]
    pub to: Option<String>,
    /// Transaction data (calldata)
    pub input: String,
    /// Transaction value (in wei, hex string)
    pub value: String,
    /// Gas used (hex string)
    #[serde(rename = "gas")]
    pub gas: String,
    /// Gas price (hex string)
    #[serde(rename = "gasPrice")]
    pub gas_price: String,
    /// Transaction status (1 = success, 0 = failure, null = pending)
    pub status: Option<String>,
}
