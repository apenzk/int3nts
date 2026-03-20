//! Hub-only MVM client tests
//!
//! These tests query the MVM hub chain specifically (solver registry, public keys,
//! registration, outflow requirements). The hub is always MVM — no VM symmetry applies.
//! NOT tracked in the extension checklist.
//!
//! Consolidated from coordinator/tests/mvm_client_tests.rs and
//! integrated-gmp/tests/mvm_client_tests.rs to eliminate duplicate tests.

use chain_clients_mvm::MvmClient;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// CONSTANTS
// ============================================================================

const DUMMY_SOLVER_ADDR_HUB: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000007";
const DUMMY_SOLVER_ADDR_MVMCON: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000008";
const DUMMY_SOLVER_ADDR_EVM: &str = "0x0000000000000000000000000000000000000009";
const DUMMY_SOLVER_ADDR_SVM: &str =
    "0x000000000000000000000000000000000000000000000000000000000000000b";
const DUMMY_SOLVER_REGISTRY_ADDR: &str = "0x1";
const DUMMY_PUBLIC_KEY: [u8; 4] = [1, 2, 3, 4];
const DUMMY_REGISTERED_AT: u64 = 1234567890;
const DUMMY_MODULE_ADDR: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000011";
const DUMMY_INTENT_ID: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000001";

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a mock SolverRegistry resource with MVM address (string format)
fn create_solver_registry_resource_with_mvm_address(
    solver_registry_addr: &str,
    solver_addr: &str,
    solver_connected_chain_mvm_addr: Option<&str>,
) -> serde_json::Value {
    let mvm_addr_vec = match solver_connected_chain_mvm_addr {
        Some(addr) => json!([addr]),
        None => json!([]),
    };

    json!([{
        "type": format!("{}::solver_registry::SolverRegistry", solver_registry_addr),
        "data": {
            "solvers": {
                "data": [{
                    "key": solver_addr,
                    "value": {
                        "public_key": DUMMY_PUBLIC_KEY,
                        "connected_chain_mvm_addr": {"vec": mvm_addr_vec},
                        "connected_chain_evm_addr": {"vec": []},
                        "connected_chain_svm_addr": {"vec": []},
                        "registered_at": DUMMY_REGISTERED_AT
                    }
                }]
            }
        }
    }])
}

/// Create a mock SolverRegistry resource with EVM address (array format)
fn create_solver_registry_resource_with_evm_address_array(
    solver_registry_addr: &str,
    solver_addr: &str,
    solver_connected_chain_evm_addr: Option<&str>,
) -> serde_json::Value {
    let evm_addr_vec = match solver_connected_chain_evm_addr {
        Some(evm_addr) => {
            let addr_clean = evm_addr.strip_prefix("0x").unwrap_or(evm_addr);
            let bytes: Vec<u64> = (0..addr_clean.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&addr_clean[i..i + 2], 16).unwrap() as u64)
                .collect();
            json!([bytes])
        }
        None => json!([]),
    };

    json!([{
        "type": format!("{}::solver_registry::SolverRegistry", solver_registry_addr),
        "data": {
            "solvers": {
                "data": [{
                    "key": solver_addr,
                    "value": {
                        "public_key": DUMMY_PUBLIC_KEY,
                        "connected_chain_mvm_addr": {"vec": []},
                        "connected_chain_evm_addr": {"vec": evm_addr_vec},
                        "connected_chain_svm_addr": {"vec": []},
                        "registered_at": DUMMY_REGISTERED_AT
                    }
                }]
            }
        }
    }])
}

/// Create a mock SolverRegistry resource with EVM address (hex string format)
fn create_solver_registry_resource_with_evm_address_hex_string(
    solver_registry_addr: &str,
    solver_addr: &str,
    solver_connected_chain_evm_addr: Option<&str>,
) -> serde_json::Value {
    let evm_addr_vec = match solver_connected_chain_evm_addr {
        Some(evm_addr) => json!([evm_addr]),
        None => json!([]),
    };

    json!([{
        "type": format!("{}::solver_registry::SolverRegistry", solver_registry_addr),
        "data": {
            "solvers": {
                "data": [{
                    "key": solver_addr,
                    "value": {
                        "public_key": DUMMY_PUBLIC_KEY,
                        "connected_chain_mvm_addr": {"vec": []},
                        "connected_chain_evm_addr": {"vec": evm_addr_vec},
                        "connected_chain_svm_addr": {"vec": []},
                        "registered_at": DUMMY_REGISTERED_AT
                    }
                }]
            }
        }
    }])
}

