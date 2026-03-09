# Chain Clients Test Completeness

> **IMPORTANT: When adding a new chain client, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for chain-clients. For the complete overview and other frameworks, see the [Framework Extension Guide](../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

Hub-only tests are NOT tracked in this checklist. The hub is always MVM — there is no VM symmetry to enforce. Hub tests live in `mvm/tests/mvm_client_hub_tests.rs` with their own independent numbering.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## common/tests/intent_id_tests.rs

These tests are chain-agnostic (no MVM/EVM/SVM columns). They apply universally.

| # | Test |
| --- | ------ |
| 1 | test_normalize_intent_id_strips_leading_zeros |
| 2 | test_normalize_intent_id_lowercases |
| 3 | test_normalize_intent_id_all_zeros |
| 4 | test_normalize_intent_id_no_prefix |
| 5 | test_normalize_intent_id_to_64_chars_pads |
| 6 | test_normalize_intent_id_to_64_chars_lowercases |
| 7 | test_normalize_intent_id_to_64_chars_no_prefix |

## {mvm,evm,svm}/tests/*_client_tests.rs

### client-init

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_client_new | ✅ | ✅ | ✅ |
| 2 | test_client_new_rejects_invalid | N/A | N/A | ✅ |

- MVM N/A #2: MvmClient accepts any URL string, validation happens at request time
- EVM N/A #2: Same as MVM

### escrow-release-check

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 3 | test_is_escrow_released_success | ✅ | ✅ | ✅ |
| 4 | test_is_escrow_released_false | ✅ | ✅ | ✅ |
| 5 | test_is_escrow_released_error | ✅ | ✅ | ✅ |

### balance-queries

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 6 | test_get_token_balance_success | ✅ | ✅ | ✅ |
| 7 | test_get_token_balance_error | ✅ | ✅ | ✅ |
| 8 | test_get_token_balance_zero | ✅ | ✅ | N/A |
| 9 | test_get_native_balance_success | N/A | ✅ | ✅ |
| 10 | test_get_native_balance_error | N/A | ✅ | ✅ |
| 11 | test_get_native_balance_exceeds_u64 | N/A | ✅ | N/A |
| 12 | test_get_token_balance_with_padded_address | N/A | ✅ | N/A |
| 13 | test_get_native_balance_with_padded_address | N/A | ✅ | N/A |

- MVM N/A #9-#10: MVM uses fungible assets, no native balance concept separate from FA
- MVM N/A #11-#13: EVM-specific u128/address padding
- SVM N/A #8: SVM token accounts don't return zero — they don't exist if unfunded
- SVM N/A #11-#13: EVM-specific u128/address padding

### escrow-event-parsing

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 14 | test_get_escrow_events_success | N/A | ✅ | ✅ |
| 15 | test_get_escrow_events_empty | N/A | ✅ | ✅ |
| 16 | test_get_escrow_events_error | N/A | ✅ | ✅ |
| 17 | test_get_all_escrows_parses_program_accounts | N/A | N/A | ✅ |

- MVM N/A #14-#16: MVM events are polled via Aptos REST API event stream, not eth_getLogs. Event polling is coordinator-specific (monitor/) not a generic client capability.
- EVM N/A #17: EVM doesn't use getProgramAccounts — escrows are in a single contract, queried via logs.

### address-normalization

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 18 | test_normalize_hex_to_address_full_length | ✅ | N/A | N/A |
| 19 | test_normalize_hex_to_address_short_address | ✅ | N/A | N/A |
| 20 | test_normalize_hex_to_address_odd_length | ✅ | N/A | N/A |
| 21 | test_normalize_hex_to_address_no_prefix | ✅ | N/A | N/A |
| 22 | test_normalize_evm_address_padded | N/A | ✅ | N/A |
| 23 | test_normalize_evm_address_passthrough | N/A | ✅ | N/A |
| 24 | test_normalize_evm_address_rejects_non_zero_high_bytes | N/A | ✅ | N/A |
| 25 | test_pubkey_from_hex_with_leading_zeros | N/A | N/A | ✅ |
| 26 | test_pubkey_from_hex_no_leading_zeros | N/A | N/A | ✅ |

### svm-escrow-parsing

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 27 | test_escrow_account_borsh_roundtrip | N/A | N/A | ✅ |
| 28 | test_escrow_account_invalid_base64 | N/A | N/A | ✅ |

- MVM/EVM N/A: SVM-specific Borsh serialization format. MVM uses JSON, EVM uses ABI encoding.
