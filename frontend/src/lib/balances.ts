// Balance fetching utilities for Movement and EVM chains

import type { TokenConfig } from '@/config/tokens';
import { fromSmallestUnits } from '@/config/tokens';
import { getRpcUrl } from '@/config/chains';

export interface TokenBalance {
  raw: string; // Balance in smallest units
  formatted: string; // Balance in main units
  symbol: string;
}

/**
 * Fetch Movement token balance (Fungible Asset)
 */
export async function fetchMovementBalance(
  address: string,
  token: TokenConfig
): Promise<TokenBalance | null> {
  try {
    // For native MOVE token, use coin store
    if (token.symbol === 'MOVE' && token.metadata === '0x1') {
      const rpcUrl = getRpcUrl('movement');
      const response = await fetch(`${rpcUrl}/accounts/${address}/resources`, {
        method: 'GET',
      });
      
      if (!response.ok) {
        return null;
      }
      
      const resources = await response.json();
      const coinStore = resources.find(
        (r: any) => r.type === '0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>'
      );
      
      if (!coinStore) {
        return { raw: '0', formatted: '0', symbol: token.symbol };
      }
      
      const raw = coinStore.data.coin.value || '0';
      const formatted = fromSmallestUnits(parseInt(raw), token.decimals).toFixed(token.decimals);
      
      return { raw, formatted, symbol: token.symbol };
    }
    
    // For fungible assets, use primary_fungible_store::balance
    const rpcUrl = getRpcUrl('movement');
    console.log('Fetching Movement FA balance:', { rpcUrl, address, metadata: token.metadata });
    
    const response = await fetch(`${rpcUrl}/view`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        function: '0x1::primary_fungible_store::balance',
        type_arguments: ['0x1::fungible_asset::Metadata'],
        arguments: [address, token.metadata],
      }),
    });
    
    console.log('Movement balance response status:', response.status);
    
    if (!response.ok) {
      const text = await response.text();
      console.error('Movement balance request failed:', response.status, text);
      return null;
    }
    
    const result = await response.json();
    console.log('Movement balance result:', result);
    const raw = result[0] || '0';
    const formatted = fromSmallestUnits(parseInt(raw), token.decimals).toFixed(token.decimals);
    
    return { raw, formatted, symbol: token.symbol };
  } catch (error) {
    console.error('Error fetching Movement balance:', error);
    console.error('Token:', token.symbol, 'Address:', address);
    return null;
  }
}

/**
 * Fetch EVM token balance (ERC20)
 */
export async function fetchEvmBalance(
  address: string,
  token: TokenConfig
): Promise<TokenBalance | null> {
  try {
    // Get RPC URL for this chain
    const rpcUrl = getRpcUrl(token.chain);
    console.log('Fetching EVM balance:', { chain: token.chain, rpcUrl, address, token: token.symbol });
    
    // For native ETH, use eth_getBalance
    if (token.symbol === 'ETH' && token.metadata === '0x0000000000000000000000000000000000000000') {
      console.log('Fetching native ETH balance');
      const response = await fetch(rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          method: 'eth_getBalance',
          params: [address, 'latest'],
          id: 1,
        }),
      });
      
      if (!response.ok) {
        return null;
      }
      
      const result = await response.json();
      const rawHex = result.result || '0x0';
      // Handle invalid hex values like "0x" or empty strings
      const validHex = rawHex === '0x' || rawHex === '' ? '0x0' : rawHex;
      const raw = BigInt(validHex).toString();
      const formatted = fromSmallestUnits(Number(raw), token.decimals).toFixed(token.decimals);
      
      return { raw, formatted, symbol: token.symbol };
    }
    
    // For ERC20 tokens, use balanceOf
    // balanceOf(address) selector = 0x70a08231
    // For EVM tokens, metadata field contains the contract address (20-byte or 32-byte format)
    let tokenAddress = token.metadata;
    // If it's in 32-byte format (66 chars), extract the 20-byte address
    if (token.metadata.length === 66 && token.metadata.startsWith('0x000000000000000000000000')) {
      tokenAddress = '0x' + token.metadata.slice(-40);
    }
    
    const addressPadded = address.toLowerCase().replace('0x', '').padStart(64, '0');
    const data = `0x70a08231${addressPadded}`;
    
    console.log('Fetching ERC20 balance:', { tokenAddress, data, rpcUrl });
    const response = await fetch(rpcUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        jsonrpc: '2.0',
        method: 'eth_call',
        params: [{ to: tokenAddress, data }, 'latest'],
        id: 1,
      }),
    });
    
    console.log('RPC response status:', response.status, response.statusText);
    
    if (!response.ok) {
      console.error(`RPC request failed: ${response.status} ${response.statusText}`);
      const text = await response.text();
      console.error('Response body:', text);
      return null;
    }
    
    const result = await response.json();
    console.log('RPC response:', result);
    
    // Check for JSON-RPC errors
    if (result.error) {
      console.error('RPC error:', result.error);
      return null;
    }
    
    const rawHex = result.result || '0x0';
    // Handle invalid hex values like "0x" or empty strings
    const validHex = rawHex === '0x' || rawHex === '' ? '0x0' : rawHex;
    const raw = BigInt(validHex).toString();
    const formatted = fromSmallestUnits(Number(raw), token.decimals).toFixed(token.decimals);
    
    return { raw, formatted, symbol: token.symbol };
  } catch (error) {
    console.error('Error fetching EVM balance:', error);
    console.error('Token:', token.symbol, 'Chain:', token.chain);
    return null;
  }
}

/**
 * Fetch balance for any token based on chain type
 */
export async function fetchTokenBalance(
  address: string,
  token: TokenConfig
): Promise<TokenBalance | null> {
  if (token.chain === 'movement') {
    return fetchMovementBalance(address, token);
  } else {
    return fetchEvmBalance(address, token);
  }
}

