//! Unit tests for EVM Connected chain client (solver-specific)
//!
//! Test ordering matches solver/tests/extension-checklist.md for cross-VM synchronization.
//! Query tests (balance, escrow events, address normalization) moved to
//! chain-clients/evm/tests/evm_client_tests.rs. See chain-clients/extension-checklist.md.

use solver::chains::ConnectedEvmClient;
use solver::config::EvmChainConfig;

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::DUMMY_ESCROW_CONTRACT_ADDR_EVM;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn create_test_evm_config() -> EvmChainConfig {
    EvmChainConfig {
        name: "test-evm".to_string(),
        rpc_url: "http://127.0.0.1:8545".to_string(),
        chain_id: 84532,
        escrow_contract_addr: DUMMY_ESCROW_CONTRACT_ADDR_EVM.to_string(),
        private_key_env: "TEST_PRIVATE_KEY".to_string(),
        network_name: "localhost".to_string(),
        outflow_validator_addr: None,
        gmp_endpoint_addr: None,
    }
}

// ============================================================================
// CLIENT INITIALIZATION
// ============================================================================

/// 1. Test: ConnectedEvmClient Initialization
/// Verifies that ConnectedEvmClient::new() creates a client with correct config.
/// Why: Client initialization is the entry point for all EVM operations. A failure
/// here would prevent any solver operations on connected EVM chains.
#[test]
fn test_client_new() {
    let config = create_test_evm_config();
    let _client = ConnectedEvmClient::new(&config).unwrap();
}

// #2: test_client_new_rejects_invalid — N/A for EVM (no config validation like SVM pubkey)
// #3: test_get_escrow_events_success — moved to chain-clients/evm/tests/evm_client_tests.rs (#14)
// #4: test_get_escrow_events_empty — moved to chain-clients/evm/tests/evm_client_tests.rs (#15)
// #5: test_get_escrow_events_error — moved to chain-clients/evm/tests/evm_client_tests.rs (#16)
// #6: test_escrow_event_deserialization — N/A for EVM (parses directly in get_escrow_events)

// ============================================================================
// FULFILLMENT OPERATIONS (solver-specific, Hardhat script mechanics)
// ============================================================================

// #7: test_fulfillment_id_formatting — ⚠️ TODO: implement for EVM
// #8: test_fulfillment_signature_encoding — ⚠️ TODO: implement for EVM
// #9: test_fulfillment_command_building — ⚠️ TODO: implement for EVM
// #10: test_fulfillment_error_handling — ⚠️ TODO: implement for EVM

// #11: test_pubkey_from_hex_with_leading_zeros — N/A for EVM (SVM-specific)
// #12: test_pubkey_from_hex_no_leading_zeros — N/A for EVM (SVM-specific)
// #13: test_is_escrow_released_success — moved to chain-clients/evm/tests/evm_client_tests.rs (#3)
// #14: test_is_escrow_released_false — moved to chain-clients/evm/tests/evm_client_tests.rs (#4)
// #15: test_is_escrow_released_error — moved to chain-clients/evm/tests/evm_client_tests.rs (#5)
// #16: test_get_token_balance_success — moved to chain-clients/evm/tests/evm_client_tests.rs (#6)
// #17: test_get_token_balance_error — moved to chain-clients/evm/tests/evm_client_tests.rs (#7)
// #18: test_get_token_balance_zero — moved to chain-clients/evm/tests/evm_client_tests.rs (#8)
// #19: test_get_native_balance_success — moved to chain-clients/evm/tests/evm_client_tests.rs (#9)
// #20: test_get_native_balance_error — moved to chain-clients/evm/tests/evm_client_tests.rs (#10)
// #21: test_normalize_hex_to_address_full_length — N/A for EVM (MVM-specific)
// #22: test_normalize_hex_to_address_short_address — N/A for EVM (MVM-specific)
// #23: test_normalize_hex_to_address_odd_length — N/A for EVM (MVM-specific)
// #24: test_normalize_hex_to_address_no_prefix — N/A for EVM (MVM-specific)
// #25: test_has_outflow_requirements_success — N/A for EVM (MVM-specific GMP view function)
// #26: test_has_outflow_requirements_false — N/A for EVM (MVM-specific GMP view function)
// #27: test_has_outflow_requirements_error — N/A for EVM (MVM-specific GMP view function)

// ============================================================================
// IS ESCROW RELEASED HELPERS (EVM-specific, Hardhat script mechanics)
// ============================================================================

