require("@nomicfoundation/hardhat-toolbox");

/** @type import('hardhat/config').HardhatUserConfig */
module.exports = {
  solidity: {
    version: "0.8.20",
    settings: {
      optimizer: {
        enabled: true,
        runs: 200,
      },
    },
  },
  networks: {
    hardhat: {
      chainId: parseInt(process.env.HARDHAT_CHAIN_ID || "31337"),
    },
    localhost: {
      url: "http://127.0.0.1:8545",
      chainId: 31337,
      accounts: {
        mnemonic: "test test test test test test test test test test test junk",
      },
    },
    "localhost-e2e-2": {
      url: "http://127.0.0.1:2000",
      chainId: 2,
      accounts: {
        mnemonic: "test test test test test test test test test test test junk",
      },
    },
    "localhost-e2e-3": {
      url: "http://127.0.0.1:3000",
      chainId: 3,
      accounts: {
        mnemonic: "test test test test test test test test test test test junk",
      },
    },
    ...(process.env.BASE_SEPOLIA_RPC_URL ? {
      baseSepolia: {
        url: process.env.BASE_SEPOLIA_RPC_URL,
        chainId: 84532,
        accounts: [process.env.DEPLOYER_PRIVATE_KEY, process.env.SOLVER_EVM_PRIVATE_KEY].filter(Boolean),
      },
    } : {}),
    ...(process.env.BASE_RPC_URL ? {
      baseMainnet: {
        url: process.env.BASE_RPC_URL,
        chainId: 8453,
        accounts: [process.env.DEPLOYER_PRIVATE_KEY].filter(Boolean),
      },
    } : {}),
    ...(process.env.HYPERLIQUID_RPC_URL ? {
      hyperliquidMainnet: {
        url: process.env.HYPERLIQUID_RPC_URL,
        chainId: 999,
        accounts: [process.env.DEPLOYER_PRIVATE_KEY].filter(Boolean),
      },
    } : {}),
  },
};

