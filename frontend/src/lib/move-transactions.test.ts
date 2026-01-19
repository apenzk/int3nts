/**
 * Tests for Move transaction utilities
 * 
 * These tests verify the correctness of data transformations required
 * for submitting transactions to the Movement blockchain via wallet adapters.
 */

import { describe, it, expect } from 'vitest';
import { 
  INTENT_MODULE_ADDR, 
  hexToBytes, 
  padEvmAddressToMove, 
  stripHexPrefix 
} from './move-transactions';

describe('INTENT_MODULE_ADDR', () => {
  /**
   * Test: Module address format validation
   * Why: Move addresses must be exactly 32 bytes (64 hex chars) with 0x prefix.
   *      Invalid addresses cause transaction failures on-chain.
   */
  it('should be a valid Move address', () => {
    expect(INTENT_MODULE_ADDR).toMatch(/^0x[a-fA-F0-9]{64}$/);
  });
});

describe('hexToBytes', () => {
  /**
   * Test: Basic hex to bytes conversion
   * Why: The Move contract expects vector<u8> for signatures.
   *      Incorrect conversion causes signature verification failure.
   */
  it('should convert hex string to Uint8Array', () => {
    const bytes = hexToBytes('aabbccdd');
    expect(bytes).toBeInstanceOf(Uint8Array);
    expect(bytes).toHaveLength(4);
    expect(bytes[0]).toBe(0xaa);
    expect(bytes[1]).toBe(0xbb);
    expect(bytes[2]).toBe(0xcc);
    expect(bytes[3]).toBe(0xdd);
  });

  /**
   * Test: Ed25519 signature length (64 bytes)
   * Why: Ed25519 signatures are exactly 64 bytes. The Move contract
   *      verifies this length during signature validation.
   */
  it('should handle 64-byte Ed25519 signature', () => {
    const signatureHex = 'ab'.repeat(64);
    const bytes = hexToBytes(signatureHex);
    expect(bytes).toHaveLength(64);
    expect(bytes.every(b => b === 0xab)).toBe(true);
  });

  /**
   * Test: 0x prefix handling
   * Why: Hex strings from APIs may include 0x prefix.
   *      The function must handle both formats.
   */
  it('should strip 0x prefix automatically', () => {
    const bytes = hexToBytes('0xaabbccdd');
    expect(bytes).toHaveLength(4);
    expect(bytes[0]).toBe(0xaa);
  });

  /**
   * Test: Empty input handling
   * Why: Defensive programming - should not crash on empty input.
   */
  it('should return empty array for empty string', () => {
    const bytes = hexToBytes('');
    expect(bytes).toHaveLength(0);
  });
});

describe('padEvmAddressToMove', () => {
  /**
   * Test: Pad 20-byte EVM address to 32 bytes
   * Why: The requester_addr_connected_chain parameter in the Move contract
   *      expects a 32-byte address. EVM addresses (20 bytes) must be
   *      left-padded with 12 zero bytes.
   */
  it('should pad 20-byte EVM address to 32 bytes', () => {
    const padded = padEvmAddressToMove('0x1234567890abcdef1234567890abcdef12345678');
    
    // 0x prefix (2 chars) + 64 hex chars = 66 total
    expect(padded).toHaveLength(66);
    // 24 zeros (12 bytes) + 40 hex chars (20 bytes) = 64 hex chars (32 bytes)
    expect(padded).toBe('0x0000000000000000000000001234567890abcdef1234567890abcdef12345678');
  });

  /**
   * Test: Handle addresses without 0x prefix
   * Why: Some APIs return addresses without the 0x prefix.
   *      The function must handle both formats consistently.
   */
  it('should handle address without 0x prefix', () => {
    const padded = padEvmAddressToMove('1234567890abcdef1234567890abcdef12345678');
    expect(padded).toHaveLength(66);
    expect(padded.startsWith('0x')).toBe(true);
  });

  /**
   * Test: Lowercase normalization
   * Why: Move addresses are case-insensitive but should be normalized
   *      to lowercase for consistency.
   */
  it('should normalize to lowercase', () => {
    const padded = padEvmAddressToMove('0xABCDEF1234567890ABCDEF1234567890ABCDEF12');
    expect(padded).toBe('0x000000000000000000000000abcdef1234567890abcdef1234567890abcdef12');
  });
});

describe('stripHexPrefix', () => {
  /**
   * Test: Remove 0x prefix
   * Why: Some APIs expect hex without prefix.
   */
  it('should remove 0x prefix', () => {
    expect(stripHexPrefix('0xabcd')).toBe('abcd');
  });

  /**
   * Test: No-op when no prefix
   * Why: Should not modify strings without prefix.
   */
  it('should return unchanged if no prefix', () => {
    expect(stripHexPrefix('abcd')).toBe('abcd');
  });
});
