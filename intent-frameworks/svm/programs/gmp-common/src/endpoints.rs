/// GMP endpoint configuration for LZ V2 and local testing.
///
/// Addresses and EIDs sourced from LZ documentation and the project's
/// lz-svm-integration.md / lz-mvm-integration.md.

// ---------------------------------------------------------------------------
// LZ V2 chain endpoint IDs (EIDs)
// ---------------------------------------------------------------------------

pub const SOLANA_MAINNET_EID: u32 = 30168;
pub const SOLANA_DEVNET_EID: u32 = 40168;

pub const MOVEMENT_MAINNET_EID: u32 = 30325;
/// Movement testnet EID — unconfirmed, LZ may not support it yet.
pub const MOVEMENT_TESTNET_EID: u32 = 40325;

pub const ETHEREUM_MAINNET_EID: u32 = 30101;
pub const ETHEREUM_SEPOLIA_EID: u32 = 40161;
pub const BASE_MAINNET_EID: u32 = 30184;
pub const BASE_SEPOLIA_EID: u32 = 40245;

// ---------------------------------------------------------------------------
// Solana LZ program IDs (base58-encoded strings)
// ---------------------------------------------------------------------------

/// LZ V2 Endpoint program on Solana mainnet.
pub const SOLANA_LZ_ENDPOINT: &str = "76y77prsiCMvXMjuoZ5VRrhG5qYBrUMYTE5WgHqgjEn6";
pub const SOLANA_LZ_EXECUTOR: &str = "6doghB248px58JSSwG4qejQ46kFMW4AMj7vzJnWZHNZn";
pub const SOLANA_LZ_DVN: &str = "HtEYV4xB4wvsj5fgTkcfuChYpvGYzgzwvNhgDZQNh7wW";
pub const SOLANA_LZ_ULN: &str = "7a4WjyR8VZ7yZz5XJAKm39BUGn5iT9CKcv2pmG9tdXVH";

// ---------------------------------------------------------------------------
// EVM LZ endpoint addresses
// ---------------------------------------------------------------------------

/// LZ V2 Endpoint on all EVM mainnets (Ethereum, Base, Arbitrum, etc.).
pub const EVM_LZ_ENDPOINT_MAINNET: &str = "0x1a44076050125825900e736c501f859c50fE728c";
/// LZ V2 Endpoint on all EVM testnets.
pub const EVM_LZ_ENDPOINT_TESTNET: &str = "0x6EDCE65403992e310A62460808c4b910D972f10f";

// ---------------------------------------------------------------------------
// Movement LZ endpoint
// ---------------------------------------------------------------------------

/// LZ V2 Endpoint on Movement mainnet (Move address).
/// Verify at https://docs.layerzero.network before production use.
pub const MOVEMENT_LZ_ENDPOINT_MAINNET: &str =
    "0x54ad3d30af77b60d939ae356e6606de9a4da67583f02b962d2d3f2e481484e90";

// ---------------------------------------------------------------------------
// Environment selection
// ---------------------------------------------------------------------------

/// Which GMP transport to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GmpEnvironment {
    /// Local/CI testing without LZ infrastructure.
    Local,
    /// Testnet (Sepolia, Solana devnet, Movement testnet).
    Testnet,
    /// Production (Ethereum, Solana, Movement mainnets).
    Mainnet,
}
