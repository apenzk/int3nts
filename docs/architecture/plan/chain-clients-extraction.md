# Plan: Extract Shared Chain Clients Crate

## Problem

The same chain client code is duplicated across coordinator, integrated-gmp, and solver:

- `MvmClient`: ~85% identical between coordinator (1394 lines) and integrated-gmp (1570 lines)
- `SvmClient`: ~80% identical between coordinator (314 lines) and integrated-gmp (383 lines)
- EVM JSON-RPC: 3 independent copies (coordinator inline, integrated-gmp `EvmClient`, solver `ConnectedEvmClient`)
- `normalize_intent_id()`: 3 independent implementations (2 different versions)

## Solution

Extract shared chain client code into a new `chain-clients/` workspace with 4 crates.

## Target Structure

```text
chain-clients/
├── extension-checklist.md
├── common/
│   ├── Cargo.toml                    # chain-clients-common
│   ├── src/
│   │   ├── lib.rs
│   │   └── intent_id.rs             # normalize_intent_id(), normalize_intent_id_to_64_chars()
│   └── tests/
│       └── intent_id_tests.rs
├── mvm/
│   ├── Cargo.toml                    # chain-clients-mvm (depends on chain-clients-common)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── client.rs                # MvmClient: REST API, view functions, solver registry
│   │   └── types.rs                 # AccountInfo, EventHandle, MvmEvent, MvmTransaction, etc.
│   └── tests/
│       ├── mvm_client_tests.rs      # Connected-chain client tests (tracked in checklist)
│       └── mvm_client_hub_tests.rs  # Hub-only tests (NOT tracked in checklist)
├── evm/
│   ├── Cargo.toml                    # chain-clients-evm (depends on chain-clients-common)
│   ├── src/
│   │   ├── lib.rs
│   │   ├── client.rs                # EvmClient: JSON-RPC, get_logs, get_block_number, balances
│   │   └── types.rs                 # EvmLog, EscrowCreatedEvent, JsonRpcRequest/Response
│   └── tests/
│       └── evm_client_tests.rs
└── svm/
    ├── Cargo.toml                    # chain-clients-svm (depends on chain-clients-common)
    ├── src/
    │   ├── lib.rs
    │   ├── client.rs                # SvmClient: RPC, PDA derivation, escrow parsing, balances
    │   └── types.rs                 # EscrowAccount, AccountInfoResult
    └── tests/
        └── svm_client_tests.rs
```

## Dependency Graph

```text
chain-clients-common    (no chain deps - hex/string utils only)
       ↑
  ┌────┼────┐
  │    │    │
 mvm  evm  svm         (each has its own chain-specific deps)
  ↑    ↑    ↑
  │    │    │
coordinator    integrated-gmp    solver
```

---

## Extension Checklist: chain-clients/extension-checklist.md

This is the source of truth for the extraction. Every test listed here must exist in the corresponding test file, numbered sequentially, following the standard `Test:` / `Verifies` / `Why:` documentation format. Tests marked N/A for a VM must have inline comments in the test file explaining why.

Hub-only tests are NOT tracked in this checklist. The hub is always MVM — there is no VM symmetry to enforce. Hub tests live in `mvm_client_hub_tests.rs` with their own independent numbering.

### common

Tests in: `chain-clients/common/tests/intent_id_tests.rs`

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

Source: coordinator/tests/monitor_tests.rs (3 existing tests, expanded for full coverage)

### client-init

MVM: `chain-clients/mvm/tests/mvm_client_tests.rs`
EVM: `chain-clients/evm/tests/evm_client_tests.rs`
SVM: `chain-clients/svm/tests/svm_client_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_client_new | ✅ | ✅ | ✅ |
| 2 | test_client_new_rejects_invalid | N/A | N/A | ✅ |

Source: solver/tests/{mvm,evm,svm}/chain_client_tests.rs #1-#2

Notes:

- MVM N/A #2: MvmClient accepts any URL string, validation happens at request time
- EVM N/A #2: Same as MVM

### escrow-release-check

MVM: `chain-clients/mvm/tests/mvm_client_tests.rs`
EVM: `chain-clients/evm/tests/evm_client_tests.rs`
SVM: `chain-clients/svm/tests/svm_client_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 3 | test_is_escrow_released_success | ✅ | ✅ | ✅ |
| 4 | test_is_escrow_released_false | ✅ | ✅ | ✅ |
| 5 | test_is_escrow_released_error | ✅ | ✅ | ✅ |

