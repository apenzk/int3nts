# Solver Test Completeness

> **⚠️ IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for the solver. For the complete overview and other frameworks, see the [Framework Extension Guide](../../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## *vm_tests.rs (module entrypoints)

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Module entrypoints only (no direct tests) | ✅ | ✅ | ✅ |

## tests/*vm/chain_client_tests.rs

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
