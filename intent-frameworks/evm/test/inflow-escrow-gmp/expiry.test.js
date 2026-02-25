const { expect } = require("chai");
const { ethers } = require("hardhat");
const {
  setupInflowEscrowGmpTests,
  advanceTime,
  addressToBytes32,
  getExpiryTimestamp,
  deliverRequirements,
  deliverFulfillmentProof,
  getCurrentTimestamp,
  DEFAULT_AMOUNT,
  DEFAULT_EXPIRY_OFFSET
} = require("./helpers/setup");

describe("IntentInflowEscrow - Expiry Handling", function () {
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

  /// 1. Test: test_cancel_expired_escrow: Expired Escrow Cancellation
  /// Verifies that admin can cancel escrows after expiry and funds return to requester.
  /// Why: Admin acts as operator to unstick expired escrows; funds always go to original requester.
  it("Should allow admin to cancel expired escrow", async function () {
    const [admin] = await ethers.getSigners();
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    // Deliver requirements and create escrow
    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );
    await escrow.connect(requester).createEscrowWithValidation(
      intentId,
      token.target,
      DEFAULT_AMOUNT
    );

    // Cancellation blocked before expiry
    await expect(
      escrow.connect(admin).cancel(intentId)
    ).to.be.revertedWithCustomError(escrow, "E_ESCROW_NOT_EXPIRED");

    // Advance time past expiry
    await advanceTime(DEFAULT_EXPIRY_OFFSET + 1);

    // Admin cancellation allowed after expiry, funds go to requester
    const initialBalance = await token.balanceOf(requester.address);
    await expect(escrow.connect(admin).cancel(intentId))
      .to.emit(escrow, "EscrowCancelled")
      .withArgs(intentId, requester.address, DEFAULT_AMOUNT);

    expect(await token.balanceOf(requester.address)).to.equal(initialBalance + DEFAULT_AMOUNT);
    expect(await token.balanceOf(escrow.target)).to.equal(0n);

    expect(await escrow.isReleased(intentId)).to.equal(true);
    expect(await escrow.isCancelled(intentId)).to.equal(true);
  });

  /// 2. Test: test_expiry_timestamp_validation: Expiry Timestamp Validation
  /// Verifies that expiry timestamp is correctly stored from requirements.
  /// Why: Correct expiry from hub is critical for time-based cancellation logic.
  it("Should verify expiry timestamp is stored correctly from requirements", async function () {
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

    // Create escrow
    await escrow.connect(requester).createEscrowWithValidation(
      intentId,
      token.target,
      DEFAULT_AMOUNT
    );

    // Verify requirements store the expiry
    const requirements = await escrow.getRequirements(intentId);
    expect(requirements.expiry).to.equal(expiry);

    // Verify escrow data
    const escrowData = await escrow.getEscrow(intentId);
    expect(escrowData.amount).to.equal(DEFAULT_AMOUNT);
    expect(escrowData.fulfilled).to.equal(false);
    expect(escrowData.released).to.equal(false);
  });

  /// 3. Test: test_gmp_fulfillment_after_local_expiry: GMP Fulfillment After Local Expiry
  /// Verifies that GMP fulfillment proofs are honored regardless of local expiry.
  /// Why: Hub is source of truth. If hub confirms fulfillment, escrow must release even after local expiry.
  it("Should allow GMP fulfillment after local expiry (hub is source of truth)", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    // Deliver requirements and create escrow
    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );
    await escrow.connect(requester).createEscrowWithValidation(
      intentId,
      token.target,
      DEFAULT_AMOUNT
    );

    // Advance time past expiry
    await advanceTime(DEFAULT_EXPIRY_OFFSET + 1);

    // GMP fulfillment should still work (hub is source of truth)
    const timestamp = await getCurrentTimestamp();
    await expect(
      deliverFulfillmentProof(gmpEndpoint, intentId, solverAddr32, DEFAULT_AMOUNT, timestamp)
    )
      .to.emit(escrow, "EscrowReleased")
      .withArgs(intentId, solver.address, DEFAULT_AMOUNT);

    expect(await token.balanceOf(solver.address)).to.equal(DEFAULT_AMOUNT);
    expect(await escrow.isFulfilled(intentId)).to.equal(true);
  });
});
