'use client';

import { useWallet } from '@aptos-labs/wallet-adapter-react';
import { useEffect, useState } from 'react';

/**
 * Connect/disconnect MVM wallet (Nightly).
 */
export function MvmWalletConnector() {
  const { account, connect, disconnect, connected, wallets } = useWallet();
  const [mounted, setMounted] = useState(false);
  const [detectedWallets, setDetectedWallets] = useState<string[]>([]);
  const [directNightlyAccount, setDirectNightlyAccount] = useState<string | null>(null);

  useEffect(() => {
    setMounted(true);
    
    // Restore Nightly connection from localStorage - try silent methods first
    if (typeof window !== 'undefined') {
      const savedAddress = localStorage.getItem('nightly_connected_address');
      if (savedAddress) {
        const nightlyWallet = (window as any).nightly?.aptos;
        if (nightlyWallet) {
          // Try getAccount() first - this doesn't prompt the user
          const tryGetAccount = async () => {
            try {
              // Some wallets support getAccount() for checking current state
              if (typeof nightlyWallet.getAccount === 'function') {
                const account = await nightlyWallet.getAccount();
                if (account?.address) {
                  setDirectNightlyAccount(account.address);
                  localStorage.setItem('nightly_connected_address', account.address);
                  window.dispatchEvent(new CustomEvent('nightly_wallet_changed', { detail: { address: account.address } }));
                  return true;
                }
              }
            } catch {
              // getAccount not supported or failed
            }
            
            // Check if wallet has an 'account' property (some wallets expose this)
            if (nightlyWallet.account?.address) {
              setDirectNightlyAccount(nightlyWallet.account.address);
              window.dispatchEvent(new CustomEvent('nightly_wallet_changed', { detail: { address: nightlyWallet.account.address } }));
              return true;
            }
            
            return false;
          };
          
          tryGetAccount().then((silentSuccess) => {
            if (!silentSuccess) {
              // No silent method worked - just trust localStorage for now
              // User will need to reconnect manually if the session expired
              // Don't call connect() here as it prompts the user
              setDirectNightlyAccount(savedAddress);
              // Dispatch custom event so other components know
              window.dispatchEvent(new CustomEvent('nightly_wallet_changed', { detail: { address: savedAddress } }));
            }
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
      <button
        disabled
        className="px-3 py-1.5 bg-gray-700 text-gray-400 rounded text-sm cursor-not-allowed"
      >
        MVM
      </button>
    );
  }

  const isConnected = connected || directNightlyAccount !== null;
  const displayAddress = directNightlyAccount ?? account?.address;
  if (isConnected && !displayAddress) {
    throw new Error('Connected but no address available');
  }

  const handleConnect = async () => {
    try {
      // Try adapter first
      if (wallets.length > 0) {
        const nightlyWallet = wallets.find(w => w.name.toLowerCase().includes('nightly'));
        if (nightlyWallet) {
          try {
            await connect(nightlyWallet.name);
            return;
          } catch (e) {
            console.log('Adapter connection failed, trying direct connection');
          }
        }
      }

      // Try direct connection
      if (detectedWallets.includes('Nightly')) {
        const nightly = (window as any).nightly?.aptos;
        if (nightly) {
          const response = await nightly.connect();
          console.log('Nightly connected:', response);
          
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
          localStorage.setItem('nightly_connected_address', address);
          window.dispatchEvent(new CustomEvent('nightly_wallet_changed', { detail: { address } }));
        }
      }
    } catch (error) {
      console.error('Failed to connect Nightly:', error);
      alert('Failed to connect Nightly wallet. Check console for details.');
    }
  };

  const handleDisconnect = () => {
    if (connected) {
      disconnect();
    }
    setDirectNightlyAccount(null);
    localStorage.removeItem('nightly_connected_address');
    window.dispatchEvent(new CustomEvent('nightly_wallet_changed', { detail: { address: null } }));
  };

  if (isConnected) {
    return (
      <button
        onClick={handleDisconnect}
        className="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 rounded text-sm"
      >
        Disconnect MVM
      </button>
    );
  }

  const hasWallet = wallets.length > 0 || detectedWallets.length > 0;
  if (!hasWallet) {
    return (
      <button
        disabled
        className="px-3 py-1.5 bg-gray-700 text-gray-400 rounded text-sm cursor-not-allowed"
      >
        MVM
      </button>
    );
  }

  return (
    <button
      onClick={handleConnect}
      className="px-3 py-1.5 bg-green-600 hover:bg-green-700 rounded text-sm"
    >
      Connect MVM
    </button>
  );
}

