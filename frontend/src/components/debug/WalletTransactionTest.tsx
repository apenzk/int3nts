'use client';

import { useState } from 'react';
import { Aptos, AptosConfig } from '@aptos-labs/ts-sdk';
import { INTENT_MODULE_ADDRESS } from '@/lib/move-transactions';
import { useAccount, useChainId, useSwitchChain, useWriteContract, useWaitForTransactionReceipt } from 'wagmi';
import { useWallet } from '@aptos-labs/wallet-adapter-react';
import { ERC20_ABI, INTENT_ESCROW_ABI, getEscrowContractAddress } from '@/lib/escrow';

export function WalletTransactionTest() {
  const [result, setResult] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  
  // EVM hooks
  const { address: evmAddress, isConnected: evmConnected } = useAccount();
  // MVM hooks
  const { account: mvmAccount, connected: mvmConnected } = useWallet();
  const chainId = useChainId();
  const { switchChain, isPending: isSwitching } = useSwitchChain();
  const { writeContract, data: txHash, error: writeError, isPending: isWritePending, reset } = useWriteContract();

  const test0_WhoAmI = async () => {
    setLoading(true);
    setResult(null);
    try {
      // Get MVM address
      let mvmAddr = 'Not connected';
      if (mvmAccount?.address) {
        mvmAddr = mvmAccount.address;
      } else if (typeof window !== 'undefined') {
        const savedAddress = localStorage.getItem('nightly_connected_address');
        if (savedAddress) {
          mvmAddr = savedAddress;
        }
      }
      
      // Get EVM address
      const evmAddr = evmAddress || 'Not connected';
      
      setResult(`MVM Address: ${mvmAddr}\nEVM Address: ${evmAddr}`);
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

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
      const verifierUrl = process.env.NEXT_PUBLIC_VERIFIER_URL || 'http://localhost:3333';
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

  const test7_SolverRegistry = async () => {
    setLoading(true);
    setResult(null);
    try {
      const rpcUrl = 'https://testnet.movementnetwork.xyz/v1';
      const response = await fetch(`${rpcUrl}/accounts/${INTENT_MODULE_ADDRESS}/resource/${INTENT_MODULE_ADDRESS}::solver_registry::SolverRegistry`);
      
      if (!response.ok) {
        if (response.status === 404) {
          setResult('FAIL: SolverRegistry not found - no solvers registered');
        } else {
          setResult(`FAIL: HTTP ${response.status}`);
        }
        return;
      }
      
      const data = await response.json();
      const solvers = data.data?.solvers?.data || [];
      
      if (solvers.length === 0) {
        setResult('INFO: SolverRegistry exists but no solvers registered');
        return;
      }
      
      // Format solver info
      const solverInfo = solvers.map((s: any) => {
        const addr = s.key?.slice(0, 10) + '...' + s.key?.slice(-8);
        const evmAddr = s.value?.connected_chain_evm_addr?.vec?.[0] || 'None';
        return `${addr} => EVM: ${evmAddr}`;
      }).join('; ');
      
      setResult(`PASS: ${solvers.length} solver(s) registered. ${solverInfo}`);
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test8_VerifierSignature = async () => {
    setLoading(true);
    setResult(null);
    try {
      // Get last draft ID from localStorage (set by IntentBuilder)
      const draftId = localStorage.getItem('last_draft_id');
      if (!draftId) {
        setResult('FAIL: No draft ID in localStorage. Create a draft intent first.');
        return;
      }
      const verifierUrl = process.env.NEXT_PUBLIC_VERIFIER_URL || 'http://localhost:3333';
      const url = `${verifierUrl}/draftintent/${draftId}/signature`;
      console.log('Querying verifier:', url);
      
      const response = await fetch(url);
      
      if (!response.ok) {
        const text = await response.text();
        setResult(`FAIL: HTTP ${response.status} - ${text.slice(0, 200)}`);
        return;
      }
      
      const data = await response.json();
      console.log('Verifier signature response:', data);
      
      if (!data.success) {
        setResult(`INFO: ${data.error || 'No signature yet'}`);
        return;
      }
      
      const sig = data.data;
      const evmStatus = sig.solver_evm_addr 
        ? `EVM: ${sig.solver_evm_addr}` 
        : 'EVM: NOT SET';
      
      setResult(`PASS: Signature found. Solver: ${sig.solver_addr?.slice(0, 10)}...${sig.solver_addr?.slice(-8)}, ${evmStatus}`);
    } catch (err) {
      const verifierUrl = process.env.NEXT_PUBLIC_VERIFIER_URL || 'http://localhost:3333';
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)} (verifier: ${verifierUrl})`);
    } finally {
      setLoading(false);
    }
  };

  const test9_EvmConnection = async () => {
    setLoading(true);
    setResult(null);
    try {
      if (!evmConnected || !evmAddress) {
        setResult('FAIL: EVM wallet not connected. Connect MetaMask first.');
        return;
      }
      
      const expectedChain = 84532; // Base Sepolia
      const chainName = chainId === 84532 ? 'Base Sepolia' : chainId === 11155111 ? 'Ethereum Sepolia' : `Unknown (${chainId})`;
      
      if (chainId !== expectedChain) {
        setResult(`INFO: Connected to ${chainName}. Expected Base Sepolia (84532). Click Test 10 to switch.`);
      } else {
        setResult(`PASS: EVM wallet connected. Address: ${evmAddress}, Chain: ${chainName}`);
      }
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test10_SwitchToBaseSepolia = async () => {
    setLoading(true);
    setResult(null);
    try {
      if (!evmConnected) {
        setResult('FAIL: EVM wallet not connected');
        return;
      }
      
      if (chainId === 84532) {
        setResult('PASS: Already on Base Sepolia');
        return;
      }
      
      console.log('Switching to Base Sepolia...');
      await switchChain({ chainId: 84532 });
      setResult('PASS: Switched to Base Sepolia');
    } catch (err) {
      console.error('Switch chain error:', err);
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test11_EvmTokenApprove = async () => {
    setLoading(true);
    setResult(null);
    reset(); // Reset previous tx state
    
    try {
      if (!evmConnected || !evmAddress) {
        setResult('FAIL: EVM wallet not connected');
        return;
      }
      
      if (chainId !== 84532) {
        setResult('FAIL: Wrong chain. Run Test 10 to switch to Base Sepolia');
        return;
      }
      
      const tokenAddress = '0x036CbD53842c5426634e7929541eC2318f3dCF7e' as `0x${string}`; // USDC on Base Sepolia
      const escrowAddress = getEscrowContractAddress('base-sepolia');
      // Approve max uint256 (same as real flow) to avoid repeated approvals
      const approveAmount = BigInt('0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff');
      
      console.log('Approving USDC...', { tokenAddress, escrowAddress, approveAmount: approveAmount.toString() });
      
      writeContract({
        address: tokenAddress,
        abi: ERC20_ABI,
        functionName: 'approve',
        args: [escrowAddress, approveAmount],
      });
      
      setResult('INFO: Approval submitted - check MetaMask popup. Check console for tx hash.');
    } catch (err) {
      console.error('Approve error:', err);
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test12_EvmEscrowCreate = async () => {
    setLoading(true);
    setResult(null);
    reset(); // Reset previous tx state
    
    try {
      if (!evmConnected || !evmAddress) {
        setResult('FAIL: EVM wallet not connected');
        return;
      }
      
      if (chainId !== 84532) {
        setResult('FAIL: Wrong chain. Run Test 10 to switch to Base Sepolia');
        return;
      }
      
      const escrowAddress = getEscrowContractAddress('base-sepolia');
      const tokenAddress = '0x036CbD53842c5426634e7929541eC2318f3dCF7e' as `0x${string}`; // USDC on Base Sepolia
      const amount = BigInt(1000); // 0.001 USDC (6 decimals)
      // Generate random intent ID to avoid conflicts with existing escrows
      const randomHex = Array.from({ length: 64 }, () => Math.floor(Math.random() * 16).toString(16)).join('');
      const testIntentId = BigInt('0x' + randomHex);
      // Use user's own address as solver (contract requires non-zero address)
      const solverAddress = evmAddress as `0x${string}`;
      
      console.log('Creating test escrow...', { escrowAddress, tokenAddress, amount: amount.toString(), testIntentId: '0x' + randomHex, solverAddress });
      
      writeContract({
        address: escrowAddress,
        abi: INTENT_ESCROW_ABI,
        functionName: 'createEscrow',
        args: [testIntentId, tokenAddress, amount, solverAddress],
      });
      
      setResult('INFO: Escrow creation submitted - check MetaMask popup. Check console for tx hash.');
    } catch (err) {
      console.error('Create escrow error:', err);
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test13_QueryEscrowTxReceipt = async () => {
    setLoading(true);
    setResult(null);
    try {
      if (chainId !== 84532) {
        setResult('FAIL: Wrong chain. Run Test 10 to switch to Base Sepolia');
        return;
      }
      
      // Use the known transaction hash from the logs
      const txHash = '0x428fafaaba8c9c25c29a0305755200a3d0f8f08bebccd9e91d01e16b2e18c8e1';
      
      const rpcUrl = 'https://sepolia.base.org';
      const escrowAddress = getEscrowContractAddress('base-sepolia');
      const eventTopic = '0x104303e46c846fc43f53cd6c4ab9ce96acdf68dcee176382e71fc812218a25a0';
      
      console.log('Checking transaction:', txHash);
      
      // Get the transaction to see what function was called
      const txResponse = await fetch(rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 0,
          method: 'eth_getTransactionByHash',
          params: [txHash.startsWith('0x') ? txHash : '0x' + txHash],
        }),
      });
      
      const txData = await txResponse.json();
      const functionSelector = txData.result?.input?.slice(0, 10) || 'unknown';
      
      // Get the full transaction receipt
      const receiptResponse = await fetch(rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'eth_getTransactionReceipt',
          params: [txHash.startsWith('0x') ? txHash : '0x' + txHash],
        }),
      });
      
      const receiptData = await receiptResponse.json();
      if (receiptData.error) {
        setResult(`FAIL: ${receiptData.error.message || JSON.stringify(receiptData.error)}\nChain: Base Sepolia (84532)\nRPC: ${rpcUrl}`);
        return;
      }
      
      if (!receiptData.result) {
        setResult(`FAIL: Transaction not found or not yet mined\nChain: Base Sepolia (84532)\nRPC: ${rpcUrl}\nTX: ${txHash}`);
        return;
      }
      
      const receipt = receiptData.result;
      const blockNumber = parseInt(receipt.blockNumber, 16);
      
      // Check all logs from escrow contract
      const escrowLogs = receipt.logs.filter((log: any) => 
        log.address.toLowerCase() === escrowAddress.toLowerCase()
      );
      
      // Check all event topics
      const allTopics = escrowLogs.map((log: any) => log.topics[0]);
      const hasEscrowEvent = allTopics.includes(eventTopic);
      
      // Show all logs from escrow contract with full details
      const logDetails = escrowLogs.map((log: any, i: number) => {
        const topic0 = log.topics[0];
        // Common ERC20 event signatures
        const approvalTopic = '0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925'; // Approval(address,address,uint256)
        const transferTopic = '0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef'; // Transfer(address,address,uint256)
        
        let eventType = 'Unknown';
        if (topic0 === approvalTopic) eventType = 'ERC20 Approval';
        else if (topic0 === transferTopic) eventType = 'ERC20 Transfer';
        else if (topic0 === eventTopic) eventType = 'EscrowInitialized ✅';
        else {
          // Check other escrow events
          const claimedSig = 'EscrowClaimed(uint256,address,uint256)';
          const cancelledSig = 'EscrowCancelled(uint256,address,uint256)';
          // We'd need keccak256 to calculate these, but for now just show the topic
        }
        
        return `Log ${i + 1}: ${eventType}\ntopic[0]=${topic0}\ntopics=${log.topics.length}\ndata=${log.data?.slice(0, 40)}...`;
      }).join('\n\n');
      
      // Check all logs to see if EscrowInitialized is elsewhere
      const allLogs = receipt.logs.map((log: any, i: number) => ({
        address: log.address,
        topic0: log.topics[0],
        isEscrow: log.address.toLowerCase() === escrowAddress.toLowerCase(),
      }));
      
      const escrowInitializedElsewhere = allLogs.find((log: any) => 
        log.topic0 === eventTopic && !log.isEscrow
      );
      
      // Calculate expected selector: keccak256("createEscrow(uint256,address,uint256,address)")[:4]
      // Using viem's formatAbiItem to get the correct selector
      const createEscrowSig = 'createEscrow(uint256,address,uint256,address)';
      const expectedSelector = '0x4e69d407'; // This might be wrong, let's show both
      const isCreateEscrow = functionSelector.toLowerCase() === expectedSelector.toLowerCase();
      
      // Show full transaction input to help debug
      const txInput = txData.result?.input || 'unknown';
      const inputData = txInput.length > 200 ? txInput.slice(0, 200) + '...' : txInput;
      
      // Show all log addresses to see where events came from
      const logAddresses = receipt.logs.map((log: any) => ({
        address: log.address,
        topic0: log.topics[0],
        isEscrow: log.address.toLowerCase() === escrowAddress.toLowerCase(),
      }));
      
      const resultText = `Chain: Base Sepolia (84532)
RPC: ${rpcUrl}
Escrow Contract: ${escrowAddress}
Expected Topic: ${eventTopic}

TX: ${txHash}
Block: ${blockNumber}
Status: ${receipt.status === '0x1' ? 'SUCCESS' : 'FAILED'}
Function Selector: ${functionSelector} ${isCreateEscrow ? '(createEscrow ✅)' : '(❌ NOT createEscrow!)'}
Expected Selector: ${expectedSelector}
Function Signature: ${createEscrowSig}

Transaction Input (first 200 chars):
${inputData}

Total logs: ${receipt.logs.length}
Escrow contract logs: ${escrowLogs.length}
Event topic match: ${hasEscrowEvent ? 'YES ✅' : 'NO ❌'}

${logDetails || 'No logs from escrow contract'}

All log addresses:
${logAddresses.map((log: any, i: number) => `${i + 1}. ${log.isEscrow ? 'ESCROW ✅' : 'OTHER'} ${log.address} topic0=${log.topic0}`).join('\n')}

${escrowInitializedElsewhere ? `⚠️ EscrowInitialized found in different contract: ${escrowInitializedElsewhere.address}` : ''}

${!hasEscrowEvent && !isCreateEscrow ? '⚠️ CRITICAL: Wrong function called! Expected createEscrow but got different function.' : ''}
${!hasEscrowEvent && isCreateEscrow ? '⚠️ CRITICAL: createEscrow called but EscrowInitialized event NOT emitted!' : ''}`;
      
      setResult(resultText);
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test14_CalculateEventTopic = async () => {
    setLoading(true);
    setResult(null);
    try {
      const eventSignature = 'EscrowInitialized(uint256,address,address,address,address,uint256,uint256)';
      const expectedTopic = '0x104303e46c846fc43f53cd6c4ab9ce96acdf68dcee176382e71fc812218a25a0';
      
      // Calculate function selector for createEscrow
      const functionSignature = 'createEscrow(uint256,address,uint256,address)';
      // Expected selector: first 4 bytes of keccak256(functionSignature)
      // We can't calculate keccak256 in browser easily, but we know:
      // - E2E test uses Hardhat which gets ABI from compiled contract
      // - Frontend uses manual ABI
      
      // Check what selector viem would generate
      const { encodeFunctionData } = await import('viem');
      const escrowAddress = getEscrowContractAddress('base-sepolia');
      const testIntentId = BigInt('0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef');
      const testToken = '0x036CbD53842c5426634e7929541eC2318f3dCF7e' as `0x${string}`;
      const testAmount = BigInt(1000);
      const testSolver = '0xb79c5a86c2612ed298132a559516ca8fdb316bd1' as `0x${string}`;
      
      const encoded = encodeFunctionData({
        abi: INTENT_ESCROW_ABI,
        functionName: 'createEscrow',
        args: [testIntentId, testToken, testAmount, testSolver],
      });
      
      const calculatedSelector = encoded.slice(0, 10); // First 4 bytes (0x + 8 hex chars)
      const actualSelector = '0x71fbf713';
      const expectedSelector = '0x4e69d407'; // This might be wrong
      
      setResult(`Event signature: "${eventSignature}"\nExpected topic: ${expectedTopic}\n\nFunction signature: "${functionSignature}"\nCalculated selector (viem): ${calculatedSelector}\nActual selector (from TX): ${actualSelector}\nExpected selector: ${expectedSelector}\n\n${calculatedSelector.toLowerCase() === actualSelector.toLowerCase() ? '✅ Selectors match!' : '❌ Selectors DO NOT match!'}\n\nIf selectors match, the ABI is correct but contract might be different.\nIf selectors don't match, the ABI is wrong.`);
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  const test15_QueryEthGetLogs = async () => {
    setLoading(true);
    setResult(null);
    try {
      const escrowAddress = getEscrowContractAddress('base-sepolia');
      const eventTopic = '0x104303e46c846fc43f53cd6c4ab9ce96acdf68dcee176382e71fc812218a25a0';
      
      // Query last 1000 blocks (same as solver)
      const rpcUrl = 'https://sepolia.base.org';
      
      // First get current block
      const blockResponse = await fetch(rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 1,
          method: 'eth_blockNumber',
          params: [],
        }),
      });
      
      const blockData = await blockResponse.json();
      const currentBlock = parseInt(blockData.result, 16);
      const fromBlock = Math.max(0, currentBlock - 1000);
      
      console.log('Querying eth_getLogs:', { escrowAddress, eventTopic, fromBlock, currentBlock });
      
      const logsResponse = await fetch(rpcUrl, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          jsonrpc: '2.0',
          id: 2,
          method: 'eth_getLogs',
          params: [{
            address: escrowAddress,
            fromBlock: '0x' + fromBlock.toString(16),
            toBlock: 'latest',
            topics: [eventTopic],
          }],
        }),
      });
      
      const logsData = await logsResponse.json();
      if (logsData.error) {
        setResult(`FAIL: ${logsData.error.message || JSON.stringify(logsData.error)}`);
        return;
      }
      
      const logs = logsData.result || [];
      setResult(`PASS: Chain: Base Sepolia (84532)\nRPC: https://sepolia.base.org\nEscrow Contract: ${escrowAddress}\nEvent Topic: ${eventTopic}\nFound ${logs.length} EscrowInitialized event(s) in blocks ${fromBlock}-${currentBlock}. ${logs.length > 0 ? 'First log: ' + JSON.stringify(logs[0]).slice(0, 200) : ''}`);
    } catch (err) {
      setResult(`FAIL: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setLoading(false);
    }
  };

  // Show tx hash when available
  const txStatus = txHash ? `Last TX Hash: ${txHash}` : writeError ? `Error: ${writeError.message}` : null;

  return (
    <div className="border border-gray-700 rounded-lg p-4 bg-gray-800/50">
      <h3 className="text-lg font-medium mb-4">Wallet & Chain Tests</h3>
      <p className="text-xs text-gray-400 mb-4">
        Debug tests for wallet connection and chain interactions:
      </p>
      
      <div className="grid grid-cols-1 gap-2 mb-4">
        <button
          onClick={test0_WhoAmI}
          disabled={loading}
          className="px-3 py-2 bg-blue-700 hover:bg-blue-600 rounded text-xs text-left disabled:opacity-50 font-medium"
        >
          WhoAmI: Show MVM & EVM Addresses
        </button>
        
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
        
        <button
          onClick={test7_SolverRegistry}
          disabled={loading}
          className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 7: Check Solver Registry (EVM addresses)
        </button>
        
        <button
          onClick={test8_VerifierSignature}
          disabled={loading}
          className="px-3 py-2 bg-gray-700 hover:bg-gray-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 8: Query Verifier Signature (solver_evm_addr)
        </button>
        
        <div className="border-t border-gray-600 pt-2 mt-2">
          <p className="text-xs text-gray-500 mb-2">EVM (MetaMask) Tests:</p>
        </div>
        
        <button
          onClick={test9_EvmConnection}
          disabled={loading}
          className="px-3 py-2 bg-blue-700 hover:bg-blue-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 9: Check EVM Wallet Connection
        </button>
        
        <button
          onClick={test10_SwitchToBaseSepolia}
          disabled={loading || isSwitching}
          className="px-3 py-2 bg-blue-700 hover:bg-blue-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 10: Switch to Base Sepolia
        </button>
        
        <button
          onClick={test11_EvmTokenApprove}
          disabled={loading || isWritePending}
          className="px-3 py-2 bg-blue-700 hover:bg-blue-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 11: Approve USDC (max allowance)
        </button>
        
        <button
          onClick={test12_EvmEscrowCreate}
          disabled={loading || isWritePending}
          className="px-3 py-2 bg-blue-700 hover:bg-blue-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 12: Create Test Escrow (0.001 USDC)
        </button>
        
        <div className="border-t border-gray-600 pt-2 mt-2">
          <p className="text-xs text-gray-500 mb-2">Escrow Debug Tests:</p>
        </div>
        
        <button
          onClick={test13_QueryEscrowTxReceipt}
          disabled={loading}
          className="px-3 py-2 bg-purple-700 hover:bg-purple-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 13: Query Escrow TX Receipt (check events)
        </button>
        
        <button
          onClick={test14_CalculateEventTopic}
          disabled={loading}
          className="px-3 py-2 bg-purple-700 hover:bg-purple-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 14: Show Event Topic Hash
        </button>
        
        <button
          onClick={test15_QueryEthGetLogs}
          disabled={loading}
          className="px-3 py-2 bg-purple-700 hover:bg-purple-600 rounded text-xs text-left disabled:opacity-50"
        >
          Test 15: Query eth_getLogs (solver's filter)
        </button>
      </div>
      
      {txStatus && (
        <div className="p-2 mb-2 bg-gray-700/50 rounded text-xs font-mono break-all text-gray-300">
          {txStatus}
        </div>
      )}
      
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
