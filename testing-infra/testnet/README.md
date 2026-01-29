# Testnet Deployment Infrastructure

This directory contains scripts and configuration for deploying the Intent Framework to public testnets (Movement Bardock Testnet and Base Sepolia).

**Note**: This is separate from `testing-infra/ci-e2e/` which is for local CI testing with Docker-based chains.

## Files

### Deployment Scripts

- **`deploy-to-movement-testnet.sh`** - Deploy Move Intent Framework to Movement Bardock Testnet
- **`deploy-to-base-testnet.sh`** - Deploy EVM IntentEscrow to Base Sepolia Testnet
- **`check-testnet-preparedness.sh`** - Check balances and deployed contracts
- **`check-testnet-balances.sh`** - Check account balances on testnets

### Local Testing Scripts

- **`run-coordinator-local.sh`** - Run coordinator service locally against testnets
- **`run-trusted-gmp-local.sh`** - Run trusted-gmp service locally against testnets
- **`run-solver-local.sh`** - Run solver service locally against testnets

### Configuration Files

- **`config/testnet-assets.toml`** - Public configuration for asset addresses and decimals

## Usage

### Deploy to Movement Bardock Testnet

```bash
./testing-infra/testnet/deploy-to-movement-testnet.sh
```

### Deploy to Base Sepolia Testnet

```bash
./testing-infra/testnet/deploy-to-base-testnet.sh
```

### Check Testnet Preparedness

```bash
./testing-infra/testnet/check-testnet-preparedness.sh
```

### Local Testing (Before EC2 Deployment)

Test the services locally before deploying to EC2:

#### Terminal 1: Start Coordinator

```bash
./testing-infra/testnet/run-coordinator-local.sh
# Or with release build (faster):
./testing-infra/testnet/run-coordinator-local.sh --release
```

#### Terminal 2: Start Trusted GMP

(after coordinator is running)

```bash
./testing-infra/testnet/run-trusted-gmp-local.sh
# Or with release build (faster):
./testing-infra/testnet/run-trusted-gmp-local.sh --release
```

#### Terminal 3: Start Solver

(after coordinator is running)

```bash
./testing-infra/testnet/run-solver-local.sh
# Or with release build (faster):
./testing-infra/testnet/run-solver-local.sh --release
```

#### Terminal 4: Start Frontend

```bash
cd frontend && npm install --legacy-peer-deps && npm run dev
```

Create `frontend/.env.local` with testnet values:

```
NEXT_PUBLIC_COORDINATOR_URL=http://localhost:3333
NEXT_PUBLIC_TRUSTED_GMP_URL=http://localhost:3334
```

Use the frontend UI to create and test intents (inflow and outflow) against the local services.

Note: chain-specific addresses and optional RPC/program overrides are configured in `frontend/src/config/chains.ts`.

#### Quick Health Check

```bash
# Check if coordinator is running
curl -s http://localhost:3333/health | jq

# Check if trusted-gmp is running
curl -s http://localhost:3334/health | jq
```

#### Prerequisites for Local Testing

- Coordinator and solver config files populated with deployed addresses:
  - `coordinator/config/coordinator_testnet.toml`
  - `trusted-gmp/config/trusted-gmp_testnet.toml`
  - `solver/config/solver_testnet.toml`
- `.env.testnet` in this directory with all required keys
- Movement CLI profile configured (solver only)
- Coordinator running and healthy (for solver and frontend)

## Configuration

All scripts read from:

- `.env.testnet` - Private keys and addresses in this directory (gitignored)
- `coordinator/config/coordinator_testnet.toml` - Coordinator service config (gitignored)
- `trusted-gmp/config/trusted-gmp_testnet.toml` - Trusted GMP service config (gitignored)
- `solver/config/solver_testnet.toml` - Solver service config (gitignored)
- `config/testnet-assets.toml` - Public asset addresses and decimals
