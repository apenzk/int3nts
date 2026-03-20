import { createConfig, http } from 'wagmi';
import { mainnet, sepolia, base, baseSepolia } from 'viem/chains';
import { hyperEvm } from 'viem/chains';

// ============================================================================
// Config
// ============================================================================

export const wagmiConfig = createConfig({
  chains: [mainnet, sepolia, base, baseSepolia, hyperEvm],
  transports: {
    [mainnet.id]: http(),
    [sepolia.id]: http(),
    [base.id]: http(),
    [baseSepolia.id]: http(),
    [hyperEvm.id]: http(),
  },
});
