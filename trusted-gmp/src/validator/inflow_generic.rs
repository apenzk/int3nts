//! Inflow-specific validation logic (chain-agnostic)
//!
//! This module handles validation logic for inflow intents.
//! Inflow intents have tokens locked on the connected chain (in escrow) and request tokens on the hub chain.

use anyhow::Result;
use tracing::info;

use super::generic::{CrossChainValidator, ValidationResult};
use super::inflow_evm;
use super::inflow_mvm;
use crate::monitor::{ChainType, EscrowEvent, IntentEvent};

/// Normalizes a metadata string for comparison.
/// Metadata is stored as JSON like `{"inner":"0x..."}`. This function extracts the address,
/// removes leading zeros after 0x prefix, and returns a normalized form for comparison.
fn normalize_metadata_for_comparison(metadata: &str) -> String {
    // Try to parse as JSON and extract the "inner" field
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(metadata) {
        if let Some(inner) = parsed.get("inner").and_then(|v| v.as_str()) {
            // Normalize the address: strip 0x, remove leading zeros, lowercase
            let addr_no_prefix = inner.strip_prefix("0x").unwrap_or(inner);
            let addr_trimmed = addr_no_prefix.trim_start_matches('0');
            // Ensure at least one character (for "0x0" edge case)
            let addr_trimmed = if addr_trimmed.is_empty() { "0" } else { addr_trimmed };
            return format!("0x{}", addr_trimmed.to_lowercase());
        }
    }
    // Fallback: return as-is, lowercased
    metadata.to_lowercase()
}

