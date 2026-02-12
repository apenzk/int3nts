/**
 * Escrow creation utilities for EVM chains
 */

import { parseUnits, type Address, getAddress } from 'viem';

// ============================================================================
// ABIs
// ============================================================================

// IntentInflowEscrow contract ABI (minimal - only what we need)
// Uses GMP-validated createEscrowWithValidation: requirements (solver, amount, token)
// are delivered via GMP from the hub chain, so only intentId/token/amount are needed.
export const INTENT_ESCROW_ABI = [
  {
    inputs: [
      { name: 'intentId', type: 'bytes32' },
      { name: 'token', type: 'address' },
      { name: 'amount', type: 'uint64' },
    ],
    name: 'createEscrowWithValidation',
    outputs: [],
    stateMutability: 'nonpayable',
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
 * Convert Movement intent ID (32-byte hex address) to EVM bytes32
 */
export function intentIdToEvmBytes32(intentId: string): `0x${string}` {
  const hex = intentId.startsWith('0x') ? intentId.slice(2) : intentId;
  // Pad to 64 hex characters (32 bytes) for bytes32
  return `0x${hex.padStart(64, '0')}` as `0x${string}`;
}

import { getEscrowContractAddress as getEscrowFromChains, getOutflowValidatorAddress, getRpcUrl } from '@/config/chains';

/**
 * Get escrow contract address from chain configuration
 * @param chainId - Chain identifier (e.g., 'base-sepolia', 'ethereum-sepolia')
 */
export function getEscrowContractAddress(chainId: string): Address {
  const address = getEscrowFromChains(chainId);
  // Normalize to checksum format (viem requires this)
  return getAddress(address);
}

/**
 * Check if IntentRequirements have been delivered via GMP for an intent.
 *
 * Calls the public `hasRequirements(bytes32)` mapping on IntentInflowEscrow
 * via eth_call.  Returns true once the GMP relay has delivered requirements.
 *
 * @param chainKey - Chain config key (e.g. 'base-sepolia')
 * @param intentId - 32-byte hex intent ID (with 0x prefix)
 */
export async function checkHasRequirements(
  chainKey: string,
  intentId: string,
): Promise<boolean> {
  const rpcUrl = getRpcUrl(chainKey);
  const escrowAddr = getEscrowFromChains(chainKey);

  // keccak256("hasRequirements(bytes32)") first 4 bytes = 0xd70af694
  const selector = 'd70af694';
  const intentHex = intentId.startsWith('0x') ? intentId.slice(2) : intentId;
  const intentPadded = intentHex.padStart(64, '0');
  const calldata = `0x${selector}${intentPadded}`;

  const response = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      method: 'eth_call',
      params: [{ to: escrowAddr, data: calldata }, 'latest'],
      id: 1,
    }),
  });

  const json = await response.json();
  if (json.error) {
    throw new Error(`eth_call hasRequirements failed: ${json.error.message}`);
  }

  // ABI bool: 32 bytes, last byte 0x01 = true
  const result: string = json.result || '0x';
  return result.endsWith('1');
}

/**
 * Check if an outflow intent has been fulfilled on the connected EVM chain.
 *
 * Calls `isFulfilled(bytes32)` on IntentOutflowValidator.
 * Returns true once the solver has fulfilled the intent (sent tokens to recipient).
 *
 * @param chainKey - Chain config key (e.g. 'base-sepolia')
 * @param intentId - 32-byte hex intent ID (with 0x prefix)
 */
export async function checkIsFulfilled(
  chainKey: string,
  intentId: string,
): Promise<boolean> {
  const rpcUrl = getRpcUrl(chainKey);
  const outflowAddr = getOutflowValidatorAddress(chainKey);

  // keccak256("isFulfilled(bytes32)") first 4 bytes = 0xed75e1cc
  const selector = 'ed75e1cc';
  const intentHex = intentId.startsWith('0x') ? intentId.slice(2) : intentId;
  const intentPadded = intentHex.padStart(64, '0');
  const calldata = `0x${selector}${intentPadded}`;

  const response = await fetch(rpcUrl, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      jsonrpc: '2.0',
      method: 'eth_call',
      params: [{ to: outflowAddr, data: calldata }, 'latest'],
      id: 1,
    }),
  });

  const json = await response.json();
  if (json.error) {
    throw new Error(`eth_call isFulfilled failed: ${json.error.message}`);
  }

  // ABI bool: 32 bytes, last byte 0x01 = true
  const result: string = json.result || '0x';
  return result.endsWith('1');
}

