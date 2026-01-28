# Trusted GMP Test Completeness

> **⚠️ IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for the trusted GMP service. For the complete overview and other frameworks, see the [Framework Extension Guide](../../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## *vm_tests.rs (module entrypoints)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Module entrypoints only (no direct tests) | ✅ | ✅ | ✅ |

## tests/*vm/validator_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_successful_*vm*_solver_validation | ✅ | ✅ | ✅ |
| 2 | test_rejection_when_solver_not_registered | ✅ | ✅ | ✅ |
| 3 | test_rejection_when_*vm*_addresses_dont_match | ✅ | ✅ | ✅ |
| 4 | test_*vm*_address_normalization | ✅ | ✅ | ✅ |
| 5 | test_error_handling_for_registry_query_failures | ✅ | ✅ | ✅ |
| 6 | test_rejection_when_intent_has_no_solver | ✅ | ✅ | ✅ |

## tests/*vm/validator_fulfillment_tests.rs

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

## tests/*vm/crypto_tests.rs

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
| 17 | test_evm_signature_intent_id_padding | N/A | ✅ | N/A |
| 18 | test_evm_signature_invalid_intent_id | N/A | ✅ | N/A |
| 19 | test_svm_signature_intent_id_validation | N/A | N/A | ✅ |

## tests/*vm/cross_chain_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_escrow_solver_address_matching_success | ✅ | N/A | N/A |
| 2 | test_escrow_solver_address_mismatch_rejection | ✅ | N/A | N/A |
| 3 | test_escrow_solver_reservation_mismatch_rejection | ✅ | N/A | N/A |
| 4 | test_evm_escrow_cross_chain_matching | N/A | ✅ | N/A |
| 5 | test_intent_id_conversion_to_evm_format | N/A | ✅ | N/A |
| 6 | test_evm_escrow_matching_with_hub_intent | N/A | ✅ | N/A |

## tests/*vm/escrow_parsing_tests.rs

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

## tests/*vm/config_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_evm_chain_config_structure | N/A | ✅ | N/A |
| 2 | test_connected_chain_evm_with_values | N/A | ✅ | N/A |
| 3 | test_evm_config_serialization | N/A | ✅ | N/A |
| 4 | test_evm_chain_config_with_all_fields | N/A | ✅ | N/A |
| 5 | test_evm_config_loading | N/A | ✅ | N/A |

## tests/*vm/monitor_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_evm_escrow_detection_logic | N/A | ✅ | N/A |
| 2 | test_evm_escrow_ecdsa_signature_creation | N/A | ✅ | N/A |
| 3 | test_evm_vs_mvm_escrow_differentiation | N/A | ✅ | N/A |
| 4 | test_evm_escrow_approval_flow | N/A | ✅ | N/A |
| 5 | test_evm_escrow_with_invalid_intent_id | N/A | ✅ | N/A |

## tests/*vm_client_tests.rs

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

## tests/svm_tests.rs (SVM-only)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_extract_svm_fulfillment_params_success | N/A | N/A | ✅ |
| 2 | test_extract_svm_fulfillment_params_requires_memo_first | N/A | N/A | ✅ |
| 3 | test_extract_svm_fulfillment_params_rejects_invalid_intent_id | N/A | N/A | ✅ |
