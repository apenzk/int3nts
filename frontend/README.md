# Intent Framework Frontend

Browser UI for the cross-chain intent protocol.

📚 **Full documentation: [docs/frontend/](../docs/frontend/README.md)**

## Quick Start

```bash
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
- Create inflow/outflow intents
- Track intent lifecycle
- Submit transactions to hub and connected chains

## Environment Variables

Create a `.env.local` file:

```
NEXT_PUBLIC_VERIFIER_URL=http://localhost:3333
NEXT_PUBLIC_MVM_HUB_RPC=https://testnet.movementnetwork.xyz
NEXT_PUBLIC_EVM_RPC=https://...
```

## Tech Stack

- Next.js 14 (App Router)
- TypeScript
- Tailwind CSS
- Nightly wallet adapter
- wagmi + viem (EVM)
