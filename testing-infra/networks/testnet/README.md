# Testnet Deployment Infrastructure

Scripts and configuration for deploying the Intent Framework to public testnets (Movement Bardock, Base Sepolia, Solana Devnet).

Separate from `testing-infra/ci-e2e/` which is for local CI testing with Docker.

## Usage

### Deploy

```bash
./testing-infra/networks/testnet/deploy.sh
```

Deploys to all three chains. Prints a summary of addresses to update in `.env.testnet` and service config files.

### Configure

After updating `.env.testnet` with deployed addresses:

```bash
./testing-infra/networks/testnet/configure.sh
```

Sets up cross-chain GMP routing between deployed contracts.

### Check Preparedness

```bash
./testing-infra/networks/testnet/check-preparedness.sh
```

### Local Testing

Run services locally before EC2 deployment. Start each in a separate terminal:

```bash
./testing-infra/networks/testnet/run-coordinator-local.sh
./testing-infra/networks/testnet/run-integrated-gmp-local.sh
./testing-infra/networks/testnet/run-solver-local.sh
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

## Cost Estimates

See [supported-networks.md](../../../docs/testing-infra/supported-networks.md) for supported networks and deployment cost estimates.

## Configuration

- `.env.testnet` - Private keys and addresses (gitignored)
- `coordinator/config/coordinator_testnet.toml` - Coordinator config (gitignored)
- `integrated-gmp/config/integrated-gmp_testnet.toml` - GMP config (gitignored)
- `solver/config/solver_testnet.toml` - Solver config (gitignored)
- `config/testnet-assets.toml` - Public asset addresses and decimals
