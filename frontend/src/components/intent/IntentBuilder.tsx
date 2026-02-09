'use client';

import { useState, useMemo, useEffect, useRef, useCallback } from 'react';
import { useAccount, useWriteContract, useWaitForTransactionReceipt, useChainId, useSwitchChain } from 'wagmi';
import { useWallet as useMvmWallet } from '@aptos-labs/wallet-adapter-react';
import { useWallet as useSvmWallet } from '@solana/wallet-adapter-react';
import { coordinatorClient } from '@/lib/coordinator';
import type { DraftIntentRequest, DraftIntentSignature } from '@/lib/types';
import { generateIntentId } from '@/lib/types';
import { SUPPORTED_TOKENS, type TokenConfig, toSmallestUnits } from '@/config/tokens';
import { CHAIN_CONFIGS, getChainType, getHubChainConfig, isHubChain } from '@/config/chains';
import { fetchTokenBalance, type TokenBalance } from '@/lib/balances';
import { Aptos, AptosConfig } from '@aptos-labs/ts-sdk';
import { PublicKey } from '@solana/web3.js';
import { INTENT_MODULE_ADDR, hexToBytes, padEvmAddressToMove } from '@/lib/move-transactions';
import { INTENT_ESCROW_ABI, ERC20_ABI, intentIdToEvmFormat, getEscrowContractAddress } from '@/lib/escrow';
import {
  buildCreateEscrowInstruction,
  getSvmTokenAccount,
  svmHexToPubkey,
  svmPubkeyToHex,
} from '@/lib/svm-escrow';
import { fetchSolverSvmAddress, getSvmConnection, sendSvmTransaction } from '@/lib/svm-transactions';

// ============================================================================
// Types
// ============================================================================

type FlowType = 'inflow' | 'outflow';

// ============================================================================
// Hooks
// ============================================================================

/**
 * Track Nightly wallet connection from local storage and custom events.
 */
