import type { TokenConfig } from '@int3nts/sdk';

export const MAINNET_TOKENS: TokenConfig[] = [
  // Movement Mainnet
  {
    symbol: 'MOVE',
    name: 'MOVE (Movement)',
    metadata: '0x1',
    decimals: 8,
    chain: 'movement-mainnet',
  },
  // Base Mainnet
  {
    symbol: 'ETH',
    name: 'ETH (Base)',
    metadata: '0x0000000000000000000000000000000000000000',
    decimals: 18,
    chain: 'base-mainnet',
  },
  // HyperEVM Mainnet
  {
    symbol: 'HYPE',
    name: 'HYPE (HyperEVM)',
    metadata: '0x0000000000000000000000000000000000000000',
    decimals: 18,
    chain: 'hyperliquid',
  },
];
