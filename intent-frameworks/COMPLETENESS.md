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
| 1 | Should initialize escrow with verifier address | ✅ | ✅ |
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
| 1 | Should allow solver to claim with valid verifier signature | ✅ | ✅ |
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
