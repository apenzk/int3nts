import type { ChainConfig } from '@int3nts/sdk';

export const TESTNET_CHAINS: Record<string, ChainConfig> = {
  'movement': {
    id: 'movement',
    chainId: 250,
    rpcUrl: process.env.NEXT_PUBLIC_MOVEMENT_TESTNET_RPC_URL!,
    name: 'Movement Bardock Testnet',
    chainType: 'mvm',
    isHub: true,
    intentContractAddress: process.env.NEXT_PUBLIC_MOVEMENT_TESTNET_INTENT_CONTRACT_ADDRESS,
  },
  'svm-devnet': {
    id: 'svm-devnet',
    chainId: 901,
    rpcUrl: process.env.NEXT_PUBLIC_SOLANA_TESTNET_RPC_URL!,
    name: 'Solana Devnet',
    chainType: 'svm',
    svmProgramId: process.env.NEXT_PUBLIC_SOLANA_TESTNET_PROGRAM_ID,
    svmOutflowProgramId: process.env.NEXT_PUBLIC_SOLANA_TESTNET_OUTFLOW_PROGRAM_ID,
    svmGmpEndpointId: process.env.NEXT_PUBLIC_SOLANA_TESTNET_GMP_ENDPOINT_ID,
  },
  'base-sepolia': {
    id: 'base-sepolia',
    chainId: 84532,
    rpcUrl: process.env.NEXT_PUBLIC_BASE_TESTNET_RPC_URL!,
    name: 'Base Sepolia',
    chainType: 'evm',
    escrowContractAddress: process.env.NEXT_PUBLIC_BASE_TESTNET_ESCROW_CONTRACT_ADDRESS,
    outflowValidatorAddress: process.env.NEXT_PUBLIC_BASE_TESTNET_OUTFLOW_VALIDATOR_ADDRESS,
  },
  'ethereum-sepolia': {
    id: 'ethereum-sepolia',
    chainId: 11155111,
    rpcUrl: process.env.NEXT_PUBLIC_ETH_TESTNET_RPC_URL!,
    name: 'Ethereum Sepolia',
    chainType: 'evm',
  },
};
