'use client';

import { useAccount, useConnect, useDisconnect } from 'wagmi';
import { useEffect, useState } from 'react';

export function WalletConnector() {
  const { address, isConnected, chainId } = useAccount();
  const { connect, connectors, isPending } = useConnect();
  const { disconnect } = useDisconnect();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  const metaMaskConnector = connectors.find((c) => c.id === 'metaMaskSDK' || c.id === 'io.metamask');

  if (!mounted) {
    return (
      <div className="border border-gray-700 rounded p-4 mb-4">
        <h2 className="text-xl font-bold mb-2">EVM Wallet</h2>
        <p className="text-sm text-gray-400 mb-2">Loading...</p>
      </div>
    );
  }

  return (
    <div className="border border-gray-700 rounded p-4 mb-4">
        <h2 className="text-xl font-bold mb-2">EVM Wallet</h2>
      {isConnected ? (
        <div>
          <p className="text-sm text-gray-400 mb-2">Connected</p>
          <p className="text-xs font-mono mb-1">Address: {address}</p>
          <p className="text-xs font-mono mb-2">Chain ID: {chainId}</p>
          <button
            onClick={() => disconnect()}
            className="px-4 py-2 bg-red-600 hover:bg-red-700 rounded text-sm"
          >
            Disconnect
          </button>
        </div>
      ) : (
        <div>
          <p className="text-sm text-gray-400 mb-2">Not connected</p>
          {metaMaskConnector ? (
            <button
              onClick={() => connect({ connector: metaMaskConnector })}
              disabled={isPending}
              className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm disabled:opacity-50"
            >
              {isPending ? 'Connecting...' : 'Connect MetaMask'}
            </button>
          ) : (
            <p className="text-xs text-gray-500">MetaMask not detected</p>
          )}
        </div>
      )}
    </div>
  );
}

