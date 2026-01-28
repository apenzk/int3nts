//! IntentEscrow contract deployment utility
//!
//! This script deploys the IntentEscrow contract with a specified approver address.
//! If no approver address is provided via environment variable, uses the deployer account.

const hre = require("hardhat");

/// Deploys IntentEscrow contract with approver
///
/// # Environment Variables
/// - `APPROVER_ADDR`: Optional approver Ethereum address (defaults to deployer address)
///
/// # Returns
/// Outputs contract address, approver address, and deployment status on success.
async function main() {
  console.log("Deploying IntentEscrow...");

  // Get signers
  const [deployer] = await hre.ethers.getSigners();
  
  // Get approver address from environment variable or use deployer as fallback
  const approverAddress = process.env.APPROVER_ADDR;
  let approverAddr;
  
  if (approverAddress) {
    approverAddr = approverAddress;
    console.log("Using approver address from config:", approverAddr);
  } else {
    // Fallback to deployer as approver
    // Account 0 = deployer/approver, Account 1 = requester, Account 2 = solver
    approverAddr = deployer.address;
    console.log("Using deployer as approver:", approverAddr);
  }
  
  console.log("Deploying with account:", deployer.address);
  console.log("Approver address:", approverAddr);

  // Deploy escrow with approver address
  const IntentEscrow = await hre.ethers.getContractFactory("IntentEscrow");
  const escrow = await IntentEscrow.deploy(approverAddr);

  await escrow.waitForDeployment();

  const escrowAddress = await escrow.getAddress();
  console.log("IntentEscrow deployed to:", escrowAddress);
  console.log("Approver set to:", approverAddr);

  // Wait a moment for RPC indexing
  console.log("Waiting for RPC indexing...");
  await new Promise(r => setTimeout(r, 5000));

  // Verify deployment with retry
  let approverFromContract;
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      approverFromContract = await escrow.approver();
      break;
    } catch (err) {
      if (attempt === 3) {
        console.log("Warning: Could not verify contract state, but deployment succeeded.");
        console.log("\n✅ Deployment successful!");
        console.log("Contract address:", escrowAddress);
        process.exit(0);
      }
      console.log(`Retry ${attempt}/3...`);
      await new Promise(r => setTimeout(r, 3000));
    }
  }
  console.log("Approver from contract:", approverFromContract);
  
  if (approverFromContract.toLowerCase() !== approverAddr.toLowerCase()) {
    throw new Error("Approver address mismatch!");
  }

  console.log("\n✅ Deployment successful!");
  console.log("Contract address:", escrowAddress);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