/// Create a mock SolverRegistry resource with SVM address (array format)
fn create_solver_registry_resource_with_svm_address_array(
    solver_registry_addr: &str,
    solver_addr: &str,
    solver_connected_chain_svm_addr: Option<&str>,
) -> serde_json::Value {
    let svm_addr_vec = match solver_connected_chain_svm_addr {
        Some(svm_addr) => {
            let addr_clean = svm_addr.strip_prefix("0x").unwrap_or(svm_addr);
            let bytes: Vec<u64> = (0..addr_clean.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&addr_clean[i..i + 2], 16).unwrap() as u64)
                .collect();
            json!([bytes])
        }
        None => json!([]),
    };

    json!([{
        "type": format!("{}::solver_registry::SolverRegistry", solver_registry_addr),
        "data": {
            "solvers": {
                "data": [{
                    "key": solver_addr,
                    "value": {
                        "public_key": DUMMY_PUBLIC_KEY,
                        "connected_chain_mvm_addr": {"vec": []},
                        "connected_chain_evm_addr": {"vec": []},
                        "connected_chain_svm_addr": {"vec": svm_addr_vec},
                        "registered_at": DUMMY_REGISTERED_AT
                    }
                }]
            }
        }
    }])
}

/// Create a mock SolverRegistry resource with SVM address (hex string format)
fn create_solver_registry_resource_with_svm_address_hex_string(
    solver_registry_addr: &str,
    solver_addr: &str,
    solver_connected_chain_svm_addr: Option<&str>,
) -> serde_json::Value {
    let svm_addr_vec = match solver_connected_chain_svm_addr {
        Some(svm_addr) => json!([svm_addr]),
        None => json!([]),
    };

    json!([{
        "type": format!("{}::solver_registry::SolverRegistry", solver_registry_addr),
        "data": {
            "solvers": {
                "data": [{
                    "key": solver_addr,
                    "value": {
                        "public_key": DUMMY_PUBLIC_KEY,
                        "connected_chain_mvm_addr": {"vec": []},
                        "connected_chain_evm_addr": {"vec": []},
                        "connected_chain_svm_addr": {"vec": svm_addr_vec},
                        "registered_at": DUMMY_REGISTERED_AT
                    }
                }]
            }
        }
    }])
}

/// Helper to create a leading-zero solver entry with specified chain address field
fn create_leading_zero_resource(
    solver_registry_addr_in_type: &str,
    solver_addr: &str,
    chain_field: &str,
    chain_value: serde_json::Value,
) -> serde_json::Value {
    let mut value = json!({
        "public_key": DUMMY_PUBLIC_KEY,
        "connected_chain_mvm_addr": {"vec": []},
        "connected_chain_evm_addr": {"vec": []},
        "connected_chain_svm_addr": {"vec": []},
        "registered_at": DUMMY_REGISTERED_AT
    });
    value[chain_field] = json!({"vec": chain_value});

    json!([{
        "type": format!("{}::solver_registry::SolverRegistry", solver_registry_addr_in_type),
        "data": {
            "solvers": {
                "data": [{"key": solver_addr, "value": value}]
            }
        }
    }])
}

/// Setup a mock server that responds to get_public_key view function calls
async fn setup_mock_server_with_public_key(
    public_key: Option<&[u8]>,
) -> (MockServer, MvmClient) {
    let mock_server = MockServer::start().await;

    let view_response: Vec<serde_json::Value> = if let Some(pk) = public_key {
        vec![json!(format!("0x{}", hex::encode(pk)))]
    } else {
        vec![json!("0x")]
    };

    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(view_response))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    (mock_server, client)
}

