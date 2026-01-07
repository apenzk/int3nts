'use client';

import { useState, useMemo, useEffect, useRef } from 'react';
import { useAccount } from 'wagmi';
import { useWallet } from '@aptos-labs/wallet-adapter-react';
import { verifierClient } from '@/lib/verifier';
import type { DraftIntentRequest, DraftIntentSignature } from '@/lib/types';
import { generateIntentId } from '@/lib/types';
import { SUPPORTED_TOKENS, type TokenConfig, toSmallestUnits } from '@/config/tokens';
import { CHAIN_CONFIGS } from '@/config/chains';
import { fetchTokenBalance, type TokenBalance } from '@/lib/balances';
import { Aptos, AptosConfig } from '@aptos-labs/ts-sdk';
import { INTENT_MODULE_ADDRESS, hexToBytes, padEvmAddressToMove } from '@/lib/move-transactions';

type FlowType = 'inflow' | 'outflow';

export function IntentBuilder() {
  const { address: evmAddress } = useAccount();
  const { account: mvmAccount, signAndSubmitTransaction } = useWallet();
  const [directNightlyAddress, setDirectNightlyAddress] = useState<string | null>(null);
  const [offeredBalance, setOfferedBalance] = useState<TokenBalance | null>(null);
  const [desiredBalance, setDesiredBalance] = useState<TokenBalance | null>(null);
  const [loadingOfferedBalance, setLoadingOfferedBalance] = useState(false);
  const [loadingDesiredBalance, setLoadingDesiredBalance] = useState(false);

  // Check for direct Nightly connection from localStorage
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const savedAddress = localStorage.getItem('nightly_connected_address');
      setDirectNightlyAddress(savedAddress);
      
      // Listen for storage changes (in case wallet disconnects in another tab)
      const handleStorageChange = () => {
        const address = localStorage.getItem('nightly_connected_address');
        setDirectNightlyAddress(address);
      };
      window.addEventListener('storage', handleStorageChange);
      return () => window.removeEventListener('storage', handleStorageChange);
    }
  }, []);
  const [flowType, setFlowType] = useState<FlowType>('inflow');
  const [offeredToken, setOfferedToken] = useState<TokenConfig | null>(null);
  const [offeredAmount, setOfferedAmount] = useState('');
  const [desiredToken, setDesiredToken] = useState<TokenConfig | null>(null);
  const [desiredAmount, setDesiredAmount] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [draftId, setDraftId] = useState<string | null>(null);
  const [draftCreatedAt, setDraftCreatedAt] = useState<number | null>(null);
  const [timeRemaining, setTimeRemaining] = useState<number | null>(null);
  const [mounted, setMounted] = useState(false);
  
  // Signature polling state
  const [signature, setSignature] = useState<DraftIntentSignature | null>(null);
  const [pollingSignature, setPollingSignature] = useState(false);
  
  // Transaction submission state
  const [submittingTransaction, setSubmittingTransaction] = useState(false);
  const [transactionHash, setTransactionHash] = useState<string | null>(null);
  
  
  // Store draft data for transaction building
  const [savedDraftData, setSavedDraftData] = useState<{
    intentId: string;
    offeredMetadata: string;
    offeredAmount: string;
    offeredChainId: string;
    desiredMetadata: string;
    desiredAmount: string;
    desiredChainId: string;
    expiryTime: number;
  } | null>(null);

  // Restore draft ID from localStorage after mount (to avoid hydration mismatch)
  useEffect(() => {
    setMounted(true);
    if (typeof window !== 'undefined') {
      const savedDraftId = localStorage.getItem('last_draft_id');
      const savedCreatedAt = localStorage.getItem('last_draft_created_at');
      if (savedDraftId && savedCreatedAt) {
        setDraftId(savedDraftId);
        setDraftCreatedAt(parseInt(savedCreatedAt, 10));
      }
    }
  }, []);

  // Store the fixed expiry time (Unix timestamp in seconds) - never recalculate it
  const [fixedExpiryTime, setFixedExpiryTime] = useState<number | null>(null);

  // Set fixed expiry time when draft is created or restored
  // Always use expiry_time from the verifier (saved in savedDraftData), never recalculate
  useEffect(() => {
    if (savedDraftData?.expiryTime) {
      // Use the expiry_time from the verifier (Unix timestamp in seconds)
      setFixedExpiryTime(savedDraftData.expiryTime);
    } else {
      setFixedExpiryTime(null);
    }
  }, [savedDraftData?.expiryTime]);

  // Update countdown timer - uses fixed expiry time, never recalculates
  useEffect(() => {
    if (!fixedExpiryTime) {
      setTimeRemaining(null);
      return;
    }

    const updateTimer = () => {
      const now = Math.floor(Date.now() / 1000); // Current time in seconds
      const remaining = Math.max(0, fixedExpiryTime - now);
      setTimeRemaining(remaining * 1000); // Convert to milliseconds for display

      if (remaining === 0) {
        // Draft expired
        setDraftId(null);
        setDraftCreatedAt(null);
        setSavedDraftData(null);
        setFixedExpiryTime(null);
        if (typeof window !== 'undefined') {
          localStorage.removeItem('last_draft_id');
          localStorage.removeItem('last_draft_created_at');
        }
      }
    };

    // Update immediately
    updateTimer();

    // Update every second
    const interval = setInterval(updateTimer, 1000);

    return () => clearInterval(interval);
  }, [fixedExpiryTime]); // Only depend on fixedExpiryTime, not draftCreatedAt or savedDraftData

  // Clear draft when manually cleared
  const clearDraft = () => {
    setDraftId(null);
    setDraftCreatedAt(null);
    setSignature(null);
    setSavedDraftData(null);
    setTransactionHash(null);
    setFixedExpiryTime(null);
    if (typeof window !== 'undefined') {
      localStorage.removeItem('last_draft_id');
      localStorage.removeItem('last_draft_created_at');
    }
  };


  // Track if polling is active to prevent multiple polling loops
  const pollingActiveRef = useRef(false);

  // Poll for solver signature when draft exists
  useEffect(() => {
    if (!draftId || pollingActiveRef.current || signature) return; // Don't poll if already polling or have signature

    const pollSignature = async () => {
      pollingActiveRef.current = true;
      setPollingSignature(true);
      const maxAttempts = 60; // 60 attempts * 2 seconds = 120 seconds max (longer than 30s expiry)
      let attempts = 0;

      const poll = async () => {
        try {
          const response = await verifierClient.pollDraftSignature(draftId!);
          console.log('Poll response:', { success: response.success, hasData: !!response.data, error: response.error });
          
          // Check if we got a signature (success: true with data)
          if (response.success && response.data) {
            console.log('Signature received:', response.data);
            setSignature(response.data);
            setPollingSignature(false);
            pollingActiveRef.current = false;
            return;
          }
          
          // If error is "Draft not yet signed", continue polling
          // If error is something else, log it but continue
          if (response.error && !response.error.includes('not yet signed')) {
            console.warn('Polling error:', response.error);
          }
          
          attempts++;
          
          // Continue polling if we haven't exceeded max attempts and draft hasn't expired
          const shouldContinue = attempts < maxAttempts && 
            (fixedExpiryTime === null || Math.floor(Date.now() / 1000) < fixedExpiryTime);
          
          if (shouldContinue) {
            setTimeout(poll, 2000); // Poll every 2 seconds
          } else {
            console.log('Stopping signature polling:', { attempts, maxAttempts, fixedExpiryTime });
            setPollingSignature(false);
            pollingActiveRef.current = false;
          }
        } catch (error) {
          console.error('Error polling signature:', error);
          attempts++;
          if (attempts < maxAttempts) {
            setTimeout(poll, 2000);
          } else {
            setPollingSignature(false);
            pollingActiveRef.current = false;
          }
        }
      };

      poll();
    };

    pollSignature();
    
    // Cleanup: reset polling flag if draftId changes
    return () => {
      pollingActiveRef.current = false;
    };
  }, [draftId]); // Only depend on draftId - don't restart when fixedExpiryTime changes

  // Filter tokens based on flow type
  const offeredTokens = useMemo(() => {
    if (flowType === 'inflow') {
      // Inflow: offered tokens are on connected chain (EVM)
      return SUPPORTED_TOKENS.filter(t => t.chain === 'base-sepolia' || t.chain === 'ethereum-sepolia');
    } else {
      // Outflow: offered tokens are on hub chain (Movement)
      return SUPPORTED_TOKENS.filter(t => t.chain === 'movement');
    }
  }, [flowType]);

  const desiredTokens = useMemo(() => {
    if (flowType === 'inflow') {
      // Inflow: desired tokens are on hub chain (Movement)
      return SUPPORTED_TOKENS.filter(t => t.chain === 'movement');
    } else {
      // Outflow: desired tokens are on connected chain (EVM)
      return SUPPORTED_TOKENS.filter(t => t.chain === 'base-sepolia' || t.chain === 'ethereum-sepolia');
    }
  }, [flowType]);

  // Reset token selections when flow type changes
  const handleFlowTypeChange = (newFlowType: FlowType) => {
    setFlowType(newFlowType);
    setOfferedToken(null);
    setDesiredToken(null);
  };

  // Get requester address based on flow type
  // For inflow: requester is on connected chain (EVM), but we use MVM address for hub
  // For outflow: requester is on hub (MVM)
  // Check both adapter account and direct Nightly connection
  const requesterAddr = directNightlyAddress || mvmAccount?.address || '';
  const mvmAddress = directNightlyAddress || mvmAccount?.address || '';

  // Fetch balance when offered token is selected
  useEffect(() => {
    console.log('Offered balance effect triggered:', { offeredToken: offeredToken?.symbol, mvmAddress, evmAddress });
    if (!offeredToken) {
      console.log('No offered token, skipping balance fetch');
      setOfferedBalance(null);
      return;
    }

    const address = offeredToken.chain === 'movement' ? mvmAddress : evmAddress;
    console.log('Offered token address lookup:', { chain: offeredToken.chain, address });
    if (!address) {
      console.log('No address for offered token chain, skipping balance fetch');
      setOfferedBalance(null);
      return;
    }

    setLoadingOfferedBalance(true);
    console.log('Fetching offered balance:', { address, token: offeredToken.symbol, chain: offeredToken.chain });
    fetchTokenBalance(address, offeredToken)
      .then((balance) => {
        console.log('Offered balance result:', balance);
        setOfferedBalance(balance);
      })
      .catch((error) => {
        console.error('Error fetching offered balance:', error);
        setOfferedBalance(null);
      })
      .finally(() => {
        setLoadingOfferedBalance(false);
      });
  }, [offeredToken, mvmAddress, evmAddress]);

  // Fetch balance when desired token is selected
  useEffect(() => {
    if (!desiredToken) {
      setDesiredBalance(null);
      return;
    }

    const address = desiredToken.chain === 'movement' ? mvmAddress : evmAddress;
    if (!address) {
      setDesiredBalance(null);
      return;
    }

    setLoadingDesiredBalance(true);
    console.log('Fetching desired balance:', { address, token: desiredToken.symbol, chain: desiredToken.chain });
    fetchTokenBalance(address, desiredToken)
      .then((balance) => {
        console.log('Desired balance result:', balance);
        setDesiredBalance(balance);
      })
      .catch((error) => {
        console.error('Error fetching desired balance:', error);
        setDesiredBalance(null);
      })
      .finally(() => {
        setLoadingDesiredBalance(false);
      });
  }, [desiredToken, mvmAddress, evmAddress]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);
    
    // Clear all previous draft state
    setDraftId(null);
    setDraftCreatedAt(null);
    setSavedDraftData(null);
    setSignature(null);
    setPollingSignature(false);
    pollingActiveRef.current = false;
    setTransactionHash(null);

    // Validation
    if (!requesterAddr) {
      setError('Please connect your MVM wallet (Nightly)');
      return;
    }

    if (!offeredToken || !desiredToken) {
      setError('Please select both offered and desired tokens');
      return;
    }

    const offeredAmountNum = parseFloat(offeredAmount);
    const desiredAmountNum = parseFloat(desiredAmount);
    if (isNaN(offeredAmountNum) || offeredAmountNum <= 0) {
      setError('Offered amount must be a positive number');
      return;
    }
    if (isNaN(desiredAmountNum) || desiredAmountNum <= 0) {
      setError('Desired amount must be a positive number');
      return;
    }

    // Convert main values to smallest units using token decimals
    const offeredAmountSmallest = toSmallestUnits(offeredAmountNum, offeredToken.decimals);
    const desiredAmountSmallest = toSmallestUnits(desiredAmountNum, desiredToken.decimals);

    // Expiry is fixed to 60 seconds from now (hardcoded, not user-configurable)
    const expiryTime = Math.floor(Date.now() / 1000) + 60;

    // Get chain IDs from config
    const offeredChainId = CHAIN_CONFIGS[offeredToken.chain].chainId;
    const desiredChainId = CHAIN_CONFIGS[desiredToken.chain].chainId;

    // Generate random intent ID (32-byte hex)
    const intentId = generateIntentId();

    setLoading(true);
    try {
      const request: DraftIntentRequest = {
        requester_addr: requesterAddr,
        draft_data: {
          intent_id: intentId,
          offered_metadata: offeredToken.metadata,
          offered_amount: offeredAmountSmallest.toString(),
          offered_chain_id: offeredChainId.toString(),
          desired_metadata: desiredToken.metadata,
          desired_amount: desiredAmountSmallest.toString(),
          desired_chain_id: desiredChainId.toString(),
          expiry_time: expiryTime,
          issuer: requesterAddr,
          flow_type: flowType,
        },
        expiry_time: expiryTime,
      };

      const response = await verifierClient.createDraftIntent(request);

      if (response.success && response.data) {
        const draftId = response.data.draft_id;
        setDraftId(draftId);
        setError(null);
        
        // Fetch the draft status to get the actual expiry_time from the verifier
        // This ensures we use the server's time, not local time
        try {
          const statusResponse = await verifierClient.getDraftIntentStatus(draftId);
          if (statusResponse.success && statusResponse.data) {
            // Use the expiry_time from the verifier (Unix timestamp in seconds)
            const verifierExpiryTime = statusResponse.data.expiry_time;
            const createdAt = Date.now();
            setDraftCreatedAt(createdAt);
            
            // Save draft data for transaction building - use verifier's expiry_time
            setSavedDraftData({
              intentId,
              offeredMetadata: offeredToken.metadata,
              offeredAmount: offeredAmountSmallest.toString(),
              offeredChainId: offeredChainId.toString(),
              desiredMetadata: desiredToken.metadata,
              desiredAmount: desiredAmountSmallest.toString(),
              desiredChainId: desiredChainId.toString(),
              expiryTime: verifierExpiryTime, // Use verifier's expiry_time, not local calculation
            });
            
            // Save to localStorage
            if (typeof window !== 'undefined') {
              localStorage.setItem('last_draft_id', draftId);
              localStorage.setItem('last_draft_created_at', createdAt.toString());
            }
          } else {
            // Fallback: use local expiry_time if status fetch fails
            const createdAt = Date.now();
            setDraftCreatedAt(createdAt);
            setSavedDraftData({
              intentId,
              offeredMetadata: offeredToken.metadata,
              offeredAmount: offeredAmountSmallest.toString(),
              offeredChainId: offeredChainId.toString(),
              desiredMetadata: desiredToken.metadata,
              desiredAmount: desiredAmountSmallest.toString(),
              desiredChainId: desiredChainId.toString(),
              expiryTime, // Fallback to local calculation
            });
            if (typeof window !== 'undefined') {
              localStorage.setItem('last_draft_id', draftId);
              localStorage.setItem('last_draft_created_at', createdAt.toString());
            }
          }
        } catch (statusError) {
          // Fallback: use local expiry_time if status fetch fails
          console.error('Failed to fetch draft status, using local expiry_time:', statusError);
          const createdAt = Date.now();
          setDraftCreatedAt(createdAt);
          setSavedDraftData({
            intentId,
            offeredMetadata: offeredToken.metadata,
            offeredAmount: offeredAmountSmallest.toString(),
            offeredChainId: offeredChainId.toString(),
            desiredMetadata: desiredToken.metadata,
            desiredAmount: desiredAmountSmallest.toString(),
            desiredChainId: desiredChainId.toString(),
            expiryTime, // Fallback to local calculation
          });
          if (typeof window !== 'undefined') {
            localStorage.setItem('last_draft_id', draftId);
            localStorage.setItem('last_draft_created_at', createdAt.toString());
          }
        }
      } else {
        setError(response.error || 'Failed to create draft intent');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error occurred');
    } finally {
      setLoading(false);
    }
  };

  const handleCreateIntent = async () => {
    if (!savedDraftData || !signature || !requesterAddr) {
      setError('Missing required data to create intent');
      return;
    }

    // Verify we're using the intent ID from the draft
    if (!savedDraftData.intentId) {
      setError('Intent ID not found in saved draft data');
      return;
    }

    console.log('Creating intent on-chain with intent ID from draft:', savedDraftData.intentId);

    setSubmittingTransaction(true);
    setError(null);

    try {
      // Build transaction arguments as plain values
      let functionName: string;
      let functionArguments: any[];

      // Convert signature to array of numbers for vector<u8> serialization
      const signatureBytes = hexToBytes(signature.signature);
      const signatureArray = Array.from(signatureBytes);
      console.log('Signature array length:', signatureArray.length);

      if (flowType === 'inflow') {
        // Inflow: offered on connected chain (EVM), desired on hub (Move)
        const evmAddressForInflow = evmAddress || '0x' + '0'.repeat(40);
        const paddedRequesterAddr = padEvmAddressToMove(evmAddressForInflow);
        // Pad offered metadata (EVM token address) to 32 bytes
        const paddedOfferedMetadata = padEvmAddressToMove(savedDraftData.offeredMetadata);
        
        functionName = `${INTENT_MODULE_ADDRESS}::fa_intent_inflow::create_inflow_intent_entry`;
        functionArguments = [
          paddedOfferedMetadata,
          savedDraftData.offeredAmount,
          savedDraftData.offeredChainId,
          savedDraftData.desiredMetadata, // Move token - already 32 bytes
          savedDraftData.desiredAmount,
          savedDraftData.desiredChainId,
          savedDraftData.expiryTime.toString(),
          savedDraftData.intentId,
          signature.solver_addr,
          signatureArray,
          paddedRequesterAddr,
        ];
      } else {
        // Outflow: offered on hub (Move), desired on connected chain (EVM)
        if (!evmAddress) {
          throw new Error('EVM wallet (MetaMask) must be connected for outflow intents');
        }
        
        const paddedRequesterAddr = padEvmAddressToMove(evmAddress);
        // Pad desired metadata (EVM token address) to 32 bytes
        const paddedDesiredMetadata = padEvmAddressToMove(savedDraftData.desiredMetadata);
        console.log('Padded desired metadata:', paddedDesiredMetadata);
        
        functionName = `${INTENT_MODULE_ADDRESS}::fa_intent_outflow::create_outflow_intent_entry`;
        functionArguments = [
          savedDraftData.offeredMetadata, // Move token - already 32 bytes
          savedDraftData.offeredAmount,
          savedDraftData.offeredChainId,
          paddedDesiredMetadata,
          savedDraftData.desiredAmount,
          savedDraftData.desiredChainId,
          savedDraftData.expiryTime.toString(),
          savedDraftData.intentId,
          paddedRequesterAddr,
          signature.solver_addr,
          signatureArray,
        ];
      }

      // Use build-sign-submit pattern to work around Nightly wallet bug
      const senderAddress = mvmAccount?.address || directNightlyAddress;
      if (!senderAddress) {
        throw new Error('No MVM wallet connected');
      }

      // Configure Aptos client for Movement testnet
      const config = new AptosConfig({
        fullnode: 'https://testnet.movementnetwork.xyz/v1',
      });
      const aptos = new Aptos(config);

      // Build raw transaction using SDK
      console.log('Building transaction with SDK...');
      console.log('Function:', functionName);
      console.log('Arguments:', functionArguments);
      
      const rawTxn = await aptos.transaction.build.simple({
        sender: senderAddress as `0x${string}`,
        data: {
          function: functionName as `${string}::${string}::${string}`,
          functionArguments: functionArguments,
        },
      });
      console.log('Raw transaction built:', rawTxn);

      // Sign with wallet
      let signResponse: any;
      if (mvmAccount?.address) {
        // Connected via wallet adapter
        const nightlyWallet = (window as any).nightly?.aptos;
        if (nightlyWallet) {
          signResponse = await nightlyWallet.signTransaction(rawTxn);
        } else {
          throw new Error('Nightly wallet not available for signing');
        }
      } else if (directNightlyAddress) {
        // Connected directly to Nightly
        const nightlyWallet = (window as any).nightly?.aptos;
        if (!nightlyWallet) {
          throw new Error('Nightly wallet not available');
        }
        signResponse = await nightlyWallet.signTransaction(rawTxn);
      }

      console.log('Sign response:', signResponse);

      if (signResponse?.status === 'Rejected') {
        throw new Error('User rejected transaction');
      }

      // Extract the authenticator and submit
      const senderAuthenticator = signResponse?.args || signResponse;
      console.log('Submitting signed transaction...');
      
      const pendingTxn = await aptos.transaction.submit.simple({
        transaction: rawTxn,
        senderAuthenticator: senderAuthenticator,
      });

      console.log('Transaction submitted:', pendingTxn);
      if (pendingTxn && pendingTxn.hash) {
        setTransactionHash(pendingTxn.hash);
      } else {
        throw new Error('Transaction submitted but no hash returned');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create intent on-chain');
    } finally {
      setSubmittingTransaction(false);
    }
  };

  return (
    <div className="border border-gray-700 rounded p-6">
      <h2 className="text-2xl font-bold mb-6">Create Intent</h2>
      
      {/* Expiry Note */}
      <div className="mb-6 p-3 bg-gray-800/50 border border-gray-700 rounded text-xs text-gray-400">
        <p>⚠️ Intent expires 60 seconds after creation</p>
      </div>

      {/* Flow Type Selector */}
      <div className="mb-6">
        <label className="block text-sm font-medium mb-2">Flow Type</label>
        <div className="flex gap-4">
          <label className="flex items-center">
            <input
              type="radio"
              value="inflow"
              checked={flowType === 'inflow'}
              onChange={(e) => handleFlowTypeChange(e.target.value as FlowType)}
              className="mr-2"
            />
            <span>Inflow</span>
          </label>
          <label className="flex items-center">
            <input
              type="radio"
              value="outflow"
              checked={flowType === 'outflow'}
              onChange={(e) => handleFlowTypeChange(e.target.value as FlowType)}
              className="mr-2"
            />
            <span>Outflow</span>
          </label>
        </div>
      </div>


      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Offered Token */}
        <div>
          <label className="block text-sm font-medium mb-2">
            Offered Token
          </label>
          <select
            value={offeredToken ? `${offeredToken.chain}::${offeredToken.symbol}` : ''}
            onChange={(e) => {
              if (!e.target.value) {
                setOfferedToken(null);
                return;
              }
              const [chain, symbol] = e.target.value.split('::');
              const token = offeredTokens.find(t => t.chain === chain && t.symbol === symbol);
              setOfferedToken(token || null);
            }}
            className="w-full px-4 py-2 bg-gray-900 border border-gray-600 rounded text-sm"
            required
          >
            <option value="">Select token...</option>
            {offeredTokens.map((token) => (
              <option key={`${token.chain}::${token.symbol}`} value={`${token.chain}::${token.symbol}`}>
                {token.name} ({token.symbol})
              </option>
            ))}
          </select>
          {offeredToken && (
            <div className="mt-2 text-xs">
              {loadingOfferedBalance ? (
                <span className="text-gray-500">Loading balance...</span>
              ) : offeredBalance ? (
                <span className="text-gray-400">
                  Balance: {offeredBalance.formatted} {offeredBalance.symbol}
                </span>
              ) : (
                <span className="text-gray-500">Balance unavailable</span>
              )}
            </div>
          )}
        </div>

        <div>
          <label className="block text-sm font-medium mb-2">
            Offered Amount
          </label>
          <div className="flex items-center gap-2">
            <input
              type="number"
              value={offeredAmount}
              onChange={(e) => setOfferedAmount(e.target.value)}
              placeholder="1"
              min="0"
              step="0.000001"
              className="flex-1 px-4 py-2 bg-gray-900 border border-gray-600 rounded text-sm"
              required
            />
            {offeredToken && (
              <span className="text-sm text-gray-400 font-mono">
                {offeredToken.symbol}
              </span>
            )}
          </div>
        </div>

        {/* Desired Token */}
        <div>
          <label className="block text-sm font-medium mb-2">
            Desired Token
          </label>
          <select
            value={desiredToken ? `${desiredToken.chain}::${desiredToken.symbol}` : ''}
            onChange={(e) => {
              if (!e.target.value) {
                setDesiredToken(null);
                return;
              }
              const [chain, symbol] = e.target.value.split('::');
              const token = desiredTokens.find(t => t.chain === chain && t.symbol === symbol);
              setDesiredToken(token || null);
            }}
            className="w-full px-4 py-2 bg-gray-900 border border-gray-600 rounded text-sm"
            required
          >
            <option value="">Select token...</option>
            {desiredTokens.map((token) => (
              <option key={`${token.chain}::${token.symbol}`} value={`${token.chain}::${token.symbol}`}>
                {token.name} ({token.symbol})
              </option>
            ))}
          </select>
          {desiredToken && (
            <div className="mt-2 text-xs">
              {loadingDesiredBalance ? (
                <span className="text-gray-500">Loading balance...</span>
              ) : desiredBalance ? (
                <span className="text-gray-400">
                  Balance: {desiredBalance.formatted} {desiredBalance.symbol}
                </span>
              ) : (
                <span className="text-gray-500">Balance unavailable</span>
              )}
            </div>
          )}
        </div>

        <div>
          <label className="block text-sm font-medium mb-2">
            Desired Amount
          </label>
          <div className="flex items-center gap-2">
            <input
              type="number"
              value={desiredAmount}
              onChange={(e) => setDesiredAmount(e.target.value)}
              placeholder="1"
              min="0"
              step="0.000001"
              className="flex-1 px-4 py-2 bg-gray-900 border border-gray-600 rounded text-sm"
              required
            />
            {desiredToken && (
              <span className="text-sm text-gray-400 font-mono">
                {desiredToken.symbol}
              </span>
            )}
          </div>
        </div>

        {/* Error Display */}
        {error && (
          <div className="p-3 bg-red-900/30 border border-red-700 rounded text-sm text-red-300">
            {error}
          </div>
        )}

        {/* Success Display */}
        {mounted && draftId && (
          <div className="p-3 bg-green-900/30 border border-green-700 rounded text-sm text-green-300">
            <p className="font-bold">Draft Intent Created!</p>
            <p className="mt-1 font-mono text-xs">Draft ID: {draftId}</p>
            {timeRemaining !== null && (
              <p className="mt-2 text-xs">
                Time remaining: {Math.floor(timeRemaining / 1000)}s
                {timeRemaining === 0 && ' (Expired)'}
              </p>
            )}
            
            {/* Signature Status */}
            {pollingSignature && !signature && (
              <p className="mt-2 text-xs text-yellow-300">
                ⏳ Waiting for solver signature...
              </p>
            )}
            
            {signature && (
              <div className="mt-3 p-2 bg-gray-800/50 rounded">
                <p className="text-xs font-bold text-green-400">✅ Solver signature received!</p>
                <p className="mt-1 text-xs font-mono">Solver: {signature.solver_addr.slice(0, 10)}...{signature.solver_addr.slice(-8)}</p>
                
                {!transactionHash && (
                  <button
                    type="button"
                    onClick={handleCreateIntent}
                    disabled={submittingTransaction || !requesterAddr || (flowType === 'outflow' && !evmAddress)}
                    className="mt-2 w-full px-4 py-2 bg-green-600 hover:bg-green-700 rounded text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {submittingTransaction ? 'Creating Intent...' : 'Create Intent on Chain'}
                  </button>
                )}
                
                {transactionHash && (
                  <div className="mt-2">
                    <p className="text-xs font-bold text-green-400">✅ Intent created on-chain!</p>
                    <p className="mt-1 text-xs font-mono break-all">Tx: {transactionHash}</p>
                  </div>
                )}
              </div>
            )}
            
            <button
              type="button"
              onClick={clearDraft}
              className="mt-2 text-xs underline hover:no-underline"
            >
              Clear
            </button>
          </div>
        )}

        {/* Submit Button - Only show when no active draft */}
        {!draftId && (
          <>
            <button
              type="submit"
              disabled={loading || !requesterAddr}
              className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? 'Creating Draft Intent...' : 'Create Draft Intent'}
            </button>

            {!requesterAddr && (
              <p className="text-xs text-gray-400 text-center">
                Connect your MVM wallet (Nightly) to create an intent
              </p>
            )}
          </>
        )}
      </form>
    </div>
  );
}

