//! GMP escrow release status query utility
//!
//! This script checks if an escrow has been auto-released via FulfillmentProof.

const hre = require("hardhat");
const { requireEnvVars, toBytes32, runMain } = require("./helpers");

/// Checks if escrow is released for an intent
///
/// # Environment Variables
/// - `ESCROW_GMP_ADDR`: IntentInflowEscrow contract address
/// - `INTENT_ID_EVM`: Intent ID in EVM format (bytes32, hex with 0x prefix)
///
/// # Returns
/// Outputs "isReleased: true" or "isReleased: false" on success.
async function main() {
  const env = requireEnvVars(["ESCROW_GMP_ADDR", "INTENT_ID_EVM"]);

  const IntentInflowEscrow = await hre.ethers.getContractFactory("IntentInflowEscrow");
  const escrowGmp = IntentInflowEscrow.attach(env.ESCROW_GMP_ADDR);

  const intentId = toBytes32(env.INTENT_ID_EVM);

  const isReleased = await escrowGmp.isReleased(intentId);
  console.log(`isReleased: ${isReleased}`);
}

runMain(main, module);

module.exports = { main };
