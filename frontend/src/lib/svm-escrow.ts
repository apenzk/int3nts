/**
 * Solana escrow instruction builders and PDA helpers.
 */

import {
  Connection,
  PublicKey,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from '@solana/spl-token';
import { getSvmProgramId, getSvmOutflowProgramId, getRpcUrl } from '@/config/chains';
import { Buffer } from 'buffer';

// ============================================================================
// Constants
// ============================================================================

const STATE_SEED = 'state';
const ESCROW_SEED = 'escrow';
const VAULT_SEED = 'vault';
const REQUIREMENTS_SEED = 'requirements';
const GMP_CONFIG_SEED = 'gmp_config';

// ============================================================================
// Types
// ============================================================================

type EscrowAccount = {
  requester: PublicKey;
  tokenMint: PublicKey;
  amount: bigint;
  isClaimed: boolean;
  expiry: bigint;
  reservedSolver: PublicKey;
  intentId: Uint8Array;
  bump: number;
};

// ============================================================================
// Address Helpers
// ============================================================================

export function svmPubkeyToHex(value: PublicKey | string): string {
  const pubkey = typeof value === 'string' ? new PublicKey(value) : value;
  return `0x${Buffer.from(pubkey.toBytes()).toString('hex')}`;
}

/**
 * Convert a hex string into a 32-byte Uint8Array (zero-left-padded).
 * Handles malformed inputs like "0x0x..." by stripping all 0x prefixes.
 */
export function svmHexToBytes(hex: string): Uint8Array {
  // Strip all 0x prefixes (handles double-prefixed values like "0x0x...")
  let clean = hex;
  while (clean.startsWith('0x') || clean.startsWith('0X')) {
    clean = clean.slice(2);
  }
  const padded = clean.padStart(64, '0');
  return Uint8Array.from(Buffer.from(padded, 'hex'));
}

/**
 * Convert a 32-byte hex string into a PublicKey.
 */
export function svmHexToPubkey(hex: string): PublicKey {
  return new PublicKey(svmHexToBytes(hex));
}

// ============================================================================
// PDA Helpers
// ============================================================================

/**
 * Derive the global state PDA for the escrow program.
 */
export function getStatePda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from(STATE_SEED)], programId);
}

/**
 * Derive the escrow PDA for a given intent ID.
 */
export function getEscrowPda(intentId: string, programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(ESCROW_SEED), Buffer.from(svmHexToBytes(intentId))],
    programId
  );
}

/**
 * Derive the escrow vault PDA for a given intent ID.
 */
export function getVaultPda(intentId: string, programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(VAULT_SEED), Buffer.from(svmHexToBytes(intentId))],
    programId
  );
}

/**
 * Derive the associated token account for a mint and owner.
 */
export function getSvmTokenAccount(mint: PublicKey, owner: PublicKey): PublicKey {
  return getAssociatedTokenAddressSync(mint, owner);
}

/**
 * Derive the requirements PDA for a given intent ID (GMP mode).
 */
export function getRequirementsPda(intentId: string, programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(REQUIREMENTS_SEED), Buffer.from(svmHexToBytes(intentId))],
    programId
  );
}

/**
 * Derive the GMP config PDA for the escrow program.
 */
export function getGmpConfigPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from(GMP_CONFIG_SEED)], programId);
}

/**
 * GMP parameters for CreateEscrow with EscrowConfirmation.
 */
export type CreateEscrowGmpParams = {
  gmpEndpointProgramId: PublicKey;
  hubChainId: number;
  currentNonce: bigint;
};

/**
 * Read the current outbound nonce for a destination chain from the GMP endpoint.
 * Used to derive the message PDA for GMP Send CPI.
 */
export async function readGmpOutboundNonce(
  connection: Connection,
  gmpEndpointProgramId: PublicKey,
  hubChainId: number,
): Promise<bigint> {
  const chainIdBytes = Buffer.alloc(4);
  chainIdBytes.writeUInt32LE(hubChainId);
  const [noncePda] = PublicKey.findProgramAddressSync(
    [Buffer.from('nonce_out'), chainIdBytes],
    gmpEndpointProgramId,
  );
  const accountInfo = await connection.getAccountInfo(noncePda);
  if (accountInfo && accountInfo.data.length >= 13) {
    // OutboundNonceAccount: discriminator(1) + dst_chain_id(4) + nonce(8) + bump(1)
    return Buffer.from(accountInfo.data).readBigUInt64LE(5);
  }
  return BigInt(0);
}

// ============================================================================
// Account Parsing
// ============================================================================

/**
 * Parse escrow account data into a typed structure.
 */
