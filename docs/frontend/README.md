# Frontend

A Next.js 14 frontend for int3nts, enabling users to create and track intents through a web interface.

## Overview

The frontend provides a user-friendly interface for:

- Connecting Nightly wallet (for MVM chains), MetaMask (for EVM chains), and Phantom (for SVM chains)
- Creating inflow and outflow intents
- Submitting draft intents to the verifier and polling for solver signatures
- Tracking intent lifecycle from creation to fulfillment
- Managing escrow creation for inflow intents

## Architecture

The frontend is built with:

- **Framework**: Next.js 14 (App Router)
- **MVM Wallet**: Nightly via `@nightlylabs/aptos-wallet-adapter-react`
- **EVM Wallet**: MetaMask via `wagmi` + `viem` + `@tanstack/react-query`
- **SVM Wallet**: Phantom via `@solana/wallet-adapter-react`
- **Styling**: Tailwind CSS with dark theme
- **State Management**: React hooks (`useState`, `useEffect`, `useRef`)

### Key Components

- `app/layout.tsx` - Root layout with wallet providers (Nightly + wagmi + Phantom)
- `app/page.tsx` - Main intent creation page
- `components/intent/IntentBuilder.tsx` - Intent creation form and status tracking
- `components/wallet/` - Wallet connection UI components
- `lib/verifier.ts` - Verifier API client with polling
- `lib/types.ts` - Protocol types (DraftIntent, IntentStatus, etc.)
- `config/chains.ts` - Chain configurations and contract addresses
- `config/tokens.ts` - Supported token definitions

## User Flows

For detailed protocol flows, see [Protocol Specification](../protocol.md).

### Inflow Flow

1. User connects Nightly (MVM) plus MetaMask (EVM) or Phantom (SVM) wallet
2. User selects tokens and amounts (Send on connected chain, Receive on Movement)
3. Frontend submits draft intent to verifier
4. Frontend polls for solver signature
5. User commits intent on Movement hub chain (via Nightly)
6. User creates escrow on connected chain:
   - EVM: MetaMask signs ERC20 approval + escrow creation
   - SVM: Phantom signs escrow creation (no ERC20 approval step)
7. Frontend polls for fulfillment status
8. User receives tokens on Movement chain

### Outflow Flow

1. User connects Nightly (MVM) plus MetaMask (EVM) or Phantom (SVM) wallet
2. User selects tokens and amounts (Send on Movement, Receive on connected chain)
3. Frontend submits draft intent to verifier
4. Frontend polls for solver signature
5. User commits intent on Movement hub chain (via Nightly) - tokens sent immediately
6. Frontend uses the connected-chain wallet address for outflow intents (MetaMask for EVM, Phantom for SVM)
7. Frontend polls for fulfillment status
8. User receives tokens on connected chain

## Quick Start

See the [component README](../../frontend/README.md) for installation and development commands.

## Environment Variables

```bash
NEXT_PUBLIC_VERIFIER_URL=http://localhost:3333
NEXT_PUBLIC_HUB_RPC=https://testnet.movementnetwork.xyz/v1
NEXT_PUBLIC_SVM_RPC_URL=https://api.devnet.solana.com
NEXT_PUBLIC_SVM_PROGRAM_ID=<your-svm-program-id>
```

## Features

- **Dual Wallet Support**: Seamlessly connect and use both MVM and EVM wallets
- **Auto-calculated Exchange Rates**: Desired amount automatically calculated from solver's exchange rate
- **Real-time Status Updates**: Polling for solver signatures and fulfillment status
- **Transaction Tracking**: Display transaction hashes and intent IDs
- **Timer Management**: Visual countdown timer for intent expiry (stops after tokens sent)
- **Error Handling**: Clear error messages and recovery flows
- **Responsive UI**: Clean, dark-themed interface optimized for intent creation

## API Integration

The frontend communicates with the verifier service for:

- Draft intent submission (`POST /draftintent`)
- Signature polling (`GET /draftintent/:id/signature`)
- Exchange rate queries (`GET /acceptance`)
- Approval status checks (`GET /approved/:intent_id`)

For detailed API documentation, see [Trusted Verifier API](../trusted-verifier/api.md).
