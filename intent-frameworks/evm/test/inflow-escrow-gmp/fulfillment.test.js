const { expect } = require("chai");
const { ethers } = require("hardhat");
const {
  setupInflowEscrowGmpTests,
  addressToBytes32,
  getExpiryTimestamp,
  deliverRequirements,
  deliverFulfillmentProof,
  getCurrentTimestamp,
  DEFAULT_AMOUNT
} = require("./helpers/setup");

describe("IntentInflowEscrow - Fulfillment", function () {
  let escrow;
  let gmpEndpoint;
  let token;
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

  /// 1. Test: test_valid_fulfillment_release: Valid Fulfillment Proof Release
  /// Verifies that solvers receive escrow funds when hub sends a valid FulfillmentProof.
  /// Why: Fulfillment is the core mechanism. Solvers must receive funds after hub confirms fulfillment.
  it("Should release escrow to solver with valid fulfillment proof", async function () {
    const timestamp = await getCurrentTimestamp();

    await expect(
      deliverFulfillmentProof(gmpEndpoint, intentId, solverAddr32, DEFAULT_AMOUNT, timestamp)
    )
      .to.emit(escrow, "EscrowReleased")
      .withArgs(intentId, solver.address, DEFAULT_AMOUNT);

    expect(await token.balanceOf(solver.address)).to.equal(DEFAULT_AMOUNT);
    expect(await token.balanceOf(escrow.target)).to.equal(0n);

    expect(await escrow.isFulfilled(intentId)).to.equal(true);
    expect(await escrow.isReleased(intentId)).to.equal(true);
  });

  /// 2. Test: test_fulfillment_without_escrow: Fulfillment Without Requirements
  /// Verifies that fulfillment proofs for unknown intents are rejected.
  /// Why: Security requirement - only valid intents with escrows should be fulfillable.
  it("Should revert fulfillment without escrow", async function () {
    const unknownIntentId = "0xcc000000000000000000000000000000000000000000000000000000000000dd";
    const timestamp = await getCurrentTimestamp();

    // Note: This will fail at the escrow lookup since there's no escrow for this intent
    await expect(
      deliverFulfillmentProof(gmpEndpoint, unknownIntentId, solverAddr32, DEFAULT_AMOUNT, timestamp)
    ).to.be.revertedWithCustomError(escrow, "E_ESCROW_NOT_FOUND");
  });

  /// 3. Test: test_prevent_double_fulfillment: Prevent Double Fulfillment
  /// Verifies that the same fulfillment proof cannot be delivered twice.
  /// Why: GMP deduplication prevents double-spending at the endpoint layer.
  it("Should prevent double fulfillment", async function () {
    const timestamp = await getCurrentTimestamp();

    // First fulfillment succeeds
    await deliverFulfillmentProof(gmpEndpoint, intentId, solverAddr32, DEFAULT_AMOUNT, timestamp);

    // Second fulfillment fails - blocked by GMP (intent_id, msg_type) deduplication
    await expect(
      deliverFulfillmentProof(gmpEndpoint, intentId, solverAddr32, DEFAULT_AMOUNT, timestamp)
    ).to.be.revertedWithCustomError(gmpEndpoint, "E_ALREADY_DELIVERED");
  });

  /// 4. Test: test_fulfillment_after_cancel: Fulfillment Already Released (via cancel)
  /// Verifies that fulfillment fails if escrow was already cancelled.
  /// Why: Once funds are returned via cancel, they cannot be released to solver.
  it("Should revert if escrow already released via cancel", async function () {
    // Create a new escrow with short expiry for testing
    const shortExpiryIntentId = "0xdd000000000000000000000000000000000000000000000000000000000000ee";
    const shortExpiry = (await getCurrentTimestamp()) + 60n; // 60 seconds from now

    await deliverRequirements(
      gmpEndpoint,
      shortExpiryIntentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      shortExpiry
    );
    await escrow.connect(requester).createEscrowWithValidation(
      shortExpiryIntentId,
      token.target,
      DEFAULT_AMOUNT
    );

    // Wait for expiry and admin cancels
    const [admin] = await ethers.getSigners();
    await ethers.provider.send("evm_increaseTime", [61]);
    await ethers.provider.send("evm_mine", []);
    await escrow.connect(admin).cancel(shortExpiryIntentId);

    // Now try to fulfill - should fail because escrow was already released via cancel
    const timestamp = await getCurrentTimestamp();
    await expect(
      deliverFulfillmentProof(gmpEndpoint, shortExpiryIntentId, solverAddr32, DEFAULT_AMOUNT, timestamp)
    ).to.be.revertedWithCustomError(escrow, "E_ALREADY_RELEASED");
  });

  /// 5. Test: test_fulfillment_nonexistent_escrow: Escrow Does Not Exist
  /// Verifies that attempting to fulfill a non-existent escrow reverts.
  /// Why: Prevents fulfillment on non-existent escrows.
  it("Should revert if escrow does not exist", async function () {
    const nonExistentIntentId = "0xee000000000000000000000000000000000000000000000000000000000000ff";
    const timestamp = await getCurrentTimestamp();

    await expect(
      deliverFulfillmentProof(gmpEndpoint, nonExistentIntentId, solverAddr32, DEFAULT_AMOUNT, timestamp)
    ).to.be.revertedWithCustomError(escrow, "E_ESCROW_NOT_FOUND");
  });
});
