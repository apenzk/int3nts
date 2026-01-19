import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { PublicKey, SYSVAR_INSTRUCTIONS_PUBKEY, SystemProgram } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import {
  buildCancelInstruction,
  buildClaimInstruction,
  buildCreateEscrowInstruction,
  getEscrowPda,
  getStatePda,
  getVaultPda,
  parseEscrowAccount,
  svmHexToBytes,
  svmHexToPubkey,
  svmPubkeyToHex,
} from './svm-escrow';

// ============================================================================
// Test Fixtures
// ============================================================================

const PROGRAM_ID = SystemProgram.programId;
const INTENT_ID = '0x' + '01'.repeat(32);
const REQUESTER = new PublicKey('BPFLoader1111111111111111111111111111111111');
const TOKEN_MINT = new PublicKey('So11111111111111111111111111111111111111112');
const SOLVER = new PublicKey('Vote111111111111111111111111111111111111111');
const STATE_PDA = new PublicKey(new Uint8Array(32).fill(1));
const ESCROW_PDA = new PublicKey(new Uint8Array(32).fill(2));
const VAULT_PDA = new PublicKey(new Uint8Array(32).fill(3));

describe('svmHex helpers', () => {
  /**
   * Test: Intent ID padding
   * Why: PDA derivation requires 32-byte intent IDs.
   */
  it('should pad intent IDs to 32 bytes', () => {
    const bytes = svmHexToBytes('0x1');
    expect(bytes).toHaveLength(32);
    expect(bytes[31]).toBe(0x01);
  });

  /**
   * Test: Pubkey hex round-trip
   * Why: Address conversions must be lossless across SVM <-> hex.
   */
  it('should round-trip pubkey hex conversion', () => {
    const pubkey = new PublicKey(new Uint8Array(32).fill(7));
    const hex = svmPubkeyToHex(pubkey);
    expect(hex).toMatch(/^0x[a-f0-9]{64}$/);
    const restored = svmHexToPubkey(hex);
    expect(restored.toBase58()).toBe(pubkey.toBase58());
  });
});

beforeEach(() => {
  vi.spyOn(PublicKey, 'findProgramAddressSync').mockImplementation((seeds) => {
    const seedLabel = Buffer.from(seeds[0]).toString('utf8');
    if (seedLabel === 'state') {
      return [STATE_PDA, 255];
    }
    if (seedLabel === 'escrow') {
      return [ESCROW_PDA, 255];
    }
    if (seedLabel === 'vault') {
      return [VAULT_PDA, 255];
    }
    return [STATE_PDA, 255];
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('PDA helpers', () => {
  /**
   * Test: PDA determinism
   * Why: PDAs must be stable for a given program + intent ID.
   */
  it('should derive deterministic state/escrow/vault PDAs', () => {
    const [stateOne] = getStatePda(PROGRAM_ID);
    const [stateTwo] = getStatePda(PROGRAM_ID);
    expect(stateOne.toBase58()).toBe(stateTwo.toBase58());

    const [escrowOne] = getEscrowPda(INTENT_ID, PROGRAM_ID);
    const [escrowTwo] = getEscrowPda(INTENT_ID, PROGRAM_ID);
    expect(escrowOne.toBase58()).toBe(escrowTwo.toBase58());

    const [vaultOne] = getVaultPda(INTENT_ID, PROGRAM_ID);
    const [vaultTwo] = getVaultPda(INTENT_ID, PROGRAM_ID);
    expect(vaultOne.toBase58()).toBe(vaultTwo.toBase58());
  });
});

describe('parseEscrowAccount', () => {
  /**
   * Test: Escrow account parsing
   * Why: UI needs a stable decoding of on-chain escrow data.
   */
  it('should parse escrow account data into a structured object', () => {
    const data = Buffer.alloc(154);
    Buffer.from('intent00').copy(data, 0);
    Buffer.from(REQUESTER.toBytes()).copy(data, 8);
    Buffer.from(TOKEN_MINT.toBytes()).copy(data, 40);
    data.writeBigUInt64LE(BigInt(123), 72);
    data.writeUInt8(1, 80);
    data.writeBigInt64LE(BigInt(999), 81);
    Buffer.from(SOLVER.toBytes()).copy(data, 89);
    Buffer.from(svmHexToBytes(INTENT_ID)).copy(data, 121);
    data.writeUInt8(42, 153);

    const escrow = parseEscrowAccount(data);
    expect(escrow.requester.toBase58()).toBe(REQUESTER.toBase58());
    expect(escrow.tokenMint.toBase58()).toBe(TOKEN_MINT.toBase58());
    expect(escrow.amount).toBe(BigInt(123));
    expect(escrow.isClaimed).toBe(true);
    expect(escrow.expiry).toBe(BigInt(999));
    expect(escrow.reservedSolver.toBase58()).toBe(SOLVER.toBase58());
    expect(Buffer.from(escrow.intentId).toString('hex')).toBe(INTENT_ID.slice(2));
    expect(escrow.bump).toBe(42);
  });
});

describe('instruction builders', () => {
  /**
   * Test: CreateEscrow instruction layout
   * Why: SVM program expects specific key order and data layout.
   */
  it('should build create escrow instruction with expected layout', () => {
    const instruction = buildCreateEscrowInstruction({
      intentId: INTENT_ID,
      amount: BigInt(500),
      requester: REQUESTER,
      requesterToken: REQUESTER,
      tokenMint: TOKEN_MINT,
      reservedSolver: SOLVER,
      expiryDuration: 120,
      programId: PROGRAM_ID,
    });

    expect(instruction.programId.toBase58()).toBe(PROGRAM_ID.toBase58());
    expect(instruction.keys).toHaveLength(9);
    expect(instruction.data[0]).toBe(1);
    expect(Buffer.from(instruction.data.subarray(1, 33))).toEqual(
      Buffer.from(svmHexToBytes(INTENT_ID))
    );
    expect(instruction.data).toHaveLength(1 + 32 + 8 + 1 + 8);
  });

  /**
   * Test: Claim instruction layout
   * Why: Claim requires SYSVAR instructions and token program keys.
   */
  it('should build claim instruction with sysvar and token program keys', () => {
    const instruction = buildClaimInstruction({
      intentId: INTENT_ID,
      signature: new Uint8Array(64).fill(3),
      solverToken: SOLVER,
      programId: PROGRAM_ID,
    });

    const keyBases = instruction.keys.map((key) => key.pubkey.toBase58());
    expect(keyBases).toContain(SYSVAR_INSTRUCTIONS_PUBKEY.toBase58());
    expect(keyBases).toContain(TOKEN_PROGRAM_ID.toBase58());
    expect(instruction.data[0]).toBe(2);
  });

  /**
   * Test: Cancel instruction layout
   * Why: Cancel must target the escrow PDA and requester token account.
   */
  it('should build cancel instruction with expected layout', () => {
    const instruction = buildCancelInstruction({
      intentId: INTENT_ID,
      requester: REQUESTER,
      requesterToken: REQUESTER,
      programId: PROGRAM_ID,
    });

    expect(instruction.data[0]).toBe(3);
    expect(Buffer.from(instruction.data.subarray(1))).toEqual(
      Buffer.from(svmHexToBytes(INTENT_ID))
    );
  });
});
