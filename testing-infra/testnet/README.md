# Testnet Deployment Infrastructure

Scripts and configuration for deploying the Intent Framework to public testnets (Movement Bardock, Base Sepolia, Solana Devnet).

Separate from `testing-infra/ci-e2e/` which is for local CI testing with Docker.

## Files

- **`deploy-and-configure.sh`** - Deploy and/or configure all chains (calls per-chain scripts in `scripts/`)
- **`check-testnet-preparedness.sh`** - Check balances, contracts, and on-chain GMP configuration
- **`run-coordinator-local.sh`** / **`run-integrated-gmp-local.sh`** / **`run-solver-local.sh`** - Run services locally
- **`scripts/`** - Per-chain deploy and configure scripts (called by `deploy-and-configure.sh`)
- **`config/testnet-assets.toml`** - Public asset addresses and decimals

## Usage

### Deploy

```bash
./testing-infra/testnet/deploy-and-configure.sh
```

Prompts to deploy + configure (full fresh deploy) or configure only (contracts already deployed).

### Check Preparedness

```bash
./testing-infra/testnet/check-testnet-preparedness.sh
```

### Local Testing

Run services locally before EC2 deployment. Start each in a separate terminal:

```bash
./testing-infra/testnet/run-coordinator-local.sh [--release]
./testing-infra/testnet/run-integrated-gmp-local.sh [--release]
./testing-infra/testnet/run-solver-local.sh [--release]
```

Start the frontend:

```bash
cd frontend && npm install --legacy-peer-deps && npm run dev
```

With `frontend/.env.local`:

```bash
NEXT_PUBLIC_COORDINATOR_URL=http://localhost:3333
NEXT_PUBLIC_INTEGRATED_GMP_URL=http://localhost:3334
```

Health check:

```bash
curl -s http://localhost:3333/health | jq   # coordinator
curl -s http://localhost:3334/health | jq   # integrated-gmp
```

## Configuration

- `.env.testnet` - Private keys and addresses (gitignored)
- `coordinator/config/coordinator_testnet.toml` - Coordinator config (gitignored)
- `integrated-gmp/config/integrated-gmp_testnet.toml` - GMP config (gitignored)
- `solver/config/solver_testnet.toml` - Solver config (gitignored)
- `config/testnet-assets.toml` - Public asset addresses and decimals
