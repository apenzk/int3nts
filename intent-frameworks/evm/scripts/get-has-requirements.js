//! GMP requirements query utility
//!
//! This script checks if IntentRequirements have been delivered via GMP.

const hre = require("hardhat");
const { requireEnvVars, toBytes32, runMain } = require("./helpers");

/// Checks if requirements exist for an intent
///
/// # Environment Variables
/// - `ESCROW_GMP_ADDR`: IntentInflowEscrow contract address
/// - `INTENT_ID_EVM`: Intent ID in EVM format (bytes32, hex with 0x prefix)
///
/// # Returns
/// Outputs "hasRequirements: true" or "hasRequirements: false" on success.
async function main() {
  const env = requireEnvVars(["ESCROW_GMP_ADDR", "INTENT_ID_EVM"]);

  const IntentInflowEscrow = await hre.ethers.getContractFactory("IntentInflowEscrow");
  const escrowGmp = IntentInflowEscrow.attach(env.ESCROW_GMP_ADDR);

  const intentId = toBytes32(env.INTENT_ID_EVM);

  const hasReq = await escrowGmp.hasRequirements(intentId);
  console.log(`hasRequirements: ${hasReq}`);
}

runMain(main, module);

module.exports = { main };
