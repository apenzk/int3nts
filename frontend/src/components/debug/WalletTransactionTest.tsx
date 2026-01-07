'use client';

import { useState } from 'react';
import { Aptos, AptosConfig } from '@aptos-labs/ts-sdk';
import { INTENT_MODULE_ADDRESS } from '@/lib/move-transactions';

export function WalletTransactionTest() {
  const [result, setResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const test1_WalletConnection = async () => {
    setLoading(true);
    setResult(null);
    try {
      const nightlyWallet = (window as any).nightly?.aptos;
      if (!nightlyWallet) {
        setResult('FAIL: Nightly wallet not found on window');
        return;
      }
      const savedAddress = localStorage.getItem('nightly_connected_address');
      if (savedAddress) {
        setResult(`PASS: Wallet connected (from storage). Address: ${savedAddress}`);
        return;
      }
      const response = await nightlyWallet.connect();
      
      if (response?.status === 'Rejected') {
        setResult('FAIL: User rejected connection');
        return;
      }
      
      const address = response?.address || (Array.isArray(response) ? response[0]?.address : null);
      if (address) {
        localStorage.setItem('nightly_connected_address', address);
        setResult(`PASS: Wallet connected. Address: ${address}`);
      } else {
        setResult(`FAIL: No address in response: ${JSON.stringify(response)}`);
      }
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test2_VerifierConnection = async () => {
    setLoading(true);
    setResult(null);
    try {
      const verifierUrl = process.env.NEXT_PUBLIC_VERIFIER_URL || 'http://localhost:3030';
      const response = await fetch(`${verifierUrl}/health`);
      if (response.ok) {
        const data = await response.json();
        setResult(`PASS: Verifier reachable. Status: ${JSON.stringify(data)}`);
      } else {
        setResult(`FAIL: Verifier HTTP ${response.status}`);
      }
    } catch (err) {
      setResult(`INFO: Verifier not reachable (${err instanceof Error ? err.message : String(err)})`);
    } finally {
      setLoading(false);
    }
  };

  const test3_SignMessage = async () => {
    setLoading(true);
    setResult(null);
    try {
      const nightlyWallet = (window as any).nightly?.aptos;
      if (!nightlyWallet) {
        throw new Error('Nightly wallet not found');
      }
      
      const message = 'Test message for signing';
      
      if (nightlyWallet.signMessage) {
        const response = await nightlyWallet.signMessage({ message, nonce: '12345' });
        
        if (response?.status === 'Rejected') {
          setResult('FAIL: User rejected signing');
        } else if (response?.status === 'Approved' || response?.signature) {
          const sig = response.signature || response.args?.signature || 'present';
          setResult(`PASS: Message signed. Signature: ${typeof sig === 'string' ? sig.slice(0, 20) : sig}...`);
        } else {
          setResult(`INFO: Unexpected response: ${JSON.stringify(response)}`);
        }
      } else {
        setResult('INFO: signMessage not available on wallet');
      }
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test4_MovementBalance = async () => {
    setLoading(true);
    setResult(null);
    try {
      const address = localStorage.getItem('nightly_connected_address');
      if (!address) {
        setResult('FAIL: Wallet not connected - run Test 1 first');
        return;
      }

      const rpcUrl = 'https://testnet.movementnetwork.xyz/v1';
      
      // Test 1: Check if RPC is reachable
      console.log('Testing RPC connectivity...');
      const healthResponse = await fetch(rpcUrl);
      if (!healthResponse.ok) {
        setResult(`FAIL: RPC not reachable. HTTP ${healthResponse.status}`);
        return;
      }
      console.log('RPC is reachable');

      // Test 2: Fetch native MOVE balance via resources
      console.log('Fetching resources for:', address);
      const resourcesResponse = await fetch(`${rpcUrl}/accounts/${address}/resources`);
      console.log('Resources response status:', resourcesResponse.status);
      
      if (!resourcesResponse.ok) {
        const text = await resourcesResponse.text();
        setResult(`FAIL: Resources request failed. HTTP ${resourcesResponse.status}: ${text.slice(0, 100)}`);
        return;
      }

      const resources = await resourcesResponse.json();
      console.log('Resources count:', resources.length);
      
      const coinStore = resources.find(
        (r: any) => r.type === '0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>'
      );
      
      const moveBalance = coinStore?.data?.coin?.value || '0';
      
      // Test 3: Fetch USDC.e balance via view function
      console.log('Fetching USDC.e balance via view function...');
      const usdcMetadata = '0xb89077cfd2a82a0c1450534d49cfd5f2707643155273069bc23a912bcfefdee7';
      const viewResponse = await fetch(`${rpcUrl}/view`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          function: '0x1::primary_fungible_store::balance',
          type_arguments: ['0x1::fungible_asset::Metadata'],
          arguments: [address, usdcMetadata],
        }),
      });
      
      console.log('View response status:', viewResponse.status);
      
      if (!viewResponse.ok) {
        const text = await viewResponse.text();
        setResult(`PASS (partial): MOVE=${moveBalance}. USDC.e view failed: ${text.slice(0, 100)}`);
        return;
      }

      const viewResult = await viewResponse.json();
      console.log('View result:', viewResult);
      const usdcBalance = viewResult[0] || '0';

      setResult(`PASS: MOVE=${moveBalance} (8 decimals), USDC.e=${usdcBalance} (6 decimals)`);
    } catch (err) {
      console.error('Balance test error:', err);
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test5_BuildSignSubmit = async () => {
    setLoading(true);
    setResult(null);
    try {
      const nightlyWallet = (window as any).nightly?.aptos;
      if (!nightlyWallet) {
        throw new Error('Nightly wallet not found');
      }
      const address = localStorage.getItem('nightly_connected_address');
      if (!address) {
        throw new Error('Wallet not connected - run Test 1 first');
      }
      
      const config = new AptosConfig({
        fullnode: 'https://testnet.movementnetwork.xyz/v1',
      });
      const aptos = new Aptos(config);
      
      console.log('Building transaction with SDK...');
      const rawTxn = await aptos.transaction.build.simple({
        sender: address as `0x${string}`,
        data: {
          function: '0x1::aptos_account::transfer',
          functionArguments: [address, 1],
        },
      });
      
      console.log('Raw transaction built:', rawTxn);
      
      const signResponse = await nightlyWallet.signTransaction(rawTxn);
      console.log('Sign response:', signResponse);
      
      if (signResponse?.status === 'Rejected') {
        setResult('FAIL: User rejected signing');
        return;
      }
      
      const signedTxn = signResponse?.args || signResponse;
      
      const pendingTxn = await aptos.transaction.submit.simple({
        transaction: rawTxn,
        senderAuthenticator: signedTxn,
      });
      
      console.log('Submitted:', pendingTxn);
      setResult(`PASS: Transaction submitted! Hash: ${pendingTxn.hash}`);
    } catch (err) {
      const errMsg = err instanceof Error ? err.message : String(err);
      console.error('Transaction error:', err);
      setResult(`FAIL: ${errMsg}`);
    } finally {
      setLoading(false);
    }
  };

  const test6_VerifierConfig = async () => {
    setLoading(true);
    setResult(null);
    try {
      const rpcUrl = 'https://testnet.movementnetwork.xyz/v1';
      const response = await fetch(`${rpcUrl}/accounts/${INTENT_MODULE_ADDRESS}/resource/${INTENT_MODULE_ADDRESS}::fa_intent_outflow::VerifierConfig`);
      if (response.ok) {
        const data = await response.json();
        setResult(`PASS: VerifierConfig exists. Data: ${JSON.stringify(data).slice(0, 200)}`);
      } else if (response.status === 404) {
        setResult('FAIL: VerifierConfig not found - need to call initialize_verifier');
      } else {
        setResult(`FAIL: HTTP ${response.status}`);
      }
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="border border-gray-700 rounded-lg p-4 bg-gray-800/50">
      <h3 className="text-lg font-medium mb-4">Wallet & Chain Tests</h3>
      <p className="text-xs text-gray-400 mb-4">
        Debug tests for wallet connection and chain interactions:
      </p>
      
      <div className="grid grid-cols-1 gap-2 mb-4">
        <button
          onClick={test1_WalletConnection}
          disabled={loading}
          className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 1: Check Wallet Connection
        </button>
        
        <button
          onClick={test2_VerifierConnection}
          disabled={loading}
          className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 2: Verifier Connection
        </button>
        
        <button
          onClick={test3_SignMessage}
          disabled={loading}
          className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 3: Sign Message
        </button>
        
        <button
          onClick={test4_MovementBalance}
          disabled={loading}
          className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 4: Movement Balance (MOVE + USDC.e)
        </button>
        
        <button
          onClick={test5_BuildSignSubmit}
          disabled={loading}
          className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 5: Build, Sign, Submit Transaction
        </button>
        
        <button
          onClick={test6_VerifierConfig}
          disabled={loading}
          className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 6: Check VerifierConfig on-chain
        </button>
      </div>
      
      {loading && (
        <p className="text-xs text-yellow-400 animate-pulse mb-2">Running test...</p>
      )}
      
      {result && (
        <div className={`p-3 rounded text-xs font-mono break-all ${
          result.startsWith('PASS') 
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