/// Validates fulfillment of intent conditions on the connected chain.
///
/// This function performs comprehensive validation to ensure that:
/// 1. The intent has a connected_chain_id (required for escrow validation)
/// 2. The escrow's offered_amount matches the hub intent's offered_amount
/// 3. The escrow's offered_metadata matches the hub intent's offered_metadata
/// 4. The escrow's chain_id matches the hub intent's connected_chain_id
/// 5. The escrow's desired_amount is 0 (escrow only holds offered funds, requirement is in hub intent)
/// 6. The escrow's reserved_solver matches the hub intent's solver (with chain-specific validation)
///
/// # Arguments
///
/// * `validator` - The cross-chain validator instance
/// * `intent_event` - The intent event from the hub chain
/// * `escrow_event` - The escrow event from the connected chain
///
/// # Returns
///
/// * `Ok(ValidationResult)` - Validation result with detailed information
/// * `Err(anyhow::Error)` - Validation failed due to error
pub async fn validate_intent_fulfillment(
    validator: &CrossChainValidator,
    intent_event: &IntentEvent,
    escrow_event: &EscrowEvent,
) -> Result<ValidationResult> {
    info!(
        "Validating intent fulfillment for intent: {}, escrow: {}",
        intent_event.intent_id, escrow_event.escrow_id
    );

    if let Some(result) = validate_connected_chain_id(intent_event) {
        return Ok(result);
    }

    if let Some(result) = validate_offered_amount(intent_event, escrow_event) {
        return Ok(result);
    }

    if let Some(result) = validate_offered_metadata(intent_event, escrow_event) {
        return Ok(result);
    }

    if let Some(result) = validate_chain_id_match(intent_event, escrow_event) {
        return Ok(result);
    }

    if let Some(result) = validate_desired_amount_zero(escrow_event) {
        return Ok(result);
    }

    // Note: We don't validate escrow's desired_metadata because it's a placeholder.
    // The actual requirement is the hub intent's desired_metadata, which the solver
    // must fulfill on the hub chain before the trusted-gmp approves escrow release

    if let Some(result) =
        validate_reserved_solver(validator, intent_event, escrow_event).await?
    {
        return Ok(result);
    }

    // All validations passed
    Ok(ValidationResult {
        valid: true,
        message: "Request-intent fulfillment validation successful".to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates that the intent specifies a connected chain ID.
///
/// # Arguments
///
/// * `intent_event` - Hub intent event being fulfilled
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_connected_chain_id(intent_event: &IntentEvent) -> Option<ValidationResult> {
    if intent_event.connected_chain_id.is_some() {
        return None;
    }

    Some(ValidationResult {
        valid: false,
        message: "Request-intent must specify connected_chain_id for escrow validation"
            .to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates that the escrow offered amount matches the intent offered amount.
///
/// # Arguments
///
/// * `intent_event` - Hub intent event being fulfilled
/// * `escrow_event` - Connected chain escrow event
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_offered_amount(
    intent_event: &IntentEvent,
    escrow_event: &EscrowEvent,
) -> Option<ValidationResult> {
    if escrow_event.offered_amount == intent_event.offered_amount {
        return None;
    }

    Some(ValidationResult {
        valid: false,
        message: format!(
            "Escrow offered amount {} does not match hub intent offered amount {}",
            escrow_event.offered_amount, intent_event.offered_amount
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates that the escrow offered metadata matches the intent offered metadata.
///
/// # Arguments
///
/// * `intent_event` - Hub intent event being fulfilled
/// * `escrow_event` - Connected chain escrow event
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_offered_metadata(
    intent_event: &IntentEvent,
    escrow_event: &EscrowEvent,
) -> Option<ValidationResult> {
    let escrow_metadata_normalized =
        normalize_metadata_for_comparison(&escrow_event.offered_metadata);
    let intent_metadata_normalized =
        normalize_metadata_for_comparison(&intent_event.offered_metadata);
    if escrow_metadata_normalized == intent_metadata_normalized {
        return None;
    }

    Some(ValidationResult {
        valid: false,
        message: format!(
            "Escrow offered metadata '{}' does not match hub intent offered metadata '{}' (normalized: '{}' vs '{}')",
            escrow_event.offered_metadata,
            intent_event.offered_metadata,
            escrow_metadata_normalized,
            intent_metadata_normalized
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates that the escrow chain ID matches the intent connected chain ID.
///
/// # Arguments
///
/// * `intent_event` - Hub intent event being fulfilled
/// * `escrow_event` - Connected chain escrow event
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_chain_id_match(
    intent_event: &IntentEvent,
    escrow_event: &EscrowEvent,
) -> Option<ValidationResult> {
    let intent_chain_id = intent_event.connected_chain_id?;
    if escrow_event.chain_id == intent_chain_id {
        return None;
    }

    Some(ValidationResult {
        valid: false,
        message: format!(
            "Escrow chain_id {} does not match hub intent offered_chain_id {}. Escrow was discovered on chain {} but intent specifies chain {}",
            escrow_event.chain_id, intent_chain_id, escrow_event.chain_id, intent_chain_id
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates that the escrow desired amount is zero for inflow escrows.
///
/// # Arguments
///
/// * `escrow_event` - Connected chain escrow event
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_desired_amount_zero(escrow_event: &EscrowEvent) -> Option<ValidationResult> {
    if escrow_event.desired_amount == 0 {
        return None;
    }

    Some(ValidationResult {
        valid: false,
        message: format!(
            "Escrow desired amount must be 0, but got {}. Escrow only holds offered funds; the actual requirement is specified in the hub intent",
            escrow_event.desired_amount
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates the escrow reserved solver matches the intent reserved solver.
///
/// # Arguments
///
/// * `validator` - Cross-chain validator (for hub config access)
/// * `intent_event` - Hub intent event being fulfilled
/// * `escrow_event` - Connected chain escrow event
///
/// # Returns
///
/// * `Ok(Some(ValidationResult))` - Validation failure details
/// * `Ok(None)` - Validation passed
/// * `Err(anyhow::Error)` - Failed to query hub registry
async fn validate_reserved_solver(
    validator: &CrossChainValidator,
    intent_event: &IntentEvent,
    escrow_event: &EscrowEvent,
) -> Result<Option<ValidationResult>> {
    let (Some(escrow_solver), Some(_intent_solver)) = (
        &escrow_event.reserved_solver_addr,
        &intent_event.reserved_solver_addr,
    ) else {
        if escrow_event.reserved_solver_addr.is_some()
            || intent_event.reserved_solver_addr.is_some()
        {
            return Ok(Some(ValidationResult {
                valid: false,
                message: format!(
                    "Escrow and intent reservation mismatch: escrow reserved_solver={:?}, intent solver={:?}",
                    escrow_event.reserved_solver_addr, intent_event.reserved_solver_addr
                ),
                timestamp: chrono::Utc::now().timestamp() as u64,
            }));
        }
        return Ok(None);
    };

    let hub_rpc_url = &validator.config.hub_chain.rpc_url;
    let solver_registry_addr = &validator.config.hub_chain.intent_module_addr;

    let validation_result = match escrow_event.chain_type {
        ChainType::Evm => {
            inflow_evm::validate_evm_escrow_solver(
                intent_event,
                escrow_solver,
                hub_rpc_url,
                solver_registry_addr,
            )
            .await?
        }
        ChainType::Mvm => {
            inflow_mvm::validate_mvm_escrow_solver(
                intent_event,
                escrow_solver,
                hub_rpc_url,
                solver_registry_addr,
            )
            .await?
        }
        ChainType::Svm => {
            crate::validator::inflow_svm::validate_svm_escrow_solver(
                intent_event,
                escrow_solver,
                hub_rpc_url,
                solver_registry_addr,
            )
            .await?
        }
    };

    if validation_result.valid {
        Ok(None)
    } else {
        Ok(Some(validation_result))
    }
}
