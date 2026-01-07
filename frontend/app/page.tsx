'use client';

import { WalletConnector } from "@/components/wallet/WalletConnector";
import { MvmWalletConnector } from "@/components/wallet/MvmWalletConnector";
import { VerifierTest } from "@/components/verifier/VerifierTest";
import { WalletTransactionTest } from "@/components/debug/WalletTransactionTest";
import { IntentBuilder } from "@/components/intent/IntentBuilder";
import { Tabs } from "@/components/Tabs";

export default function Home() {
  return (
    <div className="min-h-screen p-8">
      <main className="max-w-4xl mx-auto">
        <h1 className="text-4xl font-bold mb-8">int3nts</h1>
        <p className="text-lg mb-4">Movement cross-chain intents</p>
        
        {/* Wallet connectors outside tabs so they don't remount on tab switch */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-6">
          <MvmWalletConnector />
          <WalletConnector />
        </div>
        
        <Tabs
          tabs={[
            {
              id: 'home',
              label: 'Home',
              content: <IntentBuilder />,
            },
            {
              id: 'debug',
              label: 'Debug',
              content: (
                <div className="space-y-6">
                  <WalletTransactionTest />
                  <VerifierTest />
                </div>
              ),
            },
          ]}
          defaultTab="home"
        />
      </main>
    </div>
  );
}
