// ============================================================================
// Move Transaction Utilities
// ============================================================================

/**
 * Move transaction building utilities for intent creation
 */

import { getIntentContractAddress } from '@/config/chains';

// ============================================================================
// Constants
// ============================================================================

// Intent contract address on Movement testnet (re-exported from chains config for backward compatibility)
export const INTENT_MODULE_ADDR = getIntentContractAddress();

// ============================================================================
// Helpers
// ============================================================================

/**
 * Convert a hex string to Uint8Array.
 * Handles optional 0x prefix.
 * 
 * Used for: Converting solver signatures from hex to bytes for wallet serialization.
 */
export function hexToBytes(hex: string): Uint8Array {
  const cleanHex = hex.startsWith('0x') ? hex.slice(2) : hex;
  const bytes = cleanHex.match(/.{1,2}/g)?.map(byte => parseInt(byte, 16)) || [];
  return new Uint8Array(bytes);
}

/**
 * Pad an EVM address (20 bytes) to Move address format (32 bytes).
 * Left-pads with zeros and ensures lowercase with 0x prefix.
 * 
 * Used for: requester_addr_connected_chain parameter in cross-chain intents.
 */
export function padEvmAddressToMove(evmAddress: string): string {
  const clean = evmAddress.startsWith('0x') ? evmAddress.slice(2) : evmAddress;
  return '0x' + clean.toLowerCase().padStart(64, '0');
}

/**
 * Strip 0x prefix from a hex string if present.
 */
export function stripHexPrefix(hex: string): string {
  return hex.startsWith('0x') ? hex.slice(2) : hex;
}