export function parseEscrowAccount(data: Buffer): EscrowAccount {
  const discriminator = data.slice(0, 8);
  if (discriminator.length !== 8) {
    throw new Error('Invalid escrow discriminator');
  }

  const requester = new PublicKey(data.slice(8, 40));
  const tokenMint = new PublicKey(data.slice(40, 72));
  const amount = data.readBigUInt64LE(72);
  const isClaimed = data.readUInt8(80) === 1;
  const expiry = data.readBigInt64LE(81);
  const reservedSolver = new PublicKey(data.slice(89, 121));
  const intentId = data.slice(121, 153);
  const bump = data.readUInt8(153);

  return {
    requester,
    tokenMint,
    amount,
    isClaimed,
    expiry,
    reservedSolver,
    intentId,
    bump,
  };
}

/**
 * Check if an outflow intent has been fulfilled on the SVM connected chain.
 *
 * Reads the outflow validator's requirements PDA and checks the `fulfilled` field.
 * Returns true once the solver has fulfilled the intent (sent tokens to recipient).
 *
 * @param chainKey - Chain config key (e.g. 'svm-devnet')
 * @param intentId - 32-byte hex intent ID (with 0x prefix)
 */
export async function checkIsFulfilledSvm(
  chainKey: string,
  intentId: string,
): Promise<boolean> {
  const outflowProgramId = new PublicKey(getSvmOutflowProgramId(chainKey));
  const rpcUrl = getRpcUrl(chainKey);
  const connection = new Connection(rpcUrl);

  // Derive the requirements PDA for the outflow validator program
  const [requirementsPda] = PublicKey.findProgramAddressSync(
    [Buffer.from(REQUIREMENTS_SEED), Buffer.from(svmHexToBytes(intentId))],
    outflowProgramId
  );

  const accountInfo = await connection.getAccountInfo(requirementsPda);
  if (!accountInfo || !accountInfo.data) {
    return false;
  }

  // IntentRequirementsAccount layout (147 bytes):
  //   discriminator: 1, intent_id: 32, recipient_addr: 32, amount_required: 8,
  //   token_mint: 32, authorized_solver: 32, expiry: 8, fulfilled: 1, bump: 1
  // fulfilled is at byte offset 145
  const FULFILLED_OFFSET = 145;
  if (accountInfo.data.length < FULFILLED_OFFSET + 1) {
    return false;
  }

  return accountInfo.data[FULFILLED_OFFSET] === 1;
}

// ============================================================================
// Instruction Data Encoding
// ============================================================================

function encodeU64(value: bigint | number): Buffer {
  const buffer = Buffer.alloc(8);
  buffer.writeBigUInt64LE(BigInt(value), 0);
  return buffer;
}

function encodeI64(value: bigint | number): Buffer {
  const buffer = Buffer.alloc(8);
  buffer.writeBigInt64LE(BigInt(value), 0);
  return buffer;
}

function encodeCreateEscrowData(intentId: string, amount: bigint, expiryDuration?: number): Buffer {
  const intentIdBytes = Buffer.from(svmHexToBytes(intentId));
  const expiryTag = expiryDuration === undefined ? 0 : 1;
  const expiryBytes =
    expiryDuration === undefined ? Buffer.alloc(0) : encodeI64(expiryDuration);

  return Buffer.concat([
    Buffer.from([3]), // EscrowInstruction::CreateEscrow (index 3: Initialize=0, GmpReceive=1, SetGmpConfig=2, CreateEscrow=3)
    intentIdBytes,
    encodeU64(amount),
    Buffer.from([expiryTag]),
    expiryBytes,
  ]);
}

function encodeClaimData(intentId: string, signatureBytes: Uint8Array): Buffer {
  return Buffer.concat([
    Buffer.from([4]), // EscrowInstruction::Claim (index 4)
    Buffer.from(svmHexToBytes(intentId)),
    Buffer.from(signatureBytes),
  ]);
}

function encodeCancelData(intentId: string): Buffer {
  return Buffer.concat([
    Buffer.from([5]), // EscrowInstruction::Cancel (index 5)
    Buffer.from(svmHexToBytes(intentId)),
  ]);
}

// ============================================================================
// Instruction Builders
// ============================================================================

/**
 * Build the CreateEscrow instruction for the SVM program.
 */
