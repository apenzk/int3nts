import { createConfig, http } from 'wagmi';
import { mainnet, sepolia, baseSepolia } from 'viem/chains';

// ============================================================================
// Config
// ============================================================================

export const wagmiConfig = createConfig({
  chains: [mainnet, sepolia, baseSepolia],
  transports: {
    [mainnet.id]: http(),
    [sepolia.id]: http(),
    [baseSepolia.id]: http(),
  },
});

