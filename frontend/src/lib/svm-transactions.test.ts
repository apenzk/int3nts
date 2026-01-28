import { afterEach, describe, expect, it, vi } from 'vitest';
import { Ed25519Program, TransactionInstruction } from '@solana/web3.js';
import { DUMMY_MESSAGE, DUMMY_SIGNATURE_BYTES, DUMMY_PUBKEY_BYTES } from './test-constants';

vi.mock('@/config/chains', () => ({
  getRpcUrl: (chainId: string) => {
    if (chainId === 'svm-devnet') {
      return 'https://example.invalid';
    }
    return 'https://movement.invalid';
  },
  getIntentContractAddress: () => '0x1',
}));

import {
  buildEd25519VerificationIx,
  decodeBase64,
  fetchSolverSvmAddress,
  getSvmConnection,
} from './svm-transactions';

describe('getSvmConnection', () => {
  /**
   * Test: SVM RPC selection
   * Why: Connection must use the configured SVM RPC endpoint.
   */
  it('should use the configured SVM RPC URL', () => {
    const connection = getSvmConnection();
    expect(connection.rpcEndpoint).toBe('https://example.invalid');
  });
});

describe('decodeBase64', () => {
  /**
   * Test: Base64 decoding
   * Why: Trusted-gmp signatures arrive as base64 and must be decoded for Solana.
   */
  it('should decode base64 to bytes', () => {
    const bytes = decodeBase64('AQID');
    expect(Array.from(bytes)).toEqual([1, 2, 3]);
  });

  /**
   * Test: Whitespace handling
   * Why: Inputs may contain leading/trailing whitespace.
   */
  it('should trim whitespace around base64 input', () => {
    const bytes = decodeBase64('  AQID  ');
    expect(Array.from(bytes)).toEqual([1, 2, 3]);
  });
});

describe('buildEd25519VerificationIx', () => {
  /**
   * Test: Ed25519 instruction builder
   * Why: SVM claim flow depends on a valid Ed25519 verification instruction.
   */
  it('should return an instruction targeting the Ed25519 program', () => {
    const mockInstruction = new TransactionInstruction({
      keys: [],
      programId: Ed25519Program.programId,
      data: new Uint8Array([1]),
    });
    const spy = vi
      .spyOn(Ed25519Program, 'createInstructionWithPublicKey')
      .mockReturnValue(mockInstruction);
    const instruction = buildEd25519VerificationIx({
      message: DUMMY_MESSAGE,
      signature: DUMMY_SIGNATURE_BYTES,
      publicKey: DUMMY_PUBKEY_BYTES,
    });
    expect(spy).toHaveBeenCalledWith({
      message: DUMMY_MESSAGE,
      signature: DUMMY_SIGNATURE_BYTES,
      publicKey: DUMMY_PUBKEY_BYTES,
    });
    expect(instruction.programId.toBase58()).toBe(Ed25519Program.programId.toBase58());
    expect(instruction.data.length).toBeGreaterThan(0);
  });
});

describe('fetchSolverSvmAddress', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.resetAllMocks();
  });

  /**
   * Test: Failed RPC request
   * Why: Missing registry data should resolve to null.
   */
  it('should return null when the request fails', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: false });
    vi.stubGlobal('fetch', fetchMock);

    const result = await fetchSolverSvmAddress('0xsolver');
    expect(result).toBeNull();
  });

  /**
   * Test: Empty registry entry
   * Why: Empty registry responses should resolve to null.
   */
  it('should return null when the registry vec is empty', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([{ vec: [] }]),
    });
    vi.stubGlobal('fetch', fetchMock);

    const result = await fetchSolverSvmAddress('0xsolver');
    expect(result).toBeNull();
  });

  /**
   * Test: String address normalization
   * Why: Registry can return a hex string without 0x prefix.
   */
  it('should return normalized hex when vec is a string', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([{ vec: 'abcd' }]),
    });
    vi.stubGlobal('fetch', fetchMock);

    const result = await fetchSolverSvmAddress('0xsolver');
    expect(result).toBe('0xabcd');
  });

  /**
   * Test: Vector<u8> address conversion
   * Why: Registry can return byte arrays that must be hex-encoded.
   */
  it('should convert vec byte array to hex', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve([{ vec: [1, 2, 255] }]),
    });
    vi.stubGlobal('fetch', fetchMock);

    const result = await fetchSolverSvmAddress('0xsolver');
    expect(result).toBe('0x0102ff');
  });
});
