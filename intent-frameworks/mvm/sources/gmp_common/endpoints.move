/// GMP endpoint configuration for LayerZero V2 and local testing.
///
/// Addresses and EIDs sourced from LZ documentation and the project's
/// layerzero-solana-integration.md / layerzero-movement-integration.md.
module mvmt_intent::gmp_endpoints {

    // =========================================================================
    // LayerZero V2 chain endpoint IDs (EIDs)
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

    public fun solana_mainnet_eid(): u32 { SOLANA_MAINNET_EID }
    public fun solana_devnet_eid(): u32 { SOLANA_DEVNET_EID }
    public fun movement_mainnet_eid(): u32 { MOVEMENT_MAINNET_EID }
    public fun movement_testnet_eid(): u32 { MOVEMENT_TESTNET_EID }
    public fun ethereum_mainnet_eid(): u32 { ETHEREUM_MAINNET_EID }
    public fun ethereum_sepolia_eid(): u32 { ETHEREUM_SEPOLIA_EID }
    public fun base_mainnet_eid(): u32 { BASE_MAINNET_EID }
    public fun base_sepolia_eid(): u32 { BASE_SEPOLIA_EID }
}
