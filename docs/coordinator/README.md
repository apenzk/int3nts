# Coordinator Service

A read-only service that monitors blockchain events, caches them for querying, and provides negotiation routing for solvers.

The coordinator monitors intent and escrow events across hub and connected chains:

- Monitors intent events on the hub chain (intent creation, fulfillment)
- Monitors escrow events on connected chains (MVM, EVM, and SVM)
- Caches events for efficient querying

Supports monitoring multiple connected chains simultaneously. MVM chains monitor `OracleLimitOrderEvent` events; EVM chains monitor `EscrowInitialized` events; SVM chains monitor escrow PDA accounts.

## Architecture

### Components

- **Event Monitor**: Listens for intent and escrow events on hub and connected chains (MVM, EVM, and SVM)
- **Event Cache**: Stores discovered events for API querying
- **Negotiation Router**: Coordinates draft intent submission and solver matching (FCFS)

## Project Structure

```text
coordinator/
├── config/          # Configuration files (no private keys)
├── src/
│   ├── monitor/     # Event monitoring (hub and connected chains)
│   ├── storage/     # Event caching and retrieval
│   ├── api/         # REST API server (read-only + negotiation)
│   └── config/      # Configuration loading
└── Cargo.toml
```

## Quick Start

See the [component README](../../coordinator/README.md) for quick start commands.

## API Endpoints

### Core Endpoints

- `GET /health` - Health check
- `GET /events` - Get cached intent events

### Negotiation Routing Endpoints

- `POST /draftintent` - Submit draft intent (open to any solver)
- `GET /draftintent/:id` - Get draft intent status
- `GET /draftintents/pending` - Get all pending drafts (for solvers to poll)
- `POST /draftintent/:id/signature` - Submit signature for draft (FCFS)
- `GET /draftintent/:id/signature` - Poll for signature (for requesters)

For usage guide, see [guide.md](guide.md). For negotiation routing guide, see [negotiation-routing.md](negotiation-routing.md).

## Dependencies

Uses pinned `aptos-core` version for stable Rust compatibility: `aptos-framework-v1.37.0` (SHA: `a10a3c02f16a2114ad065db6b4a525f0382e96a6`)