function useNightlyAddress(): string | null {
  const [directNightlyAddress, setDirectNightlyAddress] = useState<string | null>(null);

  useEffect(() => {
    if (typeof window !== 'undefined') {
      const savedAddress = localStorage.getItem('nightly_connected_address');
      setDirectNightlyAddress(savedAddress);

      const handleStorageChange = () => {
        const address = localStorage.getItem('nightly_connected_address');
        setDirectNightlyAddress(address);
      };

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

  return directNightlyAddress;
}

/**
 * Fetch balances for offered/desired tokens with refresh on fulfillment.
 */
function useTokenBalances(params: {
  offeredToken: TokenConfig | null;
  desiredToken: TokenConfig | null;
  resolveAddress: (chain: TokenConfig['chain']) => string;
  intentStatus: 'pending' | 'created' | 'fulfilled';
}) {
  const { offeredToken, desiredToken, resolveAddress, intentStatus } = params;
  const [offeredBalance, setOfferedBalance] = useState<TokenBalance | null>(null);
  const [desiredBalance, setDesiredBalance] = useState<TokenBalance | null>(null);
  const [loadingOfferedBalance, setLoadingOfferedBalance] = useState(false);
  const [loadingDesiredBalance, setLoadingDesiredBalance] = useState(false);

  useEffect(() => {
    if (!offeredToken) {
      setOfferedBalance(null);
      return;
    }
    const address = resolveAddress(offeredToken.chain);
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
      .catch(() => setOfferedBalance(null))
      .finally(() => setLoadingOfferedBalance(false));
  }, [offeredToken, resolveAddress]);

  useEffect(() => {
    if (!desiredToken) {
      setDesiredBalance(null);
      return;
    }
    const address = resolveAddress(desiredToken.chain);
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
      .catch(() => setDesiredBalance(null))
      .finally(() => setLoadingDesiredBalance(false));
  }, [desiredToken, resolveAddress]);

  useEffect(() => {
    if (intentStatus !== 'fulfilled') {
      return;
    }
    if (offeredToken) {
      const offeredAddress = resolveAddress(offeredToken.chain);
      if (offeredAddress) {
        setLoadingOfferedBalance(true);
        fetchTokenBalance(offeredAddress, offeredToken)
          .then(setOfferedBalance)
          .catch(() => setOfferedBalance(null))
          .finally(() => setLoadingOfferedBalance(false));
      }
    }
    if (desiredToken) {
      const desiredAddress = resolveAddress(desiredToken.chain);
      if (desiredAddress) {
        setLoadingDesiredBalance(true);
        fetchTokenBalance(desiredAddress, desiredToken)
          .then(setDesiredBalance)
          .catch(() => setDesiredBalance(null))
          .finally(() => setLoadingDesiredBalance(false));
      }
    }
  }, [intentStatus, offeredToken, desiredToken, resolveAddress]);

  return {
    offeredBalance,
    desiredBalance,
    loadingOfferedBalance,
    loadingDesiredBalance,
  };
}

// ============================================================================
// Intent Builder Component
// ============================================================================

/**
 * Intent creation flow across hub and connected chains.
 */
export function IntentBuilder() {
  const { address: evmAddress } = useAccount();
  const chainId = useChainId();
  const { switchChain } = useSwitchChain();
  const { account: mvmAccount } = useMvmWallet();
  const svmWallet = useSvmWallet();
  const svmPublicKey = svmWallet.publicKey;
  const svmAddress = svmPublicKey?.toBase58() || '';
  const directNightlyAddress = useNightlyAddress();
  const requesterAddr = directNightlyAddress || mvmAccount?.address || '';
  const mvmAddress = directNightlyAddress || mvmAccount?.address || '';
  const [offeredToken, setOfferedToken] = useState<TokenConfig | null>(null);
  const [offeredAmount, setOfferedAmount] = useState('');
  const [desiredToken, setDesiredToken] = useState<TokenConfig | null>(null);
  
  // Compute flowType dynamically based on selected tokens
  // If offered token is on Movement (hub), it's outflow; otherwise it's inflow
  const flowType: FlowType | null = useMemo(() => {
    if (!offeredToken) return null;
    return isHubChain(offeredToken.chain) ? 'outflow' : 'inflow';
  }, [offeredToken]);
  const hubChainId = getHubChainConfig().chainId;
  const hubChainIdString = hubChainId.toString();

  const isHubChainId = (chainIdValue?: string | null) => chainIdValue === hubChainIdString;
  const isSvmChain = (chain: TokenConfig['chain']) => getChainType(chain) === 'svm';
  const isEvmChain = (chain: TokenConfig['chain']) => getChainType(chain) === 'evm';

  const getConnectedChain = (offered: TokenConfig, desired: TokenConfig) =>
    isHubChain(offered.chain) ? desired.chain : offered.chain;

  const getAddressForChain = useCallback((chain: TokenConfig['chain']) => {
    if (isHubChain(chain)) {
      return mvmAddress;
    }
    if (getChainType(chain) === 'svm') {
      return svmAddress;
    }
    return evmAddress || '';
  }, [mvmAddress, svmAddress, evmAddress]);

  const getChainKeyFromId = (chainIdValue: string): TokenConfig['chain'] | null => {
    const entry = Object.entries(CHAIN_CONFIGS).find(
      ([, config]) => String(config.chainId) === chainIdValue
    );
    return entry ? (entry[0] as TokenConfig['chain']) : null;
  };
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

  // Helper to get chain name from chain ID
  const getChainNameFromId = (chainId: string): string => {
    const chainEntry = Object.entries(CHAIN_CONFIGS).find(([, config]) => String(config.chainId) === chainId);
    return chainEntry ? chainEntry[1].name : `Chain ${chainId}`;
  };

  // Helper to find token from savedDraftData (for escrow creation when offeredToken is null)
  const getOfferedTokenFromDraft = (): TokenConfig | null => {
    if (!savedDraftData) return null;
    // Find token by matching chain ID and metadata
    return SUPPORTED_TOKENS.find(t => {
      const chainConfig = CHAIN_CONFIGS[t.chain];
      if (!chainConfig || String(chainConfig.chainId) !== savedDraftData.offeredChainId) {
        return false;
      }
      if (getChainType(t.chain) === 'svm') {
        // Deviation from EVM flow: SVM mint addresses are base58 strings,
        // so we compare directly instead of hex-padding.
        return t.metadata.toLowerCase() === savedDraftData.offeredMetadata.toLowerCase();
      }
      return t.metadata
        .toLowerCase()
        .includes(savedDraftData.offeredMetadata.replace(/^0x0*/, '').toLowerCase());
    }) || null;
  };

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

  const connectedChainKey =
    offeredToken && desiredToken ? getConnectedChain(offeredToken, desiredToken) : null;
  const requiresEvmWallet = connectedChainKey ? isEvmChain(connectedChainKey) : false;
  const requiresSvmWallet = connectedChainKey ? getChainType(connectedChainKey) === 'svm' : false;
  const connectedWalletReady =
    (!requiresEvmWallet || !!evmAddress) && (!requiresSvmWallet || !!svmAddress);

  const escrowChainKey = savedDraftData ? getChainKeyFromId(savedDraftData.offeredChainId) : null;
  const escrowRequiresSvm = escrowChainKey ? getChainType(escrowChainKey) === 'svm' : false;
  const escrowWalletReady = escrowRequiresSvm ? !!svmAddress : !!evmAddress;
  const {
    offeredBalance,
    desiredBalance,
    loadingOfferedBalance,
    loadingDesiredBalance,
  } = useTokenBalances({
    offeredToken,
    desiredToken,
    resolveAddress: getAddressForChain,
    intentStatus,
  });

  // Restore draft ID from localStorage after mount (to avoid hydration mismatch)
  useEffect(() => {
    setMounted(true);
    if (typeof window !== 'undefined') {
      const savedDraftId = localStorage.getItem('last_draft_id');
      const savedCreatedAt = localStorage.getItem('last_draft_created_at');
      if (savedDraftId && savedCreatedAt) {
        setDraftId(savedDraftId);
        setDraftCreatedAt(parseInt(savedCreatedAt, 10));
      } else {
        // Clear any stale state if draft data is missing from localStorage
        // This happens on page refresh when localStorage was cleared or expired
        setDraftId(null);
        setDraftCreatedAt(null);
        setSignature(null);
        setSavedDraftData(null);
        setTransactionHash(null);
        setError(null); // Clear any stale errors
      }
    }
  }, []);

  // Clear entire draft if savedDraftData is missing (stale state after page refresh)
  // savedDraftData is not persisted, so if we have draftId but no savedDraftData, we can't use the draft
  useEffect(() => {
    if (draftId && !savedDraftData && mounted) {
      console.log('Clearing stale draft - savedDraftData missing after page refresh');
      // Clear everything - we can't use this draft without savedDraftData
      setDraftId(null);
      setDraftCreatedAt(null);
      setSignature(null);
      setTransactionHash(null);
      setError(null); // Clear any stale errors
      if (typeof window !== 'undefined') {
        localStorage.removeItem('last_draft_id');
        localStorage.removeItem('last_draft_created_at');
      }
    }
  }, [draftId, savedDraftData, mounted]);

  // Store the fixed expiry time (Unix timestamp in seconds) - never recalculate it
  const [fixedExpiryTime, setFixedExpiryTime] = useState<number | null>(null);

  // Set fixed expiry time based on when draft was created
  // Frontend shows 60 seconds, but actual intent expiry is 90 seconds
  // This gives 30 seconds buffer after frontend timer expires
  useEffect(() => {
    if (draftCreatedAt) {
      setFixedExpiryTime(Math.floor(draftCreatedAt / 1000) + 60); // Frontend timer: 60 seconds
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
    setError(null); // Clear any stale errors
    // Reset escrow state
    setEscrowHash(null);
    setApprovingToken(false);
    setCreatingEscrow(false);
    resetApprove();
    resetEscrow();
    if (typeof window !== 'undefined') {
      localStorage.removeItem('last_draft_id');
      localStorage.removeItem('last_draft_created_at');
    }
  };

  // Debug: Log escrow button state for inflow intents
  useEffect(() => {
    if (transactionHash && !escrowHash && savedDraftData) {
      const isInflowByChain = !isHubChainId(savedDraftData.offeredChainId);
      const derivedToken = getOfferedTokenFromDraft();
      console.log('🔍 Escrow button state check:', {
        isInflowByChain,
        transactionHash: !!transactionHash,
        escrowHash: !!escrowHash,
        signature: !!signature,
        savedDraftData: !!savedDraftData,
        offeredToken: !!offeredToken,
        derivedToken: derivedToken?.symbol || 'null',
        offeredChainId: savedDraftData.offeredChainId,
        willShowButton: isInflowByChain,
      });
      if (isInflowByChain && !offeredToken && !derivedToken) {
        console.error('🚨 Cannot find token for escrow! offeredChainId:', savedDraftData.offeredChainId, 'offeredMetadata:', savedDraftData.offeredMetadata);
      }
    }
  }, [transactionHash, escrowHash, savedDraftData, offeredToken, signature]);


  // Track if polling is active to prevent multiple polling loops
  const pollingActiveRef = useRef(false);

  // Poll for solver signature when draft exists
  useEffect(() => {
    if (!draftId || pollingActiveRef.current || signature) return; // Don't poll if already polling or have signature

    const pollSignature = async () => {
      pollingActiveRef.current = true;
      setPollingSignature(true);
      const maxAttempts = 60; // 60 attempts * 2 seconds = 120 seconds max (longer than 90s expiry)
      let attempts = 0;

      const poll = async () => {
        try {
          const response = await coordinatorClient.pollDraftSignature(draftId!);
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

  // Filter tokens dynamically based on selections
  // If offeredToken is selected, desiredTokens should exclude tokens from the same chain
  // If desiredToken is selected, offeredTokens should exclude tokens from the same chain
  const offeredTokens = useMemo(() => {
    if (desiredToken) {
      // If desired token is selected, exclude tokens from the same chain
      return SUPPORTED_TOKENS.filter(t => t.chain !== desiredToken.chain);
    }
    // If no desired token selected, show all tokens
    return SUPPORTED_TOKENS;
  }, [desiredToken]);

  const desiredTokens = useMemo(() => {
    if (offeredToken) {
      // If offered token is selected, exclude tokens from the same chain
      return SUPPORTED_TOKENS.filter(t => t.chain !== offeredToken.chain);
    }
    // If no offered token selected, show all tokens
    return SUPPORTED_TOKENS;
  }, [offeredToken]);

  // Helper to organize tokens for dropdown: USD tokens (USDC, USDC.e, USDT) first, then separator, then MOVE and ETH
  const organizeTokensForDropdown = (tokens: TokenConfig[]) => {
    const usdTokens = tokens.filter(t => t.symbol === 'USDC' || t.symbol === 'USDC.e' || t.symbol === 'USDT');
    const others = tokens.filter(t => t.symbol !== 'USDC' && t.symbol !== 'USDC.e' && t.symbol !== 'USDT');
    return { usdcs: usdTokens, others };
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
        const response = await coordinatorClient.getExchangeRate(
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

        // Calculate desired amount, adjusting for decimal differences
        // The exchange_rate is in smallest units, so we need to adjust for decimals
        const offeredAmountNum = parseFloat(offeredAmount);
        const decimalAdjustment = Math.pow(10, offeredToken.decimals - desiredToken.decimals);
        const desiredAmountNum = (offeredAmountNum * decimalAdjustment) / exchange_rate;
        setDesiredAmount(desiredAmountNum.toFixed(desiredToken.decimals));
        setError(null); // Clear any previous errors
      } catch (err) {
        // Exchange rate not available - show "not available yet" instead of error
        setDesiredAmount('not available yet');
        setError(null);
      }
    };

    fetchExchangeRate();
  }, [offeredToken, desiredToken, offeredAmount]);

  // Keep intent ID ref in sync for use in polling closure
  useEffect(() => {
    currentIntentIdRef.current = savedDraftData?.intentId || null;
  }, [savedDraftData?.intentId]);

  // Poll for fulfillment
  // - Outflow: Check coordinator events for fulfillment (GMP delivers FulfillmentProof to hub)
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
            // Outflow: Check coordinator events for fulfillment (GMP delivers FulfillmentProof to hub)
            console.log('Checking coordinator events for outflow intent:', currentIntentId);

            const eventsResponse = await coordinatorClient.getEvents();

            if (eventsResponse.success && eventsResponse.data) {
              const fulfillmentEvent = eventsResponse.data.fulfillment_events?.find(
                (e: any) => {
                  const normalizeId = (id: string) =>
                    id?.replace(/^0x/i, '').toLowerCase().replace(/^0+/, '') || '0';
                  return normalizeId(e.intent_id) === normalizeId(currentIntentId);
                }
              );

              if (fulfillmentEvent) {
                console.log('Found fulfillment event for outflow intent!');
                setIntentStatus('fulfilled');
                setPollingFulfillment(false);
                pollingFulfillmentRef.current = false;
                return;
              }
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
            const hubRpcUrl = getHubChainConfig().rpcUrl;
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
                      // Balance will be refreshed by the hook when intentStatus changes to 'fulfilled'
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

    const connectedChain = getConnectedChain(offeredToken, desiredToken);
    if (isEvmChain(connectedChain) && !evmAddress) {
      setError('Please connect your EVM wallet (MetaMask)');
      return;
    }
    if (isSvmChain(connectedChain) && !svmAddress) {
      setError('Please connect your SVM wallet (Phantom)');
      return;
    }

    if (!offeredToken || !desiredToken) {
      setError('Please select both offered and desired tokens');
      return;
    }
    
    // Determine flow type from selected tokens
    if (!flowType) {
      setError('Invalid token selection');
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

    // Actual expiry is 90 seconds, but frontend timer shows 60 seconds
    // This gives 30 seconds buffer after frontend timer expires for user to sign
    const expiryTime = Math.floor(Date.now() / 1000) + 90;

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

      const response = await coordinatorClient.createDraftIntent(request);

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

      const offeredChainKey = getChainKeyFromId(savedDraftData.offeredChainId);
      const desiredChainKey = getChainKeyFromId(savedDraftData.desiredChainId);
      const connectedChainKey =
        offeredChainKey && isHubChain(offeredChainKey) ? desiredChainKey : offeredChainKey;
      if (!connectedChainKey) {
        throw new Error('Unsupported connected chain for this draft');
      }

      if (flowType === 'inflow') {
        // Inflow: offered on connected chain, desired on hub (Move)
        if (connectedChainKey && getChainType(connectedChainKey) === 'svm') {
          if (!svmPublicKey) {
            throw new Error('SVM wallet (Phantom) must be connected for inflow intents');
          }
          // Deviation from EVM flow: SVM addresses are base58 pubkeys,
          // so we convert to 32-byte hex for Move address fields.
          const requesterAddrHex = svmPubkeyToHex(svmPublicKey);
          const offeredMetadataHex = svmPubkeyToHex(savedDraftData.offeredMetadata);

          if (!signature.solver_svm_addr) {
            throw new Error('Solver has no SVM address registered. The solver must register with an SVM address to fulfill SVM inflow intents.');
          }
          const solverAddrConnectedChainHex = svmPubkeyToHex(signature.solver_svm_addr);

          functionName = `${INTENT_MODULE_ADDR}::fa_intent_inflow::create_inflow_intent_entry`;
          functionArguments = [
            offeredMetadataHex,
            savedDraftData.offeredAmount,
            savedDraftData.offeredChainId,
            savedDraftData.desiredMetadata, // Move token - already 32 bytes
            savedDraftData.desiredAmount,
            savedDraftData.desiredChainId,
            savedDraftData.expiryTime.toString(),
            savedDraftData.intentId,
            signature.solver_hub_addr,
            solverAddrConnectedChainHex,
            signatureArray,
            requesterAddrHex,
          ];
        } else {
          const evmAddressForInflow = evmAddress || '0x' + '0'.repeat(40);
          const paddedRequesterAddr = padEvmAddressToMove(evmAddressForInflow);
          // Pad offered metadata (EVM token address) to 32 bytes
          const paddedOfferedMetadata = padEvmAddressToMove(savedDraftData.offeredMetadata);

          if (!signature.solver_evm_addr) {
            throw new Error('Solver has no EVM address registered. The solver must register with an EVM address to fulfill EVM inflow intents.');
          }
          const paddedSolverAddrConnectedChain = padEvmAddressToMove(signature.solver_evm_addr);

          functionName = `${INTENT_MODULE_ADDR}::fa_intent_inflow::create_inflow_intent_entry`;
          functionArguments = [
            paddedOfferedMetadata,
            savedDraftData.offeredAmount,
            savedDraftData.offeredChainId,
            savedDraftData.desiredMetadata, // Move token - already 32 bytes
            savedDraftData.desiredAmount,
            savedDraftData.desiredChainId,
            savedDraftData.expiryTime.toString(),
            savedDraftData.intentId,
            signature.solver_hub_addr,
            paddedSolverAddrConnectedChain,
            signatureArray,
            paddedRequesterAddr,
          ];
        }
      } else {
        // Outflow: offered on hub (Move), desired on connected chain
        if (connectedChainKey && getChainType(connectedChainKey) === 'svm') {
          if (!svmPublicKey) {
            throw new Error('SVM wallet (Phantom) must be connected for outflow intents');
          }
          // Deviation from EVM flow: SVM addresses are base58 pubkeys,
          // so we convert to 32-byte hex for Move address fields.
          const requesterAddrHex = svmPubkeyToHex(svmPublicKey);
          const desiredMetadataHex = svmPubkeyToHex(savedDraftData.desiredMetadata);

          if (!signature.solver_svm_addr) {
            throw new Error('Solver has no SVM address registered. The solver must register with an SVM address to fulfill SVM outflow intents.');
          }
          const solverAddrConnectedChainHex = svmPubkeyToHex(signature.solver_svm_addr);

          functionName = `${INTENT_MODULE_ADDR}::fa_intent_outflow::create_outflow_intent_entry`;
          functionArguments = [
            savedDraftData.offeredMetadata, // Move token - already 32 bytes
            savedDraftData.offeredAmount,
            savedDraftData.offeredChainId,
            desiredMetadataHex,
            savedDraftData.desiredAmount,
            savedDraftData.desiredChainId,
            savedDraftData.expiryTime.toString(),
            savedDraftData.intentId,
            requesterAddrHex,
            signature.solver_hub_addr,
            solverAddrConnectedChainHex,
            signatureArray,
          ];
        } else {
          if (!evmAddress) {
            throw new Error('EVM wallet (MetaMask) must be connected for outflow intents');
          }

          const paddedRequesterAddr = padEvmAddressToMove(evmAddress);
          // Pad desired metadata (EVM token address) to 32 bytes
          const paddedDesiredMetadata = padEvmAddressToMove(savedDraftData.desiredMetadata);
          console.log('Padded desired metadata:', paddedDesiredMetadata);

          if (!signature.solver_evm_addr) {
            throw new Error('Solver has no EVM address registered. The solver must register with an EVM address to fulfill EVM outflow intents.');
          }
          const paddedSolverAddrConnectedChain = padEvmAddressToMove(signature.solver_evm_addr);

          functionName = `${INTENT_MODULE_ADDR}::fa_intent_outflow::create_outflow_intent_entry`;
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
            signature.solver_hub_addr,
            paddedSolverAddrConnectedChain,
            signatureArray,
          ];
        }
      }

      // Use build-sign-submit pattern to work around Nightly wallet bug
      const senderAddress = mvmAccount?.address || directNightlyAddress;
      if (!senderAddress) {
        throw new Error('No MVM wallet connected');
      }

      // Configure Aptos client for Movement testnet
      const config = new AptosConfig({ 
        fullnode: getHubChainConfig().rpcUrl,
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
    // Use offeredToken if available, otherwise look it up from savedDraftData
    const effectiveOfferedToken = offeredToken || getOfferedTokenFromDraft();
    const isInflow = savedDraftData && !isHubChainId(savedDraftData.offeredChainId);
    
    console.log('handleCreateEscrow called', { savedDraftData, offeredToken, effectiveOfferedToken, isInflow, evmAddress, svmAddress, signature: !!signature, chainId });

    if (!savedDraftData || !effectiveOfferedToken || !isInflow || !signature) {
      const missing = [];
      if (!savedDraftData) missing.push('savedDraftData');
      if (!effectiveOfferedToken) missing.push('offeredToken (could not resolve from draft)');
      if (!isInflow) missing.push(`not inflow (offeredChainId=${savedDraftData?.offeredChainId})`);
      if (!signature) missing.push('signature');
      setError(`Missing required data for escrow creation: ${missing.join(', ')}`);
      return;
    }

    try {
      setError(null);

      if (getChainType(effectiveOfferedToken.chain) === 'svm') {
        if (!svmPublicKey) {
          setError('SVM wallet (Phantom) must be connected for escrow creation');
          return;
        }

        setCreatingEscrow(true);

        // Deviation from EVM flow: SVM uses a single on-chain instruction for escrow creation
        // because the escrow program transfers SPL tokens directly (no ERC20 approval step).
        console.log('SVM Escrow: Fetching solver SVM address for hub addr:', signature.solver_hub_addr);
        const solverSvmHex = await fetchSolverSvmAddress(signature.solver_hub_addr);
        console.log('SVM Escrow: Solver SVM hex:', solverSvmHex);
        if (!solverSvmHex) {
          throw new Error('Solver has no SVM address registered. The solver must register with an SVM address to fulfill SVM inflow intents.');
        }

        const tokenMint = new PublicKey(effectiveOfferedToken.metadata);
        const requesterToken = getSvmTokenAccount(tokenMint, svmPublicKey);
        const reservedSolver = svmHexToPubkey(solverSvmHex);
        console.log('SVM Escrow: Reserved solver pubkey:', reservedSolver.toBase58());
        const amount = BigInt(savedDraftData.offeredAmount);
        const createIx = buildCreateEscrowInstruction({
          intentId: savedDraftData.intentId,
          amount,
          requester: svmPublicKey,
          requesterToken,
          tokenMint,
          reservedSolver,
        });

        const connection = getSvmConnection();
        const signatureHash = await sendSvmTransaction({
          wallet: svmWallet,
          connection,
          instructions: [createIx],
        });
        setEscrowHash(signatureHash);
        setCreatingEscrow(false);
        return;
      }
      
      // Check if we're on the right chain
      const chainConfig = CHAIN_CONFIGS[effectiveOfferedToken.chain];
      if (!chainConfig) {
        setError(`Unsupported chain: ${effectiveOfferedToken.chain}`);
        return;
      }
      const requiredChainId = chainConfig.chainId;
      if (chainId !== requiredChainId) {
        console.log(`Switching chain from ${chainId} to ${requiredChainId}`);
        try {
          await switchChain({ chainId: requiredChainId });
          console.log('Chain switched successfully');
        } catch (switchError) {
          console.error('Failed to switch chain:', switchError);
          setError(`Please switch to ${chainConfig.name} in your wallet`);
          return;
        }
      }
      
      if (!evmAddress) {
        setError('EVM wallet (MetaMask) must be connected for escrow creation');
        return;
      }

      setApprovingToken(true);

      const escrowAddress = getEscrowContractAddress(effectiveOfferedToken.chain);
      console.log('Creating escrow with:', { escrowAddress, tokenAddress: effectiveOfferedToken.metadata, intentId: savedDraftData.intentId, chainId });
      const tokenAddress = effectiveOfferedToken.metadata as `0x${string}`;
      // Use amount from savedDraftData since offeredAmount state might be stale
      const amount = BigInt(savedDraftData.offeredAmount);
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
    if (getChainType(offeredToken.chain) === 'svm') {
      // SVM escrows are created directly without an approval step.
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
              // Clear desired token if it's from the same chain
              if (desiredToken && desiredToken.chain === chain) {
                setDesiredToken(null);
                setDesiredAmount('');
              }
            }}
            className="w-full px-4 py-2 bg-gray-900 border border-gray-600 rounded text-sm"
            required
          >
            <option value="">Select token...</option>
            {(() => {
              const { usdcs, others } = organizeTokensForDropdown(offeredTokens);
              return (
                <>
                  {usdcs.map((token) => (
                    <option key={`${token.chain}::${token.symbol}`} value={`${token.chain}::${token.symbol}`}>
                      {token.name}
                    </option>
                  ))}
                  {usdcs.length > 0 && others.length > 0 && (
                    <option disabled>------</option>
                  )}
                  {others.map((token) => (
                    <option key={`${token.chain}::${token.symbol}`} value={`${token.chain}::${token.symbol}`}>
                      {token.name}
                    </option>
                  ))}
                </>
              );
            })()}
          </select>
        </div>

        <div>
          <div className="flex items-center gap-2">
            <input
              type="text"
              inputMode="decimal"
              lang="en"
              value={offeredAmount}
              onChange={(e) => {
                // Normalize comma to dot for decimal separator
                const normalized = e.target.value.replace(',', '.');
                setOfferedAmount(normalized);
              }}
              placeholder="1"
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

        {/* Swap Direction Button */}
        <div className="flex justify-center -my-2">
          <button
            type="button"
            onClick={() => {
              // Swap tokens
              const tempToken = offeredToken;
              setOfferedToken(desiredToken);
              setDesiredToken(tempToken);
              // Swap amounts
              const tempAmount = offeredAmount;
              setOfferedAmount(desiredAmount === 'not available yet' ? '' : desiredAmount);
              setDesiredAmount(tempAmount === '' ? 'not available yet' : tempAmount);
            }}
            className="p-2 bg-gray-800 hover:bg-gray-700 border border-gray-600 rounded-full transition-colors"
            title="Swap Send and Receive"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth={1.5}
              stroke="currentColor"
              className="w-5 h-5 text-gray-400"
            >
              <path strokeLinecap="round" strokeLinejoin="round" d="M3 7.5L7.5 3m0 0L12 7.5M7.5 3v13.5m13.5 0L16.5 21m0 0L12 16.5m4.5 4.5V7.5" />
            </svg>
          </button>
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
              // Clear offered token if it's from the same chain
              if (offeredToken && offeredToken.chain === chain) {
                setOfferedToken(null);
              }
            }}
            className="w-full px-4 py-2 bg-gray-900 border border-gray-600 rounded text-sm"
            required
          >
            <option value="">Select token...</option>
            {(() => {
              const { usdcs, others } = organizeTokensForDropdown(desiredTokens);
              return (
                <>
                  {usdcs.map((token) => (
                    <option key={`${token.chain}::${token.symbol}`} value={`${token.chain}::${token.symbol}`}>
                      {token.name}
                    </option>
                  ))}
                  {usdcs.length > 0 && others.length > 0 && (
                    <option disabled>------</option>
                  )}
                  {others.map((token) => (
                    <option key={`${token.chain}::${token.symbol}`} value={`${token.chain}::${token.symbol}`}>
                      {token.name}
                    </option>
                  ))}
                </>
              );
            })()}
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
                      : "Enter send amount first"
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
          {/* Request Button - show when ready to request, stay visible (greyed out) after requested */}
          {!draftId && (
            <button
              type="submit"
              disabled={loading || !requesterAddr || !connectedWalletReady || desiredAmount === 'not available yet'}
              className={`w-full px-4 py-2 rounded text-sm font-medium transition-colors ${
                desiredAmount === 'not available yet' || !requesterAddr || !connectedWalletReady
                  ? 'bg-gray-600 text-gray-400 cursor-not-allowed'
                  : 'bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed'
              }`}
            >
              {loading ? 'Requesting...' : 'Request'}
            </button>
          )}
          {draftId && (
            <button
              type="button"
              disabled
              className="w-full px-4 py-2 rounded text-sm font-medium bg-gray-600 text-gray-400 cursor-not-allowed"
            >
              ✓ Requested
            </button>
          )}

          {/* Commit Button - show when signature received, stay visible (greyed out) after committed */}
          {signature && savedDraftData && !transactionHash && (
            <button
              type="button"
              onClick={handleCreateIntent}
              disabled={submittingTransaction || !requesterAddr || !connectedWalletReady}
              className="w-full px-4 py-2 rounded text-sm font-medium transition-colors bg-green-600 hover:bg-green-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {(() => {
                const isOutflow = flowType === 'outflow' || isHubChainId(savedDraftData?.offeredChainId);
                if (submittingTransaction) return isOutflow ? 'Committing and Sending...' : 'Committing...';
                return isOutflow ? 'Commit and Send' : 'Commit';
              })()}
            </button>
          )}
          {signature && savedDraftData && transactionHash && (
            <button
              type="button"
              disabled
              className="w-full px-4 py-2 rounded text-sm font-medium bg-gray-600 text-gray-400 cursor-not-allowed"
            >
              {(() => {
                const isOutflow = flowType === 'outflow' || isHubChainId(savedDraftData?.offeredChainId);
                return isOutflow ? '✓ Committed and Sent' : '✓ Committed';
              })()}
            </button>
          )}

          {/* Send Button (for inflow only) - show when committed, stay visible (greyed out) after sent */}
          {(!isHubChainId(savedDraftData?.offeredChainId)) && transactionHash && !escrowHash && (
            <button
              type="button"
              onClick={handleCreateEscrow}
              disabled={approvingToken || creatingEscrow || isApproving || isCreatingEscrow || isApprovePending || isEscrowPending || !escrowWalletReady}
              className="w-full px-4 py-2 rounded text-sm font-medium transition-colors bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {escrowRequiresSvm
                ? creatingEscrow
                  ? 'Sending...'
                  : 'Create Escrow on SVM'
                : isApprovePending
                  ? 'Confirm in wallet...'
                  : approvingToken || isApproving
                    ? 'Approving token...'
                    : isEscrowPending
                      ? 'Confirm escrow in wallet...'
                      : creatingEscrow || isCreatingEscrow
                        ? 'Sending...'
                        : 'Create Escrow on EVM'}
            </button>
          )}
          {(!isHubChainId(savedDraftData?.offeredChainId)) && transactionHash && escrowHash && (
            <button
              type="button"
              disabled
              className="w-full px-4 py-2 rounded text-sm font-medium bg-gray-600 text-gray-400 cursor-not-allowed"
            >
              ✓ Sent
            </button>
          )}
          
          {/* Status note below buttons */}
          {!requesterAddr && (
            <p className="text-xs text-gray-400 text-center">
              Connect your MVM wallet (Nightly) to create an intent
            </p>
          )}
          {requesterAddr && requiresEvmWallet && !evmAddress && (
            <p className="text-xs text-gray-400 text-center">
              Connect your EVM wallet (MetaMask) to create an intent
            </p>
          )}
          {requesterAddr && requiresSvmWallet && !svmAddress && (
            <p className="text-xs text-gray-400 text-center">
              Connect your SVM wallet (Phantom) to create an intent
            </p>
          )}
          {requesterAddr && connectedWalletReady && !draftId && (
            <p className="text-xs text-gray-500 text-center">
              Request intent for solver approval
            </p>
          )}
          {draftId && !signature && pollingSignature && (
            <p className="text-xs text-yellow-400 text-center flex items-center justify-center gap-2">
              <svg className="animate-spin h-4 w-4 text-yellow-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              Waiting for solver commitment
            </p>
          )}
          {signature && savedDraftData && !transactionHash && (
            <p className="text-xs text-green-400 text-center">
              Solver approved! Now commit to the intent.
            </p>
          )}
        </div>

        {/* Timer - outside status box */}
        {mounted && draftId && savedDraftData && signature && intentStatus !== 'fulfilled' && timeRemaining !== null && 
         ((isHubChainId(savedDraftData?.offeredChainId) && !transactionHash) || 
          (!isHubChainId(savedDraftData?.offeredChainId) && !escrowHash)) && (
          <p className="text-xs text-gray-400 text-center">
            Time remaining: {Math.floor(timeRemaining / 1000)}s
            {timeRemaining === 0 && ' (Expired)'}
          </p>
        )}

        {/* Status Display */}
        {mounted && draftId && savedDraftData && signature && (
          <div className="mt-2">
            {transactionHash && (
              <div className="mt-2 space-y-2">
                {/* Show waiting message only after tokens are sent: 
                    - Outflow: immediately after commit (tokens sent on commit)
                    - Inflow: only after escrow is created (escrowHash exists) */}
                {intentStatus === 'created' && pollingFulfillment && 
                 (isHubChainId(savedDraftData?.offeredChainId) || escrowHash) && (
                  <p className="text-xs text-yellow-400 text-center flex items-center justify-center gap-2">
                    <svg className="animate-spin h-4 w-4 text-yellow-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                      <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4"></circle>
                      <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                    </svg>
                    Waiting for funds to arrive...
                  </p>
                )}
                
                {intentStatus === 'fulfilled' && (
                  <p className="text-xs font-bold text-green-400 text-center">Funds received!</p>
                )}
              </div>
            )}
          </div>
        )}
        
        {/* Clear button at bottom when fulfilled */}
        {mounted && draftId && savedDraftData && intentStatus === 'fulfilled' && (
          <button
            type="button"
            onClick={clearDraft}
            className="mt-3 w-full px-4 py-2 bg-yellow-600 hover:bg-yellow-500 text-black font-medium rounded text-sm"
          >
            Clear & Create New Intent
          </button>
        )}

        {/* Debug info - Tx and Intent ID */}
        {transactionHash && (
          <div className="mt-2 text-xs text-gray-500 space-y-1">
            <p className="font-mono break-all">Intent Tx: {transactionHash}</p>
            {savedDraftData?.intentId && (
              <p className="font-mono break-all">Intent ID: {savedDraftData.intentId}</p>
            )}
            {escrowHash && (
              <p className="font-mono break-all">Escrow Tx: {escrowHash}</p>
            )}
          </div>
        )}
      </form>
    </div>
  );
}

