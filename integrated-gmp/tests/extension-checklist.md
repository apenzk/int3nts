# Integrated GMP Test Completeness

> **⚠️ IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for the integrated GMP relay service. For the complete overview and other frameworks, see the [Framework Extension Guide](../../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## *vm_tests.rs (module entrypoints)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Module entrypoints only (no direct tests) | N/A | ✅ | N/A |

## tests/*vm/config_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_evm_chain_config_structure | N/A | ✅ | N/A |
| 2 | test_connected_chain_evm_with_values | N/A | ✅ | N/A |
| 3 | test_evm_config_serialization | N/A | ✅ | N/A |
| 4 | test_evm_chain_config_with_all_fields | N/A | ✅ | N/A |
| 5 | test_evm_config_loading | N/A | ✅ | N/A |

## tests/*vm/escrow_parsing_tests.rs

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_escrow_account_borsh_roundtrip | N/A | N/A | ✅ |
| 2 | test_escrow_account_invalid_base64 | N/A | N/A | ✅ |

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
| 20 | test_get_all_escrows_parses_program_accounts | N/A | N/A | ✅ |