Source: solver/tests/{mvm,evm,svm}/chain_client_tests.rs #13-#15

Notes:

- MVM: Uses view function call to check escrow state
- EVM: Uses eth_call to check is_claimed on escrow contract
- SVM: Uses Borsh-decoded account data to check is_claimed field

### balance-queries

MVM: `chain-clients/mvm/tests/mvm_client_tests.rs`
EVM: `chain-clients/evm/tests/evm_client_tests.rs`
SVM: `chain-clients/svm/tests/svm_client_tests.rs`

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

Source: solver/tests/{mvm,evm,svm}/chain_client_tests.rs #16-#20, #32-#34

Notes:

- MVM N/A #9-#10: MVM uses fungible assets, no native balance concept separate from FA
- MVM N/A #11-#13: EVM-specific u128/address padding
- SVM N/A #8: SVM token accounts don't return zero — they don't exist if unfunded
- SVM N/A #11-#13: EVM-specific u128/address padding
- EVM #11: ETH balances can exceed u64 max, must use u128

### escrow-event-parsing

MVM: N/A (MVM escrow events parsed via Aptos REST event stream in coordinator monitor)
EVM: `chain-clients/evm/tests/evm_client_tests.rs`
SVM: `chain-clients/svm/tests/svm_client_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 14 | test_get_escrow_events_success | N/A | ✅ | ⚠️ |
| 15 | test_get_escrow_events_empty | N/A | ✅ | ⚠️ |
| 16 | test_get_escrow_events_error | N/A | ✅ | ⚠️ |
| 17 | test_get_all_escrows_parses_program_accounts | N/A | N/A | ✅ |

Source: solver/tests/evm/chain_client_tests.rs #3-#5, coordinator/tests/svm_client_tests.rs #1

Notes:

- MVM N/A #14-#16: MVM events are polled via Aptos REST API event stream, not eth_getLogs. Event polling is coordinator-specific (monitor/) not a generic client capability.
- EVM N/A #17: EVM doesn't use getProgramAccounts — escrows are in a single contract, queried via logs.
- SVM ⚠️ #14-#16: SVM escrow event parsing from transaction logs not yet implemented in shared client.

### address-normalization

MVM: `chain-clients/mvm/tests/mvm_client_tests.rs`
EVM: `chain-clients/evm/tests/evm_client_tests.rs`
SVM: `chain-clients/svm/tests/svm_client_tests.rs`

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

Source: solver/tests/mvm/chain_client_tests.rs #21-#24, solver/tests/evm/chain_client_tests.rs #35-#37, solver/tests/svm/chain_client_tests.rs #4-#5

Notes:

- Each VM has its own address format and normalization rules. VM-specific by nature but belong in the shared client since all consumers need them.
- MVM: 64-char hex with 0x prefix, zero-padded
- EVM: 20-byte address, sometimes received as 32-byte zero-padded
- SVM: 32-byte public key from hex

### svm-escrow-parsing

MVM: N/A
EVM: N/A
SVM: `chain-clients/svm/tests/svm_client_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 27 | test_escrow_account_borsh_roundtrip | N/A | N/A | ✅ |
| 28 | test_escrow_account_invalid_base64 | N/A | N/A | ✅ |

Source: integrated-gmp/tests/svm/escrow_parsing_tests.rs #1-#2

Notes:

- MVM/EVM N/A: SVM-specific Borsh serialization format. MVM uses JSON, EVM uses ABI encoding.

---

## Hub-Only Tests (NOT in extension checklist)

These tests query the MVM hub chain specifically (solver registry, public keys, registration, outflow requirements). The hub is always MVM — no VM symmetry applies. They live in `chain-clients/mvm/tests/mvm_client_hub_tests.rs` with independent numbering.

### solver-registry-lookup

