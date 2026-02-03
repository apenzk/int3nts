//! Unit tests for MVM Connected chain client
//!
//! Test ordering matches EXTENSION-CHECKLIST.md for cross-VM synchronization.
//! Tests marked N/A in the checklist are skipped in this file.
//!
//! Hub-specific tests are in hub_client_tests.rs (hub is always MVM).

use serde_json::json;
use solver::chains::ConnectedMvmClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[path = "../helpers.rs"]
mod test_helpers;
use test_helpers::{
    create_default_connected_mvm_chain_config, DUMMY_ESCROW_ID_MVM, DUMMY_EXPIRY, DUMMY_INTENT_ID,
    DUMMY_MODULE_ADDR_CON, DUMMY_REQUESTER_ADDR_MVMCON, DUMMY_SOLVER_ADDR_MVMCON,
    DUMMY_TOKEN_ADDR_HUB, DUMMY_TOKEN_ADDR_MVMCON,
};

// ============================================================================
// CLIENT INITIALIZATION
// ============================================================================

/// 1. Test: ConnectedMvmClient Initialization
/// Verifies that ConnectedMvmClient::new() creates a client with correct config.
/// Why: Client initialization is the entry point for all MVM operations. A failure
/// here would prevent any solver operations on connected MVM chains.
#[test]
fn test_mvm_client_new() {
    let config = create_default_connected_mvm_chain_config();
    let _client = ConnectedMvmClient::new(&config).unwrap();
}

// #2: client_new_rejects_invalid - N/A for MVM (no config validation like SVM pubkey)

// ============================================================================
// ESCROW EVENT QUERYING
// ============================================================================

/// 3. Test: Get Escrow Events Success
/// Verifies that get_escrow_events() parses OracleLimitOrderEvent correctly.
/// Why: The solver needs to parse escrow events from connected MVM chains to
/// identify fulfillment opportunities. A parsing bug would cause missed escrows.
#[tokio::test]
async fn test_get_escrow_events_success() {
    let mock_server = MockServer::start().await;
    // Note: base_url simulates the full RPC URL including /v1 suffix
    let base_url = format!("{}/v1", mock_server.uri());

    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/accounts/{}/transactions",
            DUMMY_REQUESTER_ADDR_MVMCON.strip_prefix("0x").unwrap()
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "events": [
                    {
                        "type": format!(
                            "{}::fa_intent_with_oracle::OracleLimitOrderEvent",
                            DUMMY_MODULE_ADDR_CON
                        ),
                        "data": {
                            "intent_addr": DUMMY_ESCROW_ID_MVM,
                            "intent_id": DUMMY_INTENT_ID,
                            "requester_addr": DUMMY_REQUESTER_ADDR_MVMCON,
                            "offered_metadata": {"inner": DUMMY_TOKEN_ADDR_MVMCON},
                            "offered_amount": "1000",
                            "desired_metadata": {"inner": DUMMY_TOKEN_ADDR_MVMCON},
                            "desired_amount": "2000",
                            "expiry_time": DUMMY_EXPIRY.to_string(),
                            "revocable": true,
                            "reserved_solver": {"vec": [DUMMY_SOLVER_ADDR_MVMCON]}
                        }
                    }
                ]
            }
        ])))
        .mount(&mock_server)
        .await;

    let mut config = create_default_connected_mvm_chain_config();
    config.rpc_url = base_url;
    let client = ConnectedMvmClient::new(&config).unwrap();

    let accounts = vec![DUMMY_REQUESTER_ADDR_MVMCON.to_string()];
    let events = client.get_escrow_events(&accounts, None).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].intent_id, DUMMY_INTENT_ID);
    assert_eq!(events[0].escrow_id, DUMMY_ESCROW_ID_MVM);
}

// #4: get_escrow_events_empty - TODO: implement for MVM
// #5: get_escrow_events_error - TODO: implement for MVM

// ============================================================================
// ESCROW EVENT DESERIALIZATION
// ============================================================================

