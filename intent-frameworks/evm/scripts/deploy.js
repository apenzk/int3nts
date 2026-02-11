//! GMP contract deployment utility
//!
//! This script deploys all GMP-related contracts: IntentGmp, IntentInflowEscrow, IntentOutflowValidator.
//! Configures trusted remotes and message routing for cross-chain communication.

const hre = require("hardhat");

/// Deploys all GMP contracts and configures routing
///
/// # Environment Variables
/// - `HUB_CHAIN_ID`: Hub chain endpoint ID (default: 250 for Movement Bardock)
/// - `MOVEMENT_INTENT_MODULE_ADDR`: Movement intent module address in hex format (required)
/// - `RELAY_ADDRESS`: Optional relay address to authorize (defaults to deployer)
///
/// # Returns
/// Outputs deployed contract addresses and configuration status.
async function main() {
  console.log("Deploying GMP Contracts...");
  console.log("==========================");

  // Get signers
  const [deployer] = await hre.ethers.getSigners();
  console.log("Deploying with account:", deployer.address);

  // Configuration
  const hubChainId = parseInt(process.env.HUB_CHAIN_ID || "250");
  const movementModuleAddrHex = process.env.MOVEMENT_INTENT_MODULE_ADDR;
  const relayAddress = process.env.RELAY_ADDRESS || deployer.address;

  if (!movementModuleAddrHex) {
    throw new Error("MOVEMENT_INTENT_MODULE_ADDR environment variable required (hex, 0x-prefixed)");
  }

  // Convert Movement module address to bytes32
  let movementModuleAddr = movementModuleAddrHex;
  if (!movementModuleAddr.startsWith("0x")) {
    movementModuleAddr = "0x" + movementModuleAddr;
  }
  // Pad to 64 hex characters (32 bytes)
  movementModuleAddr = "0x" + movementModuleAddr.slice(2).padStart(64, '0');

  console.log("\nConfiguration:");
  console.log("  Hub Chain ID:", hubChainId);
  console.log("  Movement Intent Module:", movementModuleAddr);
  console.log("  Relay Address:", relayAddress);

  // Deploy IntentGmp
  console.log("\n1. Deploying IntentGmp...");
  const IntentGmp = await hre.ethers.getContractFactory("IntentGmp");
  const gmpEndpoint = await IntentGmp.deploy(deployer.address);
  await gmpEndpoint.waitForDeployment();
  await gmpEndpoint.deploymentTransaction().wait(1);
  const gmpEndpointAddress = await gmpEndpoint.getAddress();
  console.log("   IntentGmp deployed to:", gmpEndpointAddress);

  // Deploy IntentInflowEscrow
  console.log("\n2. Deploying IntentInflowEscrow...");
  const IntentInflowEscrow = await hre.ethers.getContractFactory("IntentInflowEscrow");
  const escrowGmp = await IntentInflowEscrow.deploy(
    deployer.address,
    gmpEndpointAddress,
    hubChainId,
    movementModuleAddr
  );
  await escrowGmp.waitForDeployment();
  await escrowGmp.deploymentTransaction().wait(1);
  const escrowGmpAddress = await escrowGmp.getAddress();
  console.log("   IntentInflowEscrow deployed to:", escrowGmpAddress);

  // Deploy IntentOutflowValidator
  console.log("\n3. Deploying IntentOutflowValidator...");
  const IntentOutflowValidator = await hre.ethers.getContractFactory("IntentOutflowValidator");
  const outflowValidator = await IntentOutflowValidator.deploy(
    deployer.address,
    gmpEndpointAddress,
    hubChainId,
    movementModuleAddr
  );
  await outflowValidator.waitForDeployment();
  await outflowValidator.deploymentTransaction().wait(1);
  const outflowValidatorAddress = await outflowValidator.getAddress();
  console.log("   IntentOutflowValidator deployed to:", outflowValidatorAddress);

  // Configure GMP endpoint
  console.log("\n4. Configuring GMP endpoint...");

  // Set escrow handler
  console.log("   Setting escrow handler...");
  await gmpEndpoint.setEscrowHandler(escrowGmpAddress);
  console.log("   Escrow handler set to:", escrowGmpAddress);

  // Set outflow handler
  console.log("   Setting outflow handler...");
  await gmpEndpoint.setOutflowHandler(outflowValidatorAddress);
  console.log("   Outflow handler set to:", outflowValidatorAddress);

  // Set trusted remote for hub chain
  console.log("   Setting trusted remote for hub chain...");
  await gmpEndpoint.setTrustedRemote(hubChainId, movementModuleAddr);
  console.log("   Trusted remote set for chain", hubChainId);

  // Add relay if different from deployer
  if (relayAddress.toLowerCase() !== deployer.address.toLowerCase()) {
    console.log("   Adding authorized relay...");
    await gmpEndpoint.addRelay(relayAddress);
    console.log("   Relay added:", relayAddress);
  } else {
    console.log("   Deployer is already authorized as relay");
  }

  // Wait for RPC indexing
  console.log("\nWaiting for RPC indexing...");
  await new Promise(r => setTimeout(r, 3000));

  // Verify configuration
  console.log("\n5. Verifying configuration...");
  const escrowHandler = await gmpEndpoint.escrowHandler();
  const outflowHandler = await gmpEndpoint.outflowHandler();
  const isRelayAuthorized = await gmpEndpoint.isRelayAuthorized(relayAddress);
  const hasTrustedRemote = await gmpEndpoint.hasTrustedRemote(hubChainId);

  console.log("   Escrow handler:", escrowHandler);
  console.log("   Outflow handler:", outflowHandler);
  console.log("   Relay authorized:", isRelayAuthorized);
  console.log("   Has trusted remote for hub:", hasTrustedRemote);

  if (escrowHandler.toLowerCase() !== escrowGmpAddress.toLowerCase()) {
    throw new Error("Escrow handler mismatch!");
  }
  if (outflowHandler.toLowerCase() !== outflowValidatorAddress.toLowerCase()) {
    throw new Error("Outflow handler mismatch!");
  }

  // Summary
  console.log("\n========================================");
  console.log("GMP DEPLOYMENT SUCCESSFUL!");
  console.log("========================================");
  console.log("\nDeployed Contracts:");
  console.log("  IntentGmp:", gmpEndpointAddress);
  console.log("  IntentInflowEscrow:", escrowGmpAddress);
  console.log("  IntentOutflowValidator:", outflowValidatorAddress);
  console.log("\nConfiguration:");
  console.log("  Hub Chain ID:", hubChainId);
  console.log("  Movement Intent Module:", movementModuleAddr);
  console.log("  Relay Address:", relayAddress);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
