const { ethers } = require("hardhat");

// Chain IDs
const HUB_CHAIN_ID = 30325; // Movement mainnet
const TRUSTED_HUB_ADDR = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

// Default test values
const DEFAULT_AMOUNT = BigInt(1000000);
const DEFAULT_EXPIRY_OFFSET = 3600; // 1 hour from now

/// Shared test setup for IntentInflowEscrow GMP tests
/// Provides common fixtures and helper functions
async function setupInflowEscrowGmpTests() {
  const [admin, requester, solver, relay] = await ethers.getSigners();

  // Deploy mock ERC20 token
  const MockERC20 = await ethers.getContractFactory("MockERC20");
  const token = await MockERC20.deploy("Test Token", "TEST", 18);
  await token.waitForDeployment();

  // Deploy GMP endpoint
  const IntentGmp = await ethers.getContractFactory("IntentGmp");
  const gmpEndpoint = await IntentGmp.deploy(admin.address);
  await gmpEndpoint.waitForDeployment();

  // Deploy inflow escrow
  const IntentInflowEscrow = await ethers.getContractFactory("IntentInflowEscrow");
  const escrow = await IntentInflowEscrow.deploy(
    admin.address,
    gmpEndpoint.target,
    HUB_CHAIN_ID,
    TRUSTED_HUB_ADDR
  );
  await escrow.waitForDeployment();

  // Configure GMP endpoint
  await gmpEndpoint.setEscrowHandler(escrow.target);
  await gmpEndpoint.setTrustedRemote(HUB_CHAIN_ID, TRUSTED_HUB_ADDR);

  // Mint tokens to requester
  await token.mint(requester.address, DEFAULT_AMOUNT * 100n);
  await token.connect(requester).approve(escrow.target, DEFAULT_AMOUNT * 100n);

  // Generate a default intent ID
  const intentId = "0xaa000000000000000000000000000000000000000000000000000000000000bb";

  return {
    escrow,
    gmpEndpoint,
    token,
    admin,
    requester,
    solver,
    relay,
    intentId,
    hubChainId: HUB_CHAIN_ID,
    trustedHubAddr: TRUSTED_HUB_ADDR,
    defaultAmount: DEFAULT_AMOUNT
  };
}

/// Helper function to advance blockchain time for expiry testing
/// Uses Hardhat's evm_increaseTime to simulate time passage
/// @param seconds Number of seconds to advance
async function advanceTime(seconds) {
  await ethers.provider.send("evm_increaseTime", [seconds]);
  await ethers.provider.send("evm_mine", []);
}

/// Convert EVM address to bytes32 (left-padded)
function addressToBytes32(addr) {
  return ethers.zeroPadValue(addr, 32);
}

/// Convert bytes32 to EVM address (extract last 20 bytes)
function bytes32ToAddress(bytes32) {
  return ethers.getAddress("0x" + bytes32.slice(-40));
}

/// Encode IntentRequirements message
async function encodeIntentRequirements(intentId, requesterAddr, amount, tokenAddr, solverAddr, expiry) {
  const MessagesHarness = await ethers.getContractFactory("MessagesHarness");
  const harness = await MessagesHarness.deploy();
  return harness.encodeIntentRequirements(
    intentId,
    requesterAddr,
    amount,
    tokenAddr,
    solverAddr,
    expiry
  );
}

/// Encode FulfillmentProof message
async function encodeFulfillmentProof(intentId, solverAddr, amount, timestamp) {
  const MessagesHarness = await ethers.getContractFactory("MessagesHarness");
  const harness = await MessagesHarness.deploy();
  return harness.encodeFulfillmentProof(intentId, solverAddr, amount, timestamp);
}

/// Helper to deliver IntentRequirements via GMP
async function deliverRequirements(gmpEndpoint, intentId, requesterAddr, amount, tokenAddr, solverAddr, expiry) {
  const payload = await encodeIntentRequirements(
    intentId,
    requesterAddr,
    amount,
    tokenAddr,
    solverAddr,
    expiry
  );
  await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload);
}

/// Helper to deliver FulfillmentProof via GMP
/// Returns the transaction for event emission checking
async function deliverFulfillmentProof(gmpEndpoint, intentId, solverAddr, amount = DEFAULT_AMOUNT, timestamp = null) {
  if (timestamp === null) {
    const block = await ethers.provider.getBlock("latest");
    timestamp = BigInt(block.timestamp);
  }
  const payload = await encodeFulfillmentProof(intentId, solverAddr, amount, timestamp);
  return gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload);
}

/// Get current block timestamp
async function getCurrentTimestamp() {
  const block = await ethers.provider.getBlock("latest");
  return BigInt(block.timestamp);
}

/// Get expiry timestamp (current time + offset)
async function getExpiryTimestamp(offsetSeconds = DEFAULT_EXPIRY_OFFSET) {
  const current = await getCurrentTimestamp();
  return current + BigInt(offsetSeconds);
}

module.exports = {
  setupInflowEscrowGmpTests,
  advanceTime,
  addressToBytes32,
  bytes32ToAddress,
  encodeIntentRequirements,
  encodeFulfillmentProof,
  deliverRequirements,
  deliverFulfillmentProof,
  getCurrentTimestamp,
  getExpiryTimestamp,
  HUB_CHAIN_ID,
  TRUSTED_HUB_ADDR,
  DEFAULT_AMOUNT,
  DEFAULT_EXPIRY_OFFSET
};
