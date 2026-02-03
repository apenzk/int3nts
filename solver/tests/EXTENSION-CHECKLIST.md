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
| 3 | get_escrow_events_success | ✅ | ✅ | ⚠️ |
| 4 | get_escrow_events_empty | ⚠️ | ✅ | ⚠️ |
| 5 | get_escrow_events_error | ⚠️ | ✅ | ⚠️ |
| 6 | escrow_event_deserialization | ✅ | N/A | N/A |
| 7 | fulfillment_id_formatting | ✅ | ✅ | ⚠️ |
| 8 | fulfillment_signature_encoding | N/A | ✅ | N/A |
| 9 | fulfillment_command_building | ✅ | ✅ | ⚠️ |
| 10 | fulfillment_hash_extraction | ⚠️ | ✅ | ⚠️ |
| 11 | fulfillment_error_handling | ⚠️ | ✅ | ✅ |
| 12 | pubkey_from_hex_with_leading_zeros | N/A | N/A | ✅ |
| 13 | pubkey_from_hex_no_leading_zeros | N/A | N/A | ✅ |