| # | Test |
| --- | ------ |
| 1 | test_get_solver_connected_chain_mvm_addr_success |
| 2 | test_get_solver_connected_chain_mvm_addr_none |
| 3 | test_get_solver_connected_chain_mvm_addr_solver_not_found |
| 4 | test_get_solver_connected_chain_mvm_addr_registry_not_found |
| 5 | test_get_solver_connected_chain_mvm_addr_address_normalization |
| 6 | test_get_solver_evm_address_array_format |
| 7 | test_get_solver_evm_address_hex_string_format |
| 8 | test_get_solver_mvm_address_leading_zero_mismatch |
| 9 | test_get_solver_evm_address_leading_zero_mismatch |
| 10 | test_get_solver_svm_address_array_format |
| 11 | test_get_solver_svm_address_hex_string_format |
| 12 | test_get_solver_svm_address_leading_zero_mismatch |

Source: coordinator/tests/mvm_client_tests.rs #1-#6, integrated-gmp/tests/mvm_client_tests.rs #1-#5 (merged, deduplicated)

### solver-public-key

| # | Test |
| --- | ------ |
| 13 | test_get_solver_public_key_success |
| 14 | test_get_solver_public_key_not_registered |
| 15 | test_get_solver_public_key_empty_hex_string |
| 16 | test_get_solver_public_key_errors_on_unexpected_format |
| 17 | test_get_solver_public_key_ed25519_format |
| 18 | test_get_solver_public_key_errors_on_empty_array |
| 19 | test_get_solver_public_key_errors_on_non_string_element |
| 20 | test_get_solver_public_key_errors_on_invalid_hex |
| 21 | test_get_solver_public_key_errors_on_http_error |
| 22 | test_get_solver_public_key_rejects_address_without_prefix |

Source: coordinator/tests/mvm_client_tests.rs #7-#16 (identical in integrated-gmp, deduplicated)

### solver-registration-check

| # | Test |
| --- | ------ |
| 23 | test_is_solver_registered_true |
| 24 | test_is_solver_registered_false |
| 25 | test_is_solver_registered_address_normalization |
| 26 | test_is_solver_registered_http_error |
| 27 | test_is_solver_registered_invalid_json |
| 28 | test_is_solver_registered_unexpected_format |

Source: solver/tests/mvm/hub_client_tests.rs #5-#10

### outflow-requirements

| # | Test |
| --- | ------ |
| 29 | test_has_outflow_requirements_success |
| 30 | test_has_outflow_requirements_false |
| 31 | test_has_outflow_requirements_error |

Source: solver/tests/mvm/chain_client_tests.rs #25-#27

---

## Tests That Stay in Services (NOT moving to chain-clients)

These tests are service-specific and remain in their current crates.

### coordinator (stays)

- `readiness_*vm_tests.rs` — 4 tests per VM (12 total): event monitoring and intent readiness tracking
- `monitor_tests.rs` — normalize_intent_id tests move to chain-clients-common
- `api_tests.rs`, `config_tests.rs`, `storage_tests.rs`, `negotiation_validation_tests.rs` — coordinator-specific

### integrated-gmp (stays)

- `evm/config_tests.rs` — 5 tests: EvmChainConfig structure, serialization (relay-specific config fields like `approver_evm_pubkey_hash`)
- `integrated_gmp_relay_tests.rs` — relay logic tests

### solver (stays)

- `mvm/hub_client_tests.rs` #1-#4: HubChainClient init, IntentCreatedEvent deserialization, get_intent_events (hub intent discovery is solver-specific)
- `mvm/chain_client_tests.rs` #7,#9: fulfillment_id_formatting, fulfillment_command_building (aptos CLI invocation)
- `evm/chain_client_tests.rs` #28-#31: is_escrow_released via Hardhat (Hardhat script mechanics)
- `svm/chain_client_tests.rs` #3: fulfill_outflow_via_gmp error handling (GMP fulfillment flow)
- `acceptance_tests.rs`, `tracker_tests.rs`, `liquidity_tests.rs`, etc. — solver-specific

---

## Extension Checklist Updates to Existing Files

### coordinator/tests/extension-checklist.md

Remove the entire `tests/*vm_client_tests.rs` section (23 tests). These move to chain-clients. Keep `tests/readiness_*vm_tests.rs` section unchanged.

After:

- `tests/readiness_*vm_tests.rs` — 4 tests (MVM ✅, EVM ✅, SVM ✅)

### integrated-gmp/tests/extension-checklist.md

Remove the entire `tests/*vm_client_tests.rs` section (20 tests). These move to chain-clients. Move `tests/*vm/escrow_parsing_tests.rs` to chain-clients-svm. Keep `*vm_tests.rs` entrypoints and `tests/*vm/config_tests.rs`.

