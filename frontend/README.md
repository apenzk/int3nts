# Intent Framework Frontend

Browser UI for the cross-chain intent protocol.

 **Full documentation: [docs/frontend/](../docs/frontend/README.md)**

## Quick Start

```bash
# Build the SDK (required before any frontend command, run from frontend/)
cd ../packages/sdk && npm install && npm run build && cd -

# Install dependencies (use --legacy-peer-deps due to React 19 compatibility)
npm install --legacy-peer-deps

# Run development server
npm run dev

# Build for production
npm run build

# Start production server
npm start
```

## Features

- Connect Nightly wallet (MVM chains)
- Connect MetaMask (EVM chains)
- Connect Phantom (SVM chains)
- Create inflow/outflow intents
- Track intent lifecycle
- Submit transactions to hub and connected chains

## Environment Variables

Create a `.env.local` file. All chain env vars follow the pattern `NEXT_PUBLIC_{CHAIN}_{TESTNET|MAINNET}_{THING}`.

```bash
NEXT_PUBLIC_COORDINATOR_URL=http://localhost:3333

# Testnet
NEXT_PUBLIC_MOVEMENT_TESTNET_RPC_URL=https://testnet.movementnetwork.xyz/v1
NEXT_PUBLIC_MOVEMENT_TESTNET_INTENT_CONTRACT_ADDRESS=0x<address>
NEXT_PUBLIC_BASE_TESTNET_RPC_URL=https://base-sepolia.g.alchemy.com/v2/<key>
NEXT_PUBLIC_BASE_TESTNET_ESCROW_CONTRACT_ADDRESS=0x<address>
NEXT_PUBLIC_BASE_TESTNET_OUTFLOW_VALIDATOR_ADDRESS=0x<address>
NEXT_PUBLIC_ETH_TESTNET_RPC_URL=https://ethereum-sepolia-rpc.publicnode.com
NEXT_PUBLIC_SOLANA_TESTNET_RPC_URL=https://api.devnet.solana.com
NEXT_PUBLIC_SOLANA_TESTNET_PROGRAM_ID=<program-id>
NEXT_PUBLIC_SOLANA_TESTNET_OUTFLOW_PROGRAM_ID=<program-id>
NEXT_PUBLIC_SOLANA_TESTNET_GMP_ENDPOINT_ID=<program-id>

# Mainnet
NEXT_PUBLIC_MOVEMENT_MAINNET_RPC_URL=https://mainnet.movementnetwork.xyz/v1
NEXT_PUBLIC_MOVEMENT_MAINNET_INTENT_CONTRACT_ADDRESS=0x<address>
NEXT_PUBLIC_BASE_MAINNET_RPC_URL=https://mainnet.base.org
NEXT_PUBLIC_BASE_MAINNET_ESCROW_CONTRACT_ADDRESS=0x<address>
NEXT_PUBLIC_BASE_MAINNET_OUTFLOW_VALIDATOR_ADDRESS=0x<address>
NEXT_PUBLIC_HYPERLIQUID_MAINNET_RPC_URL=https://api.hyperliquid.xyz/evm
NEXT_PUBLIC_HYPERLIQUID_MAINNET_ESCROW_CONTRACT_ADDRESS=0x<address>
NEXT_PUBLIC_HYPERLIQUID_MAINNET_OUTFLOW_VALIDATOR_ADDRESS=0x<address>
```

## Tech Stack

- Next.js 14 (App Router)
- TypeScript
- Tailwind CSS
- Nightly wallet adapter
- wagmi + viem (EVM)
- Phantom wallet adapter (Solana)
