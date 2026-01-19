# Framework Extension Completeness Lists

These lists track alignment status by component category.

All tests listed here are VM-specific; generic tests are intentionally excluded
because they are not relevant when integrating a new VM.

## VM Intent Framework

Escrow test alignment for VM intent framework contracts (EVM/SVM):

- `evm-intent-framework/test/`
- `svm-intent-framework/programs/intent_escrow/tests/`

Each test file uses independent numbering starting from 1. At the end of the
implementation, check that all tests are numbered correctly and match the list
below.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

### initialization.test.js / initialization.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should initialize escrow with verifier address | ✅ | ✅ |
| 2 | Should allow requester to create an escrow | ✅ | ✅ |
| 3 | Should revert if escrow already exists | ✅ | ✅ |
| 4 | Should revert if amount is zero | ✅ | ✅ |

### deposit.test.js / deposit.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should allow requester to create escrow with tokens | ✅ | ✅ |
| 2 | Should revert if escrow is already claimed | ✅ | ✅ |
| 3 | Should support multiple escrows with different intent IDs | ✅ | ✅ |
| 4 | Should set correct expiry timestamp | ✅ | ✅ |

### claim.test.js / claim.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should allow solver to claim with valid verifier signature | ✅ | ✅ |
| 2 | Should revert with invalid signature | ✅ | ✅ |
| 3 | Should prevent signature replay across different intent_ids | ✅ | ✅ |
| 4 | Should revert if escrow already claimed | ✅ | ✅ |
| 5 | Should revert if escrow does not exist | ✅ | ✅ |

### cancel.test.js / cancel.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should revert if escrow has not expired yet | ✅ | ✅ |
| 2 | Should allow requester to cancel and reclaim funds after expiry | ✅ | ✅ |
| 3 | Should revert if not requester | ✅ | ✅ |
| 4 | Should revert if already claimed | ✅ | ✅ |
| 5 | Should revert if escrow does not exist | ✅ | ✅ |

### expiry.test.js / expiry.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should allow requester to cancel expired escrow | ✅ | ✅ |
| 2 | Should verify expiry timestamp is stored correctly | ✅ | ✅ |
| 3 | Should prevent claim on expired escrow | ✅ | ✅ |

### cross-chain.test.js / cross_chain.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should handle hex intent ID conversion to uint256/bytes32 | ✅ | ✅ |
| 2 | Should handle intent ID boundary values | ✅ | ✅ |
| 3 | Should handle intent ID zero padding correctly | ✅ | ✅ |
| 4 | Should handle multiple intent IDs from different formats | ✅ | ✅ |

### edge-cases.test.js / edge_cases.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should handle maximum values for amounts | ✅ | ✅ |
| 2 | Should handle minimum deposit amount | ✅ | ✅ |
| 3 | Should allow requester to create multiple escrows | ✅ | ✅ |
| 4 | Should handle gas/compute consumption for large operations | ✅ | ✅ |
| 5 | Should handle concurrent escrow operations | ✅ | ✅ |

### error-conditions.test.js / error_conditions.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should revert with zero amount in createEscrow | ✅ | ✅ |
| 2 | Should revert with insufficient token allowance | ✅ | N/A |
| 3 | Should handle maximum value in createEscrow | ✅ | ✅ |
| 4 | Should allow native currency escrow creation | ✅ | N/A |
| 5 | Should revert with native currency amount mismatch | ✅ | N/A |
| 6 | Should revert when native currency sent with token address | ✅ | N/A |
| 7 | Should revert with invalid signature length | ✅ | N/A |
| 8 | Should revert cancel on non-existent escrow | ✅ | ✅ |
| 9 | Should reject zero solver address | ✅ | ✅ |
| 10 | Should reject duplicate escrow creation | ✅ | ✅ |
| 11 | Should reject insufficient token balance | ✅ | ✅ |

