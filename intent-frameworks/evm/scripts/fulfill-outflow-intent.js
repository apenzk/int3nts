//! Outflow intent fulfillment script
//!
//! Called by the solver to fulfill an outflow intent on the EVM connected chain.
//! Approves the outflow validator to spend solver's tokens, then calls fulfillIntent.
//!
//! The outflow validator then sends a FulfillmentProof via GMP to the hub.

const hre = require("hardhat");
const { requireEnvVars, toBytes32, toEvmAddress, getSolverSigner, runMain } = require("./helpers");

/// Fulfill an outflow intent
///
/// # Environment Variables
/// - `OUTFLOW_VALIDATOR_ADDR`: IntentOutflowValidator contract address
/// - `TOKEN_ADDR`: ERC20 token address
/// - `INTENT_ID`: Intent ID (bytes32, hex with 0x prefix)
///
/// # Output
/// Outputs "Transaction hash: 0x..." on success.
async function main() {
  const env = requireEnvVars(["OUTFLOW_VALIDATOR_ADDR", "TOKEN_ADDR", "INTENT_ID"]);

  const intentIdBytes32 = toBytes32(env.INTENT_ID);
  const evmTokenAddr = toEvmAddress(env.TOKEN_ADDR);
  const solver = await getSolverSigner();

  console.log(`Solver address: ${solver.address}`);
  console.log(`Outflow validator: ${env.OUTFLOW_VALIDATOR_ADDR}`);
  console.log(`Token: ${evmTokenAddr}`);
  console.log(`Intent ID: ${intentIdBytes32}`);

  // Attach to the IntentOutflowValidator contract
  const IntentOutflowValidator = await hre.ethers.getContractFactory(
    "IntentOutflowValidator"
  );
  const outflowValidator = IntentOutflowValidator.attach(
    env.OUTFLOW_VALIDATOR_ADDR
  ).connect(solver);

  // Read requirements to get amount
  const req = await outflowValidator.requirements(intentIdBytes32);
  const amount = req.amountRequired;
  console.log(`Amount required: ${amount}`);

  if (amount == 0) {
    throw new Error("Amount required is 0 - requirements may not exist");
  }

  // Approve outflow validator to spend solver's tokens (skip if already approved)
  const IERC20 = await hre.ethers.getContractAt("IERC20", evmTokenAddr, solver);
  const currentAllowance = await IERC20.allowance(solver.address, env.OUTFLOW_VALIDATOR_ADDR);
  if (currentAllowance < amount) {
    const approveTx = await IERC20.approve(env.OUTFLOW_VALIDATOR_ADDR, amount);
    await approveTx.wait();
    console.log(`Approval tx: ${approveTx.hash}`);
  } else {
    console.log(`Allowance already sufficient: ${currentAllowance} >= ${amount}`);
  }

  // Call fulfillIntent(intentId, token)
  // Set gasLimit to skip estimateGas — avoids stale-state race after approval
  const fulfillTx = await outflowValidator.fulfillIntent(
    intentIdBytes32,
    evmTokenAddr,
    { gasLimit: 500_000 }
  );
  const receipt = await fulfillTx.wait();
  console.log(`Transaction hash: ${fulfillTx.hash}`);
  console.log(`Block number: ${receipt.blockNumber}`);
}

runMain(main, module);

module.exports = { main };
