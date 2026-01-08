'use client';

import { useState, useMemo, useEffect, useRef } from 'react';
import { useAccount, useWriteContract, useWaitForTransactionReceipt, useChainId, useSwitchChain } from 'wagmi';
import { useWallet } from '@aptos-labs/wallet-adapter-react';
import { verifierClient } from '@/lib/verifier';
import type { DraftIntentRequest, DraftIntentSignature } from '@/lib/types';
import { generateIntentId } from '@/lib/types';
import { SUPPORTED_TOKENS, type TokenConfig, toSmallestUnits } from '@/config/tokens';
import { CHAIN_CONFIGS } from '@/config/chains';
import { fetchTokenBalance, type TokenBalance } from '@/lib/balances';
import { Aptos, AptosConfig } from '@aptos-labs/ts-sdk';
import { INTENT_MODULE_ADDRESS, hexToBytes, padEvmAddressToMove } from '@/lib/move-transactions';
import { INTENT_ESCROW_ABI, ERC20_ABI, intentIdToEvmFormat, getEscrowContractAddress } from '@/lib/escrow';

type FlowType = 'inflow' | 'outflow';

export function IntentBuilder() {
  const { address: evmAddress } = useAccount();
  const chainId = useChainId();
  const { switchChain } = useSwitchChain();
  const { account: mvmAccount, signAndSubmitTransaction } = useWallet();
  const [directNightlyAddress, setDirectNightlyAddress] = useState<string | null>(null);
  const [offeredBalance, setOfferedBalance] = useState<TokenBalance | null>(null);
  const [desiredBalance, setDesiredBalance] = useState<TokenBalance | null>(null);
  const [loadingOfferedBalance, setLoadingOfferedBalance] = useState(false);
  const [loadingDesiredBalance, setLoadingDesiredBalance] = useState(false);

  // Check for direct Nightly connection from localStorage
  // Trust MvmWalletConnector to handle connection - just read the saved address
  useEffect(() => {
    if (typeof window !== 'undefined') {
      const savedAddress = localStorage.getItem('nightly_connected_address');
      setDirectNightlyAddress(savedAddress);
      
      // Listen for storage changes (from other tabs)
      const handleStorageChange = () => {
        const address = localStorage.getItem('nightly_connected_address');
        setDirectNightlyAddress(address);
      };
      
      // Listen for custom event (from same tab when MvmWalletConnector changes connection)
      const handleNightlyChange = (e: Event) => {
        const customEvent = e as CustomEvent<{ address: string | null }>;
        setDirectNightlyAddress(customEvent.detail.address);
      };
      
      window.addEventListener('storage', handleStorageChange);
      window.addEventListener('nightly_wallet_changed', handleNightlyChange);
      return () => {
        window.removeEventListener('storage', handleStorageChange);
        window.removeEventListener('nightly_wallet_changed', handleNightlyChange);
      };
    }
  }, []);
  const [flowType, setFlowType] = useState<FlowType>('inflow');
  const [offeredToken, setOfferedToken] = useState<TokenConfig | null>(null);
  const [offeredAmount, setOfferedAmount] = useState('');
  const [desiredToken, setDesiredToken] = useState<TokenConfig | null>(null);
  // Desired amount is auto-calculated based on solver's exchange rate
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
  
  // Fulfillment tracking state
  const [intentStatus, setIntentStatus] = useState<'pending' | 'created' | 'fulfilled'>('pending');
  const intentStatusRef = useRef<'pending' | 'created' | 'fulfilled'>('pending');
  
  // Keep ref in sync with state
  useEffect(() => {
    intentStatusRef.current = intentStatus;
  }, [intentStatus]);
  const [pollingFulfillment, setPollingFulfillment] = useState(false);
  const pollingFulfillmentRef = useRef(false);
  const currentIntentIdRef = useRef<string | null>(null);
  
  // Escrow creation state (for inflow intents)
  const [escrowHash, setEscrowHash] = useState<string | null>(null);
  const [approvingToken, setApprovingToken] = useState(false);
  const [creatingEscrow, setCreatingEscrow] = useState(false);
  
  // Wagmi hooks for escrow creation
  const { writeContract: writeApprove, data: approveHash, error: approveError, isPending: isApprovePending, reset: resetApprove } = useWriteContract();
  const { writeContract: writeCreateEscrow, data: createEscrowHash, error: escrowError, isPending: isEscrowPending, reset: resetEscrow } = useWriteContract();
  
  // Wait for approve transaction
  const { data: approveReceipt, isLoading: isApproving, error: approveReceiptError } = useWaitForTransactionReceipt({
    hash: approveHash,
  });
  
  // Wait for escrow creation transaction
  const { data: escrowReceipt, isLoading: isCreatingEscrow, error: escrowReceiptError } = useWaitForTransactionReceipt({
    hash: createEscrowHash,
  });
  
  // Handle approve errors (user rejected or tx failed)
  useEffect(() => {
    if (approveError) {
      console.error('Approval error:', approveError);
      setError(`Approval failed: ${approveError.message}`);
      setApprovingToken(false);
      resetApprove();
    }
  }, [approveError, resetApprove]);
  
  // Handle escrow creation errors
  useEffect(() => {
    if (escrowError) {
      console.error('Escrow creation error:', escrowError);
      setError(`Escrow creation failed: ${escrowError.message}`);
      setCreatingEscrow(false);
      resetEscrow();
    }
  }, [escrowError, resetEscrow]);
  
  // Handle receipt errors
  useEffect(() => {
    if (approveReceiptError) {
      console.error('Approval receipt error:', approveReceiptError);
      setError(`Approval transaction failed: ${approveReceiptError.message}`);
      setApprovingToken(false);
    }
  }, [approveReceiptError]);
  
  useEffect(() => {
    if (escrowReceiptError) {
      console.error('Escrow receipt error:', escrowReceiptError);
      setError(`Escrow transaction failed: ${escrowReceiptError.message}`);
      setCreatingEscrow(false);
    }
  }, [escrowReceiptError]);
  
  // Handle approve completion
  useEffect(() => {
    if (approveReceipt && !isApproving && !creatingEscrow && !escrowHash) {
      console.log('Approval confirmed, creating escrow...', approveReceipt.transactionHash);
      setApprovingToken(false);
      // After approval, create escrow
      handleCreateEscrowAfterApproval();
    }
  }, [approveReceipt, isApproving, creatingEscrow, escrowHash]);
  
  // Handle escrow creation completion
  useEffect(() => {
    if (escrowReceipt && !isCreatingEscrow) {
      console.log('Escrow created:', escrowReceipt.transactionHash);
      setCreatingEscrow(false);
      setEscrowHash(escrowReceipt.transactionHash);
    }
  }, [escrowReceipt, isCreatingEscrow]);
  
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

  // Set fixed expiry time based on when draft was created (60 second timeout)
  useEffect(() => {
    if (draftCreatedAt) {
      setFixedExpiryTime(Math.floor(draftCreatedAt / 1000) + 60);
    } else {
      setFixedExpiryTime(null);
    }
  }, [draftCreatedAt]);

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

      if (remaining === 0 && intentStatusRef.current !== 'fulfilled') {
        // Draft expired (but don't clear if intent was fulfilled)
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
    setIntentStatus('pending');
    setPollingFulfillment(false);
    pollingFulfillmentRef.current = false;
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
          // If error is "Draft not found", clear stale localStorage and stop
          if (response.error) {
            if (response.error.includes('not found')) {
              console.log('Draft not found - clearing stale localStorage');
              localStorage.removeItem('last_draft_id');
              localStorage.removeItem('last_draft_created_at');
              setDraftId(null);
              setDraftCreatedAt(null);
              setSavedDraftData(null);
              setPollingSignature(false);
              pollingActiveRef.current = false;
              return;
            }
            if (!response.error.includes('not yet signed')) {
              console.warn('Polling error:', response.error);
            }
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
    setDesiredAmount('');
  };

  // Auto-calculate desired amount based on solver's exchange rate
  // This runs when offered token/amount or desired token changes
  useEffect(() => {
    if (!offeredToken || !desiredToken || !offeredAmount || parseFloat(offeredAmount) <= 0) {
      // Only reset if not already showing not available yet (which indicates a fetch was attempted)
      if (desiredAmount !== 'not available yet') {
        setDesiredAmount('');
      }
      return;
    }

    const fetchExchangeRate = async () => {
      // Set to "Calculating..." immediately to show loading state
      setDesiredAmount('');
      try {
        const offeredChainId = CHAIN_CONFIGS[offeredToken.chain].chainId;
        const desiredChainId = CHAIN_CONFIGS[desiredToken.chain].chainId;
        
        // Query exchange rate for this specific token pair
        const response = await verifierClient.getExchangeRate(
          offeredChainId,
          offeredToken.metadata,
          desiredChainId,
          desiredToken.metadata
        );

        if (!response.success || !response.data) {
          // Exchange rate not available - show "not available yet" instead of error
          setDesiredAmount('not available yet');
          setError(null);
          return;
        }

        const { exchange_rate } = response.data;

        // Calculate desired amount: desired = offered / exchange_rate
        const offeredAmountNum = parseFloat(offeredAmount);
        const desiredAmountNum = offeredAmountNum / exchange_rate;
        setDesiredAmount(desiredAmountNum.toFixed(6));
        setError(null); // Clear any previous errors
      } catch (err) {
        // Exchange rate not available - show "not available yet" instead of error
        setDesiredAmount('not available yet');
        setError(null);
      }
    };

    fetchExchangeRate();
  }, [offeredToken, desiredToken, offeredAmount]);

  // Get requester address based on flow type
  // For inflow: requester is on connected chain (EVM), but we use MVM address for hub
  // For outflow: requester is on hub (MVM)
  // Check both adapter account and direct Nightly connection
  const requesterAddr = directNightlyAddress || mvmAccount?.address || '';
  const mvmAddress = directNightlyAddress || mvmAccount?.address || '';

  // Keep intent ID ref in sync for use in polling closure
  useEffect(() => {
    currentIntentIdRef.current = savedDraftData?.intentId || null;
  }, [savedDraftData?.intentId]);

  // Poll for fulfillment
  // - Outflow: Check verifier approval (verifier confirms funds received on connected chain)
  // - Inflow: Check hub chain fulfillment events (solver fulfilled intent on hub chain)
  useEffect(() => {
    if (!transactionHash || !savedDraftData || pollingFulfillmentRef.current) return;
    
    const pollFulfillment = async () => {
      pollingFulfillmentRef.current = true;
      setPollingFulfillment(true);
      setIntentStatus('created');
      
      const maxAttempts = 120; // 120 attempts * 5 seconds = 10 minutes max
      let attempts = 0;
      
      // Store initial desired balance for inflow comparison
      let initialDesiredBalance: number | null = null;
      if (flowType === 'inflow' && desiredBalance) {
        initialDesiredBalance = parseFloat(desiredBalance.formatted);
      }
      
      const poll = async () => {
        try {
          // Use ref to get latest intentId (may have been updated with on-chain ID)
          const currentIntentId = currentIntentIdRef.current;
          if (!currentIntentId) {
            console.log('No intent ID yet, waiting...');
            attempts++;
            if (attempts < maxAttempts) {
              setTimeout(poll, 5000);
            } else {
              setPollingFulfillment(false);
              pollingFulfillmentRef.current = false;
            }
            return;
          }
          
          if (flowType === 'outflow') {
            // Outflow: Check verifier approval (verifier confirms funds received on connected chain)
            console.log('Checking approval status for outflow intent:', currentIntentId);
            
            const verifierUrl = process.env.NEXT_PUBLIC_VERIFIER_URL || 'http://localhost:3333';
            const response = await fetch(`${verifierUrl}/approved/${currentIntentId}`);
            const data = await response.json();
            
            console.log('Approval check response:', data);
            
            if (data.success && data.data?.approved) {
              console.log('Intent approved!');
              setIntentStatus('fulfilled');
              setPollingFulfillment(false);
              pollingFulfillmentRef.current = false;
              return;
            }
          } else {
            // Inflow: Check hub chain for fulfillment events
            // Query requester's transactions on hub chain to find fulfillment events
            console.log('Checking hub chain fulfillment for inflow intent:', currentIntentId);
            
            if (!mvmAddress) {
              console.log('No MVM address, waiting...');
              attempts++;
              if (attempts < maxAttempts) {
                setTimeout(poll, 5000);
              } else {
                setPollingFulfillment(false);
                pollingFulfillmentRef.current = false;
              }
              return;
            }
            
            // Query hub chain for fulfillment events
            const hubRpcUrl = 'https://testnet.movementnetwork.xyz/v1';
            const accountAddress = mvmAddress.startsWith('0x') ? mvmAddress.slice(2) : mvmAddress;
            const transactionsUrl = `${hubRpcUrl}/accounts/${accountAddress}/transactions?limit=10`;
            
            try {
              const txResponse = await fetch(transactionsUrl);
              const transactions = await txResponse.json();
              
              // Look for fulfillment events in recent transactions
              let foundFulfillment = false;
              for (const tx of transactions) {
                if (tx.events && Array.isArray(tx.events)) {
                  for (const event of tx.events) {
                    // Check for LimitOrderFulfillmentEvent
                    if (event.type?.includes('LimitOrderFulfillmentEvent')) {
                      const eventIntentId = event.data?.intent_id || event.data?.intent_addr;
                      // Normalize intent IDs for comparison (remove 0x prefix, lowercase)
                      const normalizeId = (id: string) => {
                        const stripped = id?.replace(/^0x/i, '').toLowerCase() || '';
                        return stripped.replace(/^0+/, '') || '0';
                      };
                      
                      if (eventIntentId && normalizeId(eventIntentId) === normalizeId(currentIntentId)) {
                        console.log('Found fulfillment event on hub chain!');
                        foundFulfillment = true;
                        break;
                      }
                    }
                  }
                }
                if (foundFulfillment) break;
              }
              
              // Refresh desired balance to check for increase (backup method)
              if (!foundFulfillment && desiredToken && mvmAddress) {
                try {
                  const refreshedBalance = await fetchTokenBalance(mvmAddress, desiredToken);
                  if (refreshedBalance && initialDesiredBalance !== null) {
                    const currentBalance = parseFloat(refreshedBalance.formatted);
                    if (currentBalance > initialDesiredBalance) {
                      console.log('Desired balance increased - intent fulfilled!');
                      setDesiredBalance(refreshedBalance);
                      foundFulfillment = true;
                    }
                  }
                } catch (balanceError) {
                  console.error('Error refreshing balance:', balanceError);
                }
              }
              
              if (foundFulfillment) {
                console.log('Hub intent fulfilled!');
                setIntentStatus('fulfilled');
                setPollingFulfillment(false);
                pollingFulfillmentRef.current = false;
                return;
              }
            } catch (hubError) {
              console.error('Error querying hub chain:', hubError);
            }
          }
          
          attempts++;
          if (attempts < maxAttempts) {
            setTimeout(poll, 5000); // Poll every 5 seconds
          } else {
            setPollingFulfillment(false);
            pollingFulfillmentRef.current = false;
          }
        } catch (error) {
          console.error('Error polling fulfillment:', error);
          attempts++;
          if (attempts < maxAttempts) {
            setTimeout(poll, 5000);
          } else {
            setPollingFulfillment(false);
            pollingFulfillmentRef.current = false;
          }
        }
      };
      
      // Start polling after a short delay to let the solver pick up the intent
      setTimeout(poll, 3000);
    };
    
    pollFulfillment();
    
    // Don't reset pollingFulfillmentRef in cleanup - it causes re-runs when dependencies change
    // The ref is only reset explicitly in clearDraft or when polling completes
  }, [transactionHash, savedDraftData, flowType, mvmAddress]); // Removed desiredBalance - it's captured at start

  // Fetch balance when offered token is selected
  useEffect(() => {
    if (!offeredToken) {
      setOfferedBalance(null);
      return;
    }

    const address = offeredToken.chain === 'movement' ? mvmAddress : evmAddress;
    if (!address) {
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

  // Refresh balances when intent is fulfilled (funds received)
  useEffect(() => {
    if (intentStatus !== 'fulfilled') return;
    
    // Refresh offered balance
    if (offeredToken) {
      const offeredAddress = offeredToken.chain === 'movement' ? mvmAddress : evmAddress;
      if (offeredAddress) {
        setLoadingOfferedBalance(true);
        fetchTokenBalance(offeredAddress, offeredToken)
          .then(setOfferedBalance)
          .catch(() => setOfferedBalance(null))
          .finally(() => setLoadingOfferedBalance(false));
      }
    }
    
    // Refresh desired balance
    if (desiredToken) {
      const desiredAddress = desiredToken.chain === 'movement' ? mvmAddress : evmAddress;
      if (desiredAddress) {
        setLoadingDesiredBalance(true);
        fetchTokenBalance(desiredAddress, desiredToken)
          .then(setDesiredBalance)
          .catch(() => setDesiredBalance(null))
          .finally(() => setLoadingDesiredBalance(false));
      }
    }
  }, [intentStatus, offeredToken, desiredToken, mvmAddress, evmAddress]);

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
    if (!desiredAmount || desiredAmount === 'not available yet' || parseFloat(desiredAmount) <= 0) {
      setError('Exchange rate not available. Cannot create draft intent.');
      return;
    }

    const offeredAmountNum = parseFloat(offeredAmount);
    // Skip parsing if not available yet (already checked above)
    if (desiredAmount === 'not available yet') {
      setError('Exchange rate not available. Cannot create draft intent.');
      return;
    }
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
        
        const createdAt = Date.now();
        setDraftCreatedAt(createdAt);
        
        // Save draft data for transaction building
        setSavedDraftData({
          intentId,
          offeredMetadata: offeredToken.metadata,
          offeredAmount: offeredAmountSmallest.toString(),
          offeredChainId: offeredChainId.toString(),
          desiredMetadata: desiredToken.metadata,
          desiredAmount: desiredAmountSmallest.toString(),
          desiredChainId: desiredChainId.toString(),
          expiryTime,
        });
        
        // Save to localStorage
        if (typeof window !== 'undefined') {
          localStorage.setItem('last_draft_id', draftId);
          localStorage.setItem('last_draft_created_at', createdAt.toString());
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
        
        // Wait for transaction and extract on-chain intent ID from events
        try {
          const txnResult = await aptos.waitForTransaction({ transactionHash: pendingTxn.hash });
          if ('events' in txnResult && Array.isArray(txnResult.events)) {
            for (const event of txnResult.events) {
              // Look for OracleLimitOrderEvent which contains the intent IDs
              if (event.type?.includes('OracleLimitOrderEvent') || event.type?.includes('LimitOrderEvent')) {
                // Use intent_id (the original ID from draft) - this is what the solver uses for validation
                // intent_addr is the object address created on-chain (different)
                const onChainIntentId = event.data?.intent_id || event.data?.intent_addr || event.data?.id;
                if (onChainIntentId) {
                  console.log('On-chain intent_id for approval tracking:', onChainIntentId);
                  // Update savedDraftData with intent_id for approval tracking
                  setSavedDraftData(prev => prev ? { ...prev, intentId: onChainIntentId } : null);
                }
                break;
              }
            }
          }
        } catch (waitErr) {
          console.warn('Could not wait for transaction:', waitErr);
        }
      } else {
        throw new Error('Transaction submitted but no hash returned');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create intent on-chain');
    } finally {
      setSubmittingTransaction(false);
    }
  };

  // Handle escrow creation for inflow intents
  const handleCreateEscrow = async () => {
    console.log('handleCreateEscrow called', { savedDraftData, offeredToken, flowType, evmAddress, signature: !!signature, chainId });
    
    if (!savedDraftData || !offeredToken || flowType !== 'inflow' || !evmAddress || !signature) {
      const missing = [];
      if (!savedDraftData) missing.push('savedDraftData');
      if (!offeredToken) missing.push('offeredToken');
      if (flowType !== 'inflow') missing.push(`flowType=${flowType}`);
      if (!evmAddress) missing.push('evmAddress');
      if (!signature) missing.push('signature');
      setError(`Missing required data for escrow creation: ${missing.join(', ')}`);
      return;
    }

    try {
      setError(null);
      
      // Check if we're on the right chain (Base Sepolia = 84532)
      const requiredChainId = offeredToken.chain === 'base-sepolia' ? 84532 : 11155111; // Base Sepolia or Ethereum Sepolia
      if (chainId !== requiredChainId) {
        console.log(`Switching chain from ${chainId} to ${requiredChainId}`);
        try {
          await switchChain({ chainId: requiredChainId });
          console.log('Chain switched successfully');
        } catch (switchError) {
          console.error('Failed to switch chain:', switchError);
          setError(`Please switch to ${offeredToken.chain === 'base-sepolia' ? 'Base Sepolia' : 'Ethereum Sepolia'} in your wallet`);
          return;
        }
      }
      
      setApprovingToken(true);

      const escrowAddress = getEscrowContractAddress(offeredToken.chain);
      console.log('Creating escrow with:', { escrowAddress, tokenAddress: offeredToken.metadata, intentId: savedDraftData.intentId, chainId });
      const tokenAddress = offeredToken.metadata as `0x${string}`;
      const amount = BigInt(toSmallestUnits(parseFloat(offeredAmount), offeredToken.decimals));
      const intentIdEvm = intentIdToEvmFormat(savedDraftData.intentId);
      
      // Get solver's EVM address from signature response (required for inflow escrows)
      if (!signature.solver_evm_addr) {
        throw new Error('Solver has no EVM address registered. The solver must register with an EVM address to fulfill inflow intents.');
      }
      const solverAddress = signature.solver_evm_addr;
      console.log('Solver EVM address:', solverAddress);

      // First approve token (approve a large amount to avoid repeated approvals)
      const approveAmount = BigInt('0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff'); // max uint256
      
      console.log('Calling writeApprove with:', {
        address: tokenAddress,
        functionName: 'approve',
        args: [escrowAddress, approveAmount.toString()],
      });
      
      writeApprove({
        address: tokenAddress,
        abi: ERC20_ABI,
        functionName: 'approve',
        args: [escrowAddress, approveAmount],
      });
      
      console.log('writeApprove called - waiting for wallet response');
    } catch (err) {
      console.error('handleCreateEscrow error:', err);
      setError(err instanceof Error ? err.message : 'Failed to start escrow creation');
      setApprovingToken(false);
    }
  };

  // Create escrow after token is approved
  const handleCreateEscrowAfterApproval = () => {
    console.log('handleCreateEscrowAfterApproval called');
    
    if (!savedDraftData || !offeredToken || flowType !== 'inflow' || !evmAddress || !signature) {
      console.error('handleCreateEscrowAfterApproval: missing data');
      return;
    }

    try {
      setCreatingEscrow(true);

      const escrowAddress = getEscrowContractAddress(offeredToken.chain);
      const tokenAddress = offeredToken.metadata as `0x${string}`;
      const amount = BigInt(toSmallestUnits(parseFloat(offeredAmount), offeredToken.decimals));
      const intentIdEvm = intentIdToEvmFormat(savedDraftData.intentId);
      
      // Get solver's EVM address from signature response (required for inflow escrows)
      if (!signature.solver_evm_addr) {
        throw new Error('Solver has no EVM address registered. The solver must register with an EVM address to fulfill inflow intents.');
      }
      const solverAddress = signature.solver_evm_addr;
      
      console.log('Creating escrow:', { escrowAddress, tokenAddress, amount: amount.toString(), intentIdEvm: intentIdEvm.toString(), solverAddress });

      writeCreateEscrow({
        address: escrowAddress,
        abi: INTENT_ESCROW_ABI,
        functionName: 'createEscrow',
        args: [intentIdEvm, tokenAddress, amount, solverAddress as `0x${string}`],
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create escrow');
      setCreatingEscrow(false);
    }
  };

  return (
    <div className="border border-gray-700 rounded p-6">
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
        
        {/* Quick fill buttons for testing */}
        <div className="flex gap-2 mt-3">
          <button
            type="button"
            onClick={() => {
              handleFlowTypeChange('inflow');
              const usdcBase = SUPPORTED_TOKENS.find(t => t.symbol === 'USDC' && t.chain === 'base-sepolia');
              const usdcMovement = SUPPORTED_TOKENS.find(t => t.symbol === 'USDC.e' && t.chain === 'movement');
              if (usdcBase) setOfferedToken(usdcBase);
              if (usdcMovement) setDesiredToken(usdcMovement);
              setOfferedAmount('0.001');
              setDesiredAmount('0.001');
            }}
            className="px-3 py-1 text-xs bg-gray-700 hover:bg-gray-600 rounded"
          >
            Default Inflow
          </button>
          <button
            type="button"
            onClick={() => {
              handleFlowTypeChange('outflow');
              const usdcMovement = SUPPORTED_TOKENS.find(t => t.symbol === 'USDC.e' && t.chain === 'movement');
              const usdcBase = SUPPORTED_TOKENS.find(t => t.symbol === 'USDC' && t.chain === 'base-sepolia');
              if (usdcMovement) setOfferedToken(usdcMovement);
              if (usdcBase) setDesiredToken(usdcBase);
              setOfferedAmount('0.001');
              setDesiredAmount('0.001');
            }}
            className="px-3 py-1 text-xs bg-gray-700 hover:bg-gray-600 rounded"
          >
            Default Outflow
          </button>
        </div>
      </div>


      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Offered Token */}
        <div>
          <label className="block text-sm font-medium mb-2">
            Send
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
        </div>

        <div>
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

        {/* Desired Token */}
        <div>
          <label className="block text-sm font-medium mb-2">
            Receive
          </label>
          <select
            value={desiredToken ? `${desiredToken.chain}::${desiredToken.symbol}` : ''}
            onChange={(e) => {
              if (!e.target.value) {
                setDesiredToken(null);
                setDesiredAmount('');
                return;
              }
              const [chain, symbol] = e.target.value.split('::');
              const token = desiredTokens.find(t => t.chain === chain && t.symbol === symbol);
              setDesiredToken(token || null);
              setDesiredAmount(''); // Reset amount when token changes
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
        </div>

        {/* Desired Amount (auto-calculated from solver's exchange rate) */}
        {desiredToken && (
          <div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={desiredAmount}
                readOnly
                placeholder={
                  desiredAmount && desiredAmount !== 'not available yet'
                    ? '' 
                    : offeredToken && offeredAmount 
                      ? "Calculating..." 
                      : "Enter offered amount first"
                }
                className="flex-1 px-4 py-2 bg-gray-800 border border-gray-700 rounded text-sm text-gray-300 cursor-not-allowed"
              />
              <span className="text-sm text-gray-400 font-mono">
                {desiredToken.symbol}
              </span>
            </div>
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
        )}

        {/* Error Display */}
        {error && (
          <div className="p-3 bg-red-900/30 border border-red-700 rounded text-sm text-red-300">
            {error}
          </div>
        )}

        {/* Action Buttons */}
        <div className="space-y-3">
          <button
            type="submit"
            disabled={loading || !requesterAddr || !!draftId || desiredAmount === 'not available yet'}
            className={`w-full px-4 py-2 rounded text-sm font-medium transition-colors ${
              draftId 
                ? 'bg-gray-600 text-gray-400 cursor-not-allowed' 
                : desiredAmount === 'not available yet'
                  ? 'bg-gray-600 text-gray-400 cursor-not-allowed'
                  : 'bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed'
            }`}
          >
            {loading ? 'Creating Draft Intent...' : draftId ? '✓ Draft Created' : 'Create Draft Intent'}
          </button>

          {/* Step 2: Create Intent Button */}
          <button
            type="button"
            onClick={handleCreateIntent}
            disabled={!signature || submittingTransaction || !requesterAddr || (flowType === 'outflow' && !evmAddress) || !!transactionHash}
            className={`w-full px-4 py-2 rounded text-sm font-medium transition-colors ${
              transactionHash
                ? 'bg-gray-600 text-gray-400 cursor-not-allowed'
                : signature 
                  ? 'bg-green-600 hover:bg-green-700' 
                  : 'bg-gray-600 text-gray-400 cursor-not-allowed'
            }`}
          >
            {transactionHash 
              ? '✓ Intent Created' 
              : submittingTransaction 
                ? 'Creating Intent...' 
                : signature 
                  ? 'Create Intent' 
                  : 'Create Intent'}
          </button>
          
          {/* Status note below buttons */}
          {!requesterAddr && (
            <p className="text-xs text-gray-400 text-center">
              Connect your MVM wallet (Nightly) to create an intent
            </p>
          )}
          {requesterAddr && !draftId && (
            <p className="text-xs text-gray-500 text-center">
              Step 1: Create a draft intent for solver approval
            </p>
          )}
          {draftId && !signature && pollingSignature && (
            <p className="text-xs text-yellow-400 text-center">
              ⏳ Waiting for solver signature...
            </p>
          )}
          {signature && !transactionHash && (
            <p className="text-xs text-green-400 text-center">
              ✅ Solver approved! Click "Create Intent" to submit on-chain
            </p>
          )}
        </div>

        {/* Status Display */}
        {mounted && draftId && (
          <div className="p-3 bg-gray-800/50 border border-gray-700 rounded text-sm text-gray-300">
            <div className="flex justify-between items-start">
              <div>
                {/* Only show timer if not fulfilled */}
                {intentStatus !== 'fulfilled' && timeRemaining !== null && (
                  <p className="mt-1 text-xs">
                    Time remaining: {Math.floor(timeRemaining / 1000)}s
                    {timeRemaining === 0 && ' (Expired)'}
                  </p>
                )}
              </div>
            </div>
            
            {signature && (
              <div className="mt-2">
                {transactionHash && (
                  <div className="mt-2 space-y-2">
                    <p className="text-xs font-mono break-all text-gray-400">Tx: {transactionHash}</p>
                    {savedDraftData?.intentId && (
                      <p className="text-xs font-mono break-all text-gray-400">Intent ID: {savedDraftData.intentId}</p>
                    )}
                    
                    {/* Escrow creation for inflow intents */}
                    {flowType === 'inflow' && transactionHash && !escrowHash && (
                      <div className="mt-3 p-2 bg-blue-900/30 rounded border border-blue-600/50">
                        <p className="text-xs text-blue-400 mb-2">
                          ⚠️ Create escrow on {offeredToken?.chain === 'base-sepolia' ? 'Base Sepolia' : 'EVM chain'} to complete intent
                        </p>
                        <button
                          type="button"
                          onClick={handleCreateEscrow}
                          disabled={approvingToken || creatingEscrow || isApproving || isCreatingEscrow || isApprovePending || isEscrowPending || !evmAddress || !!escrowHash}
                          className="w-full px-3 py-1.5 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed rounded text-xs font-medium"
                        >
                          {isApprovePending
                            ? 'Confirm in wallet...'
                            : approvingToken || isApproving
                            ? 'Approving token...'
                            : isEscrowPending
                            ? 'Confirm escrow in wallet...'
                            : creatingEscrow || isCreatingEscrow
                            ? 'Creating escrow...'
                            : escrowHash
                            ? '✓ Escrow Created'
                            : 'Create Escrow'}
                        </button>
                        {escrowHash && (
                          <p className="mt-1 text-xs font-mono break-all text-gray-400">Escrow Tx: {escrowHash}</p>
                        )}
                      </div>
                    )}
                    
                    {intentStatus === 'created' && pollingFulfillment && (
                      <div className="p-2 bg-yellow-900/30 rounded border border-yellow-600/50">
                        <p className="text-xs text-yellow-400">
                          ⏳ Waiting for funds to arrive...
                        </p>
                      </div>
                    )}
                    
                    {intentStatus === 'fulfilled' && (
                      <div className="p-2 bg-green-900/30 rounded border border-green-600/50">
                        <p className="text-xs font-bold text-green-400">🎉 Funds received!</p>
                        <p className="mt-1 text-xs text-gray-400">Verified by trusted verifier</p>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
            
            {/* Clear button at bottom when fulfilled */}
            {intentStatus === 'fulfilled' && (
              <button
                type="button"
                onClick={clearDraft}
                className="mt-3 w-full px-4 py-2 bg-yellow-600 hover:bg-yellow-500 text-black font-medium rounded text-sm"
              >
                Clear & Create New Intent
              </button>
            )}
          </div>
        )}
      </form>
    </div>
  );
}