### integration.test.js / integration.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should complete full deposit to claim workflow | ✅ | ✅ |
| 2 | Should handle multiple different token types | ✅ | ✅ |
| 3 | Should emit all events/logs with correct parameters | ✅ | N/A |
| 4 | Should complete full cancellation workflow | ✅ | ✅ |

## Verifier

### tests/*vm_tests.rs (module entrypoints)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Module entrypoints only (no direct tests) | ✅ | ✅ | ✅ |

### tests/*vm/validator_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_successful_*vm*_solver_validation | ✅ | ✅ | ✅ |
| 2 | test_rejection_when_solver_not_registered | ✅ | ✅ | ✅ |
| 3 | test_rejection_when_*vm*_addresses_dont_match | ✅ | ✅ | ✅ |
| 4 | test_*vm*_address_normalization | ✅ | ✅ | ✅ |
| 5 | test_error_handling_for_registry_query_failures | ✅ | ✅ | ✅ |
| 6 | test_rejection_when_intent_has_no_solver | ✅ | ✅ | ✅ |

### tests/*vm/validator_fulfillment_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_extract_mvm_fulfillment_params_success | ✅ | N/A | N/A |
| 2 | test_extract_mvm_fulfillment_params_amount_as_number | ✅ | N/A | N/A |
| 3 | test_extract_mvm_fulfillment_params_amount_as_decimal_string | ✅ | N/A | N/A |
| 4 | test_extract_mvm_fulfillment_params_wrong_function | ✅ | N/A | N/A |
| 5 | test_extract_mvm_fulfillment_params_missing_payload | ✅ | N/A | N/A |
| 6 | test_extract_mvm_fulfillment_params_address_normalization | ✅ | N/A | N/A |
| 7 | test_extract_evm_fulfillment_params_success | N/A | ✅ | N/A |
| 8 | test_extract_evm_fulfillment_params_wrong_selector | N/A | ✅ | N/A |
| 9 | test_extract_evm_fulfillment_params_insufficient_calldata | N/A | ✅ | N/A |
| 10 | test_extract_evm_fulfillment_params_amount_exceeds_u64_max | N/A | ✅ | N/A |
| 11 | test_extract_evm_fulfillment_params_amount_equals_u64_max | N/A | ✅ | N/A |
| 12 | test_extract_evm_fulfillment_params_large_valid_amount | N/A | ✅ | N/A |
| 13 | test_extract_evm_fulfillment_params_normalizes_intent_id_with_leading_zeros | N/A | ✅ | N/A |
| 14 | test_validate_outflow_fulfillment_success | ✅ | ✅ | N/A |
| 15 | test_validate_outflow_fulfillment_succeeds_with_normalized_intent_id | N/A | ✅ | N/A |
| 16 | test_validate_outflow_fulfillment_fails_on_unsuccessful_tx | ✅ | ✅ | N/A |
| 17 | test_validate_outflow_fulfillment_fails_on_intent_id_mismatch | ✅ | ✅ | N/A |
| 18 | test_validate_outflow_fulfillment_fails_on_recipient_mismatch | ✅ | ✅ | N/A |
| 19 | test_validate_outflow_fulfillment_fails_on_amount_mismatch | ✅ | ✅ | N/A |
| 20 | test_validate_outflow_fulfillment_fails_on_solver_not_registered | ✅ | N/A | N/A |
| 21 | test_validate_outflow_fulfillment_fails_on_solver_mismatch | N/A | ✅ | N/A |

