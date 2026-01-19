/**
 * Solana transaction helpers for SVM escrow flows.
 */

import { Connection, Ed25519Program, Transaction, TransactionInstruction } from '@solana/web3.js';
import type { WalletContextState } from '@solana/wallet-adapter-react';
import { getIntentContractAddress, getRpcUrl } from '@/config/chains';

// ============================================================================
// Connections
// ============================================================================

/**
 * Build a Solana connection for the configured SVM RPC.
 */
export function getSvmConnection(): Connection {
  return new Connection(getRpcUrl('svm-devnet'), 'confirmed');
}

// ============================================================================
// Transactions
// ============================================================================

/**
 * Send a transaction using the connected Phantom wallet.
 */
export async function sendSvmTransaction(params: {
  wallet: WalletContextState;
  connection: Connection;
  instructions: TransactionInstruction[];
}): Promise<string> {
  const { wallet, connection, instructions } = params;
  if (!wallet.publicKey) {
    throw new Error('SVM wallet not connected');
  }

  const transaction = new Transaction().add(...instructions);
  transaction.feePayer = wallet.publicKey;
  const { blockhash, lastValidBlockHeight } = await connection.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;

  // Debug: Log transaction details
  console.log('SVM Transaction debug:', {
    feePayer: wallet.publicKey.toBase58(),
    numInstructions: instructions.length,
    programId: instructions[0]?.programId?.toBase58(),
    numAccounts: instructions[0]?.keys?.length,
    accounts: instructions[0]?.keys?.map((k, i) => `${i}: ${k.pubkey.toBase58()} (signer=${k.isSigner}, writable=${k.isWritable})`),
    dataLength: instructions[0]?.data?.length,
    dataHex: instructions[0]?.data ? Buffer.from(instructions[0].data).toString('hex').slice(0, 100) + '...' : null,
  });

  // Try to simulate first to get better error messages
  try {
    const simResult = await connection.simulateTransaction(transaction);
    if (simResult.value.err) {
      console.error('SVM Transaction simulation failed:', simResult.value.err);
      console.error('Simulation logs:', simResult.value.logs);
      throw new Error(`Transaction simulation failed: ${JSON.stringify(simResult.value.err)}. Logs: ${simResult.value.logs?.join('\n')}`);
    }
    console.log('SVM Transaction simulation succeeded. Logs:', simResult.value.logs);
  } catch (simError) {
    console.error('SVM Transaction simulation error:', simError);
    throw simError;
  }

  const signature = await wallet.sendTransaction(transaction, connection);
  await connection.confirmTransaction(
    { signature, blockhash, lastValidBlockHeight },
    'confirmed'
  );
  return signature;
}

// ============================================================================
// Helpers
// ============================================================================

/**
 * Build an Ed25519 verification instruction for the SVM program.
 */
export function buildEd25519VerificationIx(params: {
  message: Uint8Array;
  signature: Uint8Array;
  publicKey: Uint8Array;
}): TransactionInstruction {
  return Ed25519Program.createInstructionWithPublicKey({
    message: params.message,
    signature: params.signature,
    publicKey: params.publicKey,
  });
}

/**
 * Decode a base64 string into bytes (browser-safe helper).
 */
export function decodeBase64(base64: string): Uint8Array {
  const normalized = base64.trim();
  const binary = atob(normalized);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

// ============================================================================
// Registry Queries
// ============================================================================

/**
 * Fetch the solver's registered SVM address from the hub chain registry.
 */
export async function fetchSolverSvmAddress(solverAddr: string): Promise<string | null> {
  const rpcUrl = getRpcUrl('movement');
  const moduleAddr = getIntentContractAddress();

  const response = await fetch(`${rpcUrl}/view`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      function: `${moduleAddr}::solver_registry::get_connected_chain_svm_address`,
      type_arguments: [],
      arguments: [solverAddr],
    }),
  });

  if (!response.ok) {
    return null;
  }

  const result = await response.json();
  const optionValue = result?.[0];
  if (!optionValue || !optionValue.vec) {
    return null;
  }

  const vec = optionValue.vec;
  if (Array.isArray(vec) && vec.length === 0) {
    return null;
  }

  if (typeof vec === 'string') {
    // Strip any existing 0x prefix(es) and add exactly one
    let clean = vec;
    while (clean.startsWith('0x') || clean.startsWith('0X')) {
      clean = clean.slice(2);
    }
    return `0x${clean}`;
  }

  if (Array.isArray(vec)) {
    const hex = vec.map((b: number) => b.toString(16).padStart(2, '0')).join('');
    return `0x${hex}`;
  }

  return null;
}
