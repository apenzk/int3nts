//! Unit tests for SVM solver registry validation
//!
//! These tests verify that SVM escrow solver validation works correctly,
//! including registry lookup, address matching, and error handling.

use trusted_gmp::monitor::IntentEvent;

#[path = "../mod.rs"]
mod test_helpers;
use test_helpers::{
    create_default_intent_mvm, setup_mock_server_with_error,
    setup_mock_server_with_svm_address_response, DUMMY_SOLVER_ADDR_HUB,
    DUMMY_SOLVER_ADDR_SVM, DUMMY_SOLVER_REGISTRY_ADDR,
};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a test intent with the given solver
fn create_test_intent(solver_addr: Option<String>) -> IntentEvent {
    IntentEvent {
        offered_metadata: "{}".to_string(),
        desired_metadata: "{}".to_string(),
        expiry_time: 1000000,
        reserved_solver_addr: solver_addr,
        connected_chain_id: Some(4),
        ..create_default_intent_mvm()
    }
}

// ============================================================================
// TESTS
// ============================================================================

/// Test that validate_svm_escrow_solver succeeds when escrow reserved_solver matches registered SVM address
/// Why: Verify successful validation path when solver is registered and addresses match
#[tokio::test]
async fn test_successful_svm_solver_validation() {
    let _ = tracing_subscriber::fmt::try_init();

    let solver_addr = DUMMY_SOLVER_ADDR_HUB;
    let solver_connected_chain_svm_addr = DUMMY_SOLVER_ADDR_SVM;
    let (_mock_server, config, _validator) =
        setup_mock_server_with_svm_address_response(solver_addr, Some(solver_connected_chain_svm_addr))
            .await;

    let intent = create_test_intent(Some(solver_addr.to_string()));

    let result = trusted_gmp::validator::inflow_svm::validate_svm_escrow_solver(
        &intent,
        solver_connected_chain_svm_addr,
        &config.hub_chain.rpc_url,
        DUMMY_SOLVER_REGISTRY_ADDR,
    )
    .await;

    assert!(result.is_ok(), "Validation should succeed");
    let validation_result = result.unwrap();
    assert!(validation_result.valid, "Validation should be valid");
    assert!(
        validation_result.message.contains("successful"),
        "Message should indicate success"
    );
}

/// Test that validate_svm_escrow_solver rejects when solver is not found in registry
/// Why: Verify error handling when solver is not registered
#[tokio::test]
async fn test_rejection_when_solver_not_registered() {
    let _ = tracing_subscriber::fmt::try_init();

    let solver_addr = DUMMY_SOLVER_ADDR_HUB;
    let (_mock_server, config, _validator) =
        setup_mock_server_with_svm_address_response(solver_addr, None).await;

    let intent = create_test_intent(Some(solver_addr.to_string()));

    let result = trusted_gmp::validator::inflow_svm::validate_svm_escrow_solver(
        &intent,
        DUMMY_SOLVER_ADDR_SVM,
        &config.hub_chain.rpc_url,
        DUMMY_SOLVER_REGISTRY_ADDR,
    )
    .await;

    assert!(result.is_ok(), "Validation should complete without error");
    let validation_result = result.unwrap();
    assert!(
        !validation_result.valid,
        "Validation should fail when solver is not registered or has no connected chain SVM address"
    );
    assert!(
        validation_result.message.contains("not registered")
            || validation_result.message.contains("no connected chain SVM address"),
        "Error message should indicate solver not registered or missing address"
    );
}

/// Test that validate_svm_escrow_solver rejects when registered SVM address doesn't match escrow reserved_solver
/// Why: Verify validation fails when addresses don't match
#[tokio::test]
async fn test_rejection_when_svm_addresses_dont_match() {
    let _ = tracing_subscriber::fmt::try_init();

    let solver_addr = DUMMY_SOLVER_ADDR_HUB;
    let solver_connected_chain_svm_addr = DUMMY_SOLVER_ADDR_SVM;
    let (_mock_server, config, _validator) =
        setup_mock_server_with_svm_address_response(solver_addr, Some(solver_connected_chain_svm_addr))
            .await;

    let intent = create_test_intent(Some(solver_addr.to_string()));

    let escrow_reserved_solver = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let result = trusted_gmp::validator::inflow_svm::validate_svm_escrow_solver(
        &intent,
        escrow_reserved_solver,
        &config.hub_chain.rpc_url,
        DUMMY_SOLVER_REGISTRY_ADDR,
    )
    .await;

    assert!(result.is_ok(), "Validation should complete without error");
    let validation_result = result.unwrap();
    assert!(
        !validation_result.valid,
        "Validation should fail when addresses don't match"
    );
    assert!(
        validation_result.message.contains("does not match")
            || validation_result.message.contains("match"),
        "Error message should indicate address mismatch"
    );
}

