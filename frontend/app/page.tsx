import { WalletConnector } from "@/components/wallet/WalletConnector";
import { MvmWalletConnector } from "@/components/wallet/MvmWalletConnector";

export default function Home() {
  return (
    <div className="min-h-screen p-8">
      <main className="max-w-4xl mx-auto">
        <h1 className="text-4xl font-bold mb-8">int3nts</h1>
        <p className="text-lg mb-4">cross-chain intents</p>
        
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mb-8">
          <MvmWalletConnector />
          <WalletConnector />
        </div>
        
        <p className="text-sm text-gray-400 mt-8">Coming soon...</p>
      </main>
    </div>
  );
}
