import { describe, expect, it, vi } from 'vitest';
import { getAddress } from 'viem';
import { getEscrowContractAddress, intentIdToEvmFormat } from './escrow';
import { DUMMY_ESCROW_CONTRACT_ADDR_EVM, DUMMY_INTENT_ID } from './test-constants';

vi.mock('@/config/chains', () => ({
  getEscrowContractAddress: (chainId: string) => {
    if (chainId === 'base-sepolia') {
      return DUMMY_ESCROW_CONTRACT_ADDR_EVM;
    }
    throw new Error(`Missing escrow contract address for chain: ${chainId}`);
  },
}));

describe('intentIdToEvmFormat', () => {
  /**
   * Test: Intent ID conversion with 0x prefix
   * Why: EVM uses uint256 for intent IDs, derived from 32-byte hex.
   */
  it('should convert 0x-prefixed intent IDs to uint256 bigint', () => {
    const intentId = DUMMY_INTENT_ID;
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
    expect(address).toBe(getAddress(DUMMY_ESCROW_CONTRACT_ADDR_EVM));
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
