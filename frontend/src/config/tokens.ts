import type { TokenConfig } from '@int3nts/sdk';
import { TESTNET_TOKENS } from './tokens-testnet';
import { MAINNET_TOKENS } from './tokens-mainnet';

export const SUPPORTED_TOKENS: TokenConfig[] = [
  ...TESTNET_TOKENS,
  ...MAINNET_TOKENS,
];
