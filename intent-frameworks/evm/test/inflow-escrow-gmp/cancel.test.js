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

describe("IntentInflowEscrow - Cancel", function () {
  let escrow;
  let gmpEndpoint;
  let token;
  let admin;
  let requester;
  let solver;
  let intentId;
  let tokenAddr32;
  let requesterAddr32;
  let solverAddr32;
  let expiry;

  beforeEach(async function () {
    const fixtures = await setupInflowEscrowGmpTests();
    escrow = fixtures.escrow;
    gmpEndpoint = fixtures.gmpEndpoint;
    token = fixtures.token;
    admin = fixtures.admin;
    requester = fixtures.requester;
    solver = fixtures.solver;
    intentId = fixtures.intentId;

    tokenAddr32 = addressToBytes32(token.target);
    requesterAddr32 = addressToBytes32(requester.address);
    solverAddr32 = addressToBytes32(solver.address);
    expiry = await getExpiryTimestamp();

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
  });

  /// 1. Test: test_cancel_before_expiry: Cancellation Before Expiry Prevention
  /// Verifies that admin cannot cancel escrows before expiry.
  /// Why: Funds must remain locked until expiry to give solvers time to fulfill.
  it("Should revert if escrow has not expired yet", async function () {
    await expect(
      escrow.connect(admin).cancel(intentId)
    ).to.be.revertedWithCustomError(escrow, "E_ESCROW_NOT_EXPIRED");
  });

  /// 2. Test: test_cancel_after_expiry: Admin Cancellation After Expiry
  /// Verifies that admin can cancel escrows after expiry and funds return to requester.
  /// Why: Admin acts as operator to unstick expired escrows; funds always go to original requester.
  it("Should allow admin to cancel and return funds to requester after expiry", async function () {
    // Advance time past expiry
    await advanceTime(DEFAULT_EXPIRY_OFFSET + 1);

    const initialBalance = await token.balanceOf(requester.address);

    await expect(escrow.connect(admin).cancel(intentId))
      .to.emit(escrow, "EscrowCancelled")
      .withArgs(intentId, requester.address, DEFAULT_AMOUNT);

    expect(await token.balanceOf(requester.address)).to.equal(initialBalance + DEFAULT_AMOUNT);
    expect(await token.balanceOf(escrow.target)).to.equal(0n);

    // Verify escrow state
    expect(await escrow.isReleased(intentId)).to.equal(true);
    expect(await escrow.isFulfilled(intentId)).to.equal(false);
    expect(await escrow.isCancelled(intentId)).to.equal(true);
  });

  /// 3. Test: test_cancel_unauthorized: Unauthorized Cancellation Prevention
  /// Verifies that non-admin callers cannot cancel escrows (even after expiry).
  /// Why: Only admin should be able to cancel; requester and solver are not authorized.
  it("Should revert if caller is not admin", async function () {
    // Advance time past expiry
    await advanceTime(DEFAULT_EXPIRY_OFFSET + 1);

    // Requester cannot cancel
    await expect(
      escrow.connect(requester).cancel(intentId)
    ).to.be.revertedWithCustomError(escrow, "E_UNAUTHORIZED_CALLER");

    // Solver cannot cancel
    await expect(
      escrow.connect(solver).cancel(intentId)
    ).to.be.revertedWithCustomError(escrow, "E_UNAUTHORIZED_CALLER");
  });

  /// 4. Test: test_cancel_after_fulfillment: Cancellation After Fulfillment Prevention
  /// Verifies that attempting to cancel an already-fulfilled escrow reverts.
  /// Why: Once funds are released via fulfillment, they cannot be cancelled.
  it("Should revert if already fulfilled", async function () {
    const timestamp = await getCurrentTimestamp();

    // Deliver fulfillment proof (releases escrow to solver)
    await deliverFulfillmentProof(
      gmpEndpoint,
      intentId,
      solverAddr32,
      DEFAULT_AMOUNT,
      timestamp
    );

    // Advance time past expiry
    await advanceTime(DEFAULT_EXPIRY_OFFSET + 1);

    await expect(
      escrow.connect(admin).cancel(intentId)
    ).to.be.revertedWithCustomError(escrow, "E_ALREADY_RELEASED");
  });

  /// 5. Test: test_cancel_nonexistent_escrow: Non-Existent Escrow Prevention
  /// Verifies that canceling a non-existent escrow reverts.
  /// Why: Prevents invalid operations on non-existent escrows.
  it("Should revert if escrow does not exist", async function () {
    const nonExistentIntentId = "0xcc000000000000000000000000000000000000000000000000000000000000dd";

    await expect(
      escrow.connect(admin).cancel(nonExistentIntentId)
    ).to.be.revertedWithCustomError(escrow, "E_ESCROW_NOT_FOUND");
  });

  /// 6. Test: test_double_cancel: Double Cancellation Prevention
  /// Verifies that canceling an already-cancelled escrow reverts.
  /// Why: Prevents double-refund by ensuring released escrows cannot be cancelled again.
  it("Should revert if already cancelled", async function () {
    // Advance time past expiry
    await advanceTime(DEFAULT_EXPIRY_OFFSET + 1);

    // First cancel succeeds
    await escrow.connect(admin).cancel(intentId);

    // Second cancel reverts
    await expect(
      escrow.connect(admin).cancel(intentId)
    ).to.be.revertedWithCustomError(escrow, "E_ALREADY_RELEASED");
  });
});
