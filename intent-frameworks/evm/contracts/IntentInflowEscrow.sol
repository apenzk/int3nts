// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import "./gmp-common/Messages.sol";
import "./IntentGmp.sol";

/// @title IntentInflowEscrow
/// @notice Inflow escrow for GMP-based cross-chain intents
/// @dev Handles inflow: tokens locked on EVM connected chain, desired on hub
contract IntentInflowEscrow is IMessageHandler, Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    // ============================================================================
    // ERRORS
    // ============================================================================

    /// @notice Caller is not the GMP endpoint
    error E_UNAUTHORIZED_ENDPOINT();
    /// @notice Invalid source chain
    error E_INVALID_SOURCE_CHAIN();
    /// @notice Invalid source address
    error E_INVALID_SOURCE_ADDRESS();
    /// @notice Requirements already stored (idempotent)
    error E_REQUIREMENTS_ALREADY_STORED();
    /// @notice Requirements not found for intent
    error E_REQUIREMENTS_NOT_FOUND();
    /// @notice Escrow already created for intent
    error E_ESCROW_ALREADY_CREATED();
    /// @notice Intent has expired
    error E_INTENT_EXPIRED();
    /// @notice Amount does not match requirements
    error E_AMOUNT_MISMATCH();
    /// @notice Token does not match requirements
    error E_TOKEN_MISMATCH();
    /// @notice Requester does not match requirements
    error E_REQUESTER_MISMATCH();
    /// @notice Escrow not found
    error E_ESCROW_NOT_FOUND();
    /// @notice Already fulfilled
    error E_ALREADY_FULFILLED();
    /// @notice Already released (via fulfillment or cancel)
    error E_ALREADY_RELEASED();
    /// @notice Invalid address
    error E_INVALID_ADDRESS();
    /// @notice Amount is zero
    error E_ZERO_AMOUNT();
    /// @notice Escrow has not expired yet
    error E_ESCROW_NOT_EXPIRED();
    /// @notice Caller is not the requester or admin
    error E_UNAUTHORIZED_CALLER();

    // ============================================================================
    // EVENTS
    // ============================================================================

    /// @notice Emitted when IntentRequirements is received from hub
    event IntentRequirementsReceived(
        bytes32 indexed intentId,
        uint32 srcChainId,
        bytes32 requesterAddr,
        uint64 amountRequired,
        bytes32 tokenAddr,
        bytes32 solverAddr,
        uint64 expiry
    );

    /// @notice Emitted when duplicate requirements received (idempotent)
    event IntentRequirementsDuplicate(bytes32 indexed intentId);

    /// @notice Emitted when escrow is created
    event EscrowCreated(
        bytes32 indexed intentId,
        bytes32 escrowId,
        address indexed requester,
        uint64 amount,
        address indexed token,
        bytes32 reservedSolver,
        uint64 expiry
    );

    /// @notice Emitted when EscrowConfirmation is sent to hub
    event EscrowConfirmationSent(
        bytes32 indexed intentId,
        bytes32 escrowId,
        uint64 amountEscrowed,
        uint32 dstChainId
    );

    /// @notice Emitted when FulfillmentProof is received
    event FulfillmentProofReceived(
        bytes32 indexed intentId,
        uint32 srcChainId,
        bytes32 solverAddr,
        uint64 amountFulfilled,
        uint64 timestamp
    );

    /// @notice Emitted when escrow is released to solver
    event EscrowReleased(
        bytes32 indexed intentId,
        address indexed solver,
        uint64 amount
    );

    /// @notice Emitted when escrow is cancelled and funds returned to requester
    event EscrowCancelled(
        bytes32 indexed intentId,
        address indexed requester,
        uint64 amount
    );

    // ============================================================================
    // STRUCTS
    // ============================================================================

    /// @notice Stored requirements from hub
    struct StoredRequirements {
        bytes32 requesterAddr;
        uint64 amountRequired;
        bytes32 tokenAddr;
        bytes32 solverAddr;
        uint64 expiry;
        bool escrowCreated;
    }

    /// @notice Stored escrow info
    struct StoredEscrow {
        bytes32 escrowId;
        bytes32 creatorAddr;
        uint64 amount;
        address token;
        bytes32 solverAddr;
        bool fulfilled;
        bool released;
    }

    // ============================================================================
    // STATE
    // ============================================================================

    /// @notice GMP endpoint address
    address public gmpEndpoint;

    /// @notice Hub chain ID
    uint32 public hubChainId;

    /// @notice Hub GMP endpoint address (32 bytes)
    bytes32 public hubGmpEndpointAddr;

    /// @notice Stored requirements (intentId => requirements)
    mapping(bytes32 => StoredRequirements) public requirements;

    /// @notice Whether requirements exist for an intent
    mapping(bytes32 => bool) public hasRequirements;

    /// @notice Stored escrows (intentId => escrow)
    mapping(bytes32 => StoredEscrow) public escrows;

    /// @notice Whether escrow exists for an intent
    mapping(bytes32 => bool) public hasEscrow;

    // ============================================================================
    // CONSTRUCTOR
    // ============================================================================

    /// @notice Initialize the inflow escrow GMP
    /// @param admin Admin/owner address
    /// @param _gmpEndpoint GMP endpoint address
    /// @param _hubChainId Hub chain endpoint ID
    /// @param _hubGmpEndpointAddr Hub GMP endpoint address (32 bytes)
    constructor(
        address admin,
        address _gmpEndpoint,
        uint32 _hubChainId,
        bytes32 _hubGmpEndpointAddr
    ) Ownable(admin) {
        if (_gmpEndpoint == address(0)) revert E_INVALID_ADDRESS();
        gmpEndpoint = _gmpEndpoint;
        hubChainId = _hubChainId;
        hubGmpEndpointAddr = _hubGmpEndpointAddr;
    }

    // ============================================================================
    // MODIFIERS
    // ============================================================================

    /// @notice Only the GMP endpoint can call
    modifier onlyGmpEndpoint() {
        if (msg.sender != gmpEndpoint) revert E_UNAUTHORIZED_ENDPOINT();
        _;
    }

    // ============================================================================
    // ADMIN FUNCTIONS
    // ============================================================================

    /// @notice Update hub configuration
    /// @param _hubChainId New hub chain ID
    /// @param _hubGmpEndpointAddr New hub GMP endpoint address
    function updateHubConfig(
        uint32 _hubChainId,
        bytes32 _hubGmpEndpointAddr
    ) external onlyOwner {
        hubChainId = _hubChainId;
        hubGmpEndpointAddr = _hubGmpEndpointAddr;
    }

    /// @notice Update GMP endpoint
    /// @param _gmpEndpoint New GMP endpoint address
    function setGmpEndpoint(address _gmpEndpoint) external onlyOwner {
        if (_gmpEndpoint == address(0)) revert E_INVALID_ADDRESS();
        gmpEndpoint = _gmpEndpoint;
    }

    // ============================================================================
    // INBOUND: Hub -> Connected Chain (IntentRequirements)
    // ============================================================================

    /// @notice Receive IntentRequirements from hub
    /// @dev Called by GMP endpoint when message is delivered
    /// @param srcChainId Source chain endpoint ID
    /// @param remoteGmpEndpointAddr Source address (32 bytes)
    /// @param payload Encoded IntentRequirements
    function receiveIntentRequirements(
        uint32 srcChainId,
        bytes32 remoteGmpEndpointAddr,
        bytes calldata payload
    ) external override onlyGmpEndpoint {
        // Verify source
        if (srcChainId != hubChainId) revert E_INVALID_SOURCE_CHAIN();
        if (remoteGmpEndpointAddr != hubGmpEndpointAddr) revert E_INVALID_SOURCE_ADDRESS();

        // Decode message
        Messages.IntentRequirements memory msg_ = Messages.decodeIntentRequirements(payload);

        // Idempotency check
        if (hasRequirements[msg_.intentId]) {
            emit IntentRequirementsDuplicate(msg_.intentId);
            return;
        }

        // Store requirements
        requirements[msg_.intentId] = StoredRequirements({
            requesterAddr: msg_.requesterAddr,
            amountRequired: msg_.amountRequired,
            tokenAddr: msg_.tokenAddr,
            solverAddr: msg_.solverAddr,
            expiry: msg_.expiry,
            escrowCreated: false
        });
        hasRequirements[msg_.intentId] = true;

        emit IntentRequirementsReceived(
            msg_.intentId,
            srcChainId,
            msg_.requesterAddr,
            msg_.amountRequired,
            msg_.tokenAddr,
            msg_.solverAddr,
            msg_.expiry
        );
    }

    // ============================================================================
    // ESCROW CREATION
    // ============================================================================

    /// @notice Create escrow with validation against stored requirements
    /// @dev Validates amount, token, requester, and expiry
    /// @param intentId 32-byte intent identifier
    /// @param token Token address to escrow
    /// @param amount Amount of tokens to escrow
    function createEscrowWithValidation(
        bytes32 intentId,
        address token,
        uint64 amount
    ) external nonReentrant {
        // Verify requirements exist
        if (!hasRequirements[intentId]) revert E_REQUIREMENTS_NOT_FOUND();

        StoredRequirements storage req = requirements[intentId];

        // Verify escrow not already created
        if (req.escrowCreated) revert E_ESCROW_ALREADY_CREATED();

        // Verify not expired
        if (block.timestamp > req.expiry) revert E_INTENT_EXPIRED();

        // Verify amount matches
        if (amount != req.amountRequired) revert E_AMOUNT_MISMATCH();
        if (amount == 0) revert E_ZERO_AMOUNT();

        // Verify token matches
        bytes32 tokenAddr32 = Messages.addressToBytes32(token);
        if (tokenAddr32 != req.tokenAddr) revert E_TOKEN_MISMATCH();

        // Verify requester matches
        bytes32 requesterAddr32 = Messages.addressToBytes32(msg.sender);
        if (requesterAddr32 != req.requesterAddr) revert E_REQUESTER_MISMATCH();

        // Generate escrow ID
        bytes32 escrowId = keccak256(abi.encodePacked(intentId, msg.sender));

        // Mark requirements as having escrow
        req.escrowCreated = true;

        // Store escrow
        escrows[intentId] = StoredEscrow({
            escrowId: escrowId,
            creatorAddr: requesterAddr32,
            amount: amount,
            token: token,
            solverAddr: req.solverAddr,
            fulfilled: false,
            released: false
        });
        hasEscrow[intentId] = true;

        // Transfer tokens from creator to this contract
        IERC20(token).safeTransferFrom(msg.sender, address(this), amount);

        emit EscrowCreated(intentId, escrowId, msg.sender, amount, token, req.solverAddr, req.expiry);

        // Send EscrowConfirmation to hub
        _sendEscrowConfirmation(intentId, escrowId, amount, req.tokenAddr, requesterAddr32);
    }

    /// @notice Send EscrowConfirmation to hub via GMP
    function _sendEscrowConfirmation(
        bytes32 intentId,
        bytes32 escrowId,
        uint64 amount,
        bytes32 tokenAddr,
        bytes32 creatorAddr
    ) internal {
        Messages.EscrowConfirmation memory confirmation = Messages.EscrowConfirmation({
            intentId: intentId,
            escrowId: escrowId,
            amountEscrowed: amount,
            tokenAddr: tokenAddr,
            creatorAddr: creatorAddr
        });

        bytes memory payload = Messages.encodeEscrowConfirmation(confirmation);

        IntentGmp(gmpEndpoint).sendMessage(hubChainId, hubGmpEndpointAddr, payload);

        emit EscrowConfirmationSent(intentId, escrowId, amount, hubChainId);
    }

    // ============================================================================
    // INBOUND: Hub -> Connected Chain (FulfillmentProof)
    // ============================================================================

    /// @notice Receive FulfillmentProof and auto-release escrow to solver
    /// @dev Called by GMP endpoint when hub confirms fulfillment
    /// @param srcChainId Source chain endpoint ID
    /// @param remoteGmpEndpointAddr Source address (32 bytes)
    /// @param payload Encoded FulfillmentProof
    function receiveFulfillmentProof(
        uint32 srcChainId,
        bytes32 remoteGmpEndpointAddr,
        bytes calldata payload
    ) external override onlyGmpEndpoint nonReentrant {
        // Verify source
        if (srcChainId != hubChainId) revert E_INVALID_SOURCE_CHAIN();
        if (remoteGmpEndpointAddr != hubGmpEndpointAddr) revert E_INVALID_SOURCE_ADDRESS();

        // Decode message
        Messages.FulfillmentProof memory proof = Messages.decodeFulfillmentProof(payload);

        // Verify escrow exists
        if (!hasEscrow[proof.intentId]) revert E_ESCROW_NOT_FOUND();

        StoredEscrow storage escrow = escrows[proof.intentId];

        // Verify not already released (via fulfillment or cancel)
        if (escrow.released) revert E_ALREADY_RELEASED();

        // Mark as fulfilled and released
        escrow.fulfilled = true;
        escrow.released = true;

        // Get solver address from proof
        address solver = Messages.bytes32ToAddress(proof.solverAddr);

        // Transfer tokens to solver
        IERC20(escrow.token).safeTransfer(solver, escrow.amount);

        emit FulfillmentProofReceived(
            proof.intentId,
            srcChainId,
            proof.solverAddr,
            proof.amountFulfilled,
            proof.timestamp
        );

        emit EscrowReleased(proof.intentId, solver, escrow.amount);
    }

    // ============================================================================
    // CANCEL: Admin returns funds to requester after expiry
    // ============================================================================

    /// @notice Cancel escrow and return funds to requester after expiry
    /// @dev Only the admin (owner) can cancel after expiry.
    ///      Funds always return to the original requester.
    /// @param intentId 32-byte intent identifier
    function cancel(bytes32 intentId) external nonReentrant {
        // Verify escrow exists
        if (!hasEscrow[intentId]) revert E_ESCROW_NOT_FOUND();

        StoredEscrow storage escrow = escrows[intentId];

        // Verify not already released (fulfilled or cancelled)
        if (escrow.released) revert E_ALREADY_RELEASED();

        // Verify caller is admin (only admin can cancel expired escrows)
        if (msg.sender != owner()) revert E_UNAUTHORIZED_CALLER();

        // Verify intent has expired
        StoredRequirements storage req = requirements[intentId];
        if (block.timestamp <= req.expiry) revert E_ESCROW_NOT_EXPIRED();

        // Mark as released (prevents double-cancel)
        escrow.released = true;

        // Transfer tokens back to original requester (not the caller)
        address requester = Messages.bytes32ToAddress(escrow.creatorAddr);
        IERC20(escrow.token).safeTransfer(requester, escrow.amount);

        emit EscrowCancelled(intentId, requester, escrow.amount);
    }

    // ============================================================================
    // VIEW FUNCTIONS
    // ============================================================================

    /// @notice Check if escrow is fulfilled
    /// @param intentId Intent identifier
    /// @return True if fulfilled
    function isFulfilled(bytes32 intentId) external view returns (bool) {
        if (!hasEscrow[intentId]) return false;
        return escrows[intentId].fulfilled;
    }

    /// @notice Check if escrow is released
    /// @param intentId Intent identifier
    /// @return True if released
    function isReleased(bytes32 intentId) external view returns (bool) {
        if (!hasEscrow[intentId]) return false;
        return escrows[intentId].released;
    }

    /// @notice Check if escrow is cancelled (released but not fulfilled)
    /// @param intentId Intent identifier
    /// @return True if cancelled
    function isCancelled(bytes32 intentId) external view returns (bool) {
        if (!hasEscrow[intentId]) return false;
        StoredEscrow storage escrow = escrows[intentId];
        return escrow.released && !escrow.fulfilled;
    }

    /// @notice Get amount required for an intent
    /// @param intentId Intent identifier
    /// @return Amount required
    function getAmountRequired(bytes32 intentId) external view returns (uint64) {
        return requirements[intentId].amountRequired;
    }

    /// @notice Get escrow details
    /// @param intentId Intent identifier
    /// @return Escrow details
    function getEscrow(bytes32 intentId) external view returns (StoredEscrow memory) {
        return escrows[intentId];
    }

    /// @notice Get requirements details
    /// @param intentId Intent identifier
    /// @return Requirements details
    function getRequirements(bytes32 intentId) external view returns (StoredRequirements memory) {
        return requirements[intentId];
    }
}