### tests/*vm/crypto_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_unique_key_generation | ✅ | N/A | ✅ |
| 2 | test_signature_creation_and_verification | ✅ | N/A | N/A |
| 3 | test_svm_signature_creation_and_verification | N/A | N/A | ✅ |
| 4 | test_signature_verification_fails_for_wrong_message | ✅ | N/A | ✅ |
| 5 | test_signatures_differ_for_different_intent_ids | ✅ | N/A | ✅ |
| 6 | test_escrow_approval_signature | ✅ | N/A | N/A |
| 7 | test_public_key_consistency | ✅ | N/A | ✅ |
| 8 | test_signature_contains_timestamp | ✅ | N/A | ✅ |
| 9 | test_mvm_signature_intent_id_validation | ✅ | N/A | N/A |
| 10 | test_create_evm_approval_signature_success | N/A | ✅ | N/A |
| 11 | test_create_evm_approval_signature_format_65_bytes | N/A | ✅ | N/A |
| 12 | test_create_evm_approval_signature_verification | N/A | ✅ | N/A |
| 13 | test_get_ethereum_address_derivation | N/A | ✅ | N/A |
| 14 | test_evm_signature_recovery_id_calculation | N/A | ✅ | N/A |
| 15 | test_evm_signature_keccak256_hashing | N/A | ✅ | N/A |
| 16 | test_evm_signature_ethereum_message_prefix | N/A | ✅ | N/A |
| 17 | test_evm_intent_id_padding | N/A | ✅ | N/A |
| 18 | test_evm_signature_invalid_intent_id | N/A | ✅ | N/A |
| 19 | test_svm_signature_intent_id_validation | N/A | N/A | ✅ |

### tests/*vm/cross_chain_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_escrow_solver_address_matching_success | ✅ | N/A | N/A |
| 2 | test_escrow_solver_address_mismatch_rejection | ✅ | N/A | N/A |
| 3 | test_escrow_solver_reservation_mismatch_rejection | ✅ | N/A | N/A |
| 4 | test_evm_escrow_cross_chain_matching | N/A | ✅ | N/A |
| 5 | test_intent_id_conversion_to_evm_format | N/A | ✅ | N/A |
| 6 | test_evm_escrow_matching_with_hub_intent | N/A | ✅ | N/A |

### tests/*vm/escrow_parsing_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_escrow_initialized_event_has_amount_and_expiry | N/A | ✅ | ⚠️ |
| 2 | test_escrow_amount_is_not_hardcoded_zero | N/A | ✅ | ⚠️ |
| 3 | test_amount_hex_parsing | N/A | ✅ | ⚠️ |
| 4 | test_expiry_hex_parsing | N/A | ✅ | ⚠️ |
| 5 | test_zero_amount_escrow_fails_validation | N/A | ✅ | ⚠️ |
| 6 | test_correct_amount_escrow_passes_validation | N/A | ✅ | ⚠️ |
| 7 | test_escrow_account_borsh_roundtrip | N/A | ⚠️ | ✅ |
| 8 | test_escrow_account_invalid_base64 | N/A | ⚠️ | ✅ |

### tests/*vm/config_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_evm_chain_config_structure | N/A | ✅ | N/A |
| 2 | test_connected_chain_evm_with_values | N/A | ✅ | N/A |
| 3 | test_evm_config_serialization | N/A | ✅ | N/A |
| 4 | test_evm_chain_config_with_all_fields | N/A | ✅ | N/A |
| 5 | test_evm_config_loading | N/A | ✅ | N/A |

### tests/*vm/monitor_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_evm_escrow_detection_logic | N/A | ✅ | N/A |
| 2 | test_evm_escrow_ecdsa_signature_creation | N/A | ✅ | N/A |
| 3 | test_evm_vs_mvm_escrow_differentiation | N/A | ✅ | N/A |
| 4 | test_evm_escrow_approval_flow | N/A | ✅ | N/A |
| 5 | test_evm_escrow_with_invalid_intent_id | N/A | ✅ | N/A |

