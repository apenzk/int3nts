'use client';

import { useWallet } from '@aptos-labs/wallet-adapter-react';
import { useEffect, useState } from 'react';

export function MvmWalletConnector() {
  const { account, connect, disconnect, connected, wallets } = useWallet();
  const [mounted, setMounted] = useState(false);
  const [detectedWallets, setDetectedWallets] = useState<string[]>([]);
  const [directNightlyAccount, setDirectNightlyAccount] = useState<string | null>(null);

  useEffect(() => {
    setMounted(true);
    
    // Restore Nightly connection from localStorage and try to reconnect silently
    if (typeof window !== 'undefined') {
      const savedAddress = localStorage.getItem('nightly_connected_address');
      if (savedAddress) {
        const nightlyWallet = (window as any).nightly?.aptos;
        if (nightlyWallet) {
          // Try to silently reconnect
          nightlyWallet.connect().then((response: any) => {
            const address = response?.address || (Array.isArray(response) ? response[0]?.address : null);
            if (address) {
              setDirectNightlyAccount(address);
              localStorage.setItem('nightly_connected_address', address);
            } else if (response?.status === 'Approved' || response?.status === 'AlreadyConnected') {
              // Wallet approved but didn't return address, use saved
              setDirectNightlyAccount(savedAddress);
            } else {
              // Connection failed or rejected
              localStorage.removeItem('nightly_connected_address');
              setDirectNightlyAccount(null);
            }
          }).catch(() => {
            // Silent reconnect failed, clear state
            localStorage.removeItem('nightly_connected_address');
            setDirectNightlyAccount(null);
          });
        } else {
          // Wallet extension not found, clear stale state
          localStorage.removeItem('nightly_connected_address');
          setDirectNightlyAccount(null);
        }
      }
    }
    
    // Check for wallets directly on window object
    // Use a small delay to ensure extensions are loaded
    const checkWallets = () => {
      const windowWallets: string[] = [];
      if (typeof window !== 'undefined') {
        if ((window as any).nightly?.aptos) {
          windowWallets.push('Nightly');
        }
        if ((window as any).martian) {
          windowWallets.push('Martian');
        }
        if ((window as any).pontem) {
          windowWallets.push('Pontem');
        }
        if ((window as any).petra) {
          windowWallets.push('Petra');
        }
      }
      console.log('Detected window wallets:', windowWallets);
      setDetectedWallets(windowWallets);
    };
    
    // Check immediately
    checkWallets();
    
    // Also check after a short delay in case extensions load later
    const timeout = setTimeout(checkWallets, 500);
    
    return () => clearTimeout(timeout);
  }, []);

  // Debug: log all available wallets
  useEffect(() => {
    if (mounted) {
      console.log('Adapter detected wallets:', wallets.map(w => w.name));
      console.log('Window wallets:', detectedWallets);
    }
  }, [mounted, wallets, detectedWallets]);

  if (!mounted) {
    return (
      <div className="border border-gray-700 rounded p-4 mb-4">
        <h2 className="text-xl font-bold mb-2">MVM Wallet</h2>
        <p className="text-sm text-gray-400 mb-2">Loading...</p>
      </div>
    );
  }

  const isConnected = connected || directNightlyAccount !== null;
  const displayAddress = directNightlyAccount ?? account?.address;
  if (isConnected && !displayAddress) {
    throw new Error('Connected but no address available');
  }

  return (
    <div className="border border-gray-700 rounded p-4 mb-4">
      <h2 className="text-xl font-bold mb-2">MVM Wallet</h2>
      {isConnected ? (
        <div>
          <p className="text-sm text-gray-400 mb-2">Connected</p>
          <p className="text-xs font-mono mb-1">Address: {displayAddress}</p>
          <button
            onClick={() => {
              if (connected) {
                disconnect();
              }
              setDirectNightlyAccount(null);
              localStorage.removeItem('nightly_connected_address');
            }}
            className="px-4 py-2 bg-red-600 hover:bg-red-700 rounded text-sm"
          >
            Disconnect
          </button>
        </div>
      ) : (
        <div>
          <p className="text-sm text-gray-400 mb-2">Not connected</p>
          
          {/* Show detected wallets from adapter */}
          {wallets.length > 0 && (
            <div className="mb-4">
              <p className="text-xs text-gray-500 mb-2">Adapter detected wallets:</p>
              <div className="space-y-2">
                {wallets.map((wallet) => (
                  <button
                    key={wallet.name}
                    onClick={() => connect(wallet.name)}
                    className="block w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm text-left"
                  >
                    Connect {wallet.name}
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Show wallets detected on window object with direct connect */}
          {detectedWallets.length > 0 && (
            <div className="mb-4">
              <div className="space-y-2">
                {detectedWallets.map((wallet) => {
                  if (wallet.includes('Nightly')) {
                    return (
                      <button
                        key={wallet}
                        onClick={async () => {
                          try {
                            const nightly = (window as any).nightly?.aptos;
                            if (nightly) {
                              const response = await nightly.connect();
                              console.log('Nightly connected:', response);
                              
                              // Store the connected account
                              // Response structure: { status: 'Approved', address: '...', publicKey: '...' }
                              let address: string;
                              if (response && response.address) {
                                address = response.address;
                              } else if (Array.isArray(response) && response.length > 0) {
                                const first = response[0];
                                if (first?.address) {
                                  address = first.address;
                                } else if (typeof first === 'string') {
                                  address = first;
                                } else {
                                  throw new Error('Invalid response format: address not found');
                                }
                              } else {
                                throw new Error('Invalid response format: no address in response');
                              }
                              
                              setDirectNightlyAccount(address);
                              // Persist to localStorage
                              localStorage.setItem('nightly_connected_address', address);
                              
                              // Also try to connect via adapter if possible
                              const nightlyWallet = wallets.find(w => w.name.toLowerCase().includes('nightly'));
                              if (nightlyWallet) {
                                try {
                                  await connect(nightlyWallet.name);
                                } catch (e) {
                                  console.log('Adapter connection failed, using direct connection');
                                }
                              }
                            }
                          } catch (error) {
                            console.error('Failed to connect Nightly:', error);
                            alert('Failed to connect Nightly wallet. Check console for details.');
                          }
                        }}
                        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm"
                      >
                        Connect Nightly
                      </button>
                    );
                  }
                  return (
                    <p key={wallet} className="text-xs font-mono text-gray-400">• {wallet}</p>
                  );
                })}
              </div>
            </div>
          )}

          {wallets.length === 0 && detectedWallets.length === 0 && (
            <div>
              <p className="text-xs text-gray-500 mb-2">No Aptos wallets detected</p>
              <a 
                href="https://nightly.app/download" 
                target="_blank" 
                rel="noopener noreferrer"
                className="text-xs text-blue-400 hover:underline"
              >
                Install Nightly Wallet →
              </a>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

