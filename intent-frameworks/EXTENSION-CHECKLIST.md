# VM Intent Framework Test Completeness

> **⚠️ IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for VM intent framework contracts (EVM/SVM/MVM). For the complete overview and other frameworks, see the [Framework Extension Guide](../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

Each test file uses independent numbering starting from 1. At the end of the implementation, check that all tests are numbered correctly and match the list below.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## initialization

MVM: `intent-frameworks/mvm/tests/initialization_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/initialization.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/initialization.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should initialize escrow with approver address | ⚠️ | ✅ | ✅ |
| 2 | Should allow requester to create an escrow | ⚠️ | ✅ | ✅ |
| 3 | Should revert if escrow already exists | ⚠️ | ✅ | ✅ |
| 4 | Should revert if amount is zero | ⚠️ | ✅ | ✅ |

## deposit

MVM: `intent-frameworks/mvm/tests/deposit_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/deposit.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/deposit.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should allow requester to create escrow with tokens | ⚠️ | ✅ | ✅ |
| 2 | Should revert if escrow is already claimed | ⚠️ | ✅ | ✅ |
| 3 | Should support multiple escrows with different intent IDs | ⚠️ | ✅ | ✅ |
| 4 | Should set correct expiry timestamp | ⚠️ | ✅ | ✅ |

## claim

MVM: `intent-frameworks/mvm/tests/claim_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/claim.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/claim.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should allow solver to claim with valid approver signature (EVM) / fulfillment proof (SVM) | ⚠️ | ✅ | ✅ |
| 2 | Should revert with invalid signature (EVM) / without requirements (SVM) | ⚠️ | ✅ | ✅ |
| 3 | Should prevent signature replay (EVM) / double fulfillment (SVM) | ⚠️ | ✅ | ✅ |
| 4 | Should revert if escrow already claimed | ⚠️ | ✅ | ✅ |
| 5 | Should revert if escrow does not exist | ⚠️ | ✅ | ✅ |

> **Note:** SVM uses GMP-based claim via `LzReceiveFulfillmentProof` instruction. EVM uses signature-based claim.

## cancel

MVM: `intent-frameworks/mvm/tests/cancel_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/cancel.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/cancel.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should revert if escrow has not expired yet | ⚠️ | ✅ | ✅ |
| 2 | Should allow requester to cancel and reclaim funds after expiry | ⚠️ | ✅ | ✅ |
| 3 | Should revert if not requester | ⚠️ | ✅ | ✅ |
| 4 | Should revert if already claimed | ⚠️ | ✅ | ✅ |
| 5 | Should revert if escrow does not exist | ⚠️ | ✅ | ✅ |

## expiry

MVM: `intent-frameworks/mvm/tests/expiry_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/expiry.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/expiry.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should allow requester to cancel expired escrow | ⚠️ | ✅ | ✅ |
| 2 | Should verify expiry timestamp is stored correctly | ⚠️ | ✅ | ✅ |
| 3 | Should prevent claim on expired escrow (EVM) / allow GMP fulfillment after local expiry (SVM) | ⚠️ | ✅ | ✅ |

> **Note:** SVM honors GMP fulfillment proofs regardless of local expiry (hub is source of truth). Local expiry only affects cancel operation.

## cross-chain

MVM: `intent-frameworks/mvm/tests/cross_chain_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/cross-chain.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/cross_chain.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should handle hex intent ID conversion to uint256/bytes32 | ⚠️ | ✅ | ✅ |
| 2 | Should handle intent ID boundary values | ⚠️ | ✅ | ✅ |
| 3 | Should handle intent ID zero padding correctly | ⚠️ | ✅ | ✅ |
| 4 | Should handle multiple intent IDs from different formats | ⚠️ | ✅ | ✅ |

## edge-cases

MVM: `intent-frameworks/mvm/tests/edge_cases_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/edge-cases.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/edge_cases.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should handle maximum values for amounts | ⚠️ | ✅ | ✅ |
| 2 | Should handle minimum deposit amount | ⚠️ | ✅ | ✅ |
| 3 | Should allow requester to create multiple escrows | ⚠️ | ✅ | ✅ |
| 4 | Should handle gas/compute consumption for large operations | ⚠️ | ✅ | ✅ |
| 5 | Should handle concurrent escrow operations | ⚠️ | ✅ | ✅ |

## error-conditions

MVM: `intent-frameworks/mvm/tests/error_conditions_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/error-conditions.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/error_conditions.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should revert with zero amount in createEscrow | ⚠️ | ✅ | ✅ |
| 2 | Should revert with insufficient token allowance | N/A | ✅ | N/A |
| 3 | Should handle maximum value in createEscrow | ⚠️ | ✅ | ✅ |
| 4 | Should allow native currency escrow creation | N/A | ✅ | N/A |
| 5 | Should revert with native currency amount mismatch | N/A | ✅ | N/A |
| 6 | Should revert when native currency sent with token address | N/A | ✅ | N/A |
| 7 | Should revert with invalid signature length | N/A | ✅ | N/A |
| 8 | Should revert cancel on non-existent escrow | ⚠️ | ✅ | ✅ |
| 9 | Should reject zero solver address | ⚠️ | ✅ | ✅ |
| 10 | Should reject duplicate escrow creation | ⚠️ | ✅ | ✅ |
| 11 | Should reject insufficient token balance | ⚠️ | ✅ | ✅ |

## integration

MVM: `intent-frameworks/mvm/tests/integration_tests.move` ⚠️
EVM: `intent-frameworks/evm/test/integration.test.js`
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/integration.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should complete full deposit to claim workflow | ⚠️ | ✅ | ✅ |
| 2 | Should handle multiple different token types | ⚠️ | ✅ | ✅ |
| 3 | Should emit all events/logs with correct parameters | ⚠️ | ✅ | N/A |
| 4 | Should complete full cancellation workflow | ⚠️ | ✅ | ✅ |

---

## GMP message encoding/decoding test alignment

MVM: `intent-frameworks/mvm/tests/gmp_common_tests.move`
EVM: `intent-frameworks/evm/test/gmp-common/` ⚠️
SVM: `intent-frameworks/svm/programs/gmp-common/tests/gmp_common_tests.rs`

**Per-message-type test symmetry**

Each message type has a symmetric set of tests. The table below shows how test concepts map across types.

| Concept | IntentRequirements (0x01) | EscrowConfirmation (0x02) | FulfillmentProof (0x03) |
| --- | --- | --- | --- |
| Encoded size | 1 | 8 | 13 |
| Discriminator byte | 2 | 9 | 14 |
| Encode/decode roundtrip | 3 | 10 | 15 |
| Big-endian amount | 4 | 11 | 16 |
| Big-endian second u64 | 5 (expiry) | N/A | 16 (timestamp) |
| Field offsets | 6 | 12 | 17 |
| EVM address encoding | 7 | N/A | N/A |

**IntentRequirements (0x01)**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_intent_requirements_encode_size | ✅ | ⚠️ | ✅ |
| 2 | test_intent_requirements_discriminator | ✅ | ⚠️ | ✅ |
| 3 | test_intent_requirements_roundtrip | ✅ | ⚠️ | ✅ |
| 4 | test_intent_requirements_big_endian_amount | ✅ | ⚠️ | ✅ |
| 5 | test_intent_requirements_big_endian_expiry | ✅ | ⚠️ | ✅ |
| 6 | test_intent_requirements_field_offsets | ✅ | ⚠️ | ✅ |
| 7 | test_intent_requirements_evm_address | ✅ | ⚠️ | ✅ |

**EscrowConfirmation (0x02)**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 8 | test_escrow_confirmation_encode_size | ✅ | ⚠️ | ✅ |
| 9 | test_escrow_confirmation_discriminator | ✅ | ⚠️ | ✅ |
| 10 | test_escrow_confirmation_roundtrip | ✅ | ⚠️ | ✅ |
| 11 | test_escrow_confirmation_big_endian_amount | ✅ | ⚠️ | ✅ |
| 12 | test_escrow_confirmation_field_offsets | ✅ | ⚠️ | ✅ |

**FulfillmentProof (0x03)**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 13 | test_fulfillment_proof_encode_size | ✅ | ⚠️ | ✅ |
| 14 | test_fulfillment_proof_discriminator | ✅ | ⚠️ | ✅ |
| 15 | test_fulfillment_proof_roundtrip | ✅ | ⚠️ | ✅ |
| 16 | test_fulfillment_proof_big_endian_fields | ✅ | ⚠️ | ✅ |
| 17 | test_fulfillment_proof_field_offsets | ✅ | ⚠️ | ✅ |

**Peek Message Type**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 18 | test_peek_intent_requirements | ✅ | ⚠️ | ✅ |
| 19 | test_peek_escrow_confirmation | ✅ | ⚠️ | ✅ |
| 20 | test_peek_fulfillment_proof | ✅ | ⚠️ | ✅ |

**Error Conditions**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 21 | test_reject_wrong_discriminator | ✅ | ⚠️ | ✅ |
| 22 | test_reject_wrong_length | ✅ | ⚠️ | ✅ |
| 23 | test_reject_empty_buffer | ✅ | ⚠️ | ✅ |
| 24 | test_peek_reject_empty_buffer | ✅ | ⚠️ | ✅ |
| 25 | test_peek_reject_unknown_type | ✅ | ⚠️ | ✅ |
| 26 | test_reject_wrong_discriminator_escrow_confirmation | ✅ | ⚠️ | ✅ |
| 27 | test_reject_wrong_discriminator_fulfillment_proof | ✅ | ⚠️ | ✅ |
| 28 | test_reject_wrong_length_escrow_confirmation | ✅ | ⚠️ | ✅ |
| 29 | test_reject_wrong_length_fulfillment_proof | ✅ | ⚠️ | ✅ |
| 30 | test_reject_off_by_one_length | ✅ | ⚠️ | ✅ |

**Known Byte Sequences**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 31 | test_decode_known_intent_requirements_bytes | ✅ | ⚠️ | ✅ |
| 32 | test_decode_known_escrow_confirmation_bytes | ✅ | ⚠️ | ✅ |
| 33 | test_decode_known_fulfillment_proof_bytes | ✅ | ⚠️ | ✅ |

**Boundary Conditions**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 34 | test_max_u64_amount_roundtrip | ✅ | ⚠️ | ✅ |
| 35 | test_zero_solver_addr_means_any | ✅ | ⚠️ | ✅ |

**Cross-Chain Encoding Compatibility**

These tests verify that encoding produces identical bytes across all frameworks. Expected bytes are defined in `intent-frameworks/common/testing/gmp-encoding-test-vectors.json`.

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 36 | test_cross_chain_encoding_intent_requirements | ✅ | ⚠️ | ✅ |
| 37 | test_cross_chain_encoding_escrow_confirmation | ✅ | ⚠️ | ✅ |
| 38 | test_cross_chain_encoding_fulfillment_proof | ✅ | ⚠️ | ✅ |
| 39 | test_cross_chain_encoding_intent_requirements_zeros | ✅ | ⚠️ | ✅ |
| 40 | test_cross_chain_encoding_intent_requirements_max | ✅ | ⚠️ | ✅ |

---

## Outflow Validator test alignment

Outflow validator handles the connected chain side of outflow intents (tokens flow OUT of Movement TO connected chain). The solver fulfills on the connected chain, and the validator sends proof back to the hub.

### Outflow Validator Interface Tests

MVM: `intent-frameworks/mvm/tests/interface_tests.move`
EVM: `intent-frameworks/evm/test/outflow-validator/` ⚠️
SVM: `intent-frameworks/svm/programs/outflow-validator/tests/interface_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_initialize_instruction_roundtrip | N/A | ⚠️ | ✅ |
| 2 | test_receive_instruction_roundtrip | ✅ | ⚠️ | ✅ |
| 3 | test_fulfill_intent_instruction_roundtrip | ⚠️ | ⚠️ | ✅ |
| 4 | test_intent_requirements_account_roundtrip | N/A | ⚠️ | ✅ |
| 5 | test_config_account_roundtrip | N/A | ⚠️ | ✅ |
| 6 | test_error_conversion | N/A | ⚠️ | ✅ |
| 7 | test_error_codes_unique | N/A | ⚠️ | ✅ |

### Outflow Validator Integration Tests

MVM: `intent-frameworks/mvm/tests/outflow_validator_tests.move`
EVM: ⚠️
SVM: `intent-frameworks/svm/programs/outflow-validator/tests/validator_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_initialize_creates_config | ✅ | ⚠️ | ✅ |
| 2 | test_initialize_rejects_double_init | ✅ | ⚠️ | ✅ |
| 3 | test_receive_stores_requirements | ✅ | ⚠️ | ✅ |
| 4 | test_receive_idempotent | ✅ | ⚠️ | ✅ |
| 5 | test_receive_rejects_untrusted_source | ✅ | ⚠️ | ✅ |
| 6 | test_receive_rejects_invalid_payload | ✅ | ⚠️ | ✅ |
| 7 | test_fulfill_intent_rejects_already_fulfilled | ✅ | ⚠️ | ✅ |
| 8 | test_fulfill_intent_rejects_expired | ✅ | ⚠️ | ✅ |
| 9 | test_fulfill_intent_rejects_unauthorized_solver | ✅ | ⚠️ | ✅ |
| 10 | test_fulfill_intent_rejects_token_mismatch | ✅ | ⚠️ | ✅ |
| 11 | test_fulfill_intent_rejects_requirements_not_found | ✅ | ⚠️ | ✅ |
| 12 | test_fulfill_intent_rejects_recipient_mismatch | ✅ | ⚠️ | ✅ |
| 13 | test_fulfill_intent_succeeds | ✅ | ⚠️ | ✅ |

---

## Native GMP Endpoint test alignment

Native GMP endpoint provides a standardized interface for cross-chain messaging. Can be used for local testing, CI, or production with your own relay infrastructure. In production, this can also be replaced by LZ's endpoint.

### Native GMP Endpoint Interface Tests

MVM: `intent-frameworks/mvm/tests/native_gmp_endpoint_tests.move`
EVM: `intent-frameworks/evm/test/native-gmp-endpoint/` ⚠️
SVM: `intent-frameworks/svm/programs/native-gmp-endpoint/tests/endpoint_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_send_instruction_serialization | N/A | ⚠️ | ✅ |
| 2 | test_deliver_message_instruction_serialization | N/A | ⚠️ | ✅ |
| 3 | test_initialize_instruction_serialization | N/A | ⚠️ | ✅ |
| 4 | test_add_relay_instruction_serialization | N/A | ⚠️ | ✅ |
| 5 | test_set_trusted_remote_instruction_serialization | N/A | ⚠️ | ✅ |
| 6 | test_set_routing_instruction_serialization | N/A | ⚠️ | ✅ |
| 7 | test_routing_config_serialization | N/A | ⚠️ | ✅ |
| 8 | test_config_account_serialization | N/A | ⚠️ | ✅ |
| 9 | test_relay_account_serialization | N/A | ⚠️ | ✅ |
| 10 | test_trusted_remote_account_serialization | N/A | ⚠️ | ✅ |
| 11 | test_outbound_nonce_account | N/A | ⚠️ | ✅ |
| 12 | test_inbound_nonce_account_replay_detection | N/A | ⚠️ | ✅ |
| 13 | test_error_conversion | N/A | ⚠️ | ✅ |
| 14 | test_error_codes_unique | N/A | ⚠️ | ✅ |
| 15 | test_send_updates_nonce_state | ✅ | ⚠️ | ✅ |
| 16 | test_deliver_message_calls_receiver | ✅ | ⚠️ | ✅ |
| 17 | test_deliver_message_rejects_replay | ✅ | ⚠️ | ✅ |
| 18 | test_deliver_message_rejects_unauthorized_relay | ✅ | ⚠️ | ✅ |
| 19 | test_deliver_message_authorized_relay | ✅ | ⚠️ | ✅ |
| 20 | test_deliver_message_rejects_untrusted_remote | ✅ | ⚠️ | ✅ |
| 21 | test_deliver_message_rejects_no_trusted_remote | ✅ | ⚠️ | ✅ |
| 22 | test_set_trusted_remote_unauthorized | ✅ | ⚠️ | ✅ |
| 23 | test_deliver_message_rejects_lower_nonce | ✅ | ⚠️ | ✅ |
| 24 | test_deliver_intent_requirements_stores_in_both_handlers | ✅ | N/A | N/A |
| 25 | test_deliver_intent_requirements_fails_without_outflow_init | ✅ | N/A | N/A |
| 26 | test_fulfillment_proof_routes_to_intent_escrow | N/A | N/A | ✅ |
| 27 | test_fulfillment_proof_fails_with_insufficient_accounts | N/A | N/A | ✅ |

---

## Inflow Escrow GMP test alignment

Inflow escrow handles the connected chain side of inflow intents (tokens locked on connected chain, desired on hub). The user locks tokens in escrow on the connected chain (SVM/EVM/MVM), the solver fulfills by delivering tokens on the hub (Movement), and the hub sends fulfillment proof via GMP to release the escrowed tokens to the solver.

### Inflow Escrow GMP Tests

MVM: `intent-frameworks/mvm/tests/inflow_escrow_gmp_tests.move`
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/` ⚠️
SVM: `intent-frameworks/svm/programs/intent_escrow/tests/gmp.rs`

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_set_gmp_config / test_initialize_creates_config | ✅ | ⚠️ | ✅ |
| 2 | test_set_gmp_config_rejects_unauthorized / test_initialize_rejects_double_init | ✅ | ⚠️ | ✅ |
| 3 | test_receive_requirements_stores_requirements | ✅ | ⚠️ | ✅ |
| 4 | test_receive_requirements_idempotent | ✅ | ⚠️ | ✅ |
| 5 | test_receive_requirements_rejects_untrusted_source | ✅ | ⚠️ | ✅ |
| 6 | test_receive_fulfillment_proof_releases_escrow (SVM) / test_receive_fulfillment_proof_marks_fulfilled (MVM) | ✅ | ⚠️ | ✅ |
| 7 | test_receive_fulfillment_rejects_untrusted_source | ✅ | ⚠️ | ✅ |
| 8 | test_receive_fulfillment_proof_rejects_already_fulfilled | ✅ | ⚠️ | ✅ |
| 9 | test_create_escrow_validates_against_requirements / test_create_escrow_validates_requirements | ✅ | ⚠️ | ✅ |
| 10 | test_create_escrow_rejects_amount_mismatch | ✅ | ⚠️ | ✅ |
| 11 | test_create_escrow_rejects_token_mismatch | ✅ | ⚠️ | ✅ |
| 12 | test_create_escrow_sends_escrow_confirmation | ✅ | ⚠️ | ✅ |
| 13 | test_full_inflow_gmp_workflow | ✅ | ⚠️ | ✅ |
| 14 | test_create_escrow_rejects_no_requirements | ✅ | ⚠️ | N/A |
| 15 | test_create_escrow_rejects_double_create | ✅ | ⚠️ | N/A |
| 16 | test_release_escrow_succeeds_after_fulfillment | ✅ | ⚠️ | N/A |
| 17 | test_release_escrow_rejects_without_fulfillment | ✅ | ⚠️ | N/A |
| 18 | test_release_escrow_rejects_unauthorized_solver | ✅ | ⚠️ | N/A |
| 19 | test_release_escrow_rejects_double_release | ✅ | ⚠️ | N/A |
| 20 | test_generic_lz_receive_routes_requirements | N/A | N/A | ✅ |
| 21 | test_generic_lz_receive_routes_fulfillment_proof | N/A | N/A | ✅ |
| 22 | test_generic_lz_receive_rejects_unknown_message_type | N/A | N/A | ✅ |
