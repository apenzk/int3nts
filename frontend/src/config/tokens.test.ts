import { describe, expect, it } from 'vitest';
import { fromSmallestUnits, getTokensByChain, toSmallestUnits } from './tokens';

describe('getTokensByChain', () => {
  /**
   * Test: SVM token list
   * Why: UI needs chain-specific token options to render correctly.
   */
  it('should return SVM tokens for svm-devnet', () => {
    const tokens = getTokensByChain('svm-devnet');
    const symbols = tokens.map((token) => token.symbol);
    expect(symbols).toContain('SOL');
    expect(symbols).toContain('USDC');
  });
});

describe('unit conversions', () => {
  /**
   * Test: Decimal to smallest units
   * Why: Token amounts must be serialized as integers for on-chain usage.
   */
  it('should convert to smallest units', () => {
    expect(toSmallestUnits(1.5, 6)).toBe(1_500_000);
  });

  /**
   * Test: Smallest units to decimal
   * Why: UI display must convert from on-chain units to human-readable values.
   */
  it('should convert from smallest units', () => {
    expect(fromSmallestUnits(1_500_000, 6)).toBe(1.5);
  });
});
