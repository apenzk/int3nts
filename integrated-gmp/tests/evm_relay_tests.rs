//! EVM counterpart of *vm_relay_tests.rs.
//!
//! SVM-specific tests (pubkey parsing, message parsing, fulfillment proof, ATA derivation)
//! are N/A for EVM. Relay config tests apply across VMs.

mod helpers;

use helpers::build_test_config_with_evm;
use integrated_gmp::integrated_gmp_relay::NativeGmpRelayConfig;

// ============================================================================
// SVM PUBKEY PARSING TESTS
// ============================================================================

// 1. Test: Parse SVM Pubkey From Hex Format
// NOTE: N/A for EVM - Solana pubkey parsing is SVM-specific

// 2. Test: Parse SVM Pubkey From Base58 Format
// NOTE: N/A for EVM - Solana pubkey parsing is SVM-specific

// ============================================================================
// SVM MESSAGE PARSING TESTS
// ============================================================================

// 3. Test: Parse SVM MessageSent Log Format
// NOTE: N/A for EVM - SVM log format parsing is SVM-specific

// 4. Test: Non-MessageSent Log Is Ignored
// NOTE: N/A for EVM - SVM log format parsing is SVM-specific

// 5. Test: Solana Pubkey To Hex Conversion
// NOTE: N/A for EVM - Solana pubkey conversion is SVM-specific

// ============================================================================
// RELAY CONFIG TESTS
// ============================================================================

// 6. Test: Relay Config Extracts MVM Connected Chain
// NOTE: N/A for EVM - MVM connected chain extraction is MVM-specific

// 7. TODO test_relay_config_extracts_both_connected_chains — not yet implemented for EVM

// 8. Test: Relay Config Handles Missing MVM Connected Chain
// NOTE: N/A for EVM - tests SVM config when MVM is absent

// 9. Test: Relay Config Extracts EVM Connected Chain
/// Verifies that NativeGmpRelayConfig correctly extracts EVM connected chain fields from config.
/// Why: The relay needs to route messages to connected EVM chains.
#[test]
fn test_relay_config_extracts_evm_connected_chain() {
    let config = build_test_config_with_evm();

    let relay_config = NativeGmpRelayConfig::from_config(&config).unwrap();

    assert_eq!(relay_config.evm_chains.len(), 1, "Should have one EVM connected chain");
    assert_eq!(
        relay_config.evm_chains[0].chain_id, 31337,
        "EVM connected chain ID should be extracted"
    );
    assert_eq!(
        relay_config.evm_chains[0].rpc_url, "http://127.0.0.1:8545",
        "EVM connected RPC URL should be extracted"
    );
}

// ============================================================================
// FULFILLMENT PROOF PAYLOAD PARSING TESTS
// ============================================================================

// 10. Test: FulfillmentProof Payload Intent ID Extraction
// NOTE: N/A for EVM - FulfillmentProof payload parsing uses SVM byte offsets and PDA derivation

// 11. Test: FulfillmentProof Payload Minimum Length
// NOTE: N/A for EVM - FulfillmentProof payload length check is SVM-specific

// ============================================================================
// ATA DERIVATION TESTS
// ============================================================================

// 12. Test: ATA Derivation Formula
// NOTE: N/A for EVM - Associated Token Accounts are a Solana concept

// 13. Test: ATA Derivation Is Deterministic
// NOTE: N/A for EVM - Associated Token Accounts are a Solana concept

// 14. Test: ATA Differs By Owner
// NOTE: N/A for EVM - Associated Token Accounts are a Solana concept

// ============================================================================
// EVM EVENT TOPIC TESTS
// ============================================================================

// 15. TODO test_evm_event_topic_produces_known_keccak_hash — not yet implemented for EVM
// 16. TODO test_evm_event_topic_is_deterministic — not yet implemented for EVM

// ============================================================================
// EVM ABI ENCODING TESTS
// ============================================================================

// 17. TODO test_evm_encode_deliver_message_calldata — not yet implemented for EVM
// 18. TODO test_evm_encode_deliver_message_with_empty_payload — not yet implemented for EVM

// ============================================================================
// EVM LOG PARSING TESTS
// ============================================================================

// 19. TODO test_parse_evm_message_sent_log — not yet implemented for EVM
// 20. TODO test_evm_message_sent_log_short_data_ignored — not yet implemented for EVM
// 21. TODO test_evm_message_sent_log_missing_topics_ignored — not yet implemented for EVM

// ============================================================================
// RLP ENCODING TESTS
// ============================================================================

// 22. TODO test_rlp_encode_u64_known_values — not yet implemented for EVM
// 23. TODO test_rlp_encode_item_short_string — not yet implemented for EVM
// 24. TODO test_rlp_encode_list_basic — not yet implemented for EVM

// ============================================================================
// MVM OUTBOX MESSAGE PARSING TESTS
// ============================================================================

// 25. Test: MVM Get Message Response Parsing
// NOTE: N/A for EVM - MVM outbox message parsing is MVM-specific

// 26. Test: MVM Get Next Nonce Response Parsing
// NOTE: N/A for EVM - MVM outbox nonce parsing is MVM-specific

// ============================================================================
// SVM ACCOUNT DATA PARSING TESTS
// ============================================================================

// 27. Test: SVM Outbound Nonce Account Layout
// NOTE: N/A for EVM - SVM account data parsing is SVM-specific

// 28. Test: SVM Outbound Nonce Account Too Short
// NOTE: N/A for EVM - SVM account data parsing is SVM-specific

// 29. Test: SVM Message Account Field Extraction
// NOTE: N/A for EVM - SVM account data parsing is SVM-specific

// 30. Test: SVM Message Account Discriminator Check
// NOTE: N/A for EVM - SVM account data parsing is SVM-specific

// 31. Test: SVM Message Account Payload Truncation
// NOTE: N/A for EVM - SVM account data parsing is SVM-specific
