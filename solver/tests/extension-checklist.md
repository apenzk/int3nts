# Solver Test Completeness

> **⚠️ IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for the solver connected chain clients. For the complete overview and other frameworks, see the [Framework Extension Guide](../../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

Each test file uses independent numbering starting from 1. At the end of the implementation, check that all tests are numbered correctly and match the list below.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## module-entrypoints

MVM: `solver/tests/mvm_tests.rs`
EVM: `solver/tests/evm_tests.rs`
SVM: `solver/tests/svm_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Module entrypoints only (no direct tests) | ✅ | ✅ | ✅ |

## chain-client

MVM: `solver/tests/mvm/chain_client_tests.rs`
EVM: `solver/tests/evm/chain_client_tests.rs`
SVM: `solver/tests/svm/chain_client_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | client_new | ✅ | ✅ | ✅ |
| 2 | client_new_rejects_invalid | N/A | N/A | ✅ |
| 3 | get_escrow_events_success | N/A | ✅ | ⚠️ |
| 4 | get_escrow_events_empty | N/A | ✅ | ⚠️ |
| 5 | get_escrow_events_error | N/A | ✅ | ⚠️ |
| 6 | escrow_event_deserialization | N/A | N/A | N/A |
| 7 | fulfillment_id_formatting | ✅ | ⚠️ | ⚠️ |
| 8 | fulfillment_signature_encoding | N/A | ⚠️ | N/A |
| 9 | fulfillment_command_building | ✅ | ⚠️ | ⚠️ |
| 10 | fulfillment_error_handling | ⚠️ | ⚠️ | ✅ |
| 11 | pubkey_from_hex_with_leading_zeros | N/A | N/A | ✅ |
| 12 | pubkey_from_hex_no_leading_zeros | N/A | N/A | ✅ |
| 13 | is_escrow_released_success | ✅ | ⚠️ | ✅ |
| 14 | is_escrow_released_false | ✅ | ⚠️ | ✅ |
| 15 | is_escrow_released_error | ✅ | ⚠️ | ✅ |
| 16 | get_token_balance_success | ✅ | ✅ | ✅ |
| 17 | get_token_balance_error | ✅ | ✅ | ✅ |
| 18 | get_token_balance_zero | ✅ | ✅ | N/A |
| 19 | get_native_balance_success | N/A | ✅ | ✅ |
| 20 | get_native_balance_error | N/A | ✅ | ✅ |
| 21 | normalize_hex_to_address_full_length | ✅ | N/A | N/A |
| 22 | normalize_hex_to_address_short_address | ✅ | N/A | N/A |
| 23 | normalize_hex_to_address_odd_length | ✅ | N/A | N/A |
| 24 | normalize_hex_to_address_no_prefix | ✅ | N/A | N/A |
| 25 | has_outflow_requirements_success | ✅ | N/A | N/A |
| 26 | has_outflow_requirements_false | ✅ | N/A | N/A |
| 27 | has_outflow_requirements_error | ✅ | N/A | N/A |
| 28 | is_escrow_released_id_formatting | N/A | ✅ | N/A |
| 29 | is_escrow_released_output_parsing | N/A | ✅ | N/A |
| 30 | is_escrow_released_command_building | N/A | ✅ | N/A |
| 31 | is_escrow_released_error_handling | N/A | ✅ | N/A |
| 32 | get_native_balance_exceeds_u64 | N/A | ✅ | N/A |
| 33 | get_token_balance_with_padded_address | N/A | ✅ | N/A |
| 34 | get_native_balance_with_padded_address | N/A | ✅ | N/A |
| 35 | normalize_evm_address_padded | N/A | ✅ | N/A |
| 36 | normalize_evm_address_passthrough | N/A | ✅ | N/A |
| 37 | normalize_evm_address_rejects_non_zero_high_bytes | N/A | ✅ | N/A |
