/**
 * Escrow creation utilities for EVM chains
 */

import { parseUnits, type Address, getAddress } from 'viem';

// ============================================================================
// ABIs
// ============================================================================

// IntentEscrow contract ABI (minimal - only what we need)
export const INTENT_ESCROW_ABI = [
  {
    inputs: [
      { name: 'intentId', type: 'uint256' },
      { name: 'token', type: 'address' },
      { name: 'amount', type: 'uint256' },
      { name: 'reservedSolver', type: 'address' },
    ],
    name: 'createEscrow',
    outputs: [],
    stateMutability: 'payable',
    type: 'function',
  },
  {
    inputs: [{ name: 'intentId', type: 'uint256' }],
    name: 'getEscrow',
    outputs: [
      { name: 'requester', type: 'address' },
      { name: 'token', type: 'address' },
      { name: 'amount', type: 'uint256' },
      { name: 'isClaimed', type: 'bool' },
      { name: 'expiry', type: 'uint256' },
      { name: 'reservedSolver', type: 'address' },
    ],
    stateMutability: 'view',
    type: 'function',
  },
] as const;

// ERC20 ABI (minimal - only approve)
export const ERC20_ABI = [
  {
    inputs: [
      { name: 'spender', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
    name: 'approve',
    outputs: [{ name: '', type: 'bool' }],
    stateMutability: 'nonpayable',
    type: 'function',
  },
  {
    inputs: [
      { name: 'owner', type: 'address' },
      { name: 'spender', type: 'address' },
    ],
    name: 'allowance',
    outputs: [{ name: '', type: 'uint256' }],
    stateMutability: 'view',
    type: 'function',
  },
] as const;

// ============================================================================
// Helpers
// ============================================================================

/**
 * Convert Movement intent ID (32-byte hex address) to EVM uint256
 */
export function intentIdToEvmFormat(intentId: string): bigint {
  // Remove 0x prefix if present
  const hex = intentId.startsWith('0x') ? intentId.slice(2) : intentId;
  // Convert to BigInt (EVM uint256)
  return BigInt('0x' + hex);
}

import { getEscrowContractAddress as getEscrowFromChains } from '@/config/chains';

/**
 * Get escrow contract address from chain configuration
 * @param chainId - Chain identifier (e.g., 'base-sepolia', 'ethereum-sepolia')
 */
export function getEscrowContractAddress(chainId: string): Address {
  const address = getEscrowFromChains(chainId);
  // Normalize to checksum format (viem requires this)
  return getAddress(address);
}