After:

- `*vm_tests.rs` — 1 test (module entrypoints)
- `tests/*vm/config_tests.rs` — 5 tests (EVM-specific relay config)

### solver/tests/extension-checklist.md

Split the `chain-client` section. Shared tests move to chain-clients. Service-specific tests stay with renumbering from 1.

After — solver `chain-client` section keeps only:

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | test_fulfillment_id_formatting | ✅ | ⚠️ | ⚠️ |
| 2 | test_fulfillment_signature_encoding | N/A | ⚠️ | N/A |
| 3 | test_fulfillment_command_building | ✅ | ⚠️ | ⚠️ |
| 4 | test_fulfillment_error_handling | ⚠️ | ⚠️ | ✅ |
| 5 | test_is_escrow_released_id_formatting | N/A | ✅ | N/A |
| 6 | test_is_escrow_released_output_parsing | N/A | ✅ | N/A |
| 7 | test_is_escrow_released_command_building | N/A | ✅ | N/A |
| 8 | test_is_escrow_released_error_handling | N/A | ✅ | N/A |

Plus a new `hub-client` section (NOT tracked for VM symmetry — hub is always MVM):

| # | Test |
| --- | ------ |
| 1 | test_hub_client_new |
| 2 | test_intent_created_event_deserialization |
| 3 | test_get_intent_events_success |
| 4 | test_get_intent_events_empty |

### docs/intent-frameworks/framework-extension-guide.md

Add chain-clients to the Test Alignment Reference section:

```markdown
### Chain Clients

Shared chain client test alignment:

- See [`chain-clients/extension-checklist.md`](../../chain-clients/extension-checklist.md)
```

---

## Deduplication Summary

Tests that currently exist in multiple crates and will be deduplicated:

| Test | Currently in | After |
| --- | --- | --- |
| get_solver_evm_address_array_format | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_evm_address_hex_string_format | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_evm_address_leading_zero_mismatch | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_svm_address_array_format | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_svm_address_hex_string_format | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_svm_address_leading_zero_mismatch | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_success | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_not_registered | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_empty_hex_string | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_errors_on_unexpected_format | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_ed25519_format | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_errors_on_empty_array | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_errors_on_non_string_element | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_errors_on_invalid_hex | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_errors_on_http_error | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_solver_public_key_rejects_address_without_prefix | coordinator + integrated-gmp | chain-clients-mvm hub tests (once) |
| get_all_escrows_parses_program_accounts | coordinator + integrated-gmp | chain-clients-svm (once) |

Total: 17 duplicate tests eliminated.

---

## Execution Order

1. **Design first**: Finalize `chain-clients/extension-checklist.md` with exact test numbers (28 checklist tests + 31 hub tests)
2. Create `chain-clients/common/` crate: `normalize_intent_id` functions + 7 tests
3. Create `chain-clients/mvm/` crate: extract shared MvmClient + types, write `mvm_client_tests.rs` (checklist tests) and `mvm_client_hub_tests.rs` (hub tests)
4. Create `chain-clients/evm/` crate: extract shared EvmClient + types, write tests per checklist
5. Create `chain-clients/svm/` crate: extract shared SvmClient + types, write tests per checklist
6. Update coordinator: depend on chain-clients-{common,mvm,svm}, delete duplicated code, update extension-checklist.md
7. Update integrated-gmp: depend on chain-clients-{common,mvm,evm,svm}, delete duplicated code, update extension-checklist.md
8. Update solver: depend on chain-clients-{common,mvm,evm,svm}, delete duplicated code, update extension-checklist.md
9. Update framework-extension-guide.md to reference chain-clients checklist
10. Add workspace members to root Cargo.toml
11. Run all tests, verify pass
12. Verify all extension checklists are consistent

## Expected Impact

- ~2500 lines of duplicated source code removed
- 17 duplicate tests eliminated
- 28 checklist-tracked tests + 31 hub-only tests + 7 common tests = 66 tests in chain-clients
- Service checklists simplified (coordinator: 23→0 client tests, integrated-gmp: 22→0, solver: 37→8)
- Adding a new chain client = write once, test once, track in one checklist
- Adding a new VM = follow framework-extension-guide, add column to chain-clients checklist
