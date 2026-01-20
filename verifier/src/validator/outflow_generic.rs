//! Outflow-specific validation logic (chain-agnostic)
//!
//! This module handles validation logic for outflow intents.
//! Outflow intents have tokens locked on the hub chain and request tokens on the connected chain.

use anyhow::{Context, Result};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;
use tracing::info;

use super::generic::{
    validate_address_format, CrossChainValidator, FulfillmentTransactionParams, ValidationResult,
};
use crate::monitor::{ChainType, IntentEvent};

/// Validates an outflow fulfillment transaction against a intent
///
/// This function validates that a connected chain transaction properly fulfills
/// an outflow intent by checking:
/// - Transaction was confirmed and successful
/// - intent_id matches the intent
/// - Recipient address matches requester_addr_connected_chain
/// - Amount matches desired_amount
/// - Solver address matches reserved solver
///
/// ## Solver Registration Requirements
///
/// **IMPORTANT**: The solver must be registered in the solver registry with the correct
/// address for the connected chain. All addresses (Move VM address for Move VM chains,
/// EVM address for EVM chains) must be provided during registration. If the solver
/// address for the connected chain is not found in the registry, this indicates an
/// error on the solver's side - they must register correctly before attempting to
/// fulfill intents. The verifier will reject transactions from unregistered or
/// incorrectly registered solvers.
///
/// # Arguments
///
/// * `validator` - The cross-chain validator instance
/// * `intent` - The outflow intent from the hub chain
/// * `tx_params` - Extracted parameters from the connected chain transaction
/// * `tx_success` - Whether the transaction was successful
///
/// # Returns
///
/// * `Ok(ValidationResult)` - Validation result
/// * `Err(anyhow::Error)` - Validation failed due to error
pub async fn validate_outflow_fulfillment(
    validator: &CrossChainValidator,
    intent: &IntentEvent,
    tx_params: &FulfillmentTransactionParams,
    tx_success: bool,
) -> Result<ValidationResult> {
    info!(
        "Validating outflow fulfillment for intent: {}",
        intent.intent_id
    );

    if let Some(result) = validate_tx_success(tx_success) {
        return Ok(result);
    }

    if let Some(result) = validate_intent_id_matches(intent, tx_params) {
        return Ok(result);
    }

    if let Some(result) = validate_recipient_matches(validator, intent, tx_params) {
        return Ok(result);
    }

    if let Some(result) = validate_amount_matches(intent, tx_params) {
        return Ok(result);
    }

    if let Some(result) = validate_reserved_solver(validator, intent, tx_params).await? {
        return Ok(result);
    }

    // All validations passed
    Ok(ValidationResult {
        valid: true,
        message: "Outflow fulfillment validation successful".to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates the transaction success flag.
///
/// # Arguments
///
/// * `tx_success` - Whether the connected-chain transaction succeeded
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_tx_success(tx_success: bool) -> Option<ValidationResult> {
    if tx_success {
        return None;
    }

    Some(ValidationResult {
        valid: false,
        message: "Transaction was not successful".to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates the transaction intent_id matches the hub intent.
///
/// # Arguments
///
/// * `intent` - Hub intent event being fulfilled
/// * `tx_params` - Parsed fulfillment transaction parameters
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_intent_id_matches(
    intent: &IntentEvent,
    tx_params: &FulfillmentTransactionParams,
) -> Option<ValidationResult> {
    let tx_intent_id_normalized = crate::monitor::normalize_intent_id(&tx_params.intent_id);
    let intent_id_normalized = crate::monitor::normalize_intent_id(&intent.intent_id);
    if tx_intent_id_normalized == intent_id_normalized {
        return None;
    }

    Some(ValidationResult {
        valid: false,
        message: format!(
            "Transaction intent_id '{}' does not match intent '{}'",
            tx_params.intent_id, intent.intent_id
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates the connected-chain recipient matches the intent requester.
///
/// This check ensures the transfer on the connected chain sends funds to the
/// requester address recorded in the hub intent, using chain-type-specific
/// address normalization and format validation.
///
/// # Arguments
///
/// * `validator` - Cross-chain validator (for chain config lookup)
/// * `intent` - Hub intent event being fulfilled
/// * `tx_params` - Parsed fulfillment transaction parameters
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_recipient_matches(
    validator: &CrossChainValidator,
    intent: &IntentEvent,
    tx_params: &FulfillmentTransactionParams,
) -> Option<ValidationResult> {
    let requester_addr = match intent.requester_addr_connected_chain.as_ref() {
        Some(addr) => addr,
        None => {
            if intent.connected_chain_id.is_some() {
                return Some(ValidationResult {
                    valid: false,
                    message: "Request-intent has connected_chain_id but missing requester_addr_connected_chain (required for outflow validation)".to_string(),
                    timestamp: chrono::Utc::now().timestamp() as u64,
                });
            }
            return None;
        }
    };

    let chain_id = match intent.connected_chain_id {
        Some(id) => id,
        None => {
            return Some(ValidationResult {
                valid: false,
                message: "Request-intent missing connected_chain_id (required for address validation)".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
            });
        }
    };

    let chain_type = match crate::validator::generic::get_chain_type_from_chain_id(
        chain_id,
        validator.config(),
    ) {
        Ok(ct) => ct,
        Err(e) => {
            return Some(ValidationResult {
                valid: false,
                message: format!(
                    "Failed to determine chain type from connected_chain_id for address validation: {}",
                    e
                ),
                timestamp: chrono::Utc::now().timestamp() as u64,
            });
        }
    };

    let normalized_requester_addr =
        crate::validator::generic::normalize_address(requester_addr, chain_type);

    if let Err(e) = validate_address_format(&tx_params.recipient_addr, chain_type) {
        return Some(ValidationResult {
            valid: false,
            message: format!(
                "Transaction recipient address format validation failed: {}",
                e
            ),
            timestamp: chrono::Utc::now().timestamp() as u64,
        });
    }

    if let Err(e) = validate_address_format(&normalized_requester_addr, chain_type) {
        return Some(ValidationResult {
            valid: false,
            message: format!(
                "Request-intent requester_addr_connected_chain format validation failed: {}",
                e
            ),
            timestamp: chrono::Utc::now().timestamp() as u64,
        });
    }

    let tx_recipient_raw = tx_params
        .recipient_addr
        .strip_prefix("0x")
        .unwrap_or(&tx_params.recipient_addr);
    let tx_recipient = format!("{:0>64}", tx_recipient_raw).to_lowercase();
    let requester_raw = normalized_requester_addr
        .strip_prefix("0x")
        .unwrap_or(&normalized_requester_addr);
    let requester = format!("{:0>64}", requester_raw).to_lowercase();

    if tx_recipient == requester {
        return None;
    }

    if chain_type == ChainType::Svm {
        // SVM transfers use token accounts as destinations. If the intent stores
        // the requester's wallet pubkey, accept the derived ATA as a match.
        if let Ok(ata_hex) = derive_svm_ata_hex(&normalized_requester_addr, &tx_params.token_metadata) {
            let ata_raw = ata_hex.strip_prefix("0x").unwrap_or(&ata_hex);
            let ata = format!("{:0>64}", ata_raw).to_lowercase();
            if tx_recipient == ata {
                return None;
            }
        }
    }

    Some(ValidationResult {
        valid: false,
        message: format!(
            "Transaction recipient '{}' does not match intent requester_addr_connected_chain '{}'",
            tx_params.recipient_addr, requester_addr
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

const ASSOCIATED_TOKEN_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";
const TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

fn derive_svm_ata_hex(owner_hex: &str, mint_hex: &str) -> Result<String> {
    let owner = svm_hex_to_pubkey(owner_hex)?;
    let mint = svm_hex_to_pubkey(mint_hex)?;
    let program_id = Pubkey::from_str(ASSOCIATED_TOKEN_PROGRAM_ID)
        .context("Invalid associated token program id")?;
    let token_program = Pubkey::from_str(TOKEN_PROGRAM_ID)
        .context("Invalid token program id")?;
    let ata = Pubkey::find_program_address(
        &[owner.as_ref(), token_program.as_ref(), mint.as_ref()],
        &program_id,
    )
    .0;
    Ok(format!("0x{}", hex::encode(ata.to_bytes())))
}

fn svm_hex_to_pubkey(address: &str) -> Result<Pubkey> {
    let raw = address.strip_prefix("0x").unwrap_or(address);
    let bytes = hex::decode(raw).context("Invalid hex address")?;
    if bytes.len() != 32 {
        anyhow::bail!(
            "Invalid SVM address length: expected 32 bytes, got {}",
            bytes.len()
        );
    }
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("Failed to convert address bytes to pubkey"))?;
    Ok(Pubkey::new_from_array(array))
}

/// Validates the transaction amount matches the intent desired amount.
///
/// # Arguments
///
/// * `intent` - Hub intent event being fulfilled
/// * `tx_params` - Parsed fulfillment transaction parameters
///
/// # Returns
///
/// * `Some(ValidationResult)` - Validation failure details
/// * `None` - Validation passed
fn validate_amount_matches(
    intent: &IntentEvent,
    tx_params: &FulfillmentTransactionParams,
) -> Option<ValidationResult> {
    let expected_amount = intent.desired_amount;

    if expected_amount == 0 {
        return Some(ValidationResult {
            valid: false,
            message: "Request-intent desired_amount is 0 - this indicates a bug in the Move code. The event should contain the original desired_amount for the connected chain".to_string(),
            timestamp: chrono::Utc::now().timestamp() as u64,
        });
    }

    if tx_params.amount == expected_amount {
        return None;
    }

    Some(ValidationResult {
        valid: false,
        message: format!(
            "Transaction amount {} does not match intent desired amount {} (amount desired on connected chain)",
            tx_params.amount, expected_amount
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}

/// Validates the transaction solver matches the reserved solver (chain-specific).
///
/// # Arguments
///
/// * `validator` - Cross-chain validator (for hub config access)
/// * `intent` - Hub intent event being fulfilled
/// * `tx_params` - Parsed fulfillment transaction parameters
///
/// # Returns
///
/// * `Ok(Some(ValidationResult))` - Validation failure details
/// * `Ok(None)` - Validation passed
/// * `Err(anyhow::Error)` - Failed to query hub registry
async fn validate_reserved_solver(
    validator: &CrossChainValidator,
    intent: &IntentEvent,
    tx_params: &FulfillmentTransactionParams,
) -> Result<Option<ValidationResult>> {
    let reserved_solver = match intent.reserved_solver_addr.as_ref() {
        Some(solver) => solver,
        None => {
            return Ok(Some(ValidationResult {
                valid: false,
                message: "Request-intent has no reserved solver".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
            }));
        }
    };

    let hub_rpc_url = &validator.config().hub_chain.rpc_url;
    let hub_registry_addr = &validator.config().hub_chain.intent_module_addr;
    let hub_client = crate::mvm_client::MvmClient::new(hub_rpc_url)?;

    let chain_id = match intent.connected_chain_id {
        Some(id) => id,
        None => {
            return Ok(Some(ValidationResult {
                valid: false,
                message: "Request-intent missing connected_chain_id (required for outflow validation)".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
            }));
        }
    };

    let chain_type = match crate::validator::generic::get_chain_type_from_chain_id(
        chain_id,
        validator.config(),
    ) {
        Ok(ct) => ct,
        Err(e) => {
            return Ok(Some(ValidationResult {
                valid: false,
                message: format!(
                    "Failed to determine chain type from connected_chain_id: {}",
                    e
                ),
                timestamp: chrono::Utc::now().timestamp() as u64,
            }));
        }
    };

    match chain_type {
        crate::monitor::ChainType::Mvm => {
            validate_mvm_solver(&hub_client, reserved_solver, hub_registry_addr, tx_params).await
        }
        crate::monitor::ChainType::Evm => {
            validate_evm_solver(&hub_client, reserved_solver, hub_registry_addr, tx_params).await
        }
        crate::monitor::ChainType::Svm => {
            validate_svm_solver(&hub_client, reserved_solver, hub_registry_addr, tx_params).await
        }
    }
}

/// Validates the connected-chain solver for Move VM chains.
///
/// # Arguments
///
/// * `hub_client` - Hub chain client for registry lookups
/// * `reserved_solver` - Reserved solver address from the hub intent
/// * `hub_registry_addr` - Solver registry module address
/// * `tx_params` - Parsed fulfillment transaction parameters
///
/// # Returns
///
/// * `Ok(Some(ValidationResult))` - Validation failure details
/// * `Ok(None)` - Validation passed
async fn validate_mvm_solver(
    hub_client: &crate::mvm_client::MvmClient,
    reserved_solver: &str,
    hub_registry_addr: &str,
    tx_params: &FulfillmentTransactionParams,
) -> Result<Option<ValidationResult>> {
    let registered_mvm_addr = hub_client
        .get_solver_connected_chain_mvm_address(reserved_solver, hub_registry_addr)
        .await
        .context("Failed to query reserved solver connected chain Move VM address from hub chain registry")?;

    let registered_mvm_addr = match registered_mvm_addr {
        Some(addr) => addr,
        None => {
            return Ok(Some(ValidationResult {
                valid: false,
                message: format!(
                    "Reserved solver '{}' is not registered in hub chain solver registry or has no connected chain Move VM address",
                    reserved_solver
                ),
                timestamp: chrono::Utc::now().timestamp() as u64,
            }));
        }
    };

    let tx_solver_raw = tx_params
        .solver_addr
        .strip_prefix("0x")
        .unwrap_or(&tx_params.solver_addr);
    let tx_solver = format!("{:0>64}", tx_solver_raw).to_lowercase();
    let registered_mvm_raw = registered_mvm_addr
        .strip_prefix("0x")
        .unwrap_or(&registered_mvm_addr);
    let registered_mvm = format!("{:0>64}", registered_mvm_raw).to_lowercase();

    if tx_solver == registered_mvm {
        return Ok(None);
    }

    Ok(Some(ValidationResult {
        valid: false,
        message: format!(
            "Transaction solver '{}' does not match reserved solver's connected chain Move VM address '{}' (reserved solver hub chain address: '{}')",
            tx_params.solver_addr, registered_mvm_addr, reserved_solver
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    }))
}

/// Validates the connected-chain solver for EVM chains.
///
/// # Arguments
///
/// * `hub_client` - Hub chain client for registry lookups
/// * `reserved_solver` - Reserved solver address from the hub intent
/// * `hub_registry_addr` - Solver registry module address
/// * `tx_params` - Parsed fulfillment transaction parameters
///
/// # Returns
///
/// * `Ok(Some(ValidationResult))` - Validation failure details
/// * `Ok(None)` - Validation passed
async fn validate_evm_solver(
    hub_client: &crate::mvm_client::MvmClient,
    reserved_solver: &str,
    hub_registry_addr: &str,
    tx_params: &FulfillmentTransactionParams,
) -> Result<Option<ValidationResult>> {
    let registered_evm_addr = hub_client
        .get_solver_evm_address(reserved_solver, hub_registry_addr)
        .await
        .context("Failed to query reserved solver EVM address from hub chain registry")?;

    let registered_evm_addr = match registered_evm_addr {
        Some(addr) => addr,
        None => {
            tracing::warn!(
                "Failed to get EVM address for solver '{}' from registry at '{}'. This could mean:\n\
                1. Solver is not registered\n\
                2. Solver is registered but has no connected_chain_evm_addr set\n\
                3. Resource query failed or returned unexpected format\n\
                Check verifier logs for detailed parsing information.",
                reserved_solver,
                hub_registry_addr
            );
            return Ok(Some(ValidationResult {
                valid: false,
                message: format!(
                    "Reserved solver '{}' is not registered in hub chain solver registry or has no connected chain EVM address. Check verifier logs for detailed parsing information.",
                    reserved_solver
                ),
                timestamp: chrono::Utc::now().timestamp() as u64,
            }));
        }
    };

    let tx_solver = tx_params
        .solver_addr
        .strip_prefix("0x")
        .unwrap_or(&tx_params.solver_addr)
        .to_lowercase();
    let registered_evm = registered_evm_addr
        .strip_prefix("0x")
        .unwrap_or(&registered_evm_addr)
        .to_lowercase();

    if tx_solver == registered_evm {
        return Ok(None);
    }

    Ok(Some(ValidationResult {
        valid: false,
        message: format!(
            "Transaction solver '{}' does not match reserved solver's EVM address '{}' (reserved solver Move VM address: '{}')",
            tx_params.solver_addr, registered_evm_addr, reserved_solver
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    }))
}

/// Validates the connected-chain solver for SVM chains.
///
/// # Arguments
///
/// * `hub_client` - Hub chain client for registry lookups
/// * `reserved_solver` - Reserved solver address from the hub intent
/// * `hub_registry_addr` - Solver registry module address
/// * `tx_params` - Parsed fulfillment transaction parameters
///
/// # Returns
///
/// * `Ok(Some(ValidationResult))` - Validation failure details
/// * `Ok(None)` - Validation passed
async fn validate_svm_solver(
    hub_client: &crate::mvm_client::MvmClient,
    reserved_solver: &str,
    hub_registry_addr: &str,
    tx_params: &FulfillmentTransactionParams,
) -> Result<Option<ValidationResult>> {
    let registered_svm_addr = hub_client
        .get_solver_connected_chain_svm_address(reserved_solver, hub_registry_addr)
        .await
        .context("Failed to query reserved solver SVM address from hub chain registry")?;

    let registered_svm_addr = match registered_svm_addr {
        Some(addr) => addr,
        None => {
            return Ok(Some(ValidationResult {
                valid: false,
                message: format!(
                    "Reserved solver '{}' is not registered in hub chain solver registry or has no connected chain SVM address",
                    reserved_solver
                ),
                timestamp: chrono::Utc::now().timestamp() as u64,
            }));
        }
    };

    let tx_solver_raw = tx_params
        .solver_addr
        .strip_prefix("0x")
        .unwrap_or(&tx_params.solver_addr);
    let tx_solver = format!("{:0>64}", tx_solver_raw).to_lowercase();
    let registered_solver_raw = registered_svm_addr
        .strip_prefix("0x")
        .unwrap_or(&registered_svm_addr);
    let registered_solver = format!("{:0>64}", registered_solver_raw).to_lowercase();

    if tx_solver == registered_solver {
        return Ok(None);
    }

    Ok(Some(ValidationResult {
        valid: false,
        message: format!(
            "Transaction solver '{}' does not match reserved solver's connected chain SVM address '{}' (reserved solver hub chain address: '{}')",
            tx_params.solver_addr, registered_svm_addr, reserved_solver
        ),
        timestamp: chrono::Utc::now().timestamp() as u64,
    }))
}