/// Setup a mock server for resources endpoint
async fn setup_mock_server_with_resources(
    solver_registry_addr: &str,
    resources_response: serde_json::Value,
) -> (MockServer, MvmClient) {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path(format!(
            "/v1/accounts/{}/resources",
            solver_registry_addr
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(resources_response))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    (mock_server, client)
}

// ============================================================================
// SOLVER REGISTRY LOOKUP — MVM ADDRESS
// ============================================================================

/// 1. Test: get_solver_mvm_address returns address when solver is registered
/// Verifies: Successful MVM address lookup from solver registry.
/// Why: Primary happy path for connected chain address resolution.
#[tokio::test]
async fn test_get_solver_mvm_addr_success() {
    let resources = create_solver_registry_resource_with_mvm_address(
        DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SOLVER_ADDR_HUB, Some(DUMMY_SOLVER_ADDR_MVMCON),
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    let result = client
        .get_solver_mvm_address(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_MVMCON.to_string()));
}

/// 2. Test: get_solver_mvm_address returns None when address not set
/// Verifies: Correct handling when solver has no connected chain MVM address.
/// Why: Prevents false positives for uninitialized address fields.
#[tokio::test]
async fn test_get_solver_mvm_addr_none() {
    let resources = create_solver_registry_resource_with_mvm_address(
        DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SOLVER_ADDR_HUB, None,
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    let result = client
        .get_solver_mvm_address(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, None);
}

/// 3. Test: get_solver_mvm_address returns None for unregistered solver
/// Verifies: Correct handling of unregistered solvers.
/// Why: Prevents incorrect address lookups for unknown solvers.
#[tokio::test]
async fn test_get_solver_mvm_addr_solver_not_found() {
    let resources = create_solver_registry_resource_with_mvm_address(
        DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SOLVER_ADDR_HUB, Some(DUMMY_SOLVER_ADDR_MVMCON),
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    let result = client
        .get_solver_mvm_address("0xunregistered_solver_addr", DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, None);
}

/// 4. Test: get_solver_mvm_address returns None when registry missing
/// Verifies: Correct handling when SolverRegistry resource doesn't exist.
/// Why: Prevents panics on uninitialized state.
#[tokio::test]
async fn test_get_solver_mvm_addr_registry_not_found() {
    let (_s, client) =
        setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, json!([])).await;

    let result = client
        .get_solver_mvm_address(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, None);
}

/// 5. Test: get_solver_mvm_address handles address normalization
/// Verifies: Address matching works with/without 0x prefix.
/// Why: Prevents lookup failures from inconsistent formatting.
#[tokio::test]
async fn test_get_solver_mvm_addr_address_normalization() {
    let resources = create_solver_registry_resource_with_mvm_address(
        DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SOLVER_ADDR_HUB, Some(DUMMY_SOLVER_ADDR_MVMCON),
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    // Query with address without 0x prefix
    let solver_addr_without_prefix = &DUMMY_SOLVER_ADDR_HUB[2..];
    let result = client
        .get_solver_mvm_address(solver_addr_without_prefix, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_MVMCON.to_string()));
}

// ============================================================================
// SOLVER REGISTRY LOOKUP — EVM ADDRESS
// ============================================================================

/// 6. Test: get_solver_evm_address parses array format
/// Verifies: Aptos Option<vector<u8>> as {"vec": [[bytes_array]]} is parsed correctly.
/// Why: Aptos serialization format varies; array format is the most common.
#[tokio::test]
async fn test_get_solver_evm_address_array_format() {
    let resources = create_solver_registry_resource_with_evm_address_array(
        DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SOLVER_ADDR_HUB, Some(DUMMY_SOLVER_ADDR_EVM),
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    let result = client
        .get_solver_evm_address(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_EVM.to_string()));
}

/// 7. Test: get_solver_evm_address parses hex string format
/// Verifies: Aptos Option<vector<u8>> as {"vec": ["0xhexstring"]} is parsed correctly.
/// Why: This format caused EVM outflow validation failures in production.
#[tokio::test]
async fn test_get_solver_evm_address_hex_string_format() {
    let resources = create_solver_registry_resource_with_evm_address_hex_string(
        DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SOLVER_ADDR_HUB, Some(DUMMY_SOLVER_ADDR_EVM),
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    let result = client
        .get_solver_evm_address(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_EVM.to_string()));
}

/// 8. Test: get_solver_mvm_address handles leading zero mismatch
/// Verifies: Registry found despite Move stripping leading zeros from type names.
/// Why: Move strips leading zeros from addresses in type names but API uses full address.
#[tokio::test]
async fn test_get_solver_mvm_address_leading_zero_mismatch() {
    let addr_full = "0x0123456789012345678901234567890123456789012345678901234567890123";
    let addr_stripped = "0x123456789012345678901234567890123456789012345678901234567890123";

    let resources = create_leading_zero_resource(
        addr_stripped, DUMMY_SOLVER_ADDR_HUB,
        "connected_chain_mvm_addr", json!([DUMMY_SOLVER_ADDR_MVMCON]),
    );
    let (_s, client) = setup_mock_server_with_resources(addr_full, resources).await;

    let result = client
        .get_solver_mvm_address(DUMMY_SOLVER_ADDR_HUB, addr_full)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_MVMCON.to_string()));
}

