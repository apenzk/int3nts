//! Outflow intent fulfillment script
//!
//! Called by the solver to fulfill an outflow intent on the EVM connected chain.
//! Approves the outflow validator to spend solver's tokens, then calls fulfillIntent.
//!
//! The outflow validator then sends a FulfillmentProof via GMP to the hub.

const hre = require("hardhat");

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
  const outflowValidatorAddr = process.env.OUTFLOW_VALIDATOR_ADDR;
  const tokenAddr = process.env.TOKEN_ADDR;
  const intentId = process.env.INTENT_ID;

  if (!outflowValidatorAddr || !tokenAddr || !intentId) {
    const error = new Error(
      "Missing required environment variables: OUTFLOW_VALIDATOR_ADDR, TOKEN_ADDR, INTENT_ID"
    );
    console.error("Error:", error.message);
    if (require.main === module) {
      process.exit(1);
    }
    throw error;
  }

  // Ensure intentId is properly formatted as bytes32
  let intentIdBytes32 = intentId;
  if (!intentIdBytes32.startsWith("0x")) {
    intentIdBytes32 = "0x" + intentIdBytes32;
  }
  intentIdBytes32 = "0x" + intentIdBytes32.slice(2).padStart(64, "0");

  // Extract 20-byte EVM address from potentially 32-byte padded format
  // 32-byte format: 0x000000000000000000000000<20-byte-address>
  // 20-byte format: 0x<20-byte-address>
  let evmTokenAddr = tokenAddr;
  if (tokenAddr.length === 66) {
    evmTokenAddr = "0x" + tokenAddr.slice(-40);
  }

  // signers[2] is the solver account (Hardhat account #2)
  const signers = await hre.ethers.getSigners();
  const solver = signers[2];

  console.log(`Solver address: ${solver.address}`);
  console.log(`Outflow validator: ${outflowValidatorAddr}`);
  console.log(`Token: ${evmTokenAddr}`);
  console.log(`Intent ID: ${intentIdBytes32}`);

  // Attach to the IntentOutflowValidator contract
  const IntentOutflowValidator = await hre.ethers.getContractFactory(
    "IntentOutflowValidator"
  );
  const outflowValidator = IntentOutflowValidator.attach(
    outflowValidatorAddr
  ).connect(solver);

  // Read requirements to get amount
  const req = await outflowValidator.requirements(intentIdBytes32);
  const amount = req.amountRequired;
  console.log(`Amount required: ${amount}`);

  if (amount == 0) {
    throw new Error("Amount required is 0 - requirements may not exist");
  }

  // Approve outflow validator to spend solver's tokens
  const IERC20 = await hre.ethers.getContractAt("IERC20", evmTokenAddr, solver);
  const approveTx = await IERC20.approve(outflowValidatorAddr, amount);
  await approveTx.wait();
  console.log(`Approval tx: ${approveTx.hash}`);

  // Call fulfillIntent(intentId, token)
  const fulfillTx = await outflowValidator.fulfillIntent(
    intentIdBytes32,
    evmTokenAddr
  );
  const receipt = await fulfillTx.wait();
  console.log(`Transaction hash: ${fulfillTx.hash}`);
  console.log(`Block number: ${receipt.blockNumber}`);
}

if (require.main === module) {
  main()
    .then(() => process.exit(0))
    .catch((error) => {
      console.error("Error:", error.message);
      process.exit(1);
    });
}

module.exports = { main };
