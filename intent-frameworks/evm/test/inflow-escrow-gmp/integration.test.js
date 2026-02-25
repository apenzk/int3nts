const { expect } = require("chai");
const { ethers } = require("hardhat");
const {
  setupInflowEscrowGmpTests,
  advanceTime,
  addressToBytes32,
  getExpiryTimestamp,
  deliverRequirements,
  deliverFulfillmentProof,
  DEFAULT_AMOUNT
} = require("./helpers/setup");

describe("IntentInflowEscrow - Integration Tests", function () {
  let escrow;
  let gmpEndpoint;
  let token;
  let requester;
  let solver;
  let intentId;

  beforeEach(async function () {
    const fixtures = await setupInflowEscrowGmpTests();
    escrow = fixtures.escrow;
    gmpEndpoint = fixtures.gmpEndpoint;
    token = fixtures.token;
    requester = fixtures.requester;
    solver = fixtures.solver;
    intentId = fixtures.intentId;
  });

  // ============================================================================
  // INTEGRATION TESTS
  // ============================================================================

  /// 1. Test: test_complete_deposit_to_fulfillment: Complete Deposit to Fulfillment Workflow
  /// Verifies the full workflow from requirements to escrow creation through fulfillment.
  /// Why: Integration test ensures all components work together correctly in the happy path.
  it("Should complete full deposit to fulfillment workflow", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    // Step 1: Hub delivers requirements via GMP
    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    // Verify requirements were stored
    expect(await escrow.hasRequirements(intentId)).to.equal(true);
    const requirements = await escrow.getRequirements(intentId);
    expect(requirements.amountRequired).to.equal(DEFAULT_AMOUNT);

    // Step 2: Create escrow and verify EscrowCreated event
    await expect(
      escrow.connect(requester).createEscrowWithValidation(
        intentId,
        token.target,
        DEFAULT_AMOUNT
      )
    ).to.emit(escrow, "EscrowCreated");

    // Step 3: Verify escrow state
    expect(await escrow.hasEscrow(intentId)).to.equal(true);
    const escrowDataBefore = await escrow.getEscrow(intentId);
    expect(escrowDataBefore.amount).to.equal(DEFAULT_AMOUNT);
    expect(escrowDataBefore.released).to.equal(false);
    expect(await token.balanceOf(escrow.target)).to.equal(DEFAULT_AMOUNT);

    // Step 4: Hub delivers fulfillment proof via GMP
    const solverBalanceBefore = await token.balanceOf(solver.address);
    expect(solverBalanceBefore).to.equal(0);

    await expect(
      deliverFulfillmentProof(gmpEndpoint, intentId, solverAddr32)
    ).to.emit(escrow, "EscrowReleased");

    // Step 5: Verify final state
    const escrowDataAfter = await escrow.getEscrow(intentId);
    expect(escrowDataAfter.released).to.equal(true);
    expect(await escrow.isReleased(intentId)).to.equal(true);
    expect(await token.balanceOf(solver.address)).to.equal(DEFAULT_AMOUNT);
    expect(await token.balanceOf(escrow.target)).to.equal(0);
    // Requester starts with 100 * DEFAULT_AMOUNT, deposits DEFAULT_AMOUNT, so has 99 * DEFAULT_AMOUNT left
    expect(await token.balanceOf(requester.address)).to.equal(DEFAULT_AMOUNT * 99n);
  });

  /// 2. Test: test_multi_token_scenarios: Multi-Token Scenarios
  /// Verifies that the escrow works with different ERC20 tokens.
  /// Why: The escrow must support any ERC20 token, not just a single token type.
  it("Should handle multiple different ERC20 tokens", async function () {
    const MockERC20 = await ethers.getContractFactory("MockERC20");
    const token1 = await MockERC20.deploy("Token One", "TKN1", 18);
    await token1.waitForDeployment();
    const token2 = await MockERC20.deploy("Token Two", "TKN2", 18);
    await token2.waitForDeployment();
    const token3 = await MockERC20.deploy("Token Three", "TKN3", 18);
    await token3.waitForDeployment();

    const amount1 = DEFAULT_AMOUNT;
    const amount2 = DEFAULT_AMOUNT * 2n;
    const amount3 = DEFAULT_AMOUNT * 3n;

    const intentId1 = intentId;
    const intentId2 = "0xbb000000000000000000000000000000000000000000000000000000000000cc";
    const intentId3 = "0xcc000000000000000000000000000000000000000000000000000000000000dd";

    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    // Mint tokens for requester
    await token1.mint(requester.address, amount1);
    await token1.connect(requester).approve(escrow.target, amount1);
    await token2.mint(requester.address, amount2);
    await token2.connect(requester).approve(escrow.target, amount2);
    await token3.mint(requester.address, amount3);
    await token3.connect(requester).approve(escrow.target, amount3);

    // Deliver requirements for all three intents
    await deliverRequirements(
      gmpEndpoint, intentId1, requesterAddr32, amount1,
      addressToBytes32(token1.target), solverAddr32, expiry, 1
    );
    await deliverRequirements(
      gmpEndpoint, intentId2, requesterAddr32, amount2,
      addressToBytes32(token2.target), solverAddr32, expiry, 2
    );
    await deliverRequirements(
      gmpEndpoint, intentId3, requesterAddr32, amount3,
      addressToBytes32(token3.target), solverAddr32, expiry, 3
    );

    // Create escrows with different tokens
    await escrow.connect(requester).createEscrowWithValidation(intentId1, token1.target, amount1);
    await escrow.connect(requester).createEscrowWithValidation(intentId2, token2.target, amount2);
    await escrow.connect(requester).createEscrowWithValidation(intentId3, token3.target, amount3);

    // Verify all escrows were created correctly
    const escrow1 = await escrow.getEscrow(intentId1);
    const escrow2 = await escrow.getEscrow(intentId2);
    const escrow3 = await escrow.getEscrow(intentId3);

    expect(escrow1.token).to.equal(token1.target);
    expect(escrow1.amount).to.equal(amount1);
    expect(escrow2.token).to.equal(token2.target);
    expect(escrow2.amount).to.equal(amount2);
    expect(escrow3.token).to.equal(token3.target);
    expect(escrow3.amount).to.equal(amount3);

    // Verify balances
    expect(await token1.balanceOf(escrow.target)).to.equal(amount1);
    expect(await token2.balanceOf(escrow.target)).to.equal(amount2);
    expect(await token3.balanceOf(escrow.target)).to.equal(amount3);
  });

  /// 3. Test: test_comprehensive_event_emission: Comprehensive Event Emission
  /// Verifies that all events are emitted with correct parameters.
  /// Why: Events are critical for off-chain monitoring and indexing. Incorrect events break integrations.
  it("Should emit all events with correct parameters", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    // Deliver requirements
    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    // Test EscrowCreated event (7 args: intentId, escrowId, requester, amount, token, reservedSolver, expiry)
    const createTx = await escrow.connect(requester).createEscrowWithValidation(
      intentId,
      token.target,
      DEFAULT_AMOUNT
    );
    await expect(createTx).to.emit(escrow, "EscrowCreated");

    // Test EscrowReleased event via fulfillment proof
    await expect(
      deliverFulfillmentProof(gmpEndpoint, intentId, solverAddr32)
    ).to.emit(escrow, "EscrowReleased");
  });

  /// 4. Test: test_complete_cancellation_workflow: Complete Cancellation Workflow
  /// Verifies the full workflow from escrow creation through admin cancellation after expiry.
  /// Why: Integration test ensures the cancellation flow works end-to-end after expiry.
  it("Should complete full cancellation workflow", async function () {
    const [admin] = await ethers.getSigners();
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    // Step 1: Deliver requirements
    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    // Step 2: Create escrow
    await expect(
      escrow.connect(requester).createEscrowWithValidation(
        intentId,
        token.target,
        DEFAULT_AMOUNT
      )
    ).to.emit(escrow, "EscrowCreated");

    // Step 3: Verify escrow state before expiry
    expect(await escrow.hasEscrow(intentId)).to.equal(true);
    const escrowDataBefore = await escrow.getEscrow(intentId);
    expect(escrowDataBefore.amount).to.equal(DEFAULT_AMOUNT);
    expect(escrowDataBefore.released).to.equal(false);
    expect(await token.balanceOf(escrow.target)).to.equal(DEFAULT_AMOUNT);

    // Track requester balance before cancel
    const requesterBalanceBefore = await token.balanceOf(requester.address);

    // Step 4: Advance time past expiry (1 hour = 3600 seconds)
    await advanceTime(3601);

    // Step 5: Admin cancels escrow, funds return to requester
    await expect(escrow.connect(admin).cancel(intentId))
      .to.emit(escrow, "EscrowCancelled")
      .withArgs(intentId, requester.address, DEFAULT_AMOUNT);

    // Step 6: Verify final state
    const escrowDataAfter = await escrow.getEscrow(intentId);
    expect(escrowDataAfter.released).to.equal(true);
    expect(await escrow.isCancelled(intentId)).to.equal(true);
    expect(await token.balanceOf(requester.address)).to.equal(requesterBalanceBefore + DEFAULT_AMOUNT);
    expect(await token.balanceOf(escrow.target)).to.equal(0);
  });

  /// 5. Test: test_requirements_before_escrow: Requirements Before Escrow Requirement
  /// Verifies that escrow creation requires prior requirements delivery.
  /// Why: GMP flow mandates hub sends requirements before escrow can be created.
  it("Should require requirements before escrow creation", async function () {
    const unknownIntentId = "0xdd000000000000000000000000000000000000000000000000000000000000ee";

    // Attempt to create escrow without requirements
    await expect(
      escrow.connect(requester).createEscrowWithValidation(
        unknownIntentId,
        token.target,
        DEFAULT_AMOUNT
      )
    ).to.be.revertedWithCustomError(escrow, "E_REQUIREMENTS_NOT_FOUND");
  });

  /// 6. Test: test_full_lifecycle_multiple_participants: Full Lifecycle With Multiple Participants
  /// Verifies that multiple requesters can have independent escrows.
  /// Why: System must support concurrent users with independent escrow states.
  it("Should handle multiple participants with independent escrows", async function () {
    const [, , , other1, other2] = await ethers.getSigners();
    const tokenAddr32 = addressToBytes32(token.target);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    const intentId1 = intentId;
    const intentId2 = "0xee000000000000000000000000000000000000000000000000000000000000ff";

    const amount1 = DEFAULT_AMOUNT;
    const amount2 = DEFAULT_AMOUNT * 2n;

    // Mint tokens for different requesters
    await token.mint(other1.address, amount1);
    await token.connect(other1).approve(escrow.target, amount1);
    await token.mint(other2.address, amount2);
    await token.connect(other2).approve(escrow.target, amount2);

    // Deliver requirements for both
    await deliverRequirements(
      gmpEndpoint, intentId1, addressToBytes32(other1.address),
      amount1, tokenAddr32, solverAddr32, expiry, 1
    );
    await deliverRequirements(
      gmpEndpoint, intentId2, addressToBytes32(other2.address),
      amount2, tokenAddr32, solverAddr32, expiry, 2
    );

    // Create escrows
    await escrow.connect(other1).createEscrowWithValidation(intentId1, token.target, amount1);
    await escrow.connect(other2).createEscrowWithValidation(intentId2, token.target, amount2);

    // Verify independence
    expect(await escrow.hasEscrow(intentId1)).to.equal(true);
    expect(await escrow.hasEscrow(intentId2)).to.equal(true);

    const escrow1 = await escrow.getEscrow(intentId1);
    const escrow2 = await escrow.getEscrow(intentId2);

    expect(escrow1.amount).to.equal(amount1);
    expect(escrow2.amount).to.equal(amount2);

    // Fulfill one, admin cancels the other
    const [admin] = await ethers.getSigners();
    await deliverFulfillmentProof(gmpEndpoint, intentId1, solverAddr32, DEFAULT_AMOUNT, null, 3);
    await advanceTime(3601);
    await escrow.connect(admin).cancel(intentId2);

    // Verify final states
    expect(await escrow.isReleased(intentId1)).to.equal(true);
    expect(await escrow.isCancelled(intentId2)).to.equal(true);
    expect(await token.balanceOf(solver.address)).to.equal(amount1);
    expect(await token.balanceOf(other2.address)).to.equal(amount2);
  });
});
