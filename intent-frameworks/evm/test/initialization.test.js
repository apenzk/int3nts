const { expect } = require("chai");
const { ethers } = require("hardhat");
const { setupIntentEscrowTests } = require("./helpers/setup");

describe("IntentEscrow - Initialization", function () {
  let escrow;
  let token;
  let approver;
  let requester;
  let solver;
  let intentId;

  beforeEach(async function () {
    const fixtures = await setupIntentEscrowTests();
    escrow = fixtures.escrow;
    token = fixtures.token;
    approver = fixtures.approver;
    requester = fixtures.requester;
    solver = fixtures.solver;
    intentId = fixtures.intentId;
  });

  /// 1. Test: Approver Address Initialization
  /// Verifies that the escrow is deployed with the correct approver address.
  /// Why: The approver address is critical for signature validation. Incorrect initialization would break security.
  it("Should initialize escrow with approver address", async function () {
    expect(await escrow.approver()).to.equal(approver.address);
  });

  /// 2. Test: Escrow Creation
  /// Verifies that requesters can create a new escrow with funds atomically and expiry is set correctly.
  /// Why: Escrow creation must be atomic and set expiry correctly to enable time-based cancellation.
  it("Should allow requester to create an escrow", async function () {
    const amount = ethers.parseEther("100");
    await token.mint(requester.address, amount);
    await token.connect(requester).approve(escrow.target, amount);
    
    const tx = await escrow.connect(requester).createEscrow(intentId, token.target, amount, solver.address);
    const receipt = await tx.wait();
    const block = await ethers.provider.getBlock(receipt.blockNumber);
    
    const expectedExpiry = BigInt(block.timestamp) + BigInt(await escrow.EXPIRY_DURATION());
    
    await expect(tx)
      .to.emit(escrow, "EscrowInitialized")
      .withArgs(intentId, escrow.target, requester.address, token.target, solver.address, amount, expectedExpiry);

    const escrowData = await escrow.getEscrow(intentId);
    expect(escrowData.requester).to.equal(requester.address);
    expect(escrowData.token).to.equal(token.target);
    expect(escrowData.amount).to.equal(amount);
    expect(escrowData.isClaimed).to.equal(false);
    expect(escrowData.expiry).to.equal(expectedExpiry);
  });

  /// 3. Test: Duplicate Creation Prevention
  /// Verifies that attempting to create an escrow with an existing intent ID reverts.
  /// Why: Each intent ID must map to a single escrow to maintain state consistency.
  it("Should revert if escrow already exists", async function () {
    const amount = ethers.parseEther("100");
    await token.mint(requester.address, amount);
    await token.connect(requester).approve(escrow.target, amount);
    await escrow.connect(requester).createEscrow(intentId, token.target, amount, solver.address);
    
    await expect(
      escrow.connect(requester).createEscrow(intentId, token.target, amount, solver.address)
    ).to.be.revertedWith("Escrow already exists");
  });

  /// 4. Test: Zero Amount Prevention
  /// Verifies that escrows cannot be created with zero amount.
  /// Why: Zero-amount escrows are invalid.
  it("Should revert if amount is zero", async function () {
    const amount = 0n;
    
    await expect(
      escrow.connect(requester).createEscrow(intentId, token.target, amount, solver.address)
    ).to.be.revertedWith("Amount must be greater than 0");
  });
});

