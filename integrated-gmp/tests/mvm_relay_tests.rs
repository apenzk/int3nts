//! MVM counterpart of *vm_relay_tests.rs.
//!
//! SVM-specific tests (pubkey parsing, message parsing, fulfillment proof, ATA derivation)
//! are N/A for MVM. Relay config tests apply across VMs.

mod helpers;

use helpers::{build_test_config_with_mvm, build_test_config_with_svm};
use integrated_gmp::integrated_gmp_relay::NativeGmpRelayConfig;

// ============================================================================
// SVM PUBKEY PARSING TESTS
// ============================================================================

// 1. Test: Parse SVM Pubkey From Hex Format
// NOTE: N/A for MVM - Solana pubkey parsing is SVM-specific

// 2. Test: Parse SVM Pubkey From Base58 Format
// NOTE: N/A for MVM - Solana pubkey parsing is SVM-specific

// ============================================================================
// SVM MESSAGE PARSING TESTS
// ============================================================================

// 3. Test: Parse SVM MessageSent Log Format
// NOTE: N/A for MVM - SVM log format parsing is SVM-specific

// 4. Test: Non-MessageSent Log Is Ignored
// NOTE: N/A for MVM - SVM log format parsing is SVM-specific

// 5. Test: Solana Pubkey To Hex Conversion
// NOTE: N/A for MVM - Solana pubkey conversion is SVM-specific

// ============================================================================
// RELAY CONFIG TESTS
// ============================================================================

// 6. Test: Relay Config Extracts MVM Connected Chain
/// Verifies that NativeGmpRelayConfig correctly extracts MVM connected chain fields from config.
/// Why: The relay needs to route messages to connected MVM chains.
#[test]
fn test_relay_config_extracts_mvm_connected_chain() {
    let config = build_test_config_with_mvm();

    let relay_config = NativeGmpRelayConfig::from_config(&config).unwrap();

    // Hub chain should be extracted
    assert_eq!(
        relay_config.mvm_chain_id, 1,
        "Hub chain ID should be extracted"
    );
    assert_eq!(
        relay_config.mvm_rpc_url, "http://127.0.0.1:18080",
        "Hub RPC URL should be extracted"
    );
    assert_eq!(
        relay_config.mvm_module_addr, "0x1",
        "Hub module address should be extracted"
    );

    // MVM connected chain should be extracted
    assert_eq!(relay_config.mvm_chains.len(), 1, "Should have one MVM connected chain");
    assert_eq!(
        relay_config.mvm_chains[0].chain_id, 2,
        "MVM connected chain ID should be extracted"
    );
    assert_eq!(
        relay_config.mvm_chains[0].rpc_url, "http://127.0.0.1:18082",
        "MVM connected RPC URL should be extracted"
    );
    assert_eq!(
        relay_config.mvm_chains[0].module_addr, "0x2",
        "MVM connected module address should be extracted"
    );
}

// 7. Test: Relay Config Extracts Both Connected Chains
/// Verifies that NativeGmpRelayConfig correctly extracts both MVM connected and SVM chain fields simultaneously.
/// Why: Relay may need to route to both MVM and SVM connected chains.
#[test]
fn test_relay_config_extracts_both_connected_chains() {
    let config = build_test_config_with_svm();

    let relay_config = NativeGmpRelayConfig::from_config(&config).unwrap();

    assert!(
        !relay_config.mvm_chains.is_empty(),
        "MVM connected should be present"
    );
    assert!(
        !relay_config.svm_chains.is_empty(),
        "SVM should be present"
    );

    assert_ne!(
        relay_config.mvm_chains[0].chain_id,
        relay_config.svm_chains[0].chain_id,
        "MVM connected and SVM should have different chain IDs"
    );
}

// 8. Test: Relay Config Handles Missing MVM Connected Chain
// NOTE: N/A for MVM - tests SVM config when MVM is absent

// 9. Test: Relay Config Extracts EVM Connected Chain
// NOTE: N/A for MVM - EVM connected chain extraction is EVM-specific

// ============================================================================
// FULFILLMENT PROOF PAYLOAD PARSING TESTS
// ============================================================================

// 10. Test: FulfillmentProof Payload Intent ID Extraction
// NOTE: N/A for MVM - FulfillmentProof payload parsing uses SVM byte offsets and PDA derivation

// 11. Test: FulfillmentProof Payload Minimum Length
// NOTE: N/A for MVM - FulfillmentProof payload length check is SVM-specific

// ============================================================================
// ATA DERIVATION TESTS
// ============================================================================

// 12. Test: ATA Derivation Formula
// NOTE: N/A for MVM - Associated Token Accounts are a Solana concept

// 13. Test: ATA Derivation Is Deterministic
// NOTE: N/A for MVM - Associated Token Accounts are a Solana concept

// 14. Test: ATA Differs By Owner
// NOTE: N/A for MVM - Associated Token Accounts are a Solana concept

// ============================================================================
// EVM EVENT TOPIC TESTS
// ============================================================================

// 15. Test: EVM Event Topic Produces Known Keccak Hash
// NOTE: N/A for MVM - EVM event topic hashing is EVM-specific

// 16. Test: EVM Event Topic Is Deterministic
// NOTE: N/A for MVM - EVM event topic hashing is EVM-specific

// ============================================================================
// EVM ABI ENCODING TESTS
// ============================================================================

// 17. Test: EVM Encode deliverMessage Calldata
// NOTE: N/A for MVM - EVM ABI encoding is EVM-specific

// 18. Test: EVM Encode deliverMessage With Empty Payload
// NOTE: N/A for MVM - EVM ABI encoding is EVM-specific

// ============================================================================
// EVM LOG PARSING TESTS
// ============================================================================

// 19. Test: Parse EVM MessageSent Log
// NOTE: N/A for MVM - EVM log parsing is EVM-specific

// 20. Test: EVM MessageSent Log Short Data Ignored
// NOTE: N/A for MVM - EVM log parsing is EVM-specific

// 21. Test: EVM MessageSent Log Missing Topics Ignored
// NOTE: N/A for MVM - EVM log parsing is EVM-specific

// ============================================================================
// RLP ENCODING TESTS
// ============================================================================

// 22. Test: RLP Encode u64 Known Values
// NOTE: N/A for MVM - RLP encoding is EVM-specific

// 23. Test: RLP Encode Item Short String
// NOTE: N/A for MVM - RLP encoding is EVM-specific

// 24. Test: RLP Encode List Basic
// NOTE: N/A for MVM - RLP encoding is EVM-specific

// ============================================================================
// MVM OUTBOX MESSAGE PARSING TESTS
// ============================================================================

// 25. TODO test_mvm_get_message_response_parsing — not yet implemented for MVM
// 26. TODO test_mvm_get_next_nonce_response_parsing — not yet implemented for MVM

// ============================================================================
// SVM ACCOUNT DATA PARSING TESTS
// ============================================================================

// 27. Test: SVM Outbound Nonce Account Layout
// NOTE: N/A for MVM - SVM account data parsing is SVM-specific

// 28. Test: SVM Outbound Nonce Account Too Short
// NOTE: N/A for MVM - SVM account data parsing is SVM-specific

// 29. Test: SVM Message Account Field Extraction
// NOTE: N/A for MVM - SVM account data parsing is SVM-specific

// 30. Test: SVM Message Account Discriminator Check
// NOTE: N/A for MVM - SVM account data parsing is SVM-specific

// 31. Test: SVM Message Account Payload Truncation
// NOTE: N/A for MVM - SVM account data parsing is SVM-specific