### tests/*vm_client_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_get_solver_connected_chain_mvm_addr_success | ✅ | N/A | N/A |
| 2 | test_get_solver_connected_chain_mvm_addr_none | ✅ | N/A | N/A |
| 3 | test_get_solver_connected_chain_mvm_addr_solver_not_found | ✅ | N/A | N/A |
| 4 | test_get_solver_connected_chain_mvm_addr_registry_not_found | ✅ | N/A | N/A |
| 5 | test_get_solver_connected_chain_mvm_addr_address_normalization | ✅ | N/A | N/A |
| 6 | test_get_solver_evm_address_array_format | ✅ | N/A | N/A |
| 7 | test_get_solver_evm_address_hex_string_format | ✅ | N/A | N/A |
| 8 | test_get_solver_mvm_address_leading_zero_mismatch | ✅ | N/A | N/A |
| 9 | test_get_solver_evm_address_leading_zero_mismatch | ✅ | N/A | N/A |
| 10 | test_get_solver_public_key_success | ✅ | N/A | N/A |
| 11 | test_get_solver_public_key_not_registered | ✅ | N/A | N/A |
| 12 | test_get_solver_public_key_empty_hex_string | ✅ | N/A | N/A |
| 13 | test_get_solver_public_key_errors_on_unexpected_format | ✅ | N/A | N/A |
| 14 | test_get_solver_public_key_ed25519_format | ✅ | N/A | N/A |
| 15 | test_get_solver_public_key_errors_on_empty_array | ✅ | N/A | N/A |
| 16 | test_get_solver_public_key_errors_on_non_string_element | ✅ | N/A | N/A |
| 17 | test_get_solver_public_key_errors_on_invalid_hex | ✅ | N/A | N/A |
| 18 | test_get_solver_public_key_errors_on_http_error | ✅ | N/A | N/A |
| 19 | test_get_solver_public_key_rejects_address_without_prefix | ✅ | N/A | N/A |
| 20 | test_get_transaction_receipt_status_success | N/A | ✅ | N/A |
| 21 | test_get_transaction_receipt_status_failure | N/A | ✅ | N/A |
| 22 | test_get_transaction_receipt_status_not_found | N/A | ✅ | N/A |
| 23 | test_get_all_escrows_parses_program_accounts | N/A | N/A | ✅ |

### tests/svm_tests.rs (SVM-only)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_extract_svm_fulfillment_params_success | N/A | N/A | ✅ |
| 2 | test_extract_svm_fulfillment_params_requires_memo_first | N/A | N/A | ✅ |
| 3 | test_extract_svm_fulfillment_params_rejects_invalid_intent_id | N/A | N/A | ✅ |

## Solver

### tests/*vm_tests.rs (module entrypoints)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Module entrypoints only (no direct tests) | ✅ | ✅ | ✅ |

### tests/*vm/chain_client_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_intent_created_event_deserialization | ✅ | N/A | N/A |
| 2 | test_escrow_event_deserialization | ✅ | N/A | N/A |
| 3 | test_hub_client_new | ✅ | N/A | N/A |
| 4 | test_get_intent_events_success | ✅ | N/A | N/A |
| 5 | test_get_intent_events_empty | ✅ | N/A | N/A |
| 6 | test_is_solver_registered_true | ✅ | N/A | N/A |
| 7 | test_is_solver_registered_false | ✅ | N/A | N/A |
| 8 | test_is_solver_registered_address_normalization | ✅ | N/A | N/A |
| 9 | test_is_solver_registered_http_error | ✅ | N/A | N/A |
| 10 | test_is_solver_registered_invalid_json | ✅ | N/A | N/A |
| 11 | test_is_solver_registered_unexpected_format | ✅ | N/A | N/A |
| 12 | test_mvm_client_new | ✅ | N/A | N/A |
| 13 | test_get_escrow_events_success | ✅ | N/A | N/A |
| 14 | test_evm_client_new | N/A | ✅ | N/A |
| 15 | test_get_escrow_events_evm_success | N/A | ✅ | N/A |
| 16 | test_get_escrow_events_evm_empty | N/A | ✅ | N/A |
| 17 | test_get_escrow_events_evm_error | N/A | ✅ | N/A |
| 18 | test_claim_escrow_intent_id_formatting | N/A | ✅ | N/A |
| 19 | test_claim_escrow_signature_encoding | N/A | ✅ | N/A |
| 20 | test_claim_escrow_command_building | N/A | ✅ | N/A |
| 21 | test_claim_escrow_hash_extraction | N/A | ✅ | N/A |
| 22 | test_claim_escrow_missing_directory_error | N/A | ✅ | N/A |
| 23 | test_new_rejects_invalid_program_id | N/A | N/A | ✅ |
| 24 | test_new_accepts_valid_program_id | N/A | N/A | ✅ |
| 25 | test_pubkey_from_hex_with_leading_zeros | N/A | N/A | ✅ |
| 26 | test_pubkey_from_hex_no_leading_zeros | N/A | N/A | ✅ |

