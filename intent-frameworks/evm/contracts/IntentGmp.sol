// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "./gmp-common/Messages.sol";

/// @notice Interface for GMP message handlers
interface IMessageHandler {
    function receiveIntentRequirements(
        uint32 srcChainId,
        bytes32 srcAddr,
        bytes calldata payload
    ) external;

    function receiveFulfillmentProof(
        uint32 srcChainId,
        bytes32 srcAddr,
        bytes calldata payload
    ) external;
}

/// @title IntentGmp
/// @notice GMP endpoint for cross-chain message delivery and routing on EVM connected chains
/// @dev Handles inbound message delivery from relays and routes to escrow/outflow handlers
contract IntentGmp is Ownable, ReentrancyGuard {
    // ============================================================================
    // ERRORS
    // ============================================================================

    /// @notice Caller is not an authorized relay
    error E_UNAUTHORIZED_RELAY();
    /// @notice Message already delivered (duplicate delivery)
    error E_ALREADY_DELIVERED();
    /// @notice Source address is not trusted for the given chain
    error E_UNTRUSTED_REMOTE();
    /// @notice No trusted remote configured for the source chain
    error E_NO_TRUSTED_REMOTE();
    /// @notice Unknown message type in payload
    error E_UNKNOWN_MESSAGE_TYPE();
    /// @notice Invalid address (zero address)
    error E_INVALID_ADDRESS();
    /// @notice Handler not configured
    error E_HANDLER_NOT_CONFIGURED();
    /// @notice Address already in set
    error E_ALREADY_EXISTS();
    /// @notice Address not in set
    error E_NOT_FOUND();
    /// @notice Payload too short to extract intent_id
    error E_INVALID_PAYLOAD();

    // ============================================================================
    // MESSAGE TYPE CONSTANTS (from GmpTypes)
    // ============================================================================

    uint8 private constant MESSAGE_TYPE_INTENT_REQUIREMENTS = 0x01;
    uint8 private constant MESSAGE_TYPE_ESCROW_CONFIRMATION = 0x02;
    uint8 private constant MESSAGE_TYPE_FULFILLMENT_PROOF = 0x03;

    // ============================================================================
    // EVENTS
    // ============================================================================

    /// @notice Emitted when a message is delivered from another chain
    event MessageDelivered(
        uint32 indexed srcChainId,
        bytes32 srcAddr,
        bytes payload,
        bytes32 intentId
    );

    /// @notice Emitted when a message is sent to another chain
    event MessageSent(
        uint32 indexed dstChainId,
        bytes32 dstAddr,
        bytes payload,
        uint64 nonce
    );

    /// @notice Emitted when a relay is added
    event RelayAdded(address indexed relay);

    /// @notice Emitted when a relay is removed
    event RelayRemoved(address indexed relay);

    /// @notice Emitted when a trusted remote is set
    event TrustedRemoteSet(uint32 indexed chainId, bytes32 trustedAddr);

    /// @notice Emitted when a trusted remote is added
    event TrustedRemoteAdded(uint32 indexed chainId, bytes32 trustedAddr);

    /// @notice Emitted when handler is updated
    event EscrowHandlerSet(address indexed handler);

    /// @notice Emitted when handler is updated
    event OutflowHandlerSet(address indexed handler);

    // ============================================================================
    // STATE
    // ============================================================================

    /// @notice Authorized relay addresses that can call deliverMessage
    mapping(address => bool) public authorizedRelays;

    /// @notice Trusted remote addresses per source chain (chainId => list of trusted 32-byte addresses)
    mapping(uint32 => bytes32[]) private trustedRemotes;

    /// @notice Delivered messages: keccak256(intentId, msgType) => true.
    /// Replaces sequential nonce tracking — immune to contract redeployments.
    mapping(bytes32 => bool) public deliveredMessages;

    /// @notice Next outbound nonce for sending messages
    uint64 public nextOutboundNonce;

    /// @notice Escrow handler contract (receives IntentRequirements and FulfillmentProof)
    address public escrowHandler;

    /// @notice Outflow validator contract (receives IntentRequirements)
    address public outflowHandler;

    // ============================================================================
    // CONSTRUCTOR
    // ============================================================================

    /// @notice Initialize the GMP endpoint
    /// @param admin Initial admin/owner address (also initial authorized relay)
    constructor(address admin) Ownable(admin) {
        if (admin == address(0)) revert E_INVALID_ADDRESS();
        authorizedRelays[admin] = true;
        nextOutboundNonce = 1;
        emit RelayAdded(admin);
    }

    // ============================================================================
    // INBOUND: Deliver message from another chain
    // ============================================================================

    /// @notice Deliver a cross-chain message from another chain
    /// @dev Called by authorized relays after observing MessageSent on source chain.
    ///      Deduplication uses (intent_id, msg_type) extracted from the payload,
    ///      making delivery immune to contract redeployments (unlike sequential nonces).
    /// @param srcChainId Source chain endpoint ID
    /// @param srcAddr Source address (32 bytes)
    /// @param payload Message payload (encoded GMP message)
    function deliverMessage(
        uint32 srcChainId,
        bytes32 srcAddr,
        bytes calldata payload
    ) external nonReentrant {
        // Verify relay is authorized
        if (!authorizedRelays[msg.sender]) revert E_UNAUTHORIZED_RELAY();

        // Verify trusted remote
        bytes32[] storage trusted = trustedRemotes[srcChainId];
        if (trusted.length == 0) revert E_NO_TRUSTED_REMOTE();
        if (!_isTrustedAddress(trusted, srcAddr)) revert E_UNTRUSTED_REMOTE();

        // Extract intent_id and msg_type from payload for dedup
        // All GMP messages: msg_type (1 byte) + intent_id (32 bytes) at the start
        if (payload.length < 33) revert E_INVALID_PAYLOAD();
        uint8 msgType = uint8(payload[0]);
        bytes32 intentId;
        assembly {
            // payload.offset points to start of calldata payload
            // intent_id starts at byte 1 of payload
            intentId := calldataload(add(payload.offset, 1))
        }

        // Replay protection: deduplicate by (intentId, msgType)
        bytes32 dedupeKey = keccak256(abi.encodePacked(intentId, msgType));
        if (deliveredMessages[dedupeKey]) revert E_ALREADY_DELIVERED();
        deliveredMessages[dedupeKey] = true;

        // Emit delivery event
        emit MessageDelivered(srcChainId, srcAddr, payload, intentId);

        // Route message based on type
        _routeMessage(srcChainId, srcAddr, payload);
    }

    /// @notice Route a GMP message to the appropriate handler based on payload type
    /// @dev Connected chain receives IntentRequirements (0x01) and FulfillmentProof (0x03)
    function _routeMessage(
        uint32 srcChainId,
        bytes32 srcAddr,
        bytes calldata payload
    ) internal {
        uint8 msgType = Messages.peekMessageType(payload);

        if (msgType == MESSAGE_TYPE_INTENT_REQUIREMENTS) {
            // Route to both escrow and outflow handlers
            if (escrowHandler != address(0)) {
                IMessageHandler(escrowHandler).receiveIntentRequirements(
                    srcChainId,
                    srcAddr,
                    payload
                );
            }
            if (outflowHandler != address(0)) {
                IMessageHandler(outflowHandler).receiveIntentRequirements(
                    srcChainId,
                    srcAddr,
                    payload
                );
            }
        } else if (msgType == MESSAGE_TYPE_FULFILLMENT_PROOF) {
            // Route to escrow handler only (for inflow auto-release)
            if (escrowHandler == address(0)) revert E_HANDLER_NOT_CONFIGURED();
            IMessageHandler(escrowHandler).receiveFulfillmentProof(
                srcChainId,
                srcAddr,
                payload
            );
        } else {
            // Connected chain should NOT receive EscrowConfirmation (0x02)
            revert E_UNKNOWN_MESSAGE_TYPE();
        }
    }

    // ============================================================================
    // OUTBOUND: Send message to another chain
    // ============================================================================

    /// @notice Send a message to another chain
    /// @dev Emits MessageSent event for relays to observe
    /// @param dstChainId Destination chain endpoint ID
    /// @param dstAddr Destination address (32 bytes)
    /// @param payload Message payload (encoded GMP message)
    /// @return nonce The nonce assigned to this message
    function sendMessage(
        uint32 dstChainId,
        bytes32 dstAddr,
        bytes calldata payload
    ) external returns (uint64 nonce) {
        // Only handlers can send messages
        require(
            msg.sender == escrowHandler || msg.sender == outflowHandler,
            "Only handlers can send"
        );

        nonce = nextOutboundNonce++;

        emit MessageSent(dstChainId, dstAddr, payload, nonce);
    }

    // ============================================================================
    // ADMIN FUNCTIONS
    // ============================================================================

    /// @notice Set a trusted remote address for a source chain (replaces all existing)
    /// @param srcChainId Source chain endpoint ID
    /// @param trustedAddr Trusted source address (32 bytes)
    function setTrustedRemote(
        uint32 srcChainId,
        bytes32 trustedAddr
    ) external onlyOwner {
        delete trustedRemotes[srcChainId];
        trustedRemotes[srcChainId].push(trustedAddr);
        emit TrustedRemoteSet(srcChainId, trustedAddr);
    }

    /// @notice Add a trusted remote address for a source chain
    /// @param srcChainId Source chain endpoint ID
    /// @param trustedAddr Trusted source address (32 bytes) to add
    function addTrustedRemote(
        uint32 srcChainId,
        bytes32 trustedAddr
    ) external onlyOwner {
        bytes32[] storage trusted = trustedRemotes[srcChainId];
        if (_isTrustedAddress(trusted, trustedAddr)) revert E_ALREADY_EXISTS();
        trusted.push(trustedAddr);
        emit TrustedRemoteAdded(srcChainId, trustedAddr);
    }

    /// @notice Add an authorized relay
    /// @param relay Address to authorize as relay
    function addRelay(address relay) external onlyOwner {
        if (relay == address(0)) revert E_INVALID_ADDRESS();
        if (authorizedRelays[relay]) revert E_ALREADY_EXISTS();
        authorizedRelays[relay] = true;
        emit RelayAdded(relay);
    }

    /// @notice Remove an authorized relay
    /// @param relay Address to remove from authorized relays
    function removeRelay(address relay) external onlyOwner {
        if (!authorizedRelays[relay]) revert E_NOT_FOUND();
        authorizedRelays[relay] = false;
        emit RelayRemoved(relay);
    }

    /// @notice Set the escrow handler contract
    /// @param handler Address of the escrow handler
    function setEscrowHandler(address handler) external onlyOwner {
        escrowHandler = handler;
        emit EscrowHandlerSet(handler);
    }

    /// @notice Set the outflow handler contract
    /// @param handler Address of the outflow handler
    function setOutflowHandler(address handler) external onlyOwner {
        outflowHandler = handler;
        emit OutflowHandlerSet(handler);
    }

    // ============================================================================
    // VIEW FUNCTIONS
    // ============================================================================

    /// @notice Check if an address is an authorized relay
    /// @param addr Address to check
    /// @return True if authorized
    function isRelayAuthorized(address addr) external view returns (bool) {
        return authorizedRelays[addr];
    }

    /// @notice Get the trusted remote addresses for a source chain
    /// @param srcChainId Source chain endpoint ID
    /// @return List of trusted addresses
    function getTrustedRemotes(uint32 srcChainId) external view returns (bytes32[] memory) {
        return trustedRemotes[srcChainId];
    }

    /// @notice Check if a source chain has any trusted remotes configured
    /// @param srcChainId Source chain endpoint ID
    /// @return True if at least one trusted remote is configured
    function hasTrustedRemote(uint32 srcChainId) external view returns (bool) {
        return trustedRemotes[srcChainId].length > 0;
    }

    /// @notice Check if a specific message has been delivered
    /// @param intentId The intent ID (32 bytes)
    /// @param msgType The message type (0x01, 0x02, or 0x03)
    /// @return True if the message has been delivered
    function isMessageDelivered(bytes32 intentId, uint8 msgType) external view returns (bool) {
        bytes32 dedupeKey = keccak256(abi.encodePacked(intentId, msgType));
        return deliveredMessages[dedupeKey];
    }

    // ============================================================================
    // INTERNAL HELPERS
    // ============================================================================

    /// @notice Check if an address is in the trusted addresses list
    function _isTrustedAddress(
        bytes32[] storage addrs,
        bytes32 addr
    ) internal view returns (bool) {
        uint256 len = addrs.length;
        for (uint256 i = 0; i < len; i++) {
            if (addrs[i] == addr) {
                return true;
            }
        }
        return false;
    }
}
