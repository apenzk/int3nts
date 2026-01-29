# VM Intent Framework Test Completeness

> **⚠️ IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for VM intent framework contracts (EVM/SVM). For the complete overview and other frameworks, see the [Framework Extension Guide](../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

Escrow test alignment for VM intent framework contracts:

- `intent-frameworks/evm/test/`
- `intent-frameworks/svm/programs/intent_escrow/tests/`

Each test file uses independent numbering starting from 1. At the end of the implementation, check that all tests are numbered correctly and match the list below.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## initialization.test.js / initialization.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should initialize escrow with approver address | ✅ | ✅ |
| 2 | Should allow requester to create an escrow | ✅ | ✅ |
| 3 | Should revert if escrow already exists | ✅ | ✅ |
| 4 | Should revert if amount is zero | ✅ | ✅ |

## deposit.test.js / deposit.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should allow requester to create escrow with tokens | ✅ | ✅ |
| 2 | Should revert if escrow is already claimed | ✅ | ✅ |
| 3 | Should support multiple escrows with different intent IDs | ✅ | ✅ |
| 4 | Should set correct expiry timestamp | ✅ | ✅ |

## claim.test.js / claim.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should allow solver to claim with valid approver signature | ✅ | ✅ |
| 2 | Should revert with invalid signature | ✅ | ✅ |
| 3 | Should prevent signature replay across different intent_ids | ✅ | ✅ |
| 4 | Should revert if escrow already claimed | ✅ | ✅ |
| 5 | Should revert if escrow does not exist | ✅ | ✅ |

## cancel.test.js / cancel.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should revert if escrow has not expired yet | ✅ | ✅ |
| 2 | Should allow requester to cancel and reclaim funds after expiry | ✅ | ✅ |
| 3 | Should revert if not requester | ✅ | ✅ |
| 4 | Should revert if already claimed | ✅ | ✅ |
| 5 | Should revert if escrow does not exist | ✅ | ✅ |

## expiry.test.js / expiry.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should allow requester to cancel expired escrow | ✅ | ✅ |
| 2 | Should verify expiry timestamp is stored correctly | ✅ | ✅ |
| 3 | Should prevent claim on expired escrow | ✅ | ✅ |

## cross-chain.test.js / cross_chain.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should handle hex intent ID conversion to uint256/bytes32 | ✅ | ✅ |
| 2 | Should handle intent ID boundary values | ✅ | ✅ |
| 3 | Should handle intent ID zero padding correctly | ✅ | ✅ |
| 4 | Should handle multiple intent IDs from different formats | ✅ | ✅ |

## edge-cases.test.js / edge_cases.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should handle maximum values for amounts | ✅ | ✅ |
| 2 | Should handle minimum deposit amount | ✅ | ✅ |
| 3 | Should allow requester to create multiple escrows | ✅ | ✅ |
| 4 | Should handle gas/compute consumption for large operations | ✅ | ✅ |
| 5 | Should handle concurrent escrow operations | ✅ | ✅ |

## error-conditions.test.js / error_conditions.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should revert with zero amount in createEscrow | ✅ | ✅ |
| 2 | Should revert with insufficient token allowance | ✅ | N/A |
| 3 | Should handle maximum value in createEscrow | ✅ | ✅ |
| 4 | Should allow native currency escrow creation | ✅ | N/A |
| 5 | Should revert with native currency amount mismatch | ✅ | N/A |
| 6 | Should revert when native currency sent with token address | ✅ | N/A |
| 7 | Should revert with invalid signature length | ✅ | N/A |
| 8 | Should revert cancel on non-existent escrow | ✅ | ✅ |
| 9 | Should reject zero solver address | ✅ | ✅ |
| 10 | Should reject duplicate escrow creation | ✅ | ✅ |
| 11 | Should reject insufficient token balance | ✅ | ✅ |

## integration.test.js / integration.rs

| # | Test | EVM | SVM |
| --- | ------ | ----- | ----- |
| 1 | Should complete full deposit to claim workflow | ✅ | ✅ |
| 2 | Should handle multiple different token types | ✅ | ✅ |
| 3 | Should emit all events/logs with correct parameters | ✅ | N/A |
| 4 | Should complete full cancellation workflow | ✅ | ✅ |

---

## GMP message encoding/decoding test alignment

- `intent-frameworks/svm/programs/gmp-common/tests/message_tests.rs`
- `intent-frameworks/mvm/tests/gmp_common_tests.move`
- `intent-frameworks/evm/test/gmp-common/`

### Per-message-type test symmetry

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

### IntentRequirements (0x01)

| # | Test | SVM | MVM | EVM |
| --- | --- | --- | --- | --- |
| 1 | test_intent_requirements_encode_size | ✅ | ✅ | ⚠️ |
| 2 | test_intent_requirements_discriminator | ✅ | ✅ | ⚠️ |
| 3 | test_intent_requirements_roundtrip | ✅ | ✅ | ⚠️ |
| 4 | test_intent_requirements_big_endian_amount | ✅ | ✅ | ⚠️ |
| 5 | test_intent_requirements_big_endian_expiry | ✅ | ✅ | ⚠️ |
| 6 | test_intent_requirements_field_offsets | ✅ | ✅ | ⚠️ |
| 7 | test_intent_requirements_evm_address | ✅ | ✅ | ⚠️ |

### EscrowConfirmation (0x02)

| # | Test | SVM | MVM | EVM |
| --- | --- | --- | --- | --- |
| 8 | test_escrow_confirmation_encode_size | ✅ | ✅ | ⚠️ |
| 9 | test_escrow_confirmation_discriminator | ✅ | ✅ | ⚠️ |
| 10 | test_escrow_confirmation_roundtrip | ✅ | ✅ | ⚠️ |
| 11 | test_escrow_confirmation_big_endian_amount | ✅ | ✅ | ⚠️ |
| 12 | test_escrow_confirmation_field_offsets | ✅ | ✅ | ⚠️ |

### FulfillmentProof (0x03)

| # | Test | SVM | MVM | EVM |
| --- | --- | --- | --- | --- |
| 13 | test_fulfillment_proof_encode_size | ✅ | ✅ | ⚠️ |
| 14 | test_fulfillment_proof_discriminator | ✅ | ✅ | ⚠️ |
| 15 | test_fulfillment_proof_roundtrip | ✅ | ✅ | ⚠️ |
| 16 | test_fulfillment_proof_big_endian_fields | ✅ | ✅ | ⚠️ |
| 17 | test_fulfillment_proof_field_offsets | ✅ | ✅ | ⚠️ |

### Peek Message Type

| # | Test | SVM | MVM | EVM |
| --- | --- | --- | --- | --- |
| 18 | test_peek_intent_requirements | ✅ | ✅ | ⚠️ |
| 19 | test_peek_escrow_confirmation | ✅ | ✅ | ⚠️ |
| 20 | test_peek_fulfillment_proof | ✅ | ✅ | ⚠️ |

### Error Conditions

| # | Test | SVM | MVM | EVM |
| --- | --- | --- | --- | --- |
| 21 | test_reject_wrong_discriminator | ✅ | ✅ | ⚠️ |
| 22 | test_reject_wrong_length | ✅ | ✅ | ⚠️ |
| 23 | test_reject_empty_buffer | ✅ | ✅ | ⚠️ |
| 24 | test_peek_reject_empty_buffer | ✅ | ✅ | ⚠️ |
| 25 | test_peek_reject_unknown_type | ✅ | ✅ | ⚠️ |
| 26 | test_reject_wrong_discriminator_escrow_confirmation | ✅ | ✅ | ⚠️ |
| 27 | test_reject_wrong_discriminator_fulfillment_proof | ✅ | ✅ | ⚠️ |
| 28 | test_reject_wrong_length_escrow_confirmation | ✅ | ✅ | ⚠️ |
| 29 | test_reject_wrong_length_fulfillment_proof | ✅ | ✅ | ⚠️ |
| 30 | test_reject_off_by_one_length | ✅ | ✅ | ⚠️ |

### Known Byte Sequences

| # | Test | SVM | MVM | EVM |
| --- | --- | --- | --- | --- |
| 31 | test_decode_known_intent_requirements_bytes | ✅ | ✅ | ⚠️ |
| 32 | test_decode_known_escrow_confirmation_bytes | ✅ | ✅ | ⚠️ |
| 33 | test_decode_known_fulfillment_proof_bytes | ✅ | ✅ | ⚠️ |

### Boundary Conditions

| # | Test | SVM | MVM | EVM |
| --- | --- | --- | --- | --- |
| 34 | test_max_u64_amount_roundtrip | ✅ | ✅ | ⚠️ |
| 35 | test_zero_solver_addr_means_any | ✅ | ✅ | ⚠️ |

### Cross-Chain Encoding Compatibility

These tests verify that encoding produces identical bytes across all frameworks. Expected bytes are defined in `intent-frameworks/common/testing/gmp-encoding-test-vectors.json`.

| # | Test | SVM | MVM | EVM |
| --- | --- | --- | --- | --- |
| 36 | test_cross_chain_encoding_intent_requirements | ✅ | ✅ | ⚠️ |
| 37 | test_cross_chain_encoding_escrow_confirmation | ✅ | ✅ | ⚠️ |
| 38 | test_cross_chain_encoding_fulfillment_proof | ✅ | ✅ | ⚠️ |
| 39 | test_cross_chain_encoding_intent_requirements_zeros | ✅ | ✅ | ⚠️ |
| 40 | test_cross_chain_encoding_intent_requirements_max | ✅ | ✅ | ⚠️ |
