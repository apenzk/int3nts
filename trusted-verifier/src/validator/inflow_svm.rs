//! Inflow SVM-specific validation functions
//!
//! This module contains SVM-specific handlers for inflow intent validation.

use crate::monitor::IntentEvent;
use crate::validator::generic::ValidationResult;
use anyhow::{Context, Result};

/// Validates that an SVM escrow's reserved solver matches the registered solver's connected chain SVM address.
///
/// This function checks that the SVM escrow's reserved solver address matches
/// the connected chain SVM address registered in the solver registry for the hub intent's solver.
pub async fn validate_svm_escrow_solver(
    intent: &IntentEvent,
    escrow_reserved_solver_addr: &str,
    hub_chain_rpc_url: &str,
    solver_registry_addr: &str,
) -> Result<ValidationResult> {
    let intent_solver = match &intent.reserved_solver_addr {
        Some(solver) => solver,
        None => {
            return Ok(ValidationResult {
                valid: false,
                message: "Hub intent does not have a reserved solver".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
            });
        }
    };

    let mvm_client = crate::mvm_client::MvmClient::new(hub_chain_rpc_url)?;
    let registered_svm_addr = mvm_client
        .get_solver_connected_chain_svm_address(intent_solver, solver_registry_addr)
        .await
        .context("Failed to query solver connected chain SVM address from registry")?;

    let registered_svm_addr = match registered_svm_addr {
        Some(addr) => addr,
        None => {
            return Ok(ValidationResult {
                valid: false,
                message: format!(
                    "Solver '{}' is not registered in the solver registry or has no connected chain SVM address",
                    intent_solver
                ),
                timestamp: chrono::Utc::now().timestamp() as u64,
            });
        }
    };

    // Normalize addresses for comparison (remove 0x prefix, pad to 64 hex chars, lowercase)
    let escrow_solver_raw = escrow_reserved_solver_addr
        .strip_prefix("0x")
        .unwrap_or(escrow_reserved_solver_addr);
    let escrow_solver = format!("{:0>64}", escrow_solver_raw).to_lowercase();
    let registered_solver_raw = registered_svm_addr
        .strip_prefix("0x")
        .unwrap_or(&registered_svm_addr);
    let registered_solver = format!("{:0>64}", registered_solver_raw).to_lowercase();

    if escrow_solver != registered_solver {
        return Ok(ValidationResult {
            valid: false,
            message: format!(
                "SVM escrow reserved solver '{}' does not match registered solver connected chain SVM address '{}'",
                escrow_reserved_solver_addr, registered_svm_addr
            ),
            timestamp: chrono::Utc::now().timestamp() as u64,
        });
    }

    Ok(ValidationResult {
        valid: true,
        message: "SVM escrow solver validation successful".to_string(),
        timestamp: chrono::Utc::now().timestamp() as u64,
    })
}
