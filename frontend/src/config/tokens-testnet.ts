import type { TokenConfig } from '@int3nts/sdk';

export const TESTNET_TOKENS: TokenConfig[] = [
  // Movement Bardock Testnet
  {
    symbol: 'MOVE',
    name: 'MOVE (Movement)',
    metadata: '0x1',
    decimals: 8,
    chain: 'movement',
  },
  {
    symbol: 'USDC.e',
    name: 'USDC.e (Movement)',
    metadata: '0xb89077cfd2a82a0c1450534d49cfd5f2707643155273069bc23a912bcfefdee7',
    decimals: 6,
    chain: 'movement',
  },
  {
    symbol: 'USDC',
    name: 'USDC (Movement)',
    metadata: '0x351a5fbcb9ccd79a7a3c4f203dca74bb02d681221771fd37694d9cd15112f27e',
    decimals: 6,
    chain: 'movement',
    coinType: '0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::USDC',
  },
  {
    symbol: 'USDT',
    name: 'USDT (Movement)',
    metadata: '0xe8d4819362f685b3276275ab44e1a20e2a30ae8e8bbbfb5126329a45e44ac4e0',
    decimals: 6,
    chain: 'movement',
    coinType: '0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::USDT',
  },
  {
    symbol: 'WETH',
    name: 'WETH (Movement)',
    metadata: '0x2fa1ab0e37fdd22cbf9da880826e9f79f06e8e5d9df9bce774b1f47b708fe121',
    decimals: 8,
    chain: 'movement',
    coinType: '0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::WETH',
  },
  // Base Sepolia
  {
    symbol: 'USDC',
    name: 'USDC (Base Sepolia)',
    metadata: '0x036CbD53842c5426634e7929541eC2318f3dCF7e',
    decimals: 6,
    chain: 'base-sepolia',
  },
  {
    symbol: 'ETH',
    name: 'ETH (Base Sepolia)',
    metadata: '0x0000000000000000000000000000000000000000',
    decimals: 18,
    chain: 'base-sepolia',
  },
  // Ethereum Sepolia
  {
    symbol: 'USDC',
    name: 'USDC (Ethereum Sepolia)',
    metadata: '0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238',
    decimals: 6,
    chain: 'ethereum-sepolia',
  },
  {
    symbol: 'ETH',
    name: 'ETH (Ethereum Sepolia)',
    metadata: '0x0000000000000000000000000000000000000000',
    decimals: 18,
    chain: 'ethereum-sepolia',
  },
  // Solana Devnet
  {
    symbol: 'SOL',
    name: 'SOL (Solana Devnet)',
    metadata: 'SOL',
    decimals: 9,
    chain: 'svm-devnet',
  },
  {
    symbol: 'USDC',
    name: 'USDC (Solana Devnet)',
    metadata: '4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU',
    decimals: 6,
    chain: 'svm-devnet',
  },
];
