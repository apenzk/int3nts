const { expect } = require("chai");
const { ethers } = require("hardhat");
const { setupIntentEscrowTests } = require("./helpers/setup");

describe("IntentEscrow - Create Escrow (Deposit)", function () {
  let escrow;
  let token;
  let requester;
  let solver;
  let intentId;

  beforeEach(async function () {
    const fixtures = await setupIntentEscrowTests();
    escrow = fixtures.escrow;
    token = fixtures.token;
    requester = fixtures.requester;
    solver = fixtures.solver;
    intentId = fixtures.intentId;
  });

  /// 1. Test: Token Escrow Creation
  /// Verifies that requesters can create an escrow with ERC20 tokens atomically.
  /// Why: Escrow creation is the first step in the intent fulfillment flow. Requesters must be able to lock funds securely.
  it("Should allow requester to create escrow with tokens", async function () {
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

    expect(await token.balanceOf(escrow.target)).to.equal(amount);
    
    const escrowData = await escrow.getEscrow(intentId);
    expect(escrowData.amount).to.equal(amount);
  });

  /// 2. Test: Escrow Creation After Claim Prevention
  /// Verifies that escrows cannot be created with an intent ID that was already claimed.
  /// Why: Prevents duplicate escrows and ensures each intent ID maps to a single escrow state.
  it("Should revert if escrow is already claimed", async function () {
    const amount = ethers.parseEther("100");
    await token.mint(requester.address, amount);
    await token.connect(requester).approve(escrow.target, amount);
    await escrow.connect(requester).createEscrow(intentId, token.target, amount, solver.address);

    // This test is covered in claim.test.js - escrow creation with same intentId will fail
    // because escrow already exists, not because it's claimed
    await expect(
      escrow.connect(requester).createEscrow(intentId, token.target, amount, solver.address)
    ).to.be.revertedWith("Escrow already exists");
  });

  /// 3. Test: Multiple Escrows with Different Intent IDs
  /// Verifies that multiple escrows can be created for different intent IDs.
  /// Why: System must support concurrent escrows.
  it("Should support multiple escrows with different intent IDs", async function () {
    const intentId1 = intentId;
    const intentId2 = intentId + 1n;
    const amount1 = ethers.parseEther("100");
    const amount2 = ethers.parseEther("200");

    // Mint and approve for both
    await token.mint(requester.address, amount1 + amount2);
    await token.connect(requester).approve(escrow.target, amount1 + amount2);

    // Create first escrow
    await escrow.connect(requester).createEscrow(intentId1, token.target, amount1, solver.address);

    // Create second escrow
    await escrow.connect(requester).createEscrow(intentId2, token.target, amount2, solver.address);

    // Verify both escrows exist with correct amounts
    const escrow1 = await escrow.getEscrow(intentId1);
    const escrow2 = await escrow.getEscrow(intentId2);
    
    expect(escrow1.amount).to.equal(amount1);
    expect(escrow2.amount).to.equal(amount2);
    expect(await token.balanceOf(escrow.target)).to.equal(amount1 + amount2);
  });

  /// 4. Test: Escrow Expiry Timestamp
  /// Verifies that escrow expiry is set correctly (current time + EXPIRY_DURATION).
  /// Why: Expiry must be correct for time-based cancel functionality.
  it("Should set correct expiry timestamp", async function () {
    const amount = ethers.parseEther("100");
    await token.mint(requester.address, amount);
    await token.connect(requester).approve(escrow.target, amount);

    const tx = await escrow.connect(requester).createEscrow(intentId, token.target, amount, solver.address);
    const receipt = await tx.wait();
    const block = await ethers.provider.getBlock(receipt.blockNumber);
    
    const expiryDuration = await escrow.EXPIRY_DURATION();
    const expectedExpiry = BigInt(block.timestamp) + expiryDuration;

    const escrowData = await escrow.getEscrow(intentId);
    expect(escrowData.expiry).to.equal(expectedExpiry);
  });
});

