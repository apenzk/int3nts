/// GMP endpoint configuration for cross-chain communication.
///
/// Chain endpoint IDs (EIDs) for GMP message routing.
module mvmt_intent::gmp_endpoints {

    // =========================================================================
    // GMP chain endpoint IDs (EIDs)
    // =========================================================================

    const SOLANA_MAINNET_EID: u32 = 30168;
    const SOLANA_DEVNET_EID: u32 = 40168;

    const MOVEMENT_MAINNET_EID: u32 = 30325;
    /// Movement testnet EID — unconfirmed, LZ may not support it yet.
    const MOVEMENT_TESTNET_EID: u32 = 40325;

    const ETHEREUM_MAINNET_EID: u32 = 30101;
    const ETHEREUM_SEPOLIA_EID: u32 = 40161;
    const BASE_MAINNET_EID: u32 = 30184;
    const BASE_SEPOLIA_EID: u32 = 40245;

    // =========================================================================
    // Accessors (Move constants are module-private; expose via functions)
    // =========================================================================

    /// Returns the Solana mainnet endpoint ID.
    public fun solana_mainnet_eid(): u32 { SOLANA_MAINNET_EID }
    /// Returns the Solana devnet endpoint ID.
    public fun solana_devnet_eid(): u32 { SOLANA_DEVNET_EID }
    /// Returns the Movement mainnet endpoint ID.
    public fun movement_mainnet_eid(): u32 { MOVEMENT_MAINNET_EID }
    /// Returns the Movement testnet endpoint ID.
    public fun movement_testnet_eid(): u32 { MOVEMENT_TESTNET_EID }
    /// Returns the Ethereum mainnet endpoint ID.
    public fun ethereum_mainnet_eid(): u32 { ETHEREUM_MAINNET_EID }
    /// Returns the Ethereum Sepolia testnet endpoint ID.
    public fun ethereum_sepolia_eid(): u32 { ETHEREUM_SEPOLIA_EID }
    /// Returns the Base mainnet endpoint ID.
    public fun base_mainnet_eid(): u32 { BASE_MAINNET_EID }
    /// Returns the Base Sepolia testnet endpoint ID.
    public fun base_sepolia_eid(): u32 { BASE_SEPOLIA_EID }
}