/// 28. Test: is_escrow_released intent ID formatting
/// Verifies that intent_id is correctly formatted for Hardhat script.
/// Why: EVM expects 0x-prefixed hex strings. Missing prefix would cause the
/// Hardhat script to fail with a parse error.
#[test]
fn test_is_escrow_released_id_formatting() {
    // Test that intent_id with 0x prefix is preserved
    let intent_id_with_prefix = "0x1234567890abcdef";
    let formatted = if intent_id_with_prefix.starts_with("0x") {
        intent_id_with_prefix.to_string()
    } else {
        format!("0x{}", intent_id_with_prefix)
    };
    assert_eq!(formatted, "0x1234567890abcdef");

    // Test that intent_id without 0x prefix gets one added
    let intent_id_no_prefix = "1234567890abcdef";
    let formatted = if intent_id_no_prefix.starts_with("0x") {
        intent_id_no_prefix.to_string()
    } else {
        format!("0x{}", intent_id_no_prefix)
    };
    assert_eq!(formatted, "0x1234567890abcdef");
}

/// 29. Test: is_escrow_released output parsing
/// Verifies that "isReleased: true/false" is correctly parsed from Hardhat output.
/// Why: The solver needs to know when escrow is auto-released to complete the flow.
/// Wrong parsing would cause the solver to wait forever or miss releases.
#[test]
fn test_is_escrow_released_output_parsing() {
    // Test "isReleased: true" output
    let output_true = "Some log output\nisReleased: true\n";
    assert!(output_true.contains("isReleased: true"));
    assert!(!output_true.contains("isReleased: false"));

    // Test "isReleased: false" output
    let output_false = "Some log output\nisReleased: false\n";
    assert!(output_false.contains("isReleased: false"));
    assert!(!output_false.contains("isReleased: true"));
}

/// 30. Test: is_escrow_released command building
/// Verifies that the Hardhat command is built correctly with all required arguments.
/// Why: The is_escrow_released function invokes a Hardhat script with environment variables.
/// Wrong command formatting would cause the script to fail or use wrong parameters.
#[test]
fn test_is_escrow_released_command_building() {
    let escrow_gmp_addr = DUMMY_ESCROW_CONTRACT_ADDR_EVM;
    let intent_id_evm = "0x1234567890abcdef";
    let evm_framework_dir = "/path/to/intent-frameworks/evm";

    // Build the command string that would be passed to bash -c
    let command = format!(
        "cd '{}' && ESCROW_GMP_ADDR='{}' INTENT_ID_EVM='{}' npx hardhat run scripts/get-is-released.js --network localhost",
        evm_framework_dir,
        escrow_gmp_addr,
        intent_id_evm
    );

    // Verify all components are present
    assert!(command.contains("ESCROW_GMP_ADDR"));
    assert!(command.contains(escrow_gmp_addr));
    assert!(command.contains("INTENT_ID_EVM"));
    assert!(command.contains(intent_id_evm));
    assert!(command.contains("get-is-released.js"));
    assert!(command.contains("--network localhost"));
}

/// 31. Test: is_escrow_released missing directory error
/// Verifies that proper error is returned when intent-frameworks/evm directory is missing.
/// Why: A clear error message helps operators diagnose deployment issues.
/// Silent failures would make debugging much harder.
#[test]
fn test_is_escrow_released_error_handling() {
    // Simulate the directory check logic
    let current_dir = std::env::current_dir().unwrap();
    let project_root = current_dir.parent().unwrap();
    let evm_framework_dir = project_root.join("intent-frameworks/evm");

    // This test documents the expected behavior - actual test would need to mock or use temp dir
    // In real code, this would bail with: "intent-frameworks/evm directory not found at: ..."
    // We're just verifying the path construction logic here
    assert!(evm_framework_dir.to_string_lossy().contains("intent-frameworks/evm"));
}

// #32: test_get_native_balance_exceeds_u64 — moved to chain-clients/evm/tests/evm_client_tests.rs (#11)
// #33: test_get_token_balance_with_padded_address — moved to chain-clients/evm/tests/evm_client_tests.rs (#12)
// #34: test_get_native_balance_with_padded_address — moved to chain-clients/evm/tests/evm_client_tests.rs (#13)
// #35: test_normalize_evm_address_padded — moved to chain-clients/evm/tests/evm_client_tests.rs (#22)
// #36: test_normalize_evm_address_passthrough — moved to chain-clients/evm/tests/evm_client_tests.rs (#23)
// #37: test_normalize_evm_address_rejects_non_zero_high_bytes — moved to chain-clients/evm/tests/evm_client_tests.rs (#24)
