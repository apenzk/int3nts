'use client';

import { useState } from 'react';
import { Aptos, AptosConfig } from '@aptos-labs/ts-sdk';

export function WalletTransactionTest() {
  const [result, setResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Migrate tokens from CoinStore to Fungible Asset store
  // Required for USDC, USDT, WETH to work with the intent framework
  const migrateTokenToFA = async (coinType: string, tokenName: string) => {
    setLoading(true);
    setResult(null);
    try {
      const nightlyWallet = (window as any).nightly?.aptos;
      if (!nightlyWallet) {
        setResult('FAIL: Nightly wallet not found');
        return;
      }
      
      // Build transaction using Aptos SDK
      const config = new AptosConfig({ 
        fullnode: 'https://testnet.movementnetwork.xyz/v1',
        network: 'custom' as any,
      });
      const aptos = new Aptos(config);
      
      const savedAddress = localStorage.getItem('nightly_connected_address');
      if (!savedAddress) {
        setResult('FAIL: No wallet address found');
        return;
      }
      
      // Build the migration transaction
      const rawTx = await aptos.transaction.build.simple({
        sender: savedAddress,
        data: {
          function: '0x1::coin::migrate_to_fungible_store',
          typeArguments: [coinType],
          functionArguments: [],
        },
      });
      
      setResult(`Signing ${tokenName} migration transaction...`);
      
      // Sign with Nightly
      const signResponse = await nightlyWallet.signTransaction(rawTx);
      
      if (signResponse?.status === 'Rejected') {
        setResult('FAIL: User rejected the transaction');
        return;
      }
      
      // Extract the authenticator (Nightly wraps it in .args)
      const senderAuthenticator = signResponse?.args || signResponse;
      
      // Submit
      const pendingTx = await aptos.transaction.submit.simple({
        transaction: rawTx,
        senderAuthenticator: senderAuthenticator,
      });
      
      // Wait for confirmation
      const result = await aptos.waitForTransaction({
        transactionHash: pendingTx.hash,
      });
      
      if (result.success) {
        setResult(`SUCCESS: ${tokenName} migrated to Fungible Asset store!\nTx: ${pendingTx.hash}\n\nYour ${tokenName} is now available for intents.`);
      } else {
        setResult(`FAIL: Transaction failed\nTx: ${pendingTx.hash}\nStatus: ${result.vm_status}`);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      if (errorMsg.includes('ECOIN_STORE_NOT_PUBLISHED')) {
        setResult(`No CoinStore found - you don't have any of this token to migrate.`);
      } else {
        setResult(`FAIL: ${errorMsg}`);
      }
    } finally {
      setLoading(false);
    }
  };

  // Token migration coin types
  const COIN_TYPES = {
    USDC: '0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::USDC',
    USDT: '0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::USDT',
    WETH: '0xa6cc575a28e9c97d1cec569392fe6f698c593990e7029ef49fed6740a36a31b0::tokens::WETH',
  };

  return (
    <div className="border border-gray-700 rounded-lg p-4 bg-gray-800/50">
      <h3 className="text-lg font-medium mb-4">Token Migration</h3>
      
      <div className="mb-4">
        <p className="text-xs text-gray-500 mb-2">Token Migration (CoinStore → Fungible Asset):</p>
        <p className="text-xs text-gray-600 mb-4">Required for USDC/USDT/WETH to work with intents</p>
      </div>
      
      <div className="grid grid-cols-1 gap-2 mb-4">
        <button
          onClick={() => migrateTokenToFA(COIN_TYPES.USDC, 'USDC')}
          disabled={loading}
          className="px-3 py-2 bg-yellow-700 hover:bg-yellow-600 rounded text-xs text-left disabled:opacity-50"
        >
          Migrate USDC to Fungible Asset
        </button>
        
        <button
          onClick={() => migrateTokenToFA(COIN_TYPES.USDT, 'USDT')}
          disabled={loading}
          className="px-3 py-2 bg-yellow-700 hover:bg-yellow-600 rounded text-xs text-left disabled:opacity-50"
        >
          Migrate USDT to Fungible Asset
        </button>
        
        <button
          onClick={() => migrateTokenToFA(COIN_TYPES.WETH, 'WETH')}
          disabled={loading}
          className="px-3 py-2 bg-yellow-700 hover:bg-yellow-600 rounded text-xs text-left disabled:opacity-50"
        >
          Migrate WETH to Fungible Asset
        </button>
      </div>
      
      {loading && (
        <p className="text-xs text-yellow-400 animate-pulse mb-2">Processing...</p>
      )}
      
      {result && (
        <div className={`p-3 rounded text-xs font-mono break-all ${
          result.startsWith('SUCCESS') 
            ? 'bg-green-900/50 text-green-300' 
            : result.startsWith('FAIL') 
              ? 'bg-red-900/50 text-red-300'
              : 'bg-yellow-900/50 text-yellow-300'
        }`}>
          {result}
        </div>
      )}
    </div>
  );
}
