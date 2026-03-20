import type { ChainConfig } from '@int3nts/sdk';
import { TESTNET_CHAINS } from './chains-testnet';
import { MAINNET_CHAINS } from './chains-mainnet';

export const CHAIN_CONFIGS: Record<string, ChainConfig> = {
  ...TESTNET_CHAINS,
  ...MAINNET_CHAINS,
};