/// 9. Test: get_solver_evm_address handles leading zero mismatch
/// Verifies: Registry found despite Move stripping leading zeros from type names.
/// Why: Same issue as MVM but for EVM address lookup.
#[tokio::test]
async fn test_get_solver_evm_address_leading_zero_mismatch() {
    let addr_full = "0x0123456789012345678901234567890123456789012345678901234567890123";
    let addr_stripped = "0x123456789012345678901234567890123456789012345678901234567890123";

    let resources = create_leading_zero_resource(
        addr_stripped, DUMMY_SOLVER_ADDR_HUB,
        "connected_chain_evm_addr", json!([DUMMY_SOLVER_ADDR_EVM]),
    );
    let (_s, client) = setup_mock_server_with_resources(addr_full, resources).await;

    let result = client
        .get_solver_evm_address(DUMMY_SOLVER_ADDR_HUB, addr_full)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_EVM.to_string()));
}

/// 10. Test: get_solver_evm_address handles leading zero stripped from registry key
/// Verifies: Lookup succeeds when the on-chain registry key has leading zeros stripped.
/// Why: Move/Aptos may strip leading zeros from addresses stored as map keys, causing
///      a 63-char key to not match a 64-char lookup address.
#[tokio::test]
async fn test_get_solver_evm_address_leading_zero_key_mismatch() {
    let addr_full = "0x0123456789012345678901234567890123456789012345678901234567890123";
    let addr_key_stripped = "0x123456789012345678901234567890123456789012345678901234567890123";

    let resources = create_leading_zero_resource(
        DUMMY_SOLVER_REGISTRY_ADDR, addr_key_stripped,
        "connected_chain_evm_addr", json!([DUMMY_SOLVER_ADDR_EVM]),
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    let result = client
        .get_solver_evm_address(addr_full, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_EVM.to_string()));
}

// ============================================================================
// SOLVER REGISTRY LOOKUP — SVM ADDRESS
// ============================================================================

/// 11. Test: get_solver_svm_address parses array format
/// Verifies: 32-byte SVM address correctly parsed from array format.
/// Why: SVM addresses are 32 bytes (Solana public key) vs 20 bytes for EVM.
#[tokio::test]
async fn test_get_solver_svm_address_array_format() {
    let resources = create_solver_registry_resource_with_svm_address_array(
        DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SOLVER_ADDR_HUB, Some(DUMMY_SOLVER_ADDR_SVM),
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    let result = client
        .get_solver_svm_address(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_SVM.to_string()));
}

/// 12. Test: get_solver_svm_address parses hex string format
/// Verifies: 32-byte SVM address correctly parsed from hex string format.
/// Why: Aptos can serialize addresses as hex strings instead of byte arrays.
#[tokio::test]
async fn test_get_solver_svm_address_hex_string_format() {
    let resources = create_solver_registry_resource_with_svm_address_hex_string(
        DUMMY_SOLVER_REGISTRY_ADDR, DUMMY_SOLVER_ADDR_HUB, Some(DUMMY_SOLVER_ADDR_SVM),
    );
    let (_s, client) = setup_mock_server_with_resources(DUMMY_SOLVER_REGISTRY_ADDR, resources).await;

    let result = client
        .get_solver_svm_address(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_SVM.to_string()));
}