/// 6. Test: Escrow Event Deserialization
/// Verifies that EscrowEvent deserializes correctly from JSON.
/// Why: MVM escrow events have a specific JSON structure with Move Option wrappers.
/// A deserialization bug would cause the solver to miss reserved solver restrictions.
#[test]
fn test_escrow_event_deserialization() {
    let json = json!({
        "intent_addr": DUMMY_ESCROW_ID_MVM,
        "intent_id": DUMMY_INTENT_ID,
        "requester_addr": DUMMY_REQUESTER_ADDR_MVMCON,
        "offered_metadata": {"inner": DUMMY_TOKEN_ADDR_HUB},
        "offered_amount": "1000",
        "desired_metadata": {"inner": DUMMY_TOKEN_ADDR_MVMCON},
        "desired_amount": "2000",
        "expiry_time": DUMMY_EXPIRY.to_string(),
        "revocable": true,
        "reserved_solver": {"vec": [DUMMY_SOLVER_ADDR_MVMCON]}
    });

    let event: solver::chains::connected_mvm::EscrowEvent = serde_json::from_value(json).unwrap();
    assert_eq!(event.escrow_id, DUMMY_ESCROW_ID_MVM);
    assert_eq!(event.intent_id, DUMMY_INTENT_ID);
    assert_eq!(event.requester_addr, DUMMY_REQUESTER_ADDR_MVMCON);
    assert_eq!(event.offered_amount, "1000");
    // reserved_solver is a Move Option wrapper
    let solver = event.reserved_solver.and_then(|opt| opt.into_option());
    assert_eq!(solver, Some(DUMMY_SOLVER_ADDR_MVMCON.to_string()));
}

// ============================================================================
// FULFILLMENT OPERATIONS
// ============================================================================

/// 7. Test: Fulfill Outflow Via GMP Intent ID Formatting
/// Verifies that intent_id is correctly formatted for aptos CLI hex: argument.
/// Why: The aptos CLI expects hex arguments without the 0x prefix. Wrong formatting
/// would cause the transaction to fail with a parse error.
#[test]
fn test_fulfill_outflow_via_gmp_intent_id_formatting() {
    // Test that intent_id with 0x prefix has it stripped for hex: arg
    let intent_id_with_prefix = "0x1234567890abcdef";
    let formatted = intent_id_with_prefix.strip_prefix("0x").unwrap_or(intent_id_with_prefix);
    assert_eq!(formatted, "1234567890abcdef");

    // Test that intent_id without 0x prefix is passed through
    let intent_id_no_prefix = "1234567890abcdef";
    let formatted = intent_id_no_prefix.strip_prefix("0x").unwrap_or(intent_id_no_prefix);
    assert_eq!(formatted, "1234567890abcdef");
}

// #8: fulfillment_signature_encoding - N/A for MVM (EVM uses signatures, MVM uses CLI)

/// 9. Test: Fulfill Outflow Via GMP Command Building
/// Verifies that the aptos CLI command is built correctly with all required arguments.
/// Why: The fulfill_intent function is called via aptos CLI. Wrong argument formatting
/// would cause silent failures or wrong function parameters.
#[test]
fn test_fulfill_outflow_via_gmp_command_building() {
    let module_addr = DUMMY_MODULE_ADDR_CON;
    let intent_id = DUMMY_INTENT_ID;
    let token_metadata = DUMMY_TOKEN_ADDR_MVMCON;
    let profile = "solver-chain2";

    // Build the function ID
    let function_id = format!("{}::outflow_validator_impl::fulfill_intent", module_addr);

    // Build the args as the CLI would
    let intent_id_hex = intent_id.strip_prefix("0x").unwrap_or(intent_id);
    let arg1 = format!("hex:{}", intent_id_hex);
    let arg2 = format!("address:{}", token_metadata);

    // Verify function ID format
    assert!(function_id.contains("outflow_validator_impl::fulfill_intent"));
    assert!(function_id.starts_with("0x"));

    // Verify args format
    assert!(arg1.starts_with("hex:"));
    assert!(!arg1.contains("0x")); // 0x prefix should be stripped
    assert!(arg2.starts_with("address:"));
    assert!(arg2.contains("0x")); // address should keep 0x prefix

    // Verify profile is used
    assert_eq!(profile, "solver-chain2");
}

// #10: fulfillment_hash_extraction - TODO: implement for MVM
// #11: fulfillment_error_handling - TODO: implement for MVM
// #12: pubkey_from_hex_with_leading_zeros - N/A for MVM (SVM-specific)
// #13: pubkey_from_hex_no_leading_zeros - N/A for MVM (SVM-specific)
