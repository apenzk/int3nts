# Solver Test Completeness

> **IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for the solver connected chain clients. For the complete overview and other frameworks, see the [Framework Extension Guide](../../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

Each test file uses independent numbering starting from 1. At the end of the implementation, check that all tests are numbered correctly and match the list below.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented | X = Moved to chain-clients

## module-entrypoints

MVM: `solver/tests/mvm_tests.rs`
EVM: `solver/tests/evm_tests.rs`
SVM: `solver/tests/svm_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Module entrypoints only (no direct tests) | ✅ | ✅ | ✅ |

## chain-client (solver-specific)

These tests cover solver-specific functionality: CLI fulfillment operations, command building, and Hardhat script mechanics. Query tests (balance, escrow state, address normalization) moved to [chain-clients](../../chain-clients/extension-checklist.md).

MVM: `solver/tests/mvm/chain_client_tests.rs`
EVM: `solver/tests/evm/chain_client_tests.rs`
SVM: `solver/tests/svm/chain_client_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_client_new | ✅ | ✅ | ✅ |
| 2 | test_client_new_rejects_invalid | N/A | N/A | ✅ |
| 3 | test_get_escrow_events_success | N/A | X | X |
| 4 | test_get_escrow_events_empty | N/A | X | X |
| 5 | test_get_escrow_events_error | N/A | X | X |
| 6 | test_escrow_event_deserialization | N/A | N/A | N/A |
| 7 | test_fulfillment_id_formatting | ✅ | ⚠️ | ⚠️ |
| 8 | test_fulfillment_signature_encoding | N/A | ⚠️ | N/A |
| 9 | test_fulfillment_command_building | ✅ | ⚠️ | ⚠️ |
| 10 | test_fulfillment_error_handling | ⚠️ | ⚠️ | ✅ |
| 11 | test_pubkey_from_hex_with_leading_zeros | N/A | N/A | X |
| 12 | test_pubkey_from_hex_no_leading_zeros | N/A | N/A | X |
| 13 | test_is_escrow_released_success | X | X | ✅ |
| 14 | test_is_escrow_released_false | X | X | ✅ |
| 15 | test_is_escrow_released_error | X | X | ✅ |
| 16 | test_get_token_balance_success | X | X | ✅ |
| 17 | test_get_token_balance_error | X | X | ✅ |
| 18 | test_get_token_balance_zero | X | X | N/A |
| 19 | test_get_native_balance_success | N/A | X | ✅ |
| 20 | test_get_native_balance_error | N/A | X | ✅ |
| 21 | test_normalize_hex_to_address_full_length | X | N/A | N/A |
| 22 | test_normalize_hex_to_address_short_address | X | N/A | N/A |
| 23 | test_normalize_hex_to_address_odd_length | X | N/A | N/A |
| 24 | test_normalize_hex_to_address_no_prefix | X | N/A | N/A |
| 25 | test_has_outflow_requirements_success | ✅ | N/A | N/A |
| 26 | test_has_outflow_requirements_false | ✅ | N/A | N/A |
| 27 | test_has_outflow_requirements_error | ✅ | N/A | N/A |
| 28 | test_is_escrow_released_id_formatting | N/A | ✅ | N/A |
| 29 | test_is_escrow_released_output_parsing | N/A | ✅ | N/A |
| 30 | test_is_escrow_released_command_building | N/A | ✅ | N/A |
| 31 | test_is_escrow_released_error_handling | N/A | ✅ | N/A |
| 32 | test_get_native_balance_exceeds_u64 | N/A | X | N/A |
| 33 | test_get_token_balance_with_padded_address | N/A | X | N/A |
| 34 | test_get_native_balance_with_padded_address | N/A | X | N/A |
| 35 | test_normalize_evm_address_padded | N/A | X | N/A |
| 36 | test_normalize_evm_address_passthrough | N/A | X | N/A |
| 37 | test_normalize_evm_address_rejects_non_zero_high_bytes | N/A | X | N/A |

## hub-client (MVM-only, not tracked for VM symmetry)

MVM: `solver/tests/mvm/hub_client_tests.rs`

| # | Test |
| --- | ------ |
| 1 | test_hub_client_new |
| 2 | test_intent_created_event_deserialization |
| 3 | test_get_intent_events_success |
| 4 | test_get_intent_events_empty |
| 5 | test_is_solver_registered_true |
| 6 | test_is_solver_registered_false |
| 7 | test_is_solver_registered_address_normalization |
| 8 | test_is_solver_registered_http_error |
| 9 | test_is_solver_registered_invalid_json |
| 10 | test_is_solver_registered_unexpected_format |
