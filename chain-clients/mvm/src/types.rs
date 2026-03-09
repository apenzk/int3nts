//! Type definitions for Move VM REST API responses and Move event structures.

use serde::{Deserialize, Deserializer, Serialize};

// ============================================================================
// DESERIALIZATION HELPERS
// ============================================================================

/// Deserialize u64 from either string or number (Aptos returns chain_id as either).
pub fn deserialize_u64_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(s) => Ok(s),
        serde_json::Value::Number(n) => Ok(n.to_string()),
        _ => Err(D::Error::custom(format!(
            "expected string or number for chain_id, got: {:?}",
            value
        ))),
    }
}

/// Deserialize Move's Option<T> format: {"vec": [value]} for Some, {"vec": []} for None.
pub fn deserialize_move_option_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    #[derive(Deserialize)]
    struct MoveOption {
        vec: Vec<String>,
    }

    let opt: MoveOption = Deserialize::deserialize(deserializer)?;
    match opt.vec.as_slice() {
        [value] => Ok(Some(value.clone())),
        [] => Ok(None),
        _ => Err(D::Error::custom(format!(
            "expected Move Option format with 0 or 1 element in vec, got {} elements",
            opt.vec.len()
        ))),
    }
}

// ============================================================================
// API RESPONSE STRUCTURES
// ============================================================================

/// Move VM REST API response wrapper
#[derive(Debug, Deserialize)]
pub struct MvmResponse<T> {
    #[allow(dead_code)]
    pub inner: T,
}

/// Account information from Move VM chain
#[derive(Debug, Deserialize)]
pub struct AccountInfo {
    #[allow(dead_code)]
    pub sequence_number: String,
    #[allow(dead_code)]
    pub authentication_key: String,
}

/// Resource data from Move VM account
#[derive(Debug, Deserialize, Clone)]
pub struct ResourceData {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub data: serde_json::Value,
}

/// Event handle wrapper
#[derive(Debug, Deserialize, Clone)]
pub struct EventHandle {
    #[allow(dead_code)]
    pub counter: String,
    #[allow(dead_code)]
    pub guid: EventHandleGuid,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EventHandleGuid {
    #[allow(dead_code)]
    pub id: EventHandleGuidId,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EventHandleGuidId {
    #[allow(dead_code)]
    pub creation_num: String,
}

/// Module information
#[derive(Debug, Deserialize)]
pub struct ModuleInfo {
    #[allow(dead_code)]
    pub bytecode: String,
    #[allow(dead_code)]
    pub abi: serde_json::Value,
}

/// Resources wrapper
#[derive(Debug, Deserialize)]
pub struct Resources {
    #[serde(rename = "Result")]
    #[allow(dead_code)]
    pub result: Vec<ResourceData>,
}

/// Event GUID (for module events)
#[derive(Debug, Deserialize, Clone)]
pub struct EventGuid {
    #[serde(rename = "creation_number")]
    #[allow(dead_code)]
    pub creation_number: String,
    #[serde(rename = "account_address")]
    #[allow(dead_code)]
    pub account_addr: String,
}

/// Event from Move VM blockchain.
/// Can be either a module event (with guid) or legacy EventHandle event (with key).
#[derive(Debug, Deserialize, Clone)]
pub struct MvmEvent {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub guid: Option<EventGuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[allow(dead_code)]
    pub key: Option<String>,
    #[allow(dead_code)]
    pub sequence_number: String,
    pub r#type: String,
    pub data: serde_json::Value,
}

/// Transaction details from Move VM chain
#[derive(Debug, Deserialize)]
pub struct MvmTransaction {
    #[allow(dead_code)]
    pub version: String,
    #[allow(dead_code)]
    pub hash: String,
    #[allow(dead_code)]
    pub success: bool,
    #[allow(dead_code)]
    pub events: Vec<MvmEvent>,
}

// ============================================================================
// EVENT DATA STRUCTURES FOR MOVE EVENTS
// ============================================================================

/// Represents a LimitOrderEvent emitted by the Move fa_intent module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrderEvent {
    pub intent_addr: String,
    pub intent_id: String,
    pub offered_metadata: serde_json::Value,
    #[serde(
        rename = "offered_metadata_addr",
        deserialize_with = "deserialize_move_option_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub offered_metadata_address: Option<String>,
    pub offered_amount: String,
    pub offered_chain_id: String,
    pub desired_metadata: serde_json::Value,
    pub desired_amount: String,
    pub desired_chain_id: String,
    pub requester_addr: String,
    pub expiry_time: String,
    pub revocable: bool,
    #[serde(
        deserialize_with = "deserialize_move_option_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub reserved_solver: Option<String>,
    #[serde(
        deserialize_with = "deserialize_move_option_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub requester_addr_connected_chain: Option<String>,
}

/// Represents an OracleLimitOrderEvent emitted by the Move fa_intent_with_oracle module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleLimitOrderEvent {
    pub intent_addr: String,
    pub intent_id: String,
    pub offered_metadata: serde_json::Value,
    pub offered_amount: String,
    #[serde(deserialize_with = "deserialize_u64_string")]
    pub offered_chain_id: String,
    pub desired_metadata: serde_json::Value,
    #[serde(
        rename = "desired_metadata_addr",
        deserialize_with = "deserialize_move_option_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub desired_metadata_address: Option<String>,
    pub desired_amount: String,
    #[serde(deserialize_with = "deserialize_u64_string")]
    pub desired_chain_id: String,
    pub requester_addr: String,
    pub expiry_time: String,
    pub min_reported_value: String,
    pub revocable: bool,
    #[serde(
        deserialize_with = "deserialize_move_option_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub reserved_solver: Option<String>,
    #[serde(
        deserialize_with = "deserialize_move_option_string",
        skip_serializing_if = "Option::is_none"
    )]
    pub requester_addr_connected_chain: Option<String>,
}

/// Represents a LimitOrderFulfillmentEvent emitted when an intent is fulfilled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitOrderFulfillmentEvent {
    pub intent_addr: String,
    pub intent_id: String,
    #[serde(rename = "solver")]
    pub solver_addr: String,
    pub provided_metadata: serde_json::Value,
    pub provided_amount: String,
    pub timestamp: String,
}
