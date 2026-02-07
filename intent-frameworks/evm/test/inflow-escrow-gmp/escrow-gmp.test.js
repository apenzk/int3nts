const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("IntentInflowEscrow", function () {
  let escrowGmp;
  let gmpEndpoint;
  let token;
  let admin;
  let requester;
  let solver;
  let relay;

  // Chain IDs
  const HUB_CHAIN_ID = 30325; // Movement mainnet
  const TRUSTED_HUB_ADDR = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

  // Test values
  const INTENT_ID = "0xaa000000000000000000000000000000000000000000000000000000000000bb";
  const AMOUNT = BigInt(1000000);

  before(async function () {
    [admin, requester, solver, relay] = await ethers.getSigners();
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

    // Deploy escrow GMP
    const IntentInflowEscrow = await ethers.getContractFactory("IntentInflowEscrow");
    escrowGmp = await IntentInflowEscrow.deploy(
      admin.address,
      gmpEndpoint.target,
      HUB_CHAIN_ID,
      TRUSTED_HUB_ADDR
    );
    await escrowGmp.waitForDeployment();

    // Configure GMP endpoint
    await gmpEndpoint.setEscrowHandler(escrowGmp.target);
    await gmpEndpoint.setTrustedRemote(HUB_CHAIN_ID, TRUSTED_HUB_ADDR);

    // Mint tokens to requester
    await token.mint(requester.address, AMOUNT * 10n);
    await token.connect(requester).approve(escrowGmp.target, AMOUNT * 10n);
  });

  // ============================================================================
  // Initialization
  // ============================================================================

  describe("Initialization", function () {
    /// 1. Test: test_initialize_creates_config: Initialize Creates Config
    /// Verifies escrow GMP is initialized with correct config.
    /// Why: Configuration is required for GMP communication.
    it("should initialize with correct config", async function () {
      expect(await escrowGmp.gmpEndpoint()).to.equal(gmpEndpoint.target);
      expect(await escrowGmp.hubChainId()).to.equal(HUB_CHAIN_ID);
      expect(await escrowGmp.trustedHubAddr()).to.equal(TRUSTED_HUB_ADDR);
    });

    /// 2. Test: test_initialize_rejects_zero_endpoint: Initialize Rejects Zero Endpoint
    /// Verifies deployment fails with zero endpoint address.
    /// Why: GMP endpoint is required.
    it("should reject zero GMP endpoint", async function () {
      const IntentInflowEscrow = await ethers.getContractFactory("IntentInflowEscrow");
      await expect(
        IntentInflowEscrow.deploy(
          admin.address,
          ethers.ZeroAddress,
          HUB_CHAIN_ID,
          TRUSTED_HUB_ADDR
        )
      ).to.be.revertedWithCustomError(escrowGmp, "E_INVALID_ADDRESS");
    });
  });

  // ============================================================================
  // Receive Intent Requirements
  // ============================================================================

  describe("Receive Intent Requirements", function () {
    /// 3. Test: test_receive_requirements_stores_requirements: Receive Requirements Stores Requirements
    /// Verifies requirements are stored correctly.
    /// Why: Requirements are needed for escrow validation.
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

      expect(await escrowGmp.hasRequirements(INTENT_ID)).to.equal(true);
      const req = await escrowGmp.getRequirements(INTENT_ID);
      expect(req.amountRequired).to.equal(AMOUNT);
    });

    /// 4. Test: test_receive_requirements_idempotent: Receive Requirements Idempotent
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
      ).to.emit(escrowGmp, "IntentRequirementsDuplicate");
    });

    /// 5. Test: test_receive_requirements_rejects_untrusted_source: Receive Requirements Rejects Untrusted Source
    /// Verifies requirements from untrusted source are rejected.
    /// Why: Only trusted hub should send requirements.
    it("should reject requirements from untrusted source", async function () {
      const untrustedAddr = "0x9900000000000000000000000000000000000000000000000000000000000099";
      await gmpEndpoint.addTrustedRemote(HUB_CHAIN_ID, untrustedAddr);

      const payload = "0x01" + "00".repeat(144);

      await expect(
        gmpEndpoint.deliverMessage(HUB_CHAIN_ID, untrustedAddr, payload, 1)
      ).to.be.revertedWithCustomError(escrowGmp, "E_INVALID_SOURCE_ADDRESS");
    });
  });

  // ============================================================================
  // Receive Fulfillment Proof
  // ============================================================================

  describe("Receive Fulfillment Proof", function () {
    let tokenAddr32;
    let requesterAddr32;
    let solverAddr32;
    let expiry;

    beforeEach(async function () {
      tokenAddr32 = await tokenToBytes32(token.target);
      requesterAddr32 = await addressToBytes32(requester.address);
      solverAddr32 = await addressToBytes32(solver.address);
      const block = await ethers.provider.getBlock("latest");
      expiry = BigInt(block.timestamp + 3600);

      // Deliver requirements
      const reqPayload = await encodeIntentRequirements(
        INTENT_ID,
        requesterAddr32,
        AMOUNT,
        tokenAddr32,
        solverAddr32,
        expiry
      );
      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, reqPayload, 1);

      // Create escrow
      await escrowGmp.connect(requester).createEscrowWithValidation(
        INTENT_ID,
        token.target,
        AMOUNT
      );
    });

    /// 6. Test: test_receive_fulfillment_proof_releases_escrow: Receive Fulfillment Proof Releases Escrow
    /// Verifies fulfillment proof triggers auto-release.
    /// Why: Solver should receive tokens when hub confirms fulfillment.
    it("should release escrow on fulfillment proof", async function () {
      const block = await ethers.provider.getBlock("latest");
      const timestamp = BigInt(block.timestamp);

      const proofPayload = await encodeFulfillmentProof(
        INTENT_ID,
        solverAddr32,
        AMOUNT,
        timestamp
      );

      // Deliver fulfillment proof
      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, proofPayload, 2);

      // Check escrow state
      expect(await escrowGmp.isFulfilled(INTENT_ID)).to.equal(true);
      expect(await escrowGmp.isReleased(INTENT_ID)).to.equal(true);

      // Check solver received tokens
      expect(await token.balanceOf(solver.address)).to.equal(AMOUNT);
    });

    /// 7. Test: test_receive_fulfillment_rejects_untrusted_source: Receive Fulfillment Rejects Untrusted Source
    /// Verifies fulfillment from untrusted source is rejected.
    /// Why: Only trusted hub should send fulfillment proofs.
    it("should reject fulfillment from untrusted source", async function () {
      const untrustedAddr = "0x9900000000000000000000000000000000000000000000000000000000000099";
      await gmpEndpoint.addTrustedRemote(HUB_CHAIN_ID, untrustedAddr);

      const proofPayload = "0x03" + "00".repeat(80);

      await expect(
        gmpEndpoint.deliverMessage(HUB_CHAIN_ID, untrustedAddr, proofPayload, 2)
      ).to.be.revertedWithCustomError(escrowGmp, "E_INVALID_SOURCE_ADDRESS");
    });

    /// 8. Test: test_receive_fulfillment_proof_rejects_already_fulfilled: Receive Fulfillment Proof Rejects Already Fulfilled
    /// Verifies double fulfillment is rejected.
    /// Why: Prevents double-release of tokens.
    it("should reject double fulfillment", async function () {
      const block = await ethers.provider.getBlock("latest");
      const timestamp = BigInt(block.timestamp);

      const proofPayload = await encodeFulfillmentProof(
        INTENT_ID,
        solverAddr32,
        AMOUNT,
        timestamp
      );

      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, proofPayload, 2);

      await expect(
        gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, proofPayload, 3)
      ).to.be.revertedWithCustomError(escrowGmp, "E_ALREADY_RELEASED");
    });
  });

  // ============================================================================
  // Create Escrow
  // ============================================================================

  describe("Create Escrow", function () {
    let tokenAddr32;
    let requesterAddr32;
    let solverAddr32;
    let expiry;

    beforeEach(async function () {
      tokenAddr32 = await tokenToBytes32(token.target);
      requesterAddr32 = await addressToBytes32(requester.address);
      solverAddr32 = await addressToBytes32(solver.address);
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

    /// 9. Test: test_create_escrow_validates_against_requirements: Create Escrow Validates Against Requirements
    /// Verifies escrow creation validates all fields.
    /// Why: Escrow must match hub requirements.
    it("should create escrow when validated", async function () {
      await expect(
        escrowGmp.connect(requester).createEscrowWithValidation(
          INTENT_ID,
          token.target,
          AMOUNT
        )
      ).to.emit(escrowGmp, "EscrowCreated");

      expect(await escrowGmp.hasEscrow(INTENT_ID)).to.equal(true);
    });

    /// 10. Test: test_create_escrow_rejects_amount_mismatch: Create Escrow Rejects Amount Mismatch
    /// Verifies escrow creation fails if amount doesn't match.
    /// Why: Amount must match requirements.
    it("should reject amount mismatch", async function () {
      const wrongAmount = AMOUNT + 1n;

      await expect(
        escrowGmp.connect(requester).createEscrowWithValidation(
          INTENT_ID,
          token.target,
          wrongAmount
        )
      ).to.be.revertedWithCustomError(escrowGmp, "E_AMOUNT_MISMATCH");
    });

    /// 11. Test: test_create_escrow_rejects_token_mismatch: Create Escrow Rejects Token Mismatch
    /// Verifies escrow creation fails if token doesn't match.
    /// Why: Token must match requirements.
    it("should reject token mismatch", async function () {
      const MockERC20 = await ethers.getContractFactory("MockERC20");
      const wrongToken = await MockERC20.deploy("Wrong", "WRONG", 18);
      await wrongToken.mint(requester.address, AMOUNT);
      await wrongToken.connect(requester).approve(escrowGmp.target, AMOUNT);

      await expect(
        escrowGmp.connect(requester).createEscrowWithValidation(
          INTENT_ID,
          wrongToken.target,
          AMOUNT
        )
      ).to.be.revertedWithCustomError(escrowGmp, "E_TOKEN_MISMATCH");
    });

    /// 12. Test: test_create_escrow_sends_escrow_confirmation: Create Escrow Sends Escrow Confirmation
    /// Verifies EscrowConfirmation is sent to hub.
    /// Why: Hub needs confirmation to proceed with intent.
    it("should send escrow confirmation to hub", async function () {
      await expect(
        escrowGmp.connect(requester).createEscrowWithValidation(
          INTENT_ID,
          token.target,
          AMOUNT
        )
      ).to.emit(escrowGmp, "EscrowConfirmationSent");
    });
  });

  // ============================================================================
  // Full Workflow
  // ============================================================================

  describe("Full Workflow", function () {
    /// 13. Test: test_full_inflow_gmp_workflow: Full Inflow GMP Workflow
    /// Verifies complete flow from requirements to release.
    /// Why: End-to-end validation of the entire process.
    it("should complete full inflow GMP workflow", async function () {
      const tokenAddr32 = await tokenToBytes32(token.target);
      const requesterAddr32 = await addressToBytes32(requester.address);
      const solverAddr32 = await addressToBytes32(solver.address);
      const block = await ethers.provider.getBlock("latest");
      const expiry = BigInt(block.timestamp + 3600);
      const timestamp = BigInt(block.timestamp);

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
      expect(await escrowGmp.hasRequirements(INTENT_ID)).to.equal(true);

      // 2. Create escrow
      await escrowGmp.connect(requester).createEscrowWithValidation(
        INTENT_ID,
        token.target,
        AMOUNT
      );
      expect(await escrowGmp.hasEscrow(INTENT_ID)).to.equal(true);
      expect(await token.balanceOf(escrowGmp.target)).to.equal(AMOUNT);

      // 3. Receive fulfillment proof and auto-release
      const proofPayload = await encodeFulfillmentProof(
        INTENT_ID,
        solverAddr32,
        AMOUNT,
        timestamp
      );
      await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, proofPayload, 2);

      // 4. Verify final state
      expect(await escrowGmp.isFulfilled(INTENT_ID)).to.equal(true);
      expect(await escrowGmp.isReleased(INTENT_ID)).to.equal(true);
      expect(await token.balanceOf(solver.address)).to.equal(AMOUNT);
      expect(await token.balanceOf(escrowGmp.target)).to.equal(0);
    });
  });

  // ============================================================================
  // Create Escrow Validation
  // ============================================================================

  describe("Create Escrow Validation", function () {
    let tokenAddr32;
    let requesterAddr32;
    let solverAddr32;
    let expiry;

    beforeEach(async function () {
      tokenAddr32 = await tokenToBytes32(token.target);
      requesterAddr32 = await addressToBytes32(requester.address);
      solverAddr32 = await addressToBytes32(solver.address);
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

    /// 14. Test: test_create_escrow_rejects_no_requirements: Create Escrow Rejects No Requirements
    /// Verifies escrow creation fails if no requirements exist.
    /// Why: Requirements must be received first.
    it("should reject escrow without requirements", async function () {
      const unknownIntentId = "0xcc000000000000000000000000000000000000000000000000000000000000dd";

      await expect(
        escrowGmp.connect(requester).createEscrowWithValidation(
          unknownIntentId,
          token.target,
          AMOUNT
        )
      ).to.be.revertedWithCustomError(escrowGmp, "E_REQUIREMENTS_NOT_FOUND");
    });

    /// 15. Test: test_create_escrow_rejects_double_create: Create Escrow Rejects Double Create
    /// Verifies escrow cannot be created twice.
    /// Why: Each intent can have only one escrow.
    it("should reject double escrow creation", async function () {
      await escrowGmp.connect(requester).createEscrowWithValidation(
        INTENT_ID,
        token.target,
        AMOUNT
      );

      await expect(
        escrowGmp.connect(requester).createEscrowWithValidation(
          INTENT_ID,
          token.target,
          AMOUNT
        )
      ).to.be.revertedWithCustomError(escrowGmp, "E_ESCROW_ALREADY_CREATED");
    });
  });

  // ============================================================================
  // EVM-Specific Tests
  // ============================================================================

  describe("EVM-Specific Tests", function () {
    /// 23. Test: test_reject_direct_call: Reject Direct Call (Not Through GMP)
    /// Verifies only GMP endpoint can call receiveIntentRequirements.
    /// Why: Single trust point through GMP endpoint.
    it("should reject direct call", async function () {
      const payload = "0x01" + "00".repeat(144);

      await expect(
        escrowGmp.receiveIntentRequirements(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, payload)
      ).to.be.revertedWithCustomError(escrowGmp, "E_UNAUTHORIZED_ENDPOINT");
    });

    describe("Create Escrow", function () {
      let tokenAddr32;
      let requesterAddr32;
      let solverAddr32;
      let expiry;

      beforeEach(async function () {
        tokenAddr32 = await tokenToBytes32(token.target);
        requesterAddr32 = await addressToBytes32(requester.address);
        solverAddr32 = await addressToBytes32(solver.address);
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

      /// 24. Test: test_create_escrow_rejects_requester_mismatch: Create Escrow Rejects Requester Mismatch
      /// Verifies only requester can create escrow.
      /// Why: Security - only the authorized requester can escrow.
      it("should reject requester mismatch", async function () {
        await token.mint(solver.address, AMOUNT);
        await token.connect(solver).approve(escrowGmp.target, AMOUNT);

        await expect(
          escrowGmp.connect(solver).createEscrowWithValidation(
            INTENT_ID,
            token.target,
            AMOUNT
          )
        ).to.be.revertedWithCustomError(escrowGmp, "E_REQUESTER_MISMATCH");
      });

      /// 25. Test: test_create_escrow_rejects_expired_intent: Create Escrow Rejects Expired Intent
      /// Verifies expired intents cannot have escrows created.
      /// Why: Expired intents should be rejected.
      it("should reject expired intent", async function () {
        // Create new intent with past expiry
        const block = await ethers.provider.getBlock("latest");
        const pastExpiry = BigInt(block.timestamp - 3600);
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
          escrowGmp.connect(requester).createEscrowWithValidation(
            newIntentId,
            token.target,
            AMOUNT
          )
        ).to.be.revertedWithCustomError(escrowGmp, "E_INTENT_EXPIRED");
      });

      /// 26. Test: test_tokens_transferred_to_escrow: Tokens Transferred to Escrow Contract
      /// Verifies tokens are actually transferred.
      /// Why: Escrow must hold the tokens.
      it("should transfer tokens to escrow contract", async function () {
        const balanceBefore = await token.balanceOf(escrowGmp.target);

        await escrowGmp.connect(requester).createEscrowWithValidation(
          INTENT_ID,
          token.target,
          AMOUNT
        );

        const balanceAfter = await token.balanceOf(escrowGmp.target);
        expect(balanceAfter - balanceBefore).to.equal(AMOUNT);
      });
    });

    describe("Receive Fulfillment Proof", function () {
      let tokenAddr32;
      let requesterAddr32;
      let solverAddr32;
      let expiry;

      beforeEach(async function () {
        tokenAddr32 = await tokenToBytes32(token.target);
        requesterAddr32 = await addressToBytes32(requester.address);
        solverAddr32 = await addressToBytes32(solver.address);
        const block = await ethers.provider.getBlock("latest");
        expiry = BigInt(block.timestamp + 3600);

        // Deliver requirements
        const reqPayload = await encodeIntentRequirements(
          INTENT_ID,
          requesterAddr32,
          AMOUNT,
          tokenAddr32,
          solverAddr32,
          expiry
        );
        await gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, reqPayload, 1);

        // Create escrow
        await escrowGmp.connect(requester).createEscrowWithValidation(
          INTENT_ID,
          token.target,
          AMOUNT
        );
      });

      /// 27. Test: test_emit_events_on_release: Emit Events on Release
      /// Verifies correct events are emitted.
      /// Why: Events are used for monitoring and indexing.
      it("should emit events on release", async function () {
        const block = await ethers.provider.getBlock("latest");
        const timestamp = BigInt(block.timestamp);

        const proofPayload = await encodeFulfillmentProof(
          INTENT_ID,
          solverAddr32,
          AMOUNT,
          timestamp
        );

        await expect(
          gmpEndpoint.deliverMessage(HUB_CHAIN_ID, TRUSTED_HUB_ADDR, proofPayload, 2)
        )
          .to.emit(escrowGmp, "FulfillmentProofReceived")
          .and.to.emit(escrowGmp, "EscrowReleased");
      });
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
    // Deploy harness for encoding
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

  async function encodeFulfillmentProof(intentId, solverAddr, amount, timestamp) {
    const MessagesHarness = await ethers.getContractFactory("MessagesHarness");
    const harness = await MessagesHarness.deploy();
    return harness.encodeFulfillmentProof(intentId, solverAddr, amount, timestamp);
  }
});