/// Test that SVM address comparison handles 0x prefix and padding correctly
/// Why: Verify address normalization works correctly (SVM addresses are 32 bytes)
#[tokio::test]
async fn test_svm_address_normalization() {
    let _ = tracing_subscriber::fmt::try_init();

    let test_cases = vec![
        (
            "0xABC123",
            "0x0000000000000000000000000000000000000000000000000000000000ABC123",
            true,
        ),
        (
            "ABC123",
            "0x0000000000000000000000000000000000000000000000000000000000ABC123",
            true,
        ),
        (
            "0xabc123",
            "0xABC123000000000000000000000000000000000000000000000000000000",
            false,
        ),
    ];

    for (escrow_addr, registered_addr, should_match) in test_cases {
        let solver_addr = DUMMY_SOLVER_ADDR_HUB;
        let (_mock_server, config, _validator) =
            setup_mock_server_with_svm_address_response(solver_addr, Some(registered_addr))
                .await;

        let intent = create_test_intent(Some(solver_addr.to_string()));

        let result = trusted_gmp::validator::inflow_svm::validate_svm_escrow_solver(
            &intent,
            escrow_addr,
            &config.hub_chain.rpc_url,
            DUMMY_SOLVER_REGISTRY_ADDR,
        )
        .await;

        assert!(result.is_ok(), "Validation should complete");
        let validation_result = result.unwrap();
        assert_eq!(
            validation_result.valid, should_match,
            "Address normalization failed: escrow='{}', registered='{}', expected_match={}",
            escrow_addr, registered_addr, should_match
        );
    }
}

/// Test that validate_svm_escrow_solver handles network errors gracefully
/// Why: Verify error handling for external service failures
#[tokio::test]
async fn test_error_handling_for_registry_query_failures() {
    let _ = tracing_subscriber::fmt::try_init();

    let (_mock_server, config, _validator) = setup_mock_server_with_error(500).await;

    let intent = create_test_intent(Some(DUMMY_SOLVER_ADDR_HUB.to_string()));
    let result = trusted_gmp::validator::inflow_svm::validate_svm_escrow_solver(
        &intent,
        DUMMY_SOLVER_ADDR_SVM,
        &config.hub_chain.rpc_url,
        DUMMY_SOLVER_REGISTRY_ADDR,
    )
    .await;

    assert!(
        result.is_err(),
        "Validation should return an error when registry query fails"
    );
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("Failed to query")
            || error_msg.contains("resources")
            || error_msg.contains("registry"),
        "Error message should indicate registry query failure. Got: {}",
        error_msg
    );
}

/// Test that validate_svm_escrow_solver rejects when intent has no reserved solver
/// Why: Verify error handling when intent doesn't have a solver
#[tokio::test]
async fn test_rejection_when_intent_has_no_solver() {
    let _ = tracing_subscriber::fmt::try_init();

    let (_mock_server, config, _validator) =
        setup_mock_server_with_svm_address_response(DUMMY_SOLVER_ADDR_HUB, Some(DUMMY_SOLVER_ADDR_SVM))
            .await;

    let intent = create_test_intent(None);
    let result = trusted_gmp::validator::inflow_svm::validate_svm_escrow_solver(
        &intent,
        DUMMY_SOLVER_ADDR_SVM,
        &config.hub_chain.rpc_url,
        DUMMY_SOLVER_REGISTRY_ADDR,
    )
    .await;

    assert!(result.is_ok(), "Validation should complete without error");
    let validation_result = result.unwrap();
    assert!(
        !validation_result.valid,
        "Validation should fail when intent has no reserved solver"
    );
    assert!(
        validation_result.message.contains("does not have a reserved solver"),
        "Error message should indicate intent has no solver"
    );
}
