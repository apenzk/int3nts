# Frontend Test Completeness

> **⚠️ IMPORTANT: When adding a new framework, ensure maximal completeness by implementing all tests listed below.**

This document tracks test alignment status for the frontend. For the complete overview and other frameworks, see the [Framework Extension Guide](../../docs/intent-frameworks/framework-extension-guide.md#test-alignment-reference).

All tests listed here are VM-specific; generic tests are intentionally excluded because they are not relevant when integrating a new VM.

**Legend:** ✅ = Implemented | N/A = Not applicable to platform | ⚠️ = Not yet implemented

## components/wallet/*vmWalletConnector.test.tsx

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | should show connect button when disconnected | ⚠️ | ✅ | ✅ |
| 2 | should disable when no wallet is detected | ✅ | ⚠️ | ⚠️ |
| 3 | should disable when no MetaMask connector is available | ⚠️ | ✅ | ⚠️ |
| 4 | should disable when Phantom adapter is not detected | ⚠️ | ⚠️ | ✅ |
| 5 | should call connect when clicking the connect button | ⚠️ | ✅ | ⚠️ |
| 6 | should call select and connect on click | ⚠️ | ⚠️ | ✅ |
| 7 | should show disconnect button when connected | ⚠️ | ✅ | ✅ |
| 8 | should show disconnect when connected | ✅ | ⚠️ | ⚠️ |

## lib/*vm-transactions.test.ts

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | should be a valid Move address | ✅ | N/A | N/A |
| 2 | should convert hex string to Uint8Array | ✅ | N/A | N/A |
| 3 | should handle 64-byte Ed25519 signature | ✅ | N/A | N/A |
| 4 | should strip 0x prefix automatically | ✅ | N/A | N/A |
| 5 | should return empty array for empty string | ✅ | N/A | N/A |
| 6 | should pad 20-byte EVM address to 32 bytes | ✅ | N/A | N/A |
| 7 | should handle address without 0x prefix | ✅ | N/A | N/A |
| 8 | should normalize to lowercase | ✅ | N/A | N/A |
| 9 | should remove 0x prefix | ✅ | N/A | N/A |
| 10 | should return unchanged if no prefix | ✅ | N/A | N/A |
| 11 | should use the configured SVM RPC URL | N/A | N/A | ✅ |
| 12 | should decode base64 to bytes | N/A | N/A | ✅ |
| 13 | should trim whitespace around base64 input | N/A | N/A | ✅ |
| 14 | should return an instruction targeting the Ed25519 program | N/A | N/A | ✅ |
| 15 | should return null when the request fails | N/A | N/A | ✅ |
| 16 | should return null when the registry vec is empty | N/A | N/A | ✅ |
| 17 | should return normalized hex when vec is a string | N/A | N/A | ✅ |
| 18 | should convert vec byte array to hex | N/A | N/A | ✅ |

## lib/*vm-escrow.test.ts

| # | Test | MVM | EVM | SVM |
| --- | ------ | ----- | ----- | ----- |
| 1 | should convert 0x-prefixed intent IDs to uint256 bigint | N/A | ✅ | N/A |
| 2 | should convert non-prefixed intent IDs to uint256 bigint | N/A | ✅ | N/A |
| 3 | should return a checksummed EVM address | N/A | ✅ | N/A |
| 4 | should throw for missing chain config | N/A | ✅ | N/A |
| 5 | should pad intent IDs to 32 bytes | N/A | N/A | ✅ |
| 6 | should round-trip pubkey hex conversion | N/A | N/A | ✅ |
| 7 | should derive deterministic state/escrow/vault PDAs | N/A | N/A | ✅ |
| 8 | should parse escrow account data into a structured object | N/A | N/A | ✅ |
| 9 | should build create escrow instruction with expected layout | N/A | N/A | ✅ |
| 10 | should build claim instruction with sysvar and token program keys | N/A | N/A | ✅ |
| 11 | should build cancel instruction with expected layout | N/A | N/A | ✅ |
