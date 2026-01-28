// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/**
 * @title IntentEscrow
 * @notice Escrow that holds funds and releases them to solvers when approver signature checks out
 * @dev Based on Solana escrow pattern with ECDSA signature verification
 */
contract IntentEscrow {
    using SafeERC20 for IERC20;

    /// @notice Authorized approver address that can approve releases
    address public immutable approver;

    /// @notice Contract-defined expiry duration (2 minutes in seconds)
    uint256 public constant EXPIRY_DURATION = 2 minutes;

    /// @notice Escrow data structure
    struct Escrow {
        address requester;           // Requester who deposited funds (requester who created the intent on hub chain)
        address token;           // ERC20 token address
        uint256 amount;          // Amount deposited
        bool isClaimed;          // Whether funds have been claimed
        uint256 expiry;          // Expiry timestamp (contract-defined). After expiry, claims are blocked but requester can cancel
        address reservedSolver;  // Solver address that receives funds when escrow is claimed (always set, never address(0))
    }

    /// @notice Mapping from intent ID to escrow data
    mapping(uint256 => Escrow) public escrows;

    /// @notice Events
    event EscrowInitialized(uint256 indexed intentId, address indexed escrow, address indexed requester, address token, address reservedSolver, uint256 amount, uint256 expiry);
    event EscrowClaimed(uint256 indexed intentId, address indexed recipient, uint256 amount);
    event EscrowCancelled(uint256 indexed intentId, address indexed requester, uint256 amount);

    /// @notice Errors
    error EscrowAlreadyClaimed();
    error EscrowDoesNotExist();
    error NoDeposit();
    error UnauthorizedRequester();
    error InvalidSignature();
    error UnauthorizedApprover();
    error EscrowExpired(); // Escrow has expired (for claim operations)
    error EscrowNotExpiredYet(); // Escrow has not expired yet (for cancel operations)

    /**
     * @notice Initialize the escrow with approver address
     * @param _approver Address of the authorized approver
     */
    constructor(address _approver) {
        require(_approver != address(0), "Invalid approver address");
        approver = _approver;
    }

    /**
     * @notice Create a new escrow and deposit funds atomically (matches Move contract behavior)
     * @param intentId Unique intent identifier
     * @param token ERC20 token address to be deposited (use address(0) for ETH)
     * @param amount Amount of tokens/ETH to deposit
     * @param reservedSolver Solver address that will receive funds when escrow is claimed (must not be address(0))
     * @dev Expiry is automatically set to block.timestamp + EXPIRY_DURATION (contract-defined)
     * @dev This function atomically creates the escrow and deposits funds, matching Move's create_escrow_from_fa
     * @dev Funds will always be transferred to reservedSolver address regardless of who calls claim()
     */
    function createEscrow(
        uint256 intentId,
        address token,
        uint256 amount,
        address reservedSolver
    ) external payable {
        require(escrows[intentId].requester == address(0), "Escrow already exists");
        require(amount > 0, "Amount must be greater than 0");
        require(reservedSolver != address(0), "Reserved solver must be specified");

        // Create escrow
        escrows[intentId] = Escrow({
            requester: msg.sender,
            token: token,
            amount: 0,
            isClaimed: false,
            expiry: block.timestamp + EXPIRY_DURATION,
            reservedSolver: reservedSolver
        });

        // Deposit funds atomically
        if (token == address(0)) {
            // ETH deposit
            require(msg.value == amount, "ETH amount mismatch");
            escrows[intentId].amount = amount;
        } else {
            // ERC20 token deposit
            require(msg.value == 0, "ETH not accepted for token escrow");
            IERC20(token).safeTransferFrom(msg.sender, address(this), amount);
            escrows[intentId].amount = amount;
        }

        emit EscrowInitialized(intentId, address(this), msg.sender, token, reservedSolver, amount, escrows[intentId].expiry);
    }


    /**
     * @notice Claim escrow funds (solver only, requires approver signature)
     * @param intentId Intent identifier
     * @param signature Approver's ECDSA signature over keccak256(intentId) - signature itself is the approval
     */
    function claim(
        uint256 intentId,
        bytes memory signature
    ) external {
        Escrow storage escrow = escrows[intentId];
        
        if (escrow.requester == address(0)) revert EscrowDoesNotExist();
        if (escrow.isClaimed) revert EscrowAlreadyClaimed();
        if (escrow.amount == 0) revert NoDeposit();
        
        // Enforce expiry: claims are not allowed after expiry
        if (block.timestamp > escrow.expiry) revert EscrowExpired();

        // Verify signature
        // Approver signs only the intent_id (symmetric with Aptos - signature itself is the approval)
        bytes32 messageHash = keccak256(abi.encodePacked(intentId));
        bytes32 ethSignedMessageHash = keccak256(
            abi.encodePacked("\x19Ethereum Signed Message:\n32", messageHash)
        );
        
        address signer = recoverSigner(ethSignedMessageHash, signature);
        if (signer != approver) revert UnauthorizedApprover();

        uint256 amount = escrow.amount;
        address token = escrow.token;
        
        // Mark as claimed
        escrow.isClaimed = true;
        escrow.amount = 0;
        
        // Determine recipient: if escrow lists a solver, transfer to that address; otherwise transfer to msg.sender
        // Funds always go to the reserved solver (enforced at creation)
        address recipient = escrow.reservedSolver;
        
        // Transfer tokens or ETH to recipient
        if (token == address(0)) {
            // ETH transfer
            payable(recipient).transfer(amount);
        } else {
            // ERC20 token transfer
            IERC20(token).safeTransfer(recipient, amount);
        }
        
        emit EscrowClaimed(intentId, recipient, amount);
    }

    /**
     * @notice Cancel escrow and return funds to requester (only after expiry)
     * @dev Requester must wait until expiry before canceling to prevent premature withdrawal
     * @param intentId Intent identifier
     */
    function cancel(uint256 intentId) external {
        Escrow storage escrow = escrows[intentId];
        
        if (escrow.requester == address(0)) revert EscrowDoesNotExist();
        if (escrow.isClaimed) revert EscrowAlreadyClaimed();
        if (escrow.amount == 0) revert NoDeposit();
        if (msg.sender != escrow.requester) revert UnauthorizedRequester();
        
        // Enforce expiry: cancellation is only allowed after expiry
        // This ensures funds remain locked until the contract-defined expiry period
        if (block.timestamp <= escrow.expiry) revert EscrowNotExpiredYet();

        uint256 amount = escrow.amount;
        address token = escrow.token;
        
        // Reset escrow
        escrow.amount = 0;
        escrow.isClaimed = true;
        
        // Transfer tokens or ETH back to requester
        if (token == address(0)) {
            // ETH transfer
            payable(escrow.requester).transfer(amount);
        } else {
            // ERC20 token transfer
            IERC20(token).safeTransfer(escrow.requester, amount);
        }
        
        emit EscrowCancelled(intentId, escrow.requester, amount);
    }

    /**
     * @notice Recover signer address from signature
     * @param messageHash Hash of the message
     * @param signature ECDSA signature
     * @return signer Address that signed the message
     */
    function recoverSigner(
        bytes32 messageHash,
        bytes memory signature
    ) internal pure returns (address) {
        require(signature.length == 65, "Invalid signature length");

        bytes32 r;
        bytes32 s;
        uint8 v;

        assembly {
            r := mload(add(signature, 32))
            s := mload(add(signature, 64))
            v := byte(0, mload(add(signature, 96)))
        }

        // Adjust v for Ethereum (27 or 28)
        if (v < 27) {
            v += 27;
        }

        return ecrecover(messageHash, v, r, s);
    }

    /**
     * @notice Get escrow data for an intent
     * @param intentId Intent identifier
     * @return requester Requester address
     * @return token Token address
     * @return amount Amount deposited
     * @return isClaimed Whether escrow is claimed
     * @return expiry Expiry timestamp
     * @return reservedSolver Solver address that receives funds (always set)
     */
    function getEscrow(uint256 intentId)
        external
        view
        returns (
            address requester,
            address token,
            uint256 amount,
            bool isClaimed,
            uint256 expiry,
            address reservedSolver
        )
    {
        Escrow memory escrow = escrows[intentId];
        return (escrow.requester, escrow.token, escrow.amount, escrow.isClaimed, escrow.expiry, escrow.reservedSolver);
    }
}

