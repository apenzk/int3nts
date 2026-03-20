import type { ChainConfig } from '@int3nts/sdk';

export const MAINNET_CHAINS: Record<string, ChainConfig> = {
  'movement-mainnet': {
    id: 'movement-mainnet',
    chainId: 250,
    rpcUrl: process.env.NEXT_PUBLIC_MOVEMENT_MAINNET_RPC_URL!,
    name: 'Movement Mainnet',
    chainType: 'mvm',
    isHub: true,
    intentContractAddress: process.env.NEXT_PUBLIC_MOVEMENT_MAINNET_INTENT_CONTRACT_ADDRESS,
  },
  'base-mainnet': {
    id: 'base-mainnet',
    chainId: 8453,
    rpcUrl: process.env.NEXT_PUBLIC_BASE_MAINNET_RPC_URL!,
    name: 'Base',
    chainType: 'evm',
    escrowContractAddress: process.env.NEXT_PUBLIC_BASE_MAINNET_ESCROW_CONTRACT_ADDRESS,
    outflowValidatorAddress: process.env.NEXT_PUBLIC_BASE_MAINNET_OUTFLOW_VALIDATOR_ADDRESS,
  },
  'hyperliquid': {
    id: 'hyperliquid',
    chainId: 999,
    rpcUrl: process.env.NEXT_PUBLIC_HYPERLIQUID_MAINNET_RPC_URL!,
    name: 'HyperEVM',
    chainType: 'evm',
    escrowContractAddress: process.env.NEXT_PUBLIC_HYPERLIQUID_MAINNET_ESCROW_CONTRACT_ADDRESS,
    outflowValidatorAddress: process.env.NEXT_PUBLIC_HYPERLIQUID_MAINNET_OUTFLOW_VALIDATOR_ADDRESS,
  },
};