/// 13. Test: get_solver_svm_address handles leading zero mismatch
/// Verifies: Registry found despite Move stripping leading zeros from type names.
/// Why: Same issue as MVM/EVM but for SVM address lookup.
#[tokio::test]
async fn test_get_solver_svm_address_leading_zero_mismatch() {
    let addr_full = "0x0123456789012345678901234567890123456789012345678901234567890123";
    let addr_stripped = "0x123456789012345678901234567890123456789012345678901234567890123";

    let resources = create_leading_zero_resource(
        addr_stripped, DUMMY_SOLVER_ADDR_HUB,
        "connected_chain_svm_addr", json!([DUMMY_SOLVER_ADDR_SVM]),
    );
    let (_s, client) = setup_mock_server_with_resources(addr_full, resources).await;

    let result = client
        .get_solver_svm_address(DUMMY_SOLVER_ADDR_HUB, addr_full)
        .await
        .unwrap();
    assert_eq!(result, Some(DUMMY_SOLVER_ADDR_SVM.to_string()));
}

// ============================================================================
// SOLVER PUBLIC KEY
// ============================================================================

/// 14. Test: get_solver_public_key returns key when registered
/// Verifies: Successful public key retrieval from registry.
/// Why: Signature submission requires verifying solver is registered.
#[tokio::test]
async fn test_get_solver_public_key_success() {
    let public_key = vec![1u8, 2, 3, 4, 5];
    let (_s, client) = setup_mock_server_with_public_key(Some(&public_key)).await;

    let result = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, Some(public_key));
}

/// 15. Test: get_solver_public_key returns None when not registered
/// Verifies: Unregistered solver returns None.
/// Why: Unregistered solvers must be rejected.
#[tokio::test]
async fn test_get_solver_public_key_not_registered() {
    let (_s, client) = setup_mock_server_with_public_key(None).await;

    let result = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, None);
}

/// 16. Test: get_solver_public_key handles empty hex string
/// Verifies: Empty hex ("0x") treated as not registered.
/// Why: Aptos returns "0x" for empty vector<u8>.
#[tokio::test]
async fn test_get_solver_public_key_empty_hex_string() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(["0x"])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert_eq!(result, None);
}

/// 17. Test: get_solver_public_key errors on unexpected format
/// Verifies: Non-array response results in error.
/// Why: Unexpected formats must fail loudly.
#[tokio::test]
async fn test_get_solver_public_key_errors_on_unexpected_format() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"unexpected": "format"})))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expected array"));
}

/// 18. Test: get_solver_public_key handles 32-byte Ed25519 key
/// Verifies: Real-world Ed25519 32-byte public key format.
/// Why: Ed25519 keys are exactly 32 bytes.
#[tokio::test]
async fn test_get_solver_public_key_ed25519_format() {
    let public_key: Vec<u8> = (0..32).collect();
    let (_s, client) = setup_mock_server_with_public_key(Some(&public_key)).await;

    let pk = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pk.len(), 32);
    assert_eq!(pk, public_key);
}

/// 19. Test: get_solver_public_key errors on empty array
/// Verifies: Empty array response results in error.
/// Why: View function must return at least one element.
#[tokio::test]
async fn test_get_solver_public_key_errors_on_empty_array() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Empty response array"));
}

/// 20. Test: get_solver_public_key errors on non-string element
/// Verifies: Non-string element in array results in error.
/// Why: Aptos returns hex strings, not raw numbers.
#[tokio::test]
async fn test_get_solver_public_key_errors_on_non_string_element() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([12345])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expected hex string"));
}

/// 21. Test: get_solver_public_key errors on invalid hex
/// Verifies: Invalid hex characters result in error.
/// Why: Hex decode must fail on invalid characters.
#[tokio::test]
async fn test_get_solver_public_key_errors_on_invalid_hex() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!(["0xZZZZinvalidhex"])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to decode hex"));
}

/// 22. Test: get_solver_public_key errors on HTTP error
/// Verifies: HTTP errors are propagated.
/// Why: Network errors must be surfaced, not silently ignored.
#[tokio::test]
async fn test_get_solver_public_key_errors_on_http_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .get_solver_public_key(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Failed to query solver public key"));
}