## Frontend

### components/wallet/*vmWalletConnector.test.tsx

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | should show connect button when disconnected | ⚠️ | ✅ | ✅ |
| 2 | should disable when no wallet is detected | ✅ | ⚠️ | ⚠️ |
| 3 | should disable when no MetaMask connector is available | ⚠️ | ✅ | ⚠️ |
| 4 | should disable when Phantom adapter is not detected | ⚠️ | ⚠️ | ✅ |
| 5 | should call connect when clicking the connect button | ⚠️ | ✅ | ⚠️ |
| 6 | should call select and connect on click | ⚠️ | ⚠️ | ✅ |
| 7 | should show disconnect button when connected | ⚠️ | ✅ | ✅ |
| 8 | should show disconnect when connected | ✅ | ⚠️ | ⚠️ |

### lib/*vm-transactions.test.ts

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | should be a valid Move address | ✅ | N/A | N/A |
| 2 | should convert hex string to Uint8Array | ✅ | N/A | N/A |
| 3 | should handle 64-byte Ed25519 signature | ✅ | N/A | N/A |
| 4 | should strip 0x prefix automatically | ✅ | N/A | N/A |
| 5 | should return empty array for empty string | ✅ | N/A | N/A |
| 6 | should pad 20-byte EVM address to 32 bytes | ✅ | N/A | N/A |
| 7 | should handle address without 0x prefix | ✅ | N/A | N/A |
| 8 | should normalize to lowercase | ✅ | N/A | N/A |
| 9 | should remove 0x prefix | ✅ | N/A | N/A |
| 10 | should return unchanged if no prefix | ✅ | N/A | N/A |
| 11 | should use the configured SVM RPC URL | N/A | N/A | ✅ |
| 12 | should decode base64 to bytes | N/A | N/A | ✅ |
| 13 | should trim whitespace around base64 input | N/A | N/A | ✅ |
| 14 | should return an instruction targeting the Ed25519 program | N/A | N/A | ✅ |
| 15 | should return null when the request fails | N/A | N/A | ✅ |
| 16 | should return null when the registry vec is empty | N/A | N/A | ✅ |
| 17 | should return normalized hex when vec is a string | N/A | N/A | ✅ |
| 18 | should convert vec byte array to hex | N/A | N/A | ✅ |

### lib/*vm-escrow.test.ts

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | should convert 0x-prefixed intent IDs to uint256 bigint | N/A | ✅ | N/A |
| 2 | should convert non-prefixed intent IDs to uint256 bigint | N/A | ✅ | N/A |
| 3 | should return a checksummed EVM address | N/A | ✅ | N/A |
| 4 | should throw for missing chain config | N/A | ✅ | N/A |
| 5 | should pad intent IDs to 32 bytes | N/A | N/A | ✅ |
| 6 | should round-trip pubkey hex conversion | N/A | N/A | ✅ |
| 7 | should derive deterministic state/escrow/vault PDAs | N/A | N/A | ✅ |
| 8 | should parse escrow account data into a structured object | N/A | N/A | ✅ |
| 9 | should build create escrow instruction with expected layout | N/A | N/A | ✅ |
| 10 | should build claim instruction with sysvar and token program keys | N/A | N/A | ✅ |
| 11 | should build cancel instruction with expected layout | N/A | N/A | ✅ |
