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

## tests/*vm/escrow_parsing_tests.rs and tests/*vm_client_tests.rs

MVM client tests moved to `chain-clients/mvm/tests/mvm_client_hub_tests.rs`.
SVM escrow tests moved to `chain-clients/svm/tests/svm_client_tests.rs`.
See [chain-clients extension checklist](../../chain-clients/extension-checklist.md).
