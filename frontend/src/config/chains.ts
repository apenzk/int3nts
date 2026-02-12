// ============================================================================
// Types
// ============================================================================

// Chain configurations for supported networks

export interface ChainConfig {
  id: string; // Chain identifier
  chainId: number; // Network chain ID
  rpcUrl: string; // RPC endpoint URL
  name: string; // Display name
  chainType: 'mvm' | 'evm' | 'svm'; // VM type for chain-specific logic
  isHub?: boolean; // True when this chain is the hub
  intentContractAddress?: string; // Intent contract address (for Movement hub chain)
  escrowContractAddress?: string; // Escrow contract address (for EVM chains)
  outflowValidatorAddress?: string; // Outflow validator contract address (for EVM chains)
  svmProgramId?: string; // Escrow program ID (for SVM chains)
  svmOutflowProgramId?: string; // Outflow validator program ID (for SVM chains)
  svmGmpEndpointId?: string; // GMP endpoint program ID (for SVM chains)
}

// Chain configurations
// ============================================================================
// Config Definitions
// ============================================================================

export const CHAIN_CONFIGS: Record<string, ChainConfig> = {
  'movement': {
    id: 'movement',
    chainId: 250,
    rpcUrl: 'https://testnet.movementnetwork.xyz/v1',
    name: 'Movement Bardock Testnet',
    chainType: 'mvm',
    isHub: true,
    intentContractAddress: process.env.NEXT_PUBLIC_INTENT_CONTRACT_ADDRESS,
  },
  'svm-devnet': {
    id: 'svm-devnet',
    chainId: 901,
    rpcUrl: process.env.NEXT_PUBLIC_SVM_RPC_URL || 'https://api.devnet.solana.com',
    name: 'Solana Devnet',
    chainType: 'svm',
    svmProgramId: process.env.NEXT_PUBLIC_SVM_PROGRAM_ID,
    svmOutflowProgramId: process.env.NEXT_PUBLIC_SVM_OUTFLOW_PROGRAM_ID,
    svmGmpEndpointId: process.env.NEXT_PUBLIC_SVM_GMP_ENDPOINT_ID,
  },
  'base-sepolia': {
    id: 'base-sepolia',
    chainId: 84532,
    rpcUrl: process.env.NEXT_PUBLIC_BASE_SEPOLIA_RPC_URL!,
    name: 'Base Sepolia',
    chainType: 'evm',
    escrowContractAddress: process.env.NEXT_PUBLIC_BASE_ESCROW_CONTRACT_ADDRESS,
    outflowValidatorAddress: process.env.NEXT_PUBLIC_BASE_OUTFLOW_VALIDATOR_ADDRESS,
  },
  'ethereum-sepolia': {
    id: 'ethereum-sepolia',
    chainId: 11155111,
    rpcUrl: 'https://ethereum-sepolia-rpc.publicnode.com',
    name: 'Ethereum Sepolia',
    chainType: 'evm',
  },
};

// ============================================================================
// Helpers
// ============================================================================

/**
 * Get chain config by ID key.
 */
export function getChainConfig(chainId: string): ChainConfig | undefined {
  return CHAIN_CONFIGS[chainId];
}

/**
 * Get the VM type for a chain.
 */
export function getChainType(chainId: string): ChainConfig['chainType'] | undefined {
  return CHAIN_CONFIGS[chainId]?.chainType;
}

/**
 * Check if a chain is the hub chain.
 */
export function isHubChain(chainId: string): boolean {
  return !!CHAIN_CONFIGS[chainId]?.isHub;
}

/**
 * Get the hub chain config.
 */
export function getHubChainConfig(): ChainConfig {
  const entry = Object.values(CHAIN_CONFIGS).find((config) => config.isHub);
  if (!entry) {
    throw new Error('Hub chain not configured');
  }
  return entry;
}

/**
 * Get the configured RPC URL for a chain.
 */
export function getRpcUrl(chainId: string): string {
  return CHAIN_CONFIGS[chainId]?.rpcUrl || '';
}

/**
 * Get the Movement intent module address.
 */
export function getIntentContractAddress(): string {
  const hubConfig = getHubChainConfig();
  if (!hubConfig.intentContractAddress) {
    throw new Error('Intent contract address not configured for hub chain');
  }
  return hubConfig.intentContractAddress;
}

/**
 * Get the EVM escrow contract address for a chain.
 */
export function getEscrowContractAddress(chainId: string): string {
  const chainConfig = CHAIN_CONFIGS[chainId];
  if (!chainConfig?.escrowContractAddress) {
    throw new Error(`Escrow contract address not configured for chain: ${chainId}`);
  }
  return chainConfig.escrowContractAddress;
}

/**
 * Get the EVM outflow validator contract address for a chain.
 */
export function getOutflowValidatorAddress(chainId: string): string {
  const chainConfig = CHAIN_CONFIGS[chainId];
  if (!chainConfig?.outflowValidatorAddress) {
    throw new Error(`Outflow validator address not configured for chain: ${chainId}`);
  }
  return chainConfig.outflowValidatorAddress;
}

/**
 * Get the SVM outflow validator program ID for a Solana chain.
 */
export function getSvmOutflowProgramId(chainId: string): string {
  const chainConfig = CHAIN_CONFIGS[chainId];
  if (!chainConfig?.svmOutflowProgramId) {
    throw new Error(`SVM outflow program ID not configured for chain: ${chainId}`);
  }
  return chainConfig.svmOutflowProgramId;
}

/**
 * Get the SVM escrow program ID for a Solana chain.
 */
export function getSvmProgramId(chainId: string): string {
  const chainConfig = CHAIN_CONFIGS[chainId];
  if (!chainConfig?.svmProgramId) {
    throw new Error(`SVM program ID not configured for chain: ${chainId}`);
  }
  return chainConfig.svmProgramId;
}

/**
 * Get the SVM GMP endpoint program ID for a Solana chain.
 */
export function getSvmGmpEndpointId(chainId: string): string {
  const chainConfig = CHAIN_CONFIGS[chainId];
  if (!chainConfig?.svmGmpEndpointId) {
    throw new Error(`SVM GMP endpoint ID not configured for chain: ${chainId}`);
  }
  return chainConfig.svmGmpEndpointId;
}

