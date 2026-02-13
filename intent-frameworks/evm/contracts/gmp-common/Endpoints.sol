// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title Endpoints
/// @notice GMP Endpoint IDs for cross-chain communication
///
/// @dev EVM Architecture Note:
///      Unlike SVM and MVM which hardcode endpoint addresses,
///      EVM contracts receive the GMP endpoint address at deployment time
///      via constructor parameters. Endpoint addresses vary by network
///      (mainnet, testnet, local).
///
///      This file provides EID constants for reference and remote GMP endpoint configuration,
///      mirroring the structure of:
///      - SVM: gmp-common/src/endpoints.rs
///      - MVM: gmp_common/endpoints.move
library Endpoints {
    // ============================================================================
    // GMP ENDPOINT IDS
    // ============================================================================

    // Solana
    uint32 constant SOLANA_MAINNET_EID = 30168;
    uint32 constant SOLANA_DEVNET_EID = 40168;

    // Movement
    uint32 constant MOVEMENT_MAINNET_EID = 30325;
    uint32 constant MOVEMENT_TESTNET_EID = 40325;

    // Ethereum
    uint32 constant ETHEREUM_MAINNET_EID = 30101;
    uint32 constant ETHEREUM_SEPOLIA_EID = 40161;

    // Base
    uint32 constant BASE_MAINNET_EID = 30184;
    uint32 constant BASE_SEPOLIA_EID = 40245;
}
