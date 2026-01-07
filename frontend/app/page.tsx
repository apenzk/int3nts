'use client';

import { WalletConnector } from "@/components/wallet/WalletConnector";
import { MvmWalletConnector } from "@/components/wallet/MvmWalletConnector";
import { VerifierTest } from "@/components/verifier/VerifierTest";
import { Tabs } from "@/components/Tabs";

export default function Home() {
  return (
    <div className="min-h-screen p-8">
      <main className="max-w-4xl mx-auto">
        <h1 className="text-4xl font-bold mb-8">int3nts</h1>
        <p className="text-lg mb-4">cross-chain intents</p>
        
        <Tabs
          tabs={[
            {
              id: 'home',
              label: 'Home',
              content: (
                <div>
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-8">
                    <MvmWalletConnector />
                    <WalletConnector />
                  </div>
                  <p className="text-sm text-gray-400 mt-8">Coming soon...</p>
                </div>
              ),
            },
            {
              id: 'debug',
              label: 'Debug',
              content: <VerifierTest />,
            },
          ]}
          defaultTab="home"
        />
      </main>
    </div>
  );
}