/// 23. Test: get_solver_public_key rejects address without 0x prefix
/// Verifies: Address validation rejects malformed addresses.
/// Why: Missing 0x prefix indicates a bug in calling code.
#[tokio::test]
async fn test_get_solver_public_key_rejects_address_without_prefix() {
    let mock_server = MockServer::start().await;
    let client = MvmClient::new(&mock_server.uri()).unwrap();

    let solver_addr_no_prefix = &DUMMY_SOLVER_ADDR_HUB[2..];
    let result = client
        .get_solver_public_key(solver_addr_no_prefix, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("must start with 0x prefix"));
}

// ============================================================================
// SOLVER REGISTRATION CHECK
// ============================================================================

/// 24. Test: is_solver_registered returns true for registered solver
/// Verifies: View function call and boolean response parsing for registered solver.
/// Why: Solver needs to verify registration before fulfillment attempts.
#[tokio::test]
async fn test_is_solver_registered_true() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([true])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert!(result);
}

/// 25. Test: is_solver_registered returns false for unregistered solver
/// Verifies: False response correctly parsed.
/// Why: Unregistered solvers must be rejected to avoid wasting gas.
#[tokio::test]
async fn test_is_solver_registered_false() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([false])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert!(!result);
}

/// 26. Test: is_solver_registered handles address normalization
/// Verifies: Addresses with/without 0x prefix both work.
/// Why: Address format shouldn't affect registration checks.
#[tokio::test]
async fn test_is_solver_registered_address_normalization() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([true])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();

    // With 0x prefix
    let result1 = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert!(result1);

    // Without 0x prefix
    let result2 = client
        .is_solver_registered(&DUMMY_SOLVER_ADDR_HUB[2..], DUMMY_SOLVER_REGISTRY_ADDR)
        .await
        .unwrap();
    assert!(result2);
}

/// 27. Test: is_solver_registered propagates HTTP errors
/// Verifies: HTTP errors are not swallowed.
/// Why: Network errors must propagate so the caller can retry or alert.
#[tokio::test]
async fn test_is_solver_registered_http_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to query solver registration"));
}

/// 28. Test: is_solver_registered errors on invalid JSON
/// Verifies: Malformed responses result in errors, not panics.
/// Why: Invalid JSON must fail clearly.
#[tokio::test]
async fn test_is_solver_registered_invalid_json() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
}

/// 29. Test: is_solver_registered errors on unexpected format
/// Verifies: Empty array or wrong type results in error.
/// Why: Unexpected formats must fail loudly.
#[tokio::test]
async fn test_is_solver_registered_unexpected_format() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .is_solver_registered(DUMMY_SOLVER_ADDR_HUB, DUMMY_SOLVER_REGISTRY_ADDR)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unexpected response format"));
}

// ============================================================================
// OUTFLOW REQUIREMENTS
// ============================================================================

/// 30. Test: has_outflow_requirements returns true when requirements delivered
/// Verifies: View function call and boolean response parsing.
/// Why: The solver polls this before calling fulfill_intent.
#[tokio::test]
async fn test_has_outflow_requirements_success() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([true])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .has_outflow_requirements(DUMMY_INTENT_ID, DUMMY_MODULE_ADDR)
        .await
        .unwrap();
    assert!(result);
}

/// 31. Test: has_outflow_requirements returns false when not delivered
/// Verifies: False response correctly parsed.
/// Why: The solver polls this repeatedly; false must not be misinterpreted.
#[tokio::test]
async fn test_has_outflow_requirements_false() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([false])))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .has_outflow_requirements(DUMMY_INTENT_ID, DUMMY_MODULE_ADDR)
        .await
        .unwrap();
    assert!(!result);
}

/// 32. Test: has_outflow_requirements propagates HTTP errors
/// Verifies: HTTP errors are not swallowed.
/// Why: Errors must propagate so the caller can fail fast.
#[tokio::test]
async fn test_has_outflow_requirements_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/view"))
        .respond_with(ResponseTemplate::new(400).set_body_string(
            r#"{"message":"Odd number of digits","error_code":"invalid_input"}"#,
        ))
        .mount(&mock_server)
        .await;

    let client = MvmClient::new(&mock_server.uri()).unwrap();
    let result = client
        .has_outflow_requirements(DUMMY_INTENT_ID, DUMMY_MODULE_ADDR)
        .await;
    assert!(result.is_err());
}
