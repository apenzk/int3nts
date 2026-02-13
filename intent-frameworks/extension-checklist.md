# VM Intent Framework Test Completeness

> **IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for VM intent framework contracts (EVM/SVM/MVM). For the complete overview and other frameworks, see the [Framework Extension Guide](../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

Each test file uses independent numbering starting from 1. At the end of the implementation, check that all tests are numbered correctly and match the list below.

**Legend:** [x] = Implemented | [ ] = Not yet implemented | N/A = Not applicable to platform

## initialization

MVM: `intent-frameworks/mvm/tests/initialization_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/initialization.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/initialization.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should initialize escrow with approver address | [ ] | [x] | [x] |
| 2 | Should allow requester to create an escrow | [ ] | [x] | [x] |
| 3 | Should revert if escrow already exists | [ ] | [x] | [x] |
| 4 | Should revert if amount is zero | [ ] | [x] | [x] |
| 5 | Should revert if amount is zero (GMP variant) | [ ] | [x] | [ ] |

## deposit

MVM: `intent-frameworks/mvm/tests/deposit_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/deposit.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/deposit.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should allow requester to create escrow with tokens | [ ] | [x] | [x] |
| 2 | Should revert if escrow is already claimed | [ ] | [x] | [x] |
| 3 | Should support multiple escrows with different intent IDs | [ ] | [x] | [x] |
| 4 | Should set correct expiry timestamp | [ ] | [x] | [x] |

## claim

MVM: `intent-frameworks/mvm/tests/claim_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/fulfillment.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/claim.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should allow solver to claim with valid approver signature (EVM) / fulfillment proof (SVM) | [ ] | [x] | [x] |
| 2 | Should revert with invalid signature (EVM) / without requirements (SVM) | [ ] | [x] | [x] |
| 3 | Should prevent signature replay (EVM) / double fulfillment (SVM) | [ ] | [x] | [x] |
| 4 | Should revert if escrow already claimed | [ ] | [x] | [x] |
| 5 | Should revert if escrow does not exist | [ ] | [x] | [x] |

> **Note:** SVM uses GMP-based claim via `GmpReceiveFulfillmentProof` instruction. EVM uses signature-based claim.

## cancel

MVM: `intent-frameworks/mvm/tests/cancel_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/cancel.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/cancel.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should revert if escrow has not expired yet | [ ] | [x] | [x] |
| 2 | Should allow requester to cancel and reclaim funds after expiry | [ ] | [x] | [x] |
| 3 | Should revert if not requester | [ ] | [x] | [x] |
| 4 | Should revert if already claimed | [ ] | [x] | [x] |
| 5 | Should revert if escrow does not exist | [ ] | [x] | [x] |

## expiry

MVM: `intent-frameworks/mvm/tests/expiry_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/expiry.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/expiry.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should allow requester to cancel expired escrow | [ ] | [x] | [x] |
| 2 | Should verify expiry timestamp is stored correctly | [ ] | [x] | [x] |
| 3 | Should prevent claim on expired escrow (EVM) / allow GMP fulfillment after local expiry (SVM) | [ ] | [x] | [x] |

> **Note:** SVM honors GMP fulfillment proofs regardless of local expiry (hub is source of truth). Local expiry only affects cancel operation.

## cross-chain

MVM: `intent-frameworks/mvm/tests/cross_chain_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/cross-chain.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/cross_chain.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should handle hex intent ID conversion to uint256/bytes32 | [ ] | [x] | [x] |
| 2 | Should handle intent ID boundary values | [ ] | [x] | [x] |
| 3 | Should handle intent ID zero padding correctly | [ ] | [x] | [x] |
| 4 | Should handle multiple intent IDs from different formats | [ ] | [x] | [x] |

## edge-cases

MVM: `intent-frameworks/mvm/tests/edge_cases_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/edge-cases.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/edge_cases.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should handle maximum values for amounts | [ ] | [x] | [x] |
| 2 | Should handle minimum deposit amount | [ ] | [x] | [x] |
| 3 | Should allow requester to create multiple escrows | [ ] | [x] | [x] |
| 4 | Should handle gas/compute consumption for large operations | [ ] | [x] | [x] |
| 5 | Should handle concurrent escrow operations | [ ] | [x] | [x] |

## error-conditions

MVM: `intent-frameworks/mvm/tests/error_conditions_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/error-conditions.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/error_conditions.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should revert with zero amount in createEscrow | [ ] | [x] | [x] |
| 2 | Should revert with insufficient token allowance | [ ] | [x] | N/A |
| 3 | Should handle maximum value in createEscrow | [ ] | [ ] | [x] |
| 4 | Should allow native currency escrow creation | [ ] | [ ] | N/A |
| 5 | Should revert with native currency amount mismatch | [ ] | [ ] | N/A |
| 6 | Should revert when native currency sent with token address | [ ] | [ ] | N/A |
| 7 | Should revert with invalid signature length | [ ] | [ ] | N/A |
| 8 | Should revert cancel on non-existent escrow | [ ] | [x] | [x] |
| 9 | Should reject zero solver address | [ ] | [ ] | [x] |
| 10 | Should reject duplicate escrow creation | [ ] | [x] | [x] |
| 11 | Should reject insufficient token balance | [ ] | [x] | [x] |

> **Note:** EVM error-conditions file uses GMP-specific errors (requirements not found, amount/token/requester mismatch, expired intent) at positions 3-7. Template tests 3-7 are not applicable to GMP escrow.

## integration

MVM: `intent-frameworks/mvm/tests/integration_tests.move` (FILE NOT FOUND)
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/integration.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/integration.rs`

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | Should complete full deposit to claim workflow | [ ] | [x] | [x] |
| 2 | Should handle multiple different token types | [ ] | [x] | [x] |
| 3 | Should emit all events/logs with correct parameters | [ ] | [x] | N/A |
| 4 | Should complete full cancellation workflow | [ ] | [x] | [x] |
| 5 | Should require requirements before escrow creation | [ ] | [x] | N/A |
| 6 | Should handle multiple participants with independent escrows | [ ] | [x] | N/A |

---

## GMP message encoding/decoding test alignment

MVM: `intent-frameworks/mvm/intent-gmp/tests/gmp_common_tests.move`
EVM: `intent-frameworks/evm/test/messages.test.js`
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
| 1 | test_intent_requirements_encode_size | [x] | [x] | [x] |
| 2 | test_intent_requirements_discriminator | [x] | [x] | [x] |
| 3 | test_intent_requirements_roundtrip | [x] | [x] | [x] |
| 4 | test_intent_requirements_big_endian_amount | [x] | [x] | [x] |
| 5 | test_intent_requirements_big_endian_expiry | [x] | [x] | [x] |
| 6 | test_intent_requirements_field_offsets | [x] | [x] | [x] |
| 7 | test_intent_requirements_evm_address | [x] | [x] | [x] |

**EscrowConfirmation (0x02)**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 8 | test_escrow_confirmation_encode_size | [x] | [x] | [x] |
| 9 | test_escrow_confirmation_discriminator | [x] | [x] | [x] |
| 10 | test_escrow_confirmation_roundtrip | [x] | [x] | [x] |
| 11 | test_escrow_confirmation_big_endian_amount | [x] | [x] | [x] |
| 12 | test_escrow_confirmation_field_offsets | [x] | [x] | [x] |

**FulfillmentProof (0x03)**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 13 | test_fulfillment_proof_encode_size | [x] | [x] | [x] |
| 14 | test_fulfillment_proof_discriminator | [x] | [x] | [x] |
| 15 | test_fulfillment_proof_roundtrip | [x] | [x] | [x] |
| 16 | test_fulfillment_proof_big_endian_fields | [x] | [x] | [x] |
| 17 | test_fulfillment_proof_field_offsets | [x] | [x] | [x] |

**Peek Message Type**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 18 | test_peek_intent_requirements | [x] | [x] | [x] |
| 19 | test_peek_escrow_confirmation | [x] | [x] | [x] |
| 20 | test_peek_fulfillment_proof | [x] | [x] | [x] |

**Error Conditions**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 21 | test_reject_wrong_discriminator | [x] | [x] | [x] |
| 22 | test_reject_wrong_length | [x] | [x] | [x] |
| 23 | test_reject_empty_buffer | [x] | [x] | [x] |
| 24 | test_peek_reject_empty_buffer | [x] | [x] | [x] |
| 25 | test_peek_reject_unknown_type | [x] | [x] | [x] |
| 26 | test_reject_wrong_discriminator_escrow_confirmation | [x] | [x] | [x] |
| 27 | test_reject_wrong_discriminator_fulfillment_proof | [x] | [x] | [x] |
| 28 | test_reject_wrong_length_escrow_confirmation | [x] | [x] | [x] |
| 29 | test_reject_wrong_length_fulfillment_proof | [x] | [x] | [x] |
| 30 | test_reject_off_by_one_length | [x] | [x] | [x] |

**Known Byte Sequences**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 31 | test_decode_known_intent_requirements_bytes | [x] | [ ] | [x] |
| 32 | test_decode_known_escrow_confirmation_bytes | [x] | [ ] | [x] |
| 33 | test_decode_known_fulfillment_proof_bytes | [x] | [ ] | [x] |

**Boundary Conditions**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 34 | test_max_u64_amount_roundtrip | [x] | [x] | [x] |
| 35 | test_zero_solver_addr_means_any | [x] | [x] | [x] |

**Address Conversion & Constants**

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 41 | test_address_to_bytes32 | N/A | [x] | N/A |
| 42 | test_bytes32_to_address | N/A | [x] | N/A |
| 43 | test_address_conversion_roundtrip | N/A | [x] | N/A |
| 44 | test_message_type_constants | N/A | [x] | N/A |
| 45 | test_message_size_constants | N/A | [x] | N/A |

**Cross-Chain Encoding Compatibility**

These tests verify that encoding produces identical bytes across all frameworks. Expected bytes are defined in `intent-frameworks/common/testing/gmp-encoding-test-vectors.json`.

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 36 | test_cross_chain_encoding_intent_requirements | [x] | [ ] | [x] |
| 37 | test_cross_chain_encoding_escrow_confirmation | [x] | [ ] | [x] |
| 38 | test_cross_chain_encoding_fulfillment_proof | [x] | [ ] | [x] |
| 39 | test_cross_chain_encoding_intent_requirements_zeros | [x] | [ ] | [x] |
| 40 | test_cross_chain_encoding_intent_requirements_max | [x] | [ ] | [x] |

---

## Outflow Validator test alignment

Outflow validator handles the connected chain side of outflow intents (tokens flow OUT of Movement TO connected chain). The solver fulfills on the connected chain, and the validator sends proof back to the hub.

### Outflow Validator Interface Tests

MVM: `intent-frameworks/mvm/intent-connected/tests/interface_tests.move`
EVM: `intent-frameworks/evm/test/outflow-validator.test.js` (no separate interface test file)
SVM: `intent-frameworks/svm/programs/intent-outflow-validator/tests/interface_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_initialize_instruction_roundtrip | N/A | N/A | [x] |
| 2 | test_receive_instruction_roundtrip | [x] | N/A | [x] |
| 3 | test_fulfill_intent_instruction_roundtrip | [ ] | N/A | [x] |
| 4 | test_intent_requirements_account_roundtrip | N/A | N/A | [x] |
| 5 | test_config_account_roundtrip | N/A | N/A | [x] |
| 6 | test_error_conversion | N/A | N/A | [x] |
| 7 | test_error_codes_unique | N/A | N/A | [x] |

### Outflow Validator Integration Tests

MVM: `intent-frameworks/mvm/intent-connected/tests/intent_outflow_validator_tests.move`
EVM: `intent-frameworks/evm/test/outflow-validator.test.js`
SVM: `intent-frameworks/svm/programs/intent-outflow-validator/tests/validator_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_initialize_creates_config | [x] | [x] | [x] |
| 2 | test_initialize_rejects_double_init | [x] | N/A | [x] |
| 3 | test_receive_stores_requirements | [x] | [x] | [x] |
| 4 | test_receive_idempotent | [x] | [x] | [x] |
| 5 | test_receive_rejects_unauthorized_source | [x] | [x] | [x] |
| 6 | test_receive_rejects_invalid_payload | [x] | N/A | [x] |
| 7 | test_fulfill_intent_rejects_already_fulfilled | [x] | [x] | [x] |
| 8 | test_fulfill_intent_rejects_expired | [x] | [x] | [x] |
| 9 | test_fulfill_intent_rejects_unauthorized_solver | [x] | [x] | [x] |
| 10 | test_fulfill_intent_rejects_token_mismatch | [x] | [x] | [x] |
| 11 | test_fulfill_intent_rejects_requirements_not_found | [x] | [x] | [x] |
| 12 | test_fulfill_intent_rejects_recipient_mismatch | [x] | N/A | [x] |
| 13 | test_fulfill_intent_succeeds | [x] | [x] | [x] |
| 14 | test_initialize_rejects_zero_endpoint | N/A | [x] | N/A |
| 15 | test_allow_any_solver_zero_address | N/A | [x] | N/A |
| 16 | test_send_fulfillment_proof_to_hub | N/A | [x] | N/A |
| 17 | test_tokens_transferred_to_requester | N/A | [x] | N/A |
| 18 | test_complete_outflow_workflow | N/A | [x] | N/A |
| 19 | test_update_hub_config_succeeds | [ ] | [ ] | [x] |
| 20 | test_update_hub_config_rejects_non_admin | [ ] | [ ] | [x] |
| 21 | test_update_hub_config_then_gmp_receive | [ ] | [ ] | [x] |

---

## Integrated GMP Endpoint test alignment

Integrated GMP endpoint provides a standardized interface for cross-chain messaging. Can be used for local testing, CI, or production with your own relay infrastructure. In production, this can also be replaced by LZ's endpoint.

### Integrated GMP Endpoint Interface Tests

MVM: `intent-frameworks/mvm/intent-connected/tests/intent_gmp_tests.move`
EVM: `intent-frameworks/evm/test/integrated-gmp-endpoint/intent-gmp.test.js`
SVM: `intent-frameworks/svm/programs/intent-gmp/tests/endpoint_tests.rs`

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_send_instruction_serialization | N/A | N/A | [x] |
| 2 | test_deliver_message_instruction_serialization | N/A | N/A | [x] |
| 3 | test_initialize_instruction_serialization | N/A | N/A | [x] |
| 4 | test_add_relay_instruction_serialization | N/A | N/A | [x] |
| 5 | test_set_remote_gmp_endpoint_addr_instruction_serialization | N/A | N/A | [x] |
| 6 | test_set_routing_instruction_serialization | N/A | N/A | [x] |
| 7 | test_routing_config_serialization | N/A | N/A | [x] |
| 8 | test_config_account_serialization | N/A | N/A | [x] |
| 9 | test_relay_account_serialization | N/A | N/A | [x] |
| 10 | test_remote_gmp_endpoint_account_serialization | N/A | N/A | [x] |
| 11 | test_outbound_nonce_account | N/A | N/A | [x] |
| 12 | test_delivered_message_serialization | N/A | N/A | [x] |
| 13 | test_error_conversion | N/A | N/A | [x] |
| 14 | test_error_codes_unique | N/A | N/A | [x] |
| 15 | test_send_updates_nonce_state | [x] | [x] | [x] |
| 16 | test_deliver_message_calls_receiver | [ ] | [x] | [x] |
| 17 | test_deliver_message_rejects_replay (EVM/SVM revert, MVM idempotent) | [x] | [x] | [x] |
| 18 | test_deliver_message_rejects_unauthorized_relay | [x] | [x] | [x] |
| 19 | test_deliver_message_authorized_relay | [x] | [x] | [x] |
| 20 | test_deliver_message_rejects_unknown_remote_gmp_endpoint | [x] | [x] | [x] |
| 21 | test_deliver_message_rejects_no_remote_gmp_endpoint | [x] | [x] | [x] |
| 22 | test_set_remote_gmp_endpoint_addr_unauthorized | [x] | [x] | [x] |
| 23 | test_deliver_message_different_msg_type_succeeds (dedupe per intent_id+msg_type) | [x] | [x] | [x] |
| 24 | test_deliver_intent_requirements_stores_in_both_handlers | [x] | N/A | N/A |
| 25 | test_add_relay_rejects_non_admin | [x] | [x] | [x] |
| 26 | test_remove_relay_rejects_non_admin | [x] | [x] | [x] |
| 27 | test_deliver_intent_requirements_fails_without_outflow_init | [x] | N/A | N/A |
| 28 | test_fulfillment_proof_routes_to_intent_escrow | N/A | N/A | [x] |
| 29 | test_fulfillment_proof_fails_with_insufficient_accounts | N/A | N/A | [x] |
| 30 | test_initialize_creates_config | N/A | [x] | N/A |
| 31 | test_initialize_sets_nonce | N/A | [x] | N/A |
| 32 | test_initialize_rejects_zero_admin | N/A | [x] | N/A |
| 33 | test_add_relay | N/A | [x] | N/A |
| 34 | test_remove_relay | N/A | [x] | N/A |
| 35 | test_reject_duplicate_relay | N/A | [x] | N/A |
| 36 | test_reject_removing_non_existent_relay | N/A | [x] | N/A |
| 37 | test_set_remote_gmp_endpoint_addr | N/A | [x] | N/A |
| 38 | test_add_remote_gmp_endpoint_addr | N/A | [x] | N/A |
| 39 | test_has_remote_gmp_endpoint | N/A | [x] | N/A |
| 40 | test_no_remote_gmp_endpoint | N/A | [x] | N/A |
| 41 | test_deliver_fulfillment_proof_routes | N/A | [x] | N/A |
| 42 | test_reject_unknown_message_type | N/A | [x] | N/A |
| 43 | test_emit_message_delivered | N/A | [x] | N/A |
| 44 | test_is_message_delivered | N/A | [x] | N/A |
| 45 | test_emit_message_sent | N/A | [x] | N/A |
| 46 | test_only_handlers_can_send | N/A | [x] | N/A |
| 47 | test_set_escrow_handler | N/A | [x] | N/A |
| 48 | test_set_outflow_handler | N/A | [x] | N/A |
| 49 | test_route_to_both_handlers | N/A | [x] | N/A |
| 50 | test_fulfillment_proof_requires_escrow_handler | N/A | [x] | N/A |

---

## Inflow Escrow GMP test alignment

Inflow escrow handles the connected chain side of inflow intents (tokens locked on connected chain, desired on hub). The user locks tokens in escrow on the connected chain (SVM/EVM/MVM), the solver fulfills by delivering tokens on the hub (Movement), and the hub sends fulfillment proof via GMP to release the escrowed tokens to the solver.

### Inflow Escrow GMP Tests

MVM: `intent-frameworks/mvm/intent-connected/tests/intent_inflow_escrow_tests.move`
EVM: `intent-frameworks/evm/test/inflow-escrow-gmp/escrow-gmp.test.js`
SVM: `intent-frameworks/svm/programs/intent_inflow_escrow/tests/gmp.rs`

| # | Test | MVM | EVM | SVM |
| --- | --- | --- | --- | --- |
| 1 | test_set_gmp_config / test_initialize_creates_config | [x] | [x] | [x] |
| 2 | test_set_gmp_config_rejects_unauthorized / test_initialize_rejects_double_init | [x] | [x] | [x] |
| 3 | test_receive_requirements_stores_requirements | [x] | [x] | [x] |
| 4 | test_receive_requirements_idempotent | [x] | [x] | [x] |
| 5 | test_receive_requirements_rejects_unauthorized_source | [x] | [x] | [x] |
| 6 | test_receive_fulfillment_proof_releases_escrow (SVM) / test_receive_fulfillment_proof_marks_fulfilled (MVM) | [x] | [x] | [x] |
| 7 | test_receive_fulfillment_rejects_unauthorized_source | [x] | [x] | [x] |
| 8 | test_receive_fulfillment_proof_rejects_already_fulfilled | [x] | [x] | [x] |
| 9 | test_create_escrow_validates_against_requirements / test_create_escrow_validates_requirements | [x] | [x] | [x] |
| 10 | test_create_escrow_rejects_amount_mismatch | [x] | [x] | [x] |
| 11 | test_create_escrow_rejects_token_mismatch | [x] | [x] | [x] |
| 12 | test_create_escrow_sends_escrow_confirmation | [x] | [x] | [x] |
| 13 | test_full_inflow_gmp_workflow | [x] | [x] | [x] |
| 14 | test_create_escrow_rejects_no_requirements | [x] | [x] | N/A |
| 15 | test_create_escrow_rejects_double_create | [x] | [x] | N/A |
| 16 | test_release_escrow_succeeds_after_fulfillment | [x] | N/A | N/A |
| 17 | test_release_escrow_rejects_without_fulfillment | N/A | N/A | N/A |
| 18 | test_release_escrow_rejects_unauthorized_solver | N/A | N/A | N/A |
| 19 | test_release_escrow_rejects_double_release | [x] | N/A | N/A |
| 20 | test_generic_gmp_receive_routes_requirements | N/A | N/A | [x] |
| 21 | test_generic_gmp_receive_routes_fulfillment_proof | N/A | N/A | [x] |
| 22 | test_generic_gmp_receive_rejects_unknown_message_type | N/A | N/A | [x] |
| 23 | test_reject_direct_call | N/A | [x] | N/A |
| 24 | test_create_escrow_rejects_requester_mismatch | N/A | [x] | N/A |
| 25 | test_create_escrow_rejects_expired_intent | N/A | [x] | N/A |
| 26 | test_tokens_transferred_to_escrow | N/A | [x] | N/A |
| 27 | test_emit_events_on_release | N/A | [x] | N/A |
