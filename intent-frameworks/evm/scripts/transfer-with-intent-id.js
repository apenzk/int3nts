//! ERC20 transfer with intent_id metadata
//!
//! This script executes an ERC20 transfer() call with intent_id appended in calldata.
//! The calldata format is: selector (4 bytes) + recipient (32 bytes) + amount (32 bytes) + intent_id (32 bytes).
//! The ERC20 contract ignores the extra intent_id bytes, but they remain in the transaction
//! data for approver tracking.

const hre = require("hardhat");
const { requireEnvVars, toEvmAddress, runMain } = require("./helpers");

/// Executes ERC20 transfer with intent_id in calldata
///
/// # Environment Variables
/// - `TOKEN_ADDR`: ERC20 token contract address
/// - `RECIPIENT`: Recipient address (20 bytes, EVM format)
/// - `AMOUNT`: Transfer amount in base units (wei for 18 decimals)
/// - `INTENT_ID`: Intent ID to append in calldata (32 bytes, hex format with 0x prefix)
///
/// # Returns
/// Outputs transaction hash, recipient, amount, and intent_id on success.
async function main() {
  const env = requireEnvVars(["TOKEN_ADDR", "RECIPIENT", "AMOUNT", "INTENT_ID"]);

  // Get solver signer
  // For in-memory Hardhat network (unit tests): use hre.ethers.getSigners()
  // For external networks (E2E tests): use raw ethers to avoid HardhatEthersProvider.resolveName bug
  let solver;

  if (hre.network.name === "hardhat") {
    // In-memory Hardhat network (unit tests) - getSigners() works fine here
    const signers = await hre.ethers.getSigners();
    if (signers.length < 3) {
      throw new Error(`Expected at least 3 signers, got ${signers.length}`);
    }
    solver = signers[2];
  } else if (process.env.SOLVER_EVM_PRIVATE_KEY) {
    // Testnet: Create wallet from private key using raw ethers
    const { ethers } = require("ethers");
    const rpcUrl = hre.network.config.url || "http://127.0.0.1:8545";
    const provider = new ethers.JsonRpcProvider(rpcUrl);
    solver = new ethers.Wallet(process.env.SOLVER_EVM_PRIVATE_KEY, provider);
  } else {
    // External network (E2E tests): use raw ethers to avoid resolveName bug
    const { ethers } = require("ethers");
    const rpcUrl = hre.network.config.url || "http://127.0.0.1:8545";
    const provider = new ethers.JsonRpcProvider(rpcUrl);
    const accounts = await provider.send("eth_accounts", []);
    if (accounts.length < 3) {
      throw new Error(`Expected at least 3 accounts from Hardhat node, got ${accounts.length}`);
    }
    solver = await provider.getSigner(accounts[2]);
  }

  const amountBigInt = BigInt(env.AMOUNT);
  const selector = "0xa9059cbb";

  const recipientClean = env.RECIPIENT.toLowerCase().replace(/^0x/, "");
  const intentIdClean = env.INTENT_ID.toLowerCase().replace(/^0x/, "");
  const recipientPadded = "0".repeat(24) + recipientClean;

  const amountHex = amountBigInt.toString(16);
  const amountPadded = "0".repeat(64 - amountHex.length) + amountHex;
  const intentIdPadded = intentIdClean.padStart(64, "0");

  const data = selector + recipientPadded + amountPadded + intentIdPadded;

  const evmTokenAddress = toEvmAddress(env.TOKEN_ADDR);

  const tx = await solver.sendTransaction({
    to: hre.ethers.getAddress(evmTokenAddress),
    data: data,
  });

  const receipt = await tx.wait();

  if (receipt.status === 1) {
    console.log("SUCCESS");
    console.log("Transaction hash:", receipt.hash);
    console.log("Recipient:", env.RECIPIENT);
    console.log("Amount:", env.AMOUNT);
    console.log("Intent ID:", env.INTENT_ID);
  } else {
    const error = new Error("Transaction failed");
    console.error("Error:", error.message);
    if (require.main === module) {
      process.exit(1);
    }
    throw error;
  }
}

// Export main function for testing
runMain(main, module);

module.exports = { main };
