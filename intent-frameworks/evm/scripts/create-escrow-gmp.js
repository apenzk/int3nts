//! GMP-validated escrow creation utility
//!
//! This script creates an escrow on the IntentInflowEscrow contract.
//! Validates against IntentRequirements delivered via GMP.

const hre = require("hardhat");

/// Creates an escrow with GMP validation
///
/// # Environment Variables
/// - `ESCROW_GMP_ADDR`: IntentInflowEscrow contract address
/// - `TOKEN_ADDR`: ERC20 token address
/// - `INTENT_ID_EVM`: Intent ID in EVM format (bytes32, hex with 0x prefix)
/// - `AMOUNT`: Amount of tokens to lock in escrow (smallest unit, decimal string)
///
/// # Returns
/// Outputs success message with escrow details on success.
async function main() {
  const escrowGmpAddress = process.env.ESCROW_GMP_ADDR;
  const tokenAddress = process.env.TOKEN_ADDR;
  const intentIdHex = process.env.INTENT_ID_EVM;
  const amount = process.env.AMOUNT;

  if (!escrowGmpAddress || !tokenAddress || !intentIdHex || !amount) {
    throw new Error("Missing required environment variables: ESCROW_GMP_ADDR, TOKEN_ADDR, INTENT_ID_EVM, AMOUNT");
  }

  const signers = await hre.ethers.getSigners();
  const requester = signers[1]; // Requester is signer[1]

  console.log("Creating GMP-validated escrow...");
  console.log("  Escrow contract:", escrowGmpAddress);
  console.log("  Token:", tokenAddress);
  console.log("  Amount:", amount);
  console.log("  Requester:", requester.address);

  const escrowGmp = await hre.ethers.getContractAt("IntentInflowEscrow", escrowGmpAddress);
  const token = await hre.ethers.getContractAt("MockERC20", tokenAddress);

  // Ensure intentIdHex is properly formatted as bytes32
  let intentId = intentIdHex;
  if (!intentId.startsWith("0x")) {
    intentId = "0x" + intentId;
  }
  // Pad to 64 hex characters (32 bytes)
  intentId = "0x" + intentId.slice(2).padStart(64, '0');

  console.log("  Intent ID:", intentId);

  const amountBigInt = BigInt(amount);

  // Approve escrow contract to spend tokens
  console.log("\nApproving escrow contract to spend tokens...");
  const approveTx = await token.connect(requester).approve(escrowGmpAddress, amountBigInt);
  await approveTx.wait();
  console.log("  Approval confirmed");

  // Create escrow with GMP validation
  console.log("\nCreating escrow with validation...");
  const createTx = await escrowGmp.connect(requester).createEscrowWithValidation(
    intentId,
    tokenAddress,
    amountBigInt
  );
  const receipt = await createTx.wait();
  console.log("  Transaction hash:", receipt.hash);

  // Find EscrowCreated event
  const escrowCreatedEvent = receipt.logs.find(log => {
    try {
      const parsed = escrowGmp.interface.parseLog(log);
      return parsed && parsed.name === "EscrowCreated";
    } catch (err) {
      // parseLog throws for logs from other contracts (non-matching selectors) - expected
      // Log unexpected errors that aren't simple "no matching event" failures
      if (err.reason !== "no matching event" && !err.message?.includes("no matching event")) {
        console.error("Unexpected error parsing log:", err);
      }
      return false;
    }
  });

  if (escrowCreatedEvent) {
    const parsed = escrowGmp.interface.parseLog(escrowCreatedEvent);
    console.log("\nEscrow created successfully!");
    console.log("  Intent ID:", parsed.args.intentId);
    console.log("  Escrow ID:", parsed.args.escrowId);
    console.log("  Requester:", parsed.args.requester);
    console.log("  Amount:", parsed.args.amount.toString());
    console.log("  Token:", parsed.args.token);
    console.log("  Reserved Solver:", parsed.args.reservedSolver);
    console.log("  Expiry:", parsed.args.expiry.toString());
  } else {
    console.log("\nEscrow created for intent (GMP):", intentId);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