export function buildCreateEscrowInstruction(params: {
  intentId: string;
  amount: bigint;
  requester: PublicKey;
  requesterToken: PublicKey;
  tokenMint: PublicKey;
  reservedSolver: PublicKey;
  expiryDuration?: number;
  programId?: PublicKey;
  gmpParams?: CreateEscrowGmpParams;
}): TransactionInstruction {
  const programId = params.programId ?? new PublicKey(getSvmProgramId('svm-devnet'));
  const [escrowPda] = getEscrowPda(params.intentId, programId);
  const [vaultPda] = getVaultPda(params.intentId, programId);

  const keys = [
    { pubkey: escrowPda, isSigner: false, isWritable: true },
    { pubkey: params.requester, isSigner: true, isWritable: true },
    { pubkey: params.tokenMint, isSigner: false, isWritable: false },
    { pubkey: params.requesterToken, isSigner: false, isWritable: true },
    { pubkey: vaultPda, isSigner: false, isWritable: true },
    { pubkey: params.reservedSolver, isSigner: false, isWritable: false },
    { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
  ];

  // Add GMP accounts for EscrowConfirmation (accounts 9-17)
  if (params.gmpParams) {
    const gmp = params.gmpParams;
    const [requirementsPda] = getRequirementsPda(params.intentId, programId);
    const [gmpConfigPda] = getGmpConfigPda(programId);

    // GMP endpoint PDAs
    const chainIdBytes = Buffer.alloc(4);
    chainIdBytes.writeUInt32LE(gmp.hubChainId);
    const [gmpEndpointConfigPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('config')],
      gmp.gmpEndpointProgramId,
    );
    const [gmpNonceOutPda] = PublicKey.findProgramAddressSync(
      [Buffer.from('nonce_out'), chainIdBytes],
      gmp.gmpEndpointProgramId,
    );
    const nonceBytes = Buffer.alloc(8);
    nonceBytes.writeBigUInt64LE(gmp.currentNonce);
    const [messagePda] = PublicKey.findProgramAddressSync(
      [Buffer.from('message'), chainIdBytes, nonceBytes],
      gmp.gmpEndpointProgramId,
    );

    keys.push(
      // Account 9: Requirements PDA (writable - marks escrow_created=true)
      { pubkey: requirementsPda, isSigner: false, isWritable: true },
      // Account 10: GMP config PDA (escrow program's config)
      { pubkey: gmpConfigPda, isSigner: false, isWritable: false },
      // Account 11: GMP endpoint program
      { pubkey: gmp.gmpEndpointProgramId, isSigner: false, isWritable: false },
      // Accounts 12+: GMP Send CPI accounts
      { pubkey: gmpEndpointConfigPda, isSigner: false, isWritable: false },  // config
      { pubkey: gmpNonceOutPda, isSigner: false, isWritable: true },         // nonce_out
      { pubkey: params.requester, isSigner: true, isWritable: false },       // sender
      { pubkey: params.requester, isSigner: true, isWritable: true },        // payer
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // system_program
      { pubkey: messagePda, isSigner: false, isWritable: true },             // message account
    );
  }

  return new TransactionInstruction({
    programId,
    keys,
    data: encodeCreateEscrowData(params.intentId, params.amount, params.expiryDuration),
  });
}

/**
 * Build the Claim instruction for the SVM program.
 */
export function buildClaimInstruction(params: {
  intentId: string;
  signature: Uint8Array;
  solverToken: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const programId = params.programId ?? new PublicKey(getSvmProgramId('svm-devnet'));
  const [escrowPda] = getEscrowPda(params.intentId, programId);
  const [statePda] = getStatePda(programId);
  const [vaultPda] = getVaultPda(params.intentId, programId);

  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: escrowPda, isSigner: false, isWritable: true },
      { pubkey: statePda, isSigner: false, isWritable: false },
      { pubkey: vaultPda, isSigner: false, isWritable: true },
      { pubkey: params.solverToken, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_INSTRUCTIONS_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: encodeClaimData(params.intentId, params.signature),
  });
}

/**
 * Build the Cancel instruction for the SVM program.
 */
export function buildCancelInstruction(params: {
  intentId: string;
  requester: PublicKey;
  requesterToken: PublicKey;
  programId?: PublicKey;
}): TransactionInstruction {
  const programId = params.programId ?? new PublicKey(getSvmProgramId('svm-devnet'));
  const [escrowPda] = getEscrowPda(params.intentId, programId);
  const [vaultPda] = getVaultPda(params.intentId, programId);

  return new TransactionInstruction({
    programId,
    keys: [
      { pubkey: escrowPda, isSigner: false, isWritable: true },
      { pubkey: params.requester, isSigner: true, isWritable: true },
      { pubkey: vaultPda, isSigner: false, isWritable: true },
      { pubkey: params.requesterToken, isSigner: false, isWritable: true },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
    ],
    data: encodeCancelData(params.intentId),
  });
}
