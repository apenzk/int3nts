const { expect } = require("chai");
const { ethers } = require("hardhat");
const {
  setupInflowEscrowGmpTests,
  addressToBytes32,
  getExpiryTimestamp,
  deliverRequirements,
  getCurrentTimestamp,
  DEFAULT_AMOUNT
} = require("./helpers/setup");

describe("IntentInflowEscrow - Error Conditions", function () {
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

  /// 1. Test: test_zero_amount_rejection: Zero Amount Rejection
  /// Verifies that createEscrowWithValidation reverts when amount is zero.
  /// Why: Zero-amount escrows are meaningless and could cause accounting issues.
  it("Should revert with zero amount in createEscrow", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();
    const zeroAmount = 0n;

    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      zeroAmount,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    await expect(
      escrow.connect(requester).createEscrowWithValidation(intentId, token.target, zeroAmount)
    ).to.be.revertedWithCustomError(escrow, "E_ZERO_AMOUNT");
  });

  /// 2. Test: test_insufficient_allowance_rejection: Insufficient Allowance Rejection
  /// Verifies that createEscrowWithValidation reverts when ERC20 allowance is insufficient.
  /// Why: ERC20 transfers require explicit approval. Insufficient allowance must be rejected.
  it("Should revert with insufficient ERC20 allowance", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();
    const largeAmount = DEFAULT_AMOUNT * 1000n;

    // Set up requirements for large amount
    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      largeAmount,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    // Mint tokens but approve less than needed
    await token.mint(requester.address, largeAmount);
    await token.connect(requester).approve(escrow.target, DEFAULT_AMOUNT); // Less than largeAmount

    await expect(
      escrow.connect(requester).createEscrowWithValidation(intentId, token.target, largeAmount)
    ).to.be.reverted;
  });

  /// 3. Test: test_requirements_not_found: Requirements Not Found
  /// Verifies that createEscrowWithValidation reverts when no requirements exist.
  /// Why: Escrows can only be created after hub sends requirements.
  it("Should revert when requirements not found", async function () {
    const unknownIntentId = "0xcc000000000000000000000000000000000000000000000000000000000000dd";

    await expect(
      escrow.connect(requester).createEscrowWithValidation(unknownIntentId, token.target, DEFAULT_AMOUNT)
    ).to.be.revertedWithCustomError(escrow, "E_REQUIREMENTS_NOT_FOUND");
  });

  /// 4. Test: test_amount_mismatch_rejection: Amount Mismatch Rejection
  /// Verifies that createEscrowWithValidation reverts when amount doesn't match requirements.
  /// Why: Amount must match what hub specified.
  it("Should revert with amount mismatch", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    const wrongAmount = DEFAULT_AMOUNT + 1n;

    await expect(
      escrow.connect(requester).createEscrowWithValidation(intentId, token.target, wrongAmount)
    ).to.be.revertedWithCustomError(escrow, "E_AMOUNT_MISMATCH");
  });

  /// 5. Test: test_token_mismatch_rejection: Token Mismatch Rejection
  /// Verifies that createEscrowWithValidation reverts when token doesn't match requirements.
  /// Why: Token must match what hub specified.
  it("Should revert with token mismatch", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    // Deploy a different token
    const MockERC20 = await ethers.getContractFactory("MockERC20");
    const wrongToken = await MockERC20.deploy("Wrong Token", "WRONG", 18);
    await wrongToken.mint(requester.address, DEFAULT_AMOUNT);
    await wrongToken.connect(requester).approve(escrow.target, DEFAULT_AMOUNT);

    await expect(
      escrow.connect(requester).createEscrowWithValidation(intentId, wrongToken.target, DEFAULT_AMOUNT)
    ).to.be.revertedWithCustomError(escrow, "E_TOKEN_MISMATCH");
  });

  /// 6. Test: test_requester_mismatch_rejection: Requester Mismatch Rejection
  /// Verifies that only the correct requester can create escrow.
  /// Why: Security - only authorized requester can lock funds.
  it("Should revert with requester mismatch", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    // Solver tries to create escrow (wrong requester)
    await token.mint(solver.address, DEFAULT_AMOUNT);
    await token.connect(solver).approve(escrow.target, DEFAULT_AMOUNT);

    await expect(
      escrow.connect(solver).createEscrowWithValidation(intentId, token.target, DEFAULT_AMOUNT)
    ).to.be.revertedWithCustomError(escrow, "E_REQUESTER_MISMATCH");
  });

  /// 7. Test: test_expired_intent_rejection: Expired Intent Rejection
  /// Verifies that escrows cannot be created after intent expires.
  /// Why: Expired intents should be rejected.
  it("Should revert with expired intent", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const pastExpiry = (await getCurrentTimestamp()) - 100n; // Already expired

    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      pastExpiry
    );

    await expect(
      escrow.connect(requester).createEscrowWithValidation(intentId, token.target, DEFAULT_AMOUNT)
    ).to.be.revertedWithCustomError(escrow, "E_INTENT_EXPIRED");
  });

  /// 8. Test: test_cancel_nonexistent_escrow: Non-Existent Escrow Cancellation Rejection
  /// Verifies that cancel reverts for non-existent escrows.
  /// Why: Prevents cancellation of non-existent escrows.
  it("Should revert cancel on non-existent escrow", async function () {
    const nonExistentIntentId = "0xcc000000000000000000000000000000000000000000000000000000000000dd";

    const [admin] = await ethers.getSigners();
    await expect(
      escrow.connect(admin).cancel(nonExistentIntentId)
    ).to.be.revertedWithCustomError(escrow, "E_ESCROW_NOT_FOUND");
  });

  /// 9. Test: test_duplicate_escrow_creation: Duplicate Escrow Creation Rejection
  /// Verifies that escrows with duplicate intent IDs are rejected.
  /// Why: Each intent ID must map to exactly one escrow.
  it("Should revert with duplicate escrow creation", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();

    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      DEFAULT_AMOUNT,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    // Create first escrow
    await escrow.connect(requester).createEscrowWithValidation(intentId, token.target, DEFAULT_AMOUNT);

    // Try to create second escrow with same intent ID
    await expect(
      escrow.connect(requester).createEscrowWithValidation(intentId, token.target, DEFAULT_AMOUNT)
    ).to.be.revertedWithCustomError(escrow, "E_ESCROW_ALREADY_CREATED");
  });

  /// 10. Test: test_insufficient_token_balance: Insufficient Token Balance Rejection
  /// Verifies that escrow creation fails if requester has insufficient tokens.
  /// Why: Cannot deposit more tokens than available.
  it("Should revert with insufficient token balance", async function () {
    const tokenAddr32 = addressToBytes32(token.target);
    const requesterAddr32 = addressToBytes32(requester.address);
    const solverAddr32 = addressToBytes32(solver.address);
    const expiry = await getExpiryTimestamp();
    const largeAmount = DEFAULT_AMOUNT * 10000n;

    await deliverRequirements(
      gmpEndpoint,
      intentId,
      requesterAddr32,
      largeAmount,
      tokenAddr32,
      solverAddr32,
      expiry
    );

    // Approve but don't have enough balance
    await token.connect(requester).approve(escrow.target, largeAmount);

    await expect(
      escrow.connect(requester).createEscrowWithValidation(intentId, token.target, largeAmount)
    ).to.be.reverted;
  });
});
