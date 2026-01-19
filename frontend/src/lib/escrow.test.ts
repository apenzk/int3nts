import { describe, expect, it, vi } from 'vitest';

vi.mock('@/config/chains', () => ({
  getEscrowContractAddress: (chainId: string) => {
    if (chainId === 'base-sepolia') {
      return '0x0000000000000000000000000000000000000001';
    }
    throw new Error(`Missing escrow contract address for chain: ${chainId}`);
  },
}));

import { getAddress } from 'viem';
import { getEscrowContractAddress, intentIdToEvmFormat } from './escrow';

describe('intentIdToEvmFormat', () => {
  /**
   * Test: Intent ID conversion with 0x prefix
   * Why: EVM uses uint256 for intent IDs, derived from 32-byte hex.
   */
  it('should convert 0x-prefixed intent IDs to uint256 bigint', () => {
    const intentId = '0x' + '01'.repeat(32);
    expect(intentIdToEvmFormat(intentId)).toBe(BigInt(intentId));
  });

  /**
   * Test: Intent ID conversion without prefix
   * Why: Some sources omit 0x but still represent 32-byte hex.
   */
  it('should convert non-prefixed intent IDs to uint256 bigint', () => {
    const intentId = 'ab'.repeat(32);
    expect(intentIdToEvmFormat(intentId)).toBe(BigInt(`0x${intentId}`));
  });
});

describe('getEscrowContractAddress', () => {
  /**
   * Test: Escrow address normalization
   * Why: viem requires checksummed addresses for contract writes.
   */
  it('should return a checksummed EVM address', () => {
    const address = getEscrowContractAddress('base-sepolia');
    expect(address).toBe(getAddress('0x0000000000000000000000000000000000000001'));
  });

  /**
   * Test: Missing escrow address
   * Why: Misconfigured chains should fail fast.
   */
  it('should throw for missing chain config', () => {
    expect(() => getEscrowContractAddress('unknown-chain')).toThrow(
      'Missing escrow contract address for chain: unknown-chain'
    );
  });
});
