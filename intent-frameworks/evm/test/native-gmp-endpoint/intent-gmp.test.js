const { expect } = require("chai");
const { ethers } = require("hardhat");

describe("IntentGmp", function () {
  let gmpEndpoint;
  let admin;
  let relay;
  let user;
  let mockHandler;

  // Test chain IDs
  const MOVEMENT_CHAIN_ID = 30325;

  // Test addresses (32 bytes)
  const TRUSTED_REMOTE = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
  const UNTRUSTED_REMOTE = "0x9900000000000000000000000000000000000000000000000000000000000099";

  // Valid IntentRequirements payload (145 bytes)
  const VALID_PAYLOAD = "0x01" + "00".repeat(144);

  before(async function () {
    [admin, relay, user] = await ethers.getSigners();
  });

  beforeEach(async function () {
    // Deploy IntentGmp
    const IntentGmp = await ethers.getContractFactory("IntentGmp");
    gmpEndpoint = await IntentGmp.deploy(admin.address);
    await gmpEndpoint.waitForDeployment();

    // Deploy a mock handler for testing
    const MockHandler = await ethers.getContractFactory("MockMessageHandler");
    mockHandler = await MockHandler.deploy();
    await mockHandler.waitForDeployment();

    // Configure endpoint
    await gmpEndpoint.setEscrowHandler(mockHandler.target);
    await gmpEndpoint.setTrustedRemote(MOVEMENT_CHAIN_ID, TRUSTED_REMOTE);
  });

  // ============================================================================
  // Shared Cross-VM Tests (15-26)
  // ============================================================================

  describe("Message Sending", function () {
    /// 15. Test: test_send_updates_nonce_state: Send Updates Nonce State
    /// Verifies outbound nonce increments after send.
    /// Why: Each message must have unique nonce.
    it("should increment outbound nonce on send", async function () {
      const initialNonce = await gmpEndpoint.nextOutboundNonce();

      // Send from handler
      const payload = "0x02" + "00".repeat(136);
      await mockHandler.callSendMessage(
        gmpEndpoint.target,
        MOVEMENT_CHAIN_ID,
        TRUSTED_REMOTE,
        payload
      );

      expect(await gmpEndpoint.nextOutboundNonce()).to.equal(initialNonce + 1n);
    });
  });

  describe("Message Delivery", function () {
    /// 16. Test: test_deliver_message_calls_receiver: Deliver Message Calls Handler
    /// Verifies delivered message is routed to handler.
    /// Why: Message routing is core functionality.
    it("should route IntentRequirements to escrow handler", async function () {
      await gmpEndpoint.deliverMessage(
        MOVEMENT_CHAIN_ID,
        TRUSTED_REMOTE,
        VALID_PAYLOAD
      );

      // Check that handler received the message
      expect(await mockHandler.lastReceivedChainId()).to.equal(MOVEMENT_CHAIN_ID);
      expect(await mockHandler.lastReceivedSrcAddr()).to.equal(TRUSTED_REMOTE);
      expect(await mockHandler.requirementsReceived()).to.equal(true);
    });

    /// 17. Test: test_deliver_message_rejects_replay: Deliver Message Rejects Replay
    /// Verifies duplicate (intent_id, msg_type) is rejected.
    /// Why: Replay protection prevents double-processing.
    it("should reject duplicate delivery of same intent_id and msg_type", async function () {
      await gmpEndpoint.deliverMessage(
        MOVEMENT_CHAIN_ID,
        TRUSTED_REMOTE,
        VALID_PAYLOAD
      );

      await expect(
        gmpEndpoint.deliverMessage(
          MOVEMENT_CHAIN_ID,
          TRUSTED_REMOTE,
          VALID_PAYLOAD
        )
      ).to.be.revertedWithCustomError(gmpEndpoint, "E_ALREADY_DELIVERED");
    });

    /// 18. Test: test_deliver_message_rejects_unauthorized_relay: Deliver Message Rejects Unauthorized Relay
    /// Verifies unauthorized caller cannot deliver.
    /// Why: Only authorized relays should deliver messages.
    it("should reject delivery from unauthorized relay", async function () {
      await expect(
        gmpEndpoint.connect(user).deliverMessage(
          MOVEMENT_CHAIN_ID,
          TRUSTED_REMOTE,
          VALID_PAYLOAD
        )
      ).to.be.revertedWithCustomError(gmpEndpoint, "E_UNAUTHORIZED_RELAY");
    });

    /// 19. Test: test_deliver_message_authorized_relay: Deliver Message Authorized Relay
    /// Verifies authorized relay can deliver.
    /// Why: Authorized relays should be able to deliver.
    it("should allow delivery from authorized relay", async function () {
      await gmpEndpoint.addRelay(relay.address);

      await expect(
        gmpEndpoint.connect(relay).deliverMessage(
          MOVEMENT_CHAIN_ID,
          TRUSTED_REMOTE,
          VALID_PAYLOAD
        )
      ).to.emit(gmpEndpoint, "MessageDelivered");
    });

    /// 20. Test: test_deliver_message_rejects_untrusted_remote: Deliver Message Rejects Untrusted Remote
    /// Verifies untrusted source address is rejected.
    /// Why: Only trusted sources should be accepted.
    it("should reject delivery from untrusted remote", async function () {
      await expect(
        gmpEndpoint.deliverMessage(
          MOVEMENT_CHAIN_ID,
          UNTRUSTED_REMOTE,
          VALID_PAYLOAD
        )
      ).to.be.revertedWithCustomError(gmpEndpoint, "E_UNTRUSTED_REMOTE");
    });

    /// 21. Test: test_deliver_message_rejects_no_trusted_remote: Deliver Message Rejects No Trusted Remote
    /// Verifies delivery fails for unconfigured chain.
    /// Why: No trusted remote means no trusted source.
    it("should reject delivery for unconfigured chain", async function () {
      const unconfiguredChainId = 99999;

      await expect(
        gmpEndpoint.deliverMessage(
          unconfiguredChainId,
          TRUSTED_REMOTE,
          VALID_PAYLOAD
        )
      ).to.be.revertedWithCustomError(gmpEndpoint, "E_NO_TRUSTED_REMOTE");
    });
  });

  describe("Trusted Remote Configuration", function () {
    /// 22. Test: test_set_trusted_remote_unauthorized: Set Trusted Remote Unauthorized
    /// Verifies only admin can set trusted remote.
    /// Why: Trusted remote configuration is security-critical.
    it("should reject non-admin setting trusted remote", async function () {
      await expect(
        gmpEndpoint.connect(user).setTrustedRemote(MOVEMENT_CHAIN_ID, TRUSTED_REMOTE)
      ).to.be.revertedWithCustomError(gmpEndpoint, "OwnableUnauthorizedAccount");
    });
  });

  describe("Message Delivery Continued", function () {
    /// 23. Test: test_deliver_different_msg_type_succeeds: Different Msg Type Succeeds
    /// Verifies same intent_id with different msg_type is NOT a duplicate.
    /// Why: Each (intent_id, msg_type) pair is independently deliverable.
    it("should allow same intent_id with different msg_type", async function () {
      // Deliver IntentRequirements (msg_type 0x01) - 145 bytes
      await gmpEndpoint.deliverMessage(
        MOVEMENT_CHAIN_ID,
        TRUSTED_REMOTE,
        VALID_PAYLOAD
      );

      // Deliver FulfillmentProof (msg_type 0x03) with same intent_id (32 zero bytes) - 81 bytes
      const fulfillmentPayload = "0x03" + "00".repeat(80);
      await gmpEndpoint.deliverMessage(
        MOVEMENT_CHAIN_ID,
        TRUSTED_REMOTE,
        fulfillmentPayload
      );

      // Both should be marked as delivered
      const intentId = ethers.ZeroHash;
      expect(await gmpEndpoint.isMessageDelivered(intentId, 0x01)).to.equal(true);
      expect(await gmpEndpoint.isMessageDelivered(intentId, 0x03)).to.equal(true);
    });
  });

  describe("Relay Authorization", function () {
    /// 25. Test: test_add_authorized_relay_rejects_non_admin: Reject Non-Admin Add Relay
    /// Verifies only admin can add relays.
    /// Why: Relay authorization is security-critical.
    it("should reject non-admin adding relay", async function () {
      await expect(
        gmpEndpoint.connect(user).addRelay(relay.address)
      ).to.be.revertedWithCustomError(gmpEndpoint, "OwnableUnauthorizedAccount");
    });

    /// 26. Test: test_remove_authorized_relay_rejects_non_admin: Reject Non-Admin Remove Relay
    /// Verifies only admin can remove relays.
    /// Why: Relay authorization is security-critical; must be admin-only.
    it("should reject non-admin removing relay", async function () {
      await gmpEndpoint.addRelay(relay.address);
      await expect(
        gmpEndpoint.connect(user).removeRelay(relay.address)
      ).to.be.revertedWithCustomError(gmpEndpoint, "OwnableUnauthorizedAccount");
    });
  });

  // ============================================================================
  // EVM-Specific Tests (30-50)
  // ============================================================================

  // ============================================================================
  // Initialization
  // ============================================================================

  describe("Initialization", function () {
    /// 30. Test: test_initialize_creates_config: Initialize Creates Config
    /// Verifies admin is set as initial authorized relay.
    /// Why: Admin must be able to deliver messages during setup.
    it("should set admin as authorized relay on deploy", async function () {
      expect(await gmpEndpoint.isRelayAuthorized(admin.address)).to.equal(true);
    });

    /// 31. Test: test_initialize_sets_nonce: Initialize Sets Nonce
    /// Verifies outbound nonce starts at 1.
    /// Why: First message should have nonce 1, not 0.
    it("should start with outbound nonce of 1", async function () {
      expect(await gmpEndpoint.nextOutboundNonce()).to.equal(1);
    });

    /// 32. Test: test_initialize_rejects_zero_admin: Initialize Rejects Zero Admin
    /// Verifies deployment fails with zero admin address.
    /// Why: Zero address cannot be admin.
    it("should reject zero admin address", async function () {
      const IntentGmp = await ethers.getContractFactory("IntentGmp");
      await expect(
        IntentGmp.deploy(ethers.ZeroAddress)
      ).to.be.revertedWithCustomError(gmpEndpoint, "OwnableInvalidOwner");
    });
  });

  // ============================================================================
  // Relay Authorization (EVM-Specific)
  // ============================================================================

  describe("Relay Authorization (EVM-Specific)", function () {
    /// 33. Test: test_add_relay: Add Relay
    /// Verifies authorized relays can be added.
    /// Why: Multiple relays may be needed for redundancy.
    it("should allow admin to add relay", async function () {
      await expect(gmpEndpoint.addRelay(relay.address))
        .to.emit(gmpEndpoint, "RelayAdded")
        .withArgs(relay.address);

      expect(await gmpEndpoint.isRelayAuthorized(relay.address)).to.equal(true);
    });

    /// 34. Test: test_remove_relay: Remove Relay
    /// Verifies authorized relays can be removed.
    /// Why: Compromised relays must be removable.
    it("should allow admin to remove relay", async function () {
      await gmpEndpoint.addRelay(relay.address);

      await expect(gmpEndpoint.removeRelay(relay.address))
        .to.emit(gmpEndpoint, "RelayRemoved")
        .withArgs(relay.address);

      expect(await gmpEndpoint.isRelayAuthorized(relay.address)).to.equal(false);
    });

    /// 35. Test: test_reject_duplicate_relay: Reject Duplicate Relay
    /// Verifies adding existing relay fails.
    /// Why: Prevents confusion in relay management.
    it("should reject adding duplicate relay", async function () {
      await gmpEndpoint.addRelay(relay.address);
      await expect(
        gmpEndpoint.addRelay(relay.address)
      ).to.be.revertedWithCustomError(gmpEndpoint, "E_ALREADY_EXISTS");
    });

    /// 36. Test: test_reject_removing_non_existent_relay: Reject Removing Non-Existent Relay
    /// Verifies removing non-existent relay fails.
    /// Why: Prevents confusion in relay management.
    it("should reject removing non-existent relay", async function () {
      await expect(
        gmpEndpoint.removeRelay(relay.address)
      ).to.be.revertedWithCustomError(gmpEndpoint, "E_NOT_FOUND");
    });
  });

  // ============================================================================
  // Trusted Remote Configuration (EVM-Specific)
  // ============================================================================

  describe("Trusted Remote Configuration (EVM-Specific)", function () {
    /// 37. Test: test_set_trusted_remote: Set Trusted Remote
    /// Verifies trusted remote can be set.
    /// Why: Only trusted sources should be accepted.
    it("should allow admin to set trusted remote", async function () {
      const newTrusted = "0xaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd";

      await expect(gmpEndpoint.setTrustedRemote(MOVEMENT_CHAIN_ID, newTrusted))
        .to.emit(gmpEndpoint, "TrustedRemoteSet")
        .withArgs(MOVEMENT_CHAIN_ID, newTrusted);

      const remotes = await gmpEndpoint.getTrustedRemotes(MOVEMENT_CHAIN_ID);
      expect(remotes.length).to.equal(1);
      expect(remotes[0]).to.equal(newTrusted);
    });

    /// 38. Test: test_add_trusted_remote: Add Trusted Remote
    /// Verifies multiple trusted remotes can be added.
    /// Why: Connected chains may have multiple trusted programs.
    it("should allow admin to add trusted remote", async function () {
      const secondTrusted = "0xaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccddaabbccdd";

      await expect(gmpEndpoint.addTrustedRemote(MOVEMENT_CHAIN_ID, secondTrusted))
        .to.emit(gmpEndpoint, "TrustedRemoteAdded")
        .withArgs(MOVEMENT_CHAIN_ID, secondTrusted);

      const remotes = await gmpEndpoint.getTrustedRemotes(MOVEMENT_CHAIN_ID);
      expect(remotes.length).to.equal(2);
    });

    /// 39. Test: test_has_trusted_remote: Has Trusted Remote
    /// Verifies hasTrustedRemote returns correct value.
    /// Why: View function for checking configuration.
    it("should return true for configured chain", async function () {
      expect(await gmpEndpoint.hasTrustedRemote(MOVEMENT_CHAIN_ID)).to.equal(true);
    });

    /// 40. Test: test_no_trusted_remote: No Trusted Remote
    /// Verifies hasTrustedRemote returns false for unconfigured chain.
    /// Why: View function for checking configuration.
    it("should return false for unconfigured chain", async function () {
      const unconfiguredChainId = 99999;
      expect(await gmpEndpoint.hasTrustedRemote(unconfiguredChainId)).to.equal(false);
    });
  });

  // ============================================================================
  // Message Delivery (EVM-Specific)
  // ============================================================================

  describe("Message Delivery (EVM-Specific)", function () {
    /// 41. Test: test_deliver_fulfillment_proof_routes: Deliver FulfillmentProof Routes to Escrow Handler
    /// Verifies FulfillmentProof is routed correctly.
    /// Why: FulfillmentProof triggers escrow release.
    it("should route FulfillmentProof to escrow handler", async function () {
      // FulfillmentProof payload (81 bytes)
      const fulfillmentPayload = "0x03" + "00".repeat(80);

      await gmpEndpoint.deliverMessage(
        MOVEMENT_CHAIN_ID,
        TRUSTED_REMOTE,
        fulfillmentPayload
      );

      expect(await mockHandler.fulfillmentReceived()).to.equal(true);
    });

    /// 42. Test: test_reject_unknown_message_type: Reject Unknown Message Type
    /// Verifies unknown message type is rejected.
    /// Why: Connected chain should not receive EscrowConfirmation.
    it("should reject unknown message type", async function () {
      // EscrowConfirmation payload (0x02) - should not be received on connected chain
      const escrowConfirmPayload = "0x02" + "00".repeat(136);

      await expect(
        gmpEndpoint.deliverMessage(
          MOVEMENT_CHAIN_ID,
          TRUSTED_REMOTE,
          escrowConfirmPayload
        )
      ).to.be.reverted;
    });

    /// 43. Test: test_emit_message_delivered: Emit MessageDelivered Event
    /// Verifies delivery emits correct event with intent_id.
    /// Why: Events are used for relay monitoring.
    it("should emit MessageDelivered event", async function () {
      // VALID_PAYLOAD has intent_id = 32 zero bytes at positions 1-32
      const intentId = ethers.ZeroHash;

      await expect(
        gmpEndpoint.deliverMessage(
          MOVEMENT_CHAIN_ID,
          TRUSTED_REMOTE,
          VALID_PAYLOAD
        )
      ).to.emit(gmpEndpoint, "MessageDelivered")
        .withArgs(MOVEMENT_CHAIN_ID, TRUSTED_REMOTE, VALID_PAYLOAD, intentId);
    });

    /// 44. Test: test_is_message_delivered: Is Message Delivered
    /// Verifies isMessageDelivered tracks delivery status.
    /// Why: View function for checking delivery status.
    it("should mark message as delivered", async function () {
      const intentId = ethers.ZeroHash; // 32 zero bytes from VALID_PAYLOAD
      const msgType = 0x01; // IntentRequirements

      expect(await gmpEndpoint.isMessageDelivered(intentId, msgType)).to.equal(false);

      await gmpEndpoint.deliverMessage(
        MOVEMENT_CHAIN_ID,
        TRUSTED_REMOTE,
        VALID_PAYLOAD
      );

      expect(await gmpEndpoint.isMessageDelivered(intentId, msgType)).to.equal(true);
    });
  });

  // ============================================================================
  // Message Sending (EVM-Specific)
  // ============================================================================

  describe("Message Sending (EVM-Specific)", function () {
    /// 45. Test: test_emit_message_sent: Emit MessageSent Event
    /// Verifies send emits correct event.
    /// Why: Events are observed by relays.
    it("should emit MessageSent event", async function () {
      const payload = "0x02" + "00".repeat(136);

      await expect(
        mockHandler.callSendMessage(
          gmpEndpoint.target,
          MOVEMENT_CHAIN_ID,
          TRUSTED_REMOTE,
          payload
        )
      ).to.emit(gmpEndpoint, "MessageSent")
        .withArgs(MOVEMENT_CHAIN_ID, TRUSTED_REMOTE, payload, 1);
    });

    /// 46. Test: test_only_handlers_can_send: Only Handlers Can Send
    /// Verifies non-handlers cannot send messages.
    /// Why: Only registered handlers should send messages.
    it("should reject send from non-handler", async function () {
      const payload = "0x02" + "00".repeat(136);

      await expect(
        gmpEndpoint.sendMessage(
          MOVEMENT_CHAIN_ID,
          TRUSTED_REMOTE,
          payload
        )
      ).to.be.revertedWith("Only handlers can send");
    });
  });

  // ============================================================================
  // Handler Configuration
  // ============================================================================

  describe("Handler Configuration", function () {
    /// 47. Test: test_set_escrow_handler: Set Escrow Handler
    /// Verifies escrow handler can be configured.
    /// Why: Handler routing requires configuration.
    it("should allow admin to set escrow handler", async function () {
      const MockHandler = await ethers.getContractFactory("MockMessageHandler");
      const newHandler = await MockHandler.deploy();

      await expect(gmpEndpoint.setEscrowHandler(newHandler.target))
        .to.emit(gmpEndpoint, "EscrowHandlerSet")
        .withArgs(newHandler.target);

      expect(await gmpEndpoint.escrowHandler()).to.equal(newHandler.target);
    });

    /// 48. Test: test_set_outflow_handler: Set Outflow Handler
    /// Verifies outflow handler can be configured.
    /// Why: Handler routing requires configuration.
    it("should allow admin to set outflow handler", async function () {
      const MockHandler = await ethers.getContractFactory("MockMessageHandler");
      const newHandler = await MockHandler.deploy();

      await expect(gmpEndpoint.setOutflowHandler(newHandler.target))
        .to.emit(gmpEndpoint, "OutflowHandlerSet")
        .withArgs(newHandler.target);

      expect(await gmpEndpoint.outflowHandler()).to.equal(newHandler.target);
    });

    /// 49. Test: test_route_to_both_handlers: Route to Both Handlers
    /// Verifies IntentRequirements routes to both handlers.
    /// Why: Both escrow and outflow need requirements.
    it("should route IntentRequirements to both handlers", async function () {
      const MockHandler = await ethers.getContractFactory("MockMessageHandler");
      const outflowHandler = await MockHandler.deploy();
      await gmpEndpoint.setOutflowHandler(outflowHandler.target);

      await gmpEndpoint.deliverMessage(
        MOVEMENT_CHAIN_ID,
        TRUSTED_REMOTE,
        VALID_PAYLOAD
      );

      expect(await mockHandler.requirementsReceived()).to.equal(true);
      expect(await outflowHandler.requirementsReceived()).to.equal(true);
    });

    /// 50. Test: test_fulfillment_proof_requires_escrow_handler: FulfillmentProof Requires Escrow Handler
    /// Verifies FulfillmentProof fails without escrow handler.
    /// Why: FulfillmentProof must be routed to escrow.
    it("should reject FulfillmentProof without escrow handler", async function () {
      // Remove escrow handler
      await gmpEndpoint.setEscrowHandler(ethers.ZeroAddress);

      const fulfillmentPayload = "0x03" + "00".repeat(80);

      await expect(
        gmpEndpoint.deliverMessage(
          MOVEMENT_CHAIN_ID,
          TRUSTED_REMOTE,
          fulfillmentPayload
        )
      ).to.be.revertedWithCustomError(gmpEndpoint, "E_HANDLER_NOT_CONFIGURED");
    });
  });
});
