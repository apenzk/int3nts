/**
 * Shared test constants for frontend unit tests
 *
 * This module provides constants used across multiple test files, following
 * the same pattern as the Rust test helpers (verifier/tests/helpers.rs).
 *
 * Constants are numbered sequentially (0x...0001, 0x...0002, etc.) to maintain
 * consistency and make them easily identifiable in test output.
 */

import { PublicKey } from '@solana/web3.js';

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/**
 * Helper to create 32-byte array with sequential value in last byte (0x...00NN)
 * This ensures PublicKeys represent addresses like 0x...0002, 0x...0003, etc.
 */
function createSequentialBytes(value: number): Uint8Array {
  const bytes = new Uint8Array(32);
  bytes[31] = value;
  return bytes;
}

// ============================================================================
// CONSTANTS
// ============================================================================

// --------------------------------- IDs ----------------------------------

/// Dummy intent ID (64 hex characters, same format as verifier tests)
export const DUMMY_INTENT_ID = '0x0000000000000000000000000000000000000000000000000000000000000001';

// -------------------------------- USERS ---------------------------------

/// Dummy requester address (SVM format, 32 bytes, 0x...0002)
export const DUMMY_REQUESTER_ADDR_SVM = new PublicKey(createSequentialBytes(0x02));

/// Dummy token mint address (SVM format, 32 bytes, 0x...0003)
export const DUMMY_TOKEN_MINT_SVM = new PublicKey(createSequentialBytes(0x03));

/// Dummy solver address (SVM format, 32 bytes, 0x...0004)
export const DUMMY_SOLVER_ADDR_SVM = new PublicKey(createSequentialBytes(0x04));

// ------------------------- TOKENS AND CONTRACTS -------------------------

/// Dummy escrow contract address (EVM format, 20 bytes)
export const DUMMY_ESCROW_CONTRACT_ADDR_EVM = '0x0000000000000000000000000000000000000001';

// ------------------------- PROGRAM DERIVED ADDRESSES -------------------------

/// Dummy state PDA (32 bytes, 0x...0005)
export const DUMMY_STATE_PDA = new PublicKey(createSequentialBytes(0x05));

/// Dummy escrow PDA (32 bytes, 0x...0006)
export const DUMMY_ESCROW_PDA = new PublicKey(createSequentialBytes(0x06));

/// Dummy vault PDA (32 bytes, 0x...0007)
export const DUMMY_VAULT_PDA = new PublicKey(createSequentialBytes(0x07));

// ------------------------- TEST DATA -------------------------

/// Dummy pubkey for round-trip tests (32 bytes, 0x...0008)
export const DUMMY_PUBKEY_TEST = new PublicKey(createSequentialBytes(0x08));

/// Dummy signature (64 bytes, fill with 0x09)
export const DUMMY_SIGNATURE = new Uint8Array(64).fill(0x09);

/// Dummy message bytes for Ed25519 tests
export const DUMMY_MESSAGE = new Uint8Array([0x01, 0x02, 0x03]);

/// Dummy signature bytes (64 bytes, fill with 0x02)
export const DUMMY_SIGNATURE_BYTES = new Uint8Array(64).fill(0x02);

/// Dummy public key bytes (32 bytes, fill with 0x03)
export const DUMMY_PUBKEY_BYTES = new Uint8Array(32).fill(0x03);

