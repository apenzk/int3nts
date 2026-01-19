'use client';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { WagmiProvider } from 'wagmi';
import { AptosWalletAdapterProvider } from '@aptos-labs/wallet-adapter-react';
import { ConnectionProvider, WalletProvider } from '@solana/wallet-adapter-react';
import { PhantomWalletAdapter } from '@solana/wallet-adapter-wallets';
import { wagmiConfig } from '@/lib/wagmi-config';
import { useMemo, useState } from 'react';

/**
 * App-level providers for MVM, EVM, and SVM wallets plus React Query.
 */
export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => new QueryClient());
  const svmEndpoint = useMemo(
    () => process.env.NEXT_PUBLIC_SVM_RPC_URL || 'https://api.devnet.solana.com',
    []
  );
  const svmWallets = useMemo(() => [new PhantomWalletAdapter()], []);

  // AptosWalletAdapterProvider will auto-detect wallets that follow the wallet standard
  // Nightly wallet should be detected automatically if installed
  // Using empty array - wallets will be auto-detected
  const wallets: any[] = [];

  return (
    <ConnectionProvider endpoint={svmEndpoint}>
      <WalletProvider wallets={svmWallets} autoConnect={false}>
        <WagmiProvider config={wagmiConfig}>
          <QueryClientProvider client={queryClient}>
            <AptosWalletAdapterProvider plugins={wallets} autoConnect={false}>
              {children}
            </AptosWalletAdapterProvider>
          </QueryClientProvider>
        </WagmiProvider>
      </WalletProvider>
    </ConnectionProvider>
  );
}

