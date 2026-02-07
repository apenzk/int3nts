const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("IntentOutflowValidator", function () {
  let outflowValidator;
  let gmpEndpoint;
  let token;
  let admin;
  let requester;
  let solver;

  // Chain IDs
  const HUB_CHAIN_ID = 30325; // Movement mainnet
  const TRUSTED_HUB_ADDR = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

  // Test values
  const INTENT_ID = "0xaa000000000000000000000000000000000000000000000000000000000000bb";
  const AMOUNT = BigInt(1000000);

  before(async function () {
    [admin, requester, solver] = await ethers.getSigners();
  });

  beforeEach(async function () {
    // Deploy mock token
    const MockERC20 = await ethers.getContractFactory("MockERC20");
    token = await MockERC20.deploy("Test Token", "TEST", 18);
    await token.waitForDeployment();

    // Deploy GMP endpoint
    const IntentGmp = await ethers.getContractFactory("IntentGmp");
    gmpEndpoint = await IntentGmp.deploy(admin.address);
    await gmpEndpoint.waitForDeployment();

    // Deploy outflow validator
    const IntentOutflowValidator = await ethers.getContractFactory("IntentOutflowValidator");
    outflowValidator = await IntentOutflowValidator.deploy(
      admin.address,
      gmpEndpoint.target,
      HUB_CHAIN_ID,
      TRUSTED_HUB_ADDR
    );
    await outflowValidator.waitForDeployment();

    // Configure GMP endpoint
    await gmpEndpoint.setOutflowHandler(outflowValidator.target);
    await gmpEndpoint.setTrustedRemote(HUB_CHAIN_ID, TRUSTED_HUB_ADDR);

    // Mint tokens to solver
    await token.mint(solver.address, AMOUNT * 10n);
    await token.connect(solver).approve(outflowValidator.target, AMOUNT * 10n);
  });

  // ============================================================================
  // Initialization
  // ============================================================================

  describe("Initialization", function () {
    /// 1. Test: test_initialize_creates_config: Initialize Creates Config
    /// Verifies outflow validator is initialized with correct config.
    /// Why: Configuration is required for GMP communication.
    it("should initialize with correct config", async function () {
      expect(await outflowValidator.gmpEndpoint()).to.equal(gmpEndpoint.target);
      expect(await outflowValidator.hubChainId()).to.equal(HUB_CHAIN_ID);
      expect(await outflowValidator.trustedHubAddr()).to.equal(TRUSTED_HUB_ADDR);
    });

    /// 2. Test: Initialize Rejects Double Init
    /// (N/A for Solidity - constructor runs once)
  });

  // ============================================================================
  // Receive Intent Requirements
  // ============================================================================

  describe("Receive Intent Requirements", function () {
    /// 3. Test: test_receive_stores_requirements: Receive Stores Requirements
    /// Verifies requirements are stored correctly.
    /// Why: Requirements are needed for fulfillment validation.
    it("should store received requirements", async function () {
      const tokenAddr32 = await tokenToBytes32(token.target);
      const requesterAddr32 = await addressToBytes32(requester.address);
      const solverAddr32 = await addressToBytes32(solver.address);
      const block = await ethers.provider.getBlock("latest");
      const expiry = BigInt(block.timestamp + 3600);

      const payload = await encodeIntentRequirements(
        INTENT_ID,
        requesterAddr32,
        AMOUNT,
        tokenAddr32,
        solverAddr32,
        expiry
      );

      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload, 1);

      expect(await outflowValidator.hasRequirements(INTENT_ID)).to.equal(true);
      const req = await outflowValidator.getRequirements(INTENT_ID);
      expect(req.amountRequired).to.equal(AMOUNT);
    });

    /// 4. Test: test_receive_idempotent: Receive Idempotent
    /// Verifies duplicate requirements are handled gracefully.
    /// Why: GMP may deliver same message multiple times.
    it("should handle duplicate requirements idempotently", async function () {
      const tokenAddr32 = await tokenToBytes32(token.target);
      const requesterAddr32 = await addressToBytes32(requester.address);
      const solverAddr32 = await addressToBytes32(solver.address);
      const block = await ethers.provider.getBlock("latest");
      const expiry = BigInt(block.timestamp + 3600);

      const payload = await encodeIntentRequirements(
        INTENT_ID,
        requesterAddr32,
        AMOUNT,
        tokenAddr32,
        solverAddr32,
        expiry
      );

      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload, 1);

      // Second delivery should emit duplicate event
      await expect(
        gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload, 2)
      ).to.emit(outflowValidator, "IntentRequirementsDuplicate");
    });

    /// 5. Test: test_receive_rejects_untrusted_source: Receive Rejects Untrusted Source
    /// Verifies requirements from untrusted source are rejected.
    /// Why: Only trusted hub should send requirements.
    it("should reject requirements from untrusted source", async function () {
      const untrustedAddr = "0x9900000000000000000000000000000000000000000000000000000000000099";
      await gmpEndpoint.addTrustedRemote(HUB_CHAIN_ID, untrustedAddr);

      const payload = "0x01" + "00".repeat(144);

      await expect(
        gmpEndpoint.deliverMessage(HUB_CHAIN_ID, untrustedAddr, payload, 1)
      ).to.be.revertedWithCustomError(outflowValidator, "E_INVALID_SOURCE_ADDRESS");
    });

    /// 6. Test: Receive Rejects Invalid Payload
    /// (Covered by GmpTypes decode validation)
  });

  // ============================================================================
  // Fulfill Intent
  // ============================================================================

  describe("Fulfill Intent", function () {
    let tokenAddr32;
    let requesterAddr32;
    let solverAddr32;
    let expiry;

    beforeEach(async function () {
      tokenAddr32 = await tokenToBytes32(token.target);
      requesterAddr32 = await addressToBytes32(requester.address);
      solverAddr32 = await addressToBytes32(solver.address);

      // Get current block timestamp and add 1 hour
      const block = await ethers.provider.getBlock("latest");
      expiry = BigInt(block.timestamp + 3600);

      // Deliver requirements
      const payload = await encodeIntentRequirements(
        INTENT_ID,
        requesterAddr32,
        AMOUNT,
        tokenAddr32,
        solverAddr32,
        expiry
      );
      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload, 1);
    });

    /// 7. Test: test_fulfill_intent_rejects_already_fulfilled: Fulfill Rejects Already Fulfilled
    /// Verifies double fulfillment is rejected.
    /// Why: Prevents double payment.
    it("should reject already fulfilled intent", async function () {
      await outflowValidator.connect(solver).fulfillIntent(INTENT_ID, token.target);

      await expect(
        outflowValidator.connect(solver).fulfillIntent(INTENT_ID, token.target)
      ).to.be.revertedWithCustomError(outflowValidator, "E_ALREADY_FULFILLED");
    });

    /// 8. Test: test_fulfill_intent_rejects_expired: Fulfill Rejects Expired
    /// Verifies expired intents cannot be fulfilled.
    /// Why: Expired intents should not be fulfilled.
    it("should reject expired intent", async function () {
      // Create new intent with past expiry
      const pastExpiry = BigInt(Math.floor(Date.now() / 1000) - 3600);
      const newIntentId = "0xee000000000000000000000000000000000000000000000000000000000000ff";

      const payload = await encodeIntentRequirements(
        newIntentId,
        requesterAddr32,
        AMOUNT,
        tokenAddr32,
        solverAddr32,
        pastExpiry
      );
      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload, 2);

      await expect(
        outflowValidator.connect(solver).fulfillIntent(newIntentId, token.target)
      ).to.be.revertedWithCustomError(outflowValidator, "E_INTENT_EXPIRED");
    });

    /// 9. Test: test_fulfill_intent_rejects_unauthorized_solver: Fulfill Rejects Unauthorized Solver
    /// Verifies only authorized solver can fulfill.
    /// Why: Security - only designated solver should fulfill.
    it("should reject unauthorized solver", async function () {
      await token.mint(admin.address, AMOUNT);
      await token.connect(admin).approve(outflowValidator.target, AMOUNT);

      await expect(
        outflowValidator.connect(admin).fulfillIntent(INTENT_ID, token.target)
      ).to.be.revertedWithCustomError(outflowValidator, "E_UNAUTHORIZED_SOLVER");
    });

    /// 10. Test: test_fulfill_intent_rejects_token_mismatch: Fulfill Rejects Token Mismatch
    /// Verifies fulfillment fails if token doesn't match.
    /// Why: Token must match requirements.
    it("should reject token mismatch", async function () {
      const MockERC20 = await ethers.getContractFactory("MockERC20");
      const wrongToken = await MockERC20.deploy("Wrong", "WRONG", 18);
      await wrongToken.mint(solver.address, AMOUNT);
      await wrongToken.connect(solver).approve(outflowValidator.target, AMOUNT);

      await expect(
        outflowValidator.connect(solver).fulfillIntent(INTENT_ID, wrongToken.target)
      ).to.be.revertedWithCustomError(outflowValidator, "E_TOKEN_MISMATCH");
    });

    /// 11. Test: test_fulfill_intent_rejects_requirements_not_found: Fulfill Rejects Requirements Not Found
    /// Verifies fulfillment fails if no requirements exist.
    /// Why: Requirements must be received first.
    it("should reject fulfillment without requirements", async function () {
      const unknownIntentId = "0xcc000000000000000000000000000000000000000000000000000000000000dd";

      await expect(
        outflowValidator.connect(solver).fulfillIntent(unknownIntentId, token.target)
      ).to.be.revertedWithCustomError(outflowValidator, "E_REQUIREMENTS_NOT_FOUND");
    });

    /// 12. Test: Fulfill Rejects Recipient Mismatch
    /// (N/A - recipient comes from requirements, not input)

    /// 13. Test: test_fulfill_intent_succeeds: Fulfill Intent Succeeds
    /// Verifies solver can fulfill intent successfully.
    /// Why: Core functionality - solver transfers tokens to requester.
    it("should fulfill intent successfully", async function () {
      await expect(
        outflowValidator.connect(solver).fulfillIntent(INTENT_ID, token.target)
      ).to.emit(outflowValidator, "FulfillmentSucceeded");

      expect(await outflowValidator.isFulfilled(INTENT_ID)).to.equal(true);
      expect(await token.balanceOf(requester.address)).to.equal(AMOUNT);
    });

    /// 14. Test: test_initialize_rejects_zero_endpoint: Initialize Rejects Zero Endpoint
    /// Verifies deployment fails with zero endpoint address.
    /// Why: GMP endpoint is required.
    it("should reject zero GMP endpoint", async function () {
      const IntentOutflowValidator = await ethers.getContractFactory("IntentOutflowValidator");
      await expect(
        IntentOutflowValidator.deploy(
          admin.address,
          ethers.ZeroAddress,
          HUB_CHAIN_ID,
          TRUSTED_HUB_ADDR
        )
      ).to.be.revertedWithCustomError(outflowValidator, "E_INVALID_ADDRESS");
    });

    /// 15. Test: test_allow_any_solver_zero_address: Allow Any Solver When Zero Address
    /// Verifies any solver can fulfill when solver_addr is zero.
    /// Why: Zero solver address means "any solver".
    it("should allow any solver when solver_addr is zero", async function () {
      // Create intent with zero solver address
      const zeroSolverAddr = "0x0000000000000000000000000000000000000000000000000000000000000000";
      const newIntentId = "0xdd000000000000000000000000000000000000000000000000000000000000ee";

      const payload = await encodeIntentRequirements(
        newIntentId,
        requesterAddr32,
        AMOUNT,
        tokenAddr32,
        zeroSolverAddr,
        expiry
      );
      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload, 2);

      // Admin (not the designated solver) should be able to fulfill
      await token.mint(admin.address, AMOUNT);
      await token.connect(admin).approve(outflowValidator.target, AMOUNT);

      await expect(
        outflowValidator.connect(admin).fulfillIntent(newIntentId, token.target)
      ).to.emit(outflowValidator, "FulfillmentSucceeded");
    });

    /// 16. Test: test_send_fulfillment_proof_to_hub: Send Fulfillment Proof to Hub
    /// Verifies FulfillmentProof is sent to hub.
    /// Why: Hub needs proof to release escrowed funds.
    it("should send fulfillment proof to hub", async function () {
      await expect(
        outflowValidator.connect(solver).fulfillIntent(INTENT_ID, token.target)
      ).to.emit(outflowValidator, "FulfillmentProofSent");
    });

    /// 17. Test: test_tokens_transferred_to_requester: Tokens Transferred to Requester
    /// Verifies tokens are transferred to requester (recipient).
    /// Why: Requester should receive tokens.
    it("should transfer tokens to requester", async function () {
      const balanceBefore = await token.balanceOf(requester.address);

      await outflowValidator.connect(solver).fulfillIntent(INTENT_ID, token.target);

      const balanceAfter = await token.balanceOf(requester.address);
      expect(balanceAfter - balanceBefore).to.equal(AMOUNT);
    });
  });

  // ============================================================================
  // Full Workflow
  // ============================================================================

  describe("Full Workflow", function () {
    /// 18. Test: test_complete_outflow_workflow: Complete Outflow Workflow
    /// Verifies complete flow from requirements to fulfillment.
    /// Why: End-to-end validation of the entire process.
    it("should complete full outflow workflow", async function () {
      const tokenAddr32 = await tokenToBytes32(token.target);
      const requesterAddr32 = await addressToBytes32(requester.address);
      const solverAddr32 = await addressToBytes32(solver.address);
      const block = await ethers.provider.getBlock("latest");
      const expiry = BigInt(block.timestamp + 3600);

      // 1. Receive requirements from hub
      const reqPayload = await encodeIntentRequirements(
        INTENT_ID,
        requesterAddr32,
        AMOUNT,
        tokenAddr32,
        solverAddr32,
        expiry
      );
      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, reqPayload, 1);
      expect(await outflowValidator.hasRequirements(INTENT_ID)).to.equal(true);

      // 2. Solver fulfills intent
      await outflowValidator.connect(solver).fulfillIntent(INTENT_ID, token.target);

      // 3. Verify final state
      expect(await outflowValidator.isFulfilled(INTENT_ID)).to.equal(true);
      expect(await token.balanceOf(requester.address)).to.equal(AMOUNT);
    });
  });

  // ============================================================================
  // Helper Functions
  // ============================================================================

  async function addressToBytes32(addr) {
    return ethers.zeroPadValue(addr, 32);
  }

  async function tokenToBytes32(addr) {
    return ethers.zeroPadValue(addr, 32);
  }

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
});
