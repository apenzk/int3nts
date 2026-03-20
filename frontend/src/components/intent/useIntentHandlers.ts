import type { MutableRefObject, Dispatch, SetStateAction } from 'react';
import {
  CoordinatorClient,
  type TokenConfig,
  type SvmSigner,
  type DraftData,
  type DraftIntentSignature,
  type FeeInfo,
  type FlowType,
  getChainType,
  getHubChainConfig,
  getEscrowContractAddress,
  getSvmGmpEndpointId,
  INTENT_ESCROW_ABI,
  ERC20_ABI,
  intentIdToEvmBytes32,
  buildCreateEscrowInstruction,
  getSvmTokenAccount,
  svmHexToPubkey,
  readGmpOutboundNonce,
  fetchSolverSvmAddress,
  getSvmConnection,
  sendSvmTransaction,
  toSmallestUnits,
  isHubChain,
  createDraft,
  buildIntentArguments,
} from '@int3nts/sdk';
import { CHAIN_CONFIGS } from '@/config/chains';
import { SUPPORTED_TOKENS } from '@/config/tokens';
import { Aptos, AptosConfig } from '@aptos-labs/ts-sdk';
import { PublicKey } from '@solana/web3.js';

/** Resolve the offered token from saved draft data when offeredToken state is not set. */
export function getOfferedTokenFromDraft(savedDraftData: DraftData | null): TokenConfig | null {
  if (!savedDraftData) return null;
  return SUPPORTED_TOKENS.find(t => {
    const chainConfig = CHAIN_CONFIGS[t.chain];
    if (!chainConfig || String(chainConfig.chainId) !== savedDraftData.offeredChainId) {
      return false;
    }
    if (getChainType(CHAIN_CONFIGS, t.chain) === 'svm') {
      // Deviation from EVM flow: SVM mint addresses are base58 strings,
      // so we compare directly instead of hex-padding.
      return t.metadata.toLowerCase() === savedDraftData.offeredMetadata.toLowerCase();
    }
    return t.metadata
      .toLowerCase()
      .includes(savedDraftData.offeredMetadata.replace(/^0x0*/, '').toLowerCase());
  }) || null;
}

interface IntentHandlerDeps {
  coordinator: CoordinatorClient;
  flowType: FlowType | null;
  requesterAddr: string;
  offeredToken: TokenConfig | null;
  desiredToken: TokenConfig | null;
  offeredAmount: string;
  desiredAmount: string;
  feeInfo: FeeInfo | null;
  evmAddress: string | undefined;
  svmAddress: string;
  svmPublicKey: PublicKey | null;
  svmWallet: object;
  mvmAccount: { address: string } | null | undefined;
  directNightlyAddress: string;
  chainId: number;
  switchChain: (params: { chainId: number }) => void;
  savedDraftData: DraftData | null;
  signature: DraftIntentSignature | null;
  // Setters from useIntentDraft
  setDraftId: (id: string | null) => void;
  setDraftCreatedAt: (ts: number | null) => void;
  setSavedDraftData: Dispatch<SetStateAction<DraftData | null>>;
  setSignature: (sig: DraftIntentSignature | null) => void;
  setPollingSignature: (v: boolean) => void;
  pollingActiveRef: MutableRefObject<boolean>;
  setError: (msg: string | null) => void;
  // Setters for IntentBuilder local state
  setLoading: (v: boolean) => void;
  setTransactionHash: (hash: string | null) => void;
  setSubmittingTransaction: (v: boolean) => void;
  setEscrowHash: (hash: string | null) => void;
  setApprovingToken: (v: boolean) => void;
  setCreatingEscrow: (v: boolean) => void;
  // Wagmi write functions (typed loosely to avoid wagmi internal type coupling)
  writeApprove: (params: { address: `0x${string}`; abi: readonly unknown[]; functionName: string; args: readonly unknown[] }) => void;
  writeCreateEscrow: (params: { address: `0x${string}`; abi: readonly unknown[]; functionName: string; args: readonly unknown[]; gas?: bigint }) => void;
}

