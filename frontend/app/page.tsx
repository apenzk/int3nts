'use client';

import { EvmWalletConnector } from "@/components/wallet/EvmWalletConnector";
import { MvmWalletConnector } from "@/components/wallet/MvmWalletConnector";
import { SvmWalletConnector } from "@/components/wallet/SvmWalletConnector";
import { WalletTransactionTest } from "@/components/debug/WalletTransactionTest";
import { IntentBuilder } from "@/components/intent/IntentBuilder";
import { Tabs } from "@/components/Tabs";

export default function Home() {
  return (
    <div className="min-h-screen p-8">
      <main className="max-w-4xl mx-auto">
        <div className="flex justify-between items-start mb-8">
          <div>
            <h1 className="text-4xl font-bold mb-2">int3nts</h1>
            <p className="text-lg"><span className="text-yellow-400">Movement</span> powered cross-chain intents</p>
          </div>
          {/* Wallet connectors in top right */}
          <div className="flex gap-2">
            <MvmWalletConnector />
            <EvmWalletConnector />
            <SvmWalletConnector />
          </div>
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
