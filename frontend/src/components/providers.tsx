'use client';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { WagmiProvider } from 'wagmi';
import { AptosWalletAdapterProvider } from '@aptos-labs/wallet-adapter-react';
import { wagmiConfig } from '@/lib/wagmi-config';
import { useState, useEffect } from 'react';

export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => new QueryClient());

  // AptosWalletAdapterProvider will auto-detect wallets that follow the wallet standard
  // Nightly wallet should be detected automatically if installed
  // Using empty array - wallets will be auto-detected
  const wallets: any[] = [];

  return (
    <WagmiProvider config={wagmiConfig}>
      <QueryClientProvider client={queryClient}>
        <AptosWalletAdapterProvider plugins={wallets} autoConnect={false}>
          {children}
        </AptosWalletAdapterProvider>
      </QueryClientProvider>
    </WagmiProvider>
  );
}