export function useIntentHandlers(deps: IntentHandlerDeps) {
  const {
    coordinator,
    flowType,
    requesterAddr,
    offeredToken,
    desiredToken,
    offeredAmount,
    desiredAmount,
    feeInfo,
    evmAddress,
    svmAddress,
    svmPublicKey,
    svmWallet,
    mvmAccount,
    directNightlyAddress,
    chainId,
    switchChain,
    savedDraftData,
    signature,
    setDraftId,
    setDraftCreatedAt,
    setSavedDraftData,
    setSignature,
    setPollingSignature,
    pollingActiveRef,
    setError,
    setLoading,
    setTransactionHash,
    setSubmittingTransaction,
    setEscrowHash,
    setApprovingToken,
    setCreatingEscrow,
    writeApprove,
    writeCreateEscrow,
  } = deps;

  const isEvmChain = (chain: TokenConfig['chain']) => getChainType(CHAIN_CONFIGS, chain) === 'evm';
  const isSvmChain = (chain: TokenConfig['chain']) => getChainType(CHAIN_CONFIGS, chain) === 'svm';
  const getConnectedChain = (offered: TokenConfig, desired: TokenConfig) =>
    isHubChain(CHAIN_CONFIGS, offered.chain) ? desired.chain : offered.chain;
  const isHubChainId = (chainIdValue?: string | null) =>
    chainIdValue === String(getHubChainConfig(CHAIN_CONFIGS).chainId);

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

    if (!requesterAddr) {
      setError('Please connect your MVM wallet (Nightly)');
      return;
    }
    if (!offeredToken || !desiredToken) {
      setError('Please select both offered and desired tokens');
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
    if (!flowType) {
      setError('Invalid token selection');
      return;
    }
    if (!desiredAmount || desiredAmount === 'not available yet' || parseFloat(desiredAmount) <= 0) {
      setError('Exchange rate not available. Cannot create draft intent.');
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

    setLoading(true);
    try {
      const { draftId: newDraftId, draftData } = await createDraft({
        coordinator,
        requesterAddr,
        offeredToken,
        offeredAmount: offeredAmountNum,
        offeredChainId: CHAIN_CONFIGS[offeredToken.chain].chainId,
        desiredToken,
        desiredAmount,
        desiredChainId: CHAIN_CONFIGS[desiredToken.chain].chainId,
        flowType,
        feeInfo,
      });

      setDraftId(newDraftId);
      setSavedDraftData(draftData);
      setError(null);

      const createdAt = Date.now();
      setDraftCreatedAt(createdAt);

      if (typeof window !== 'undefined') {
        localStorage.setItem('last_draft_id', newDraftId);
        localStorage.setItem('last_draft_created_at', createdAt.toString());
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
    if (!savedDraftData.intentId) {
      setError('Intent ID not found in saved draft data');
      return;
    }

    console.log('Creating intent on-chain with intent ID from draft:', savedDraftData.intentId);

    setSubmittingTransaction(true);
    setError(null);

    try {
      const { functionName, functionArguments } = buildIntentArguments({
        configs: CHAIN_CONFIGS,
        draftData: savedDraftData,
        signature,
        flowType: flowType!,
        requesterAddr,
        evmAddress: evmAddress || undefined,
        svmPublicKey: svmPublicKey?.toBase58(),
      });

      // Use build-sign-submit pattern to work around Nightly wallet bug
      const senderAddress = mvmAccount?.address || directNightlyAddress;
      if (!senderAddress) {
        throw new Error('No MVM wallet connected');
      }

      const config = new AptosConfig({
        fullnode: getHubChainConfig(CHAIN_CONFIGS).rpcUrl,
      });
      const aptos = new Aptos(config);

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

      let signResponse: any;
      if (mvmAccount?.address) {
        const nightlyWallet = (window as any).nightly?.aptos;
        if (nightlyWallet) {
          signResponse = await nightlyWallet.signTransaction(rawTxn);
        } else {
          throw new Error('Nightly wallet not available for signing');
        }
      } else if (directNightlyAddress) {
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

      const senderAuthenticator = signResponse?.args || signResponse;
      console.log('Submitting signed transaction...');

      const pendingTxn = await aptos.transaction.submit.simple({
        transaction: rawTxn,
        senderAuthenticator: senderAuthenticator,
      });

      console.log('Transaction submitted:', pendingTxn);
      if (pendingTxn && pendingTxn.hash) {
        setTransactionHash(pendingTxn.hash);

        try {
          const txnResult = await aptos.waitForTransaction({ transactionHash: pendingTxn.hash });
          if ('events' in txnResult && Array.isArray(txnResult.events)) {
            for (const event of txnResult.events) {
              if (event.type?.includes('OracleLimitOrderEvent') || event.type?.includes('LimitOrderEvent')) {
                // Use intent_id (the original ID from draft) - this is what the solver uses for validation
                // intent_addr is the object address created on-chain (different)
                const onChainIntentId = event.data?.intent_id || event.data?.intent_addr || event.data?.id;
                if (onChainIntentId) {
                  console.log('On-chain intent_id for approval tracking:', onChainIntentId);
                  setSavedDraftData(prev => prev ? { ...prev, intentId: onChainIntentId } : null);
                }
                break;
              }
            }
          }
        } catch (waitErr) {
          console.error('Failed to wait for transaction confirmation:', waitErr);
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

  const handleCreateEscrow = async () => {
    const effectiveOfferedToken = offeredToken || getOfferedTokenFromDraft(savedDraftData);
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

      if (getChainType(CHAIN_CONFIGS, effectiveOfferedToken.chain) === 'svm') {
        if (!svmPublicKey) {
          setError('SVM wallet (Phantom) must be connected for escrow creation');
          return;
        }

        setCreatingEscrow(true);

        // Deviation from EVM flow: SVM uses a single on-chain instruction for escrow creation
        // because the escrow program transfers SPL tokens directly (no ERC20 approval step).
        console.log('SVM Escrow: Fetching solver SVM address for hub addr:', signature.solver_hub_addr);
        const hubCfg = getHubChainConfig(CHAIN_CONFIGS);
        const solverSvmHex = await fetchSolverSvmAddress(hubCfg.rpcUrl, hubCfg.intentContractAddress!, signature.solver_hub_addr);
        console.log('SVM Escrow: Solver SVM hex:', solverSvmHex);
        if (!solverSvmHex) {
          throw new Error('Solver has no SVM address registered. The solver must register with an SVM address to fulfill SVM inflow intents.');
        }

        const tokenMint = new PublicKey(effectiveOfferedToken.metadata);
        const requesterToken = getSvmTokenAccount(tokenMint, svmPublicKey);
        const reservedSolver = svmHexToPubkey(solverSvmHex);
        console.log('SVM Escrow: Reserved solver pubkey:', reservedSolver.toBase58());
        const amount = BigInt(savedDraftData.offeredAmount);

        const svmChainCfg = CHAIN_CONFIGS[effectiveOfferedToken.chain];
        const connection = getSvmConnection(svmChainCfg.rpcUrl);
        const gmpEndpointId = new PublicKey(getSvmGmpEndpointId(CHAIN_CONFIGS, effectiveOfferedToken.chain));
        console.log('SVM Escrow: Reading GMP global outbound nonce');
        const currentNonce = await readGmpOutboundNonce(connection, gmpEndpointId);
        console.log('SVM Escrow: Current GMP outbound nonce:', currentNonce.toString());

        const createIx = buildCreateEscrowInstruction({
          intentId: savedDraftData.intentId,
          amount,
          requester: svmPublicKey,
          requesterToken,
          tokenMint,
          reservedSolver,
          programId: new PublicKey(svmChainCfg.svmProgramId!),
          gmpParams: {
            gmpEndpointProgramId: gmpEndpointId,
            currentNonce,
          },
        });
        const signatureHash = await sendSvmTransaction({
          signer: svmWallet as unknown as SvmSigner,
          connection,
          instructions: [createIx],
        });
        setEscrowHash(signatureHash);
        setCreatingEscrow(false);
        return;
      }

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

      const escrowAddress = getEscrowContractAddress(CHAIN_CONFIGS, effectiveOfferedToken.chain) as `0x${string}`;
      console.log('Creating escrow with:', { escrowAddress, tokenAddress: effectiveOfferedToken.metadata, intentId: savedDraftData.intentId, chainId });
      const tokenAddress = effectiveOfferedToken.metadata as `0x${string}`;

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

  // Create escrow after EVM token approval completes
  const handleCreateEscrowAfterApproval = () => {
    console.log('handleCreateEscrowAfterApproval called');

    if (!savedDraftData || !offeredToken || flowType !== 'inflow' || !evmAddress || !signature) {
      console.error('handleCreateEscrowAfterApproval: missing data');
      return;
    }
    if (getChainType(CHAIN_CONFIGS, offeredToken.chain) === 'svm') {
      // SVM escrows are created directly without an approval step.
      return;
    }

    try {
      setCreatingEscrow(true);

      const escrowAddress = getEscrowContractAddress(CHAIN_CONFIGS, offeredToken.chain) as `0x${string}`;
      const tokenAddress = offeredToken.metadata as `0x${string}`;
      const amount = BigInt(toSmallestUnits(parseFloat(offeredAmount), offeredToken.decimals));
      const intentIdEvm = intentIdToEvmBytes32(savedDraftData.intentId);

      console.log('Creating escrow:', { escrowAddress, tokenAddress, amount: amount.toString(), intentIdEvm });

      writeCreateEscrow({
        address: escrowAddress,
        abi: INTENT_ESCROW_ABI,
        functionName: 'createEscrowWithValidation',
        args: [intentIdEvm, tokenAddress, amount],
        gas: BigInt(500_000),
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create escrow');
      setCreatingEscrow(false);
    }
  };

  return {
    handleSubmit,
    handleCreateIntent,
    handleCreateEscrow,
    handleCreateEscrowAfterApproval,
  };
}
