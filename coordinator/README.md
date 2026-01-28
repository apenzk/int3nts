# Coordinator Service

Service that monitors chains and provides draft-intent negotiation (no keys, no validation).

**Full documentation: [docs/coordinator/](../docs/coordinator/README.md)**

## Quick Start

```bash
# Build
cargo build

# Configure (copy template and edit)
cp config/coordinator.template.toml config/coordinator.toml

# Run (default: uses config/coordinator.toml)
cargo run --bin coordinator

# Run with testnet config
cargo run --bin coordinator -- --testnet

# Run with custom config
cargo run --bin coordinator -- --config config/my-config.toml

# Show help
cargo run --bin coordinator -- --help
```

### Command-Line Options

- `--testnet`, `-t` - Use testnet configuration (`config/coordinator_testnet.toml`)
- `--config <path>` - Use custom config file path
- `--help`, `-h` - Show help message

**Note:** The `COORDINATOR_CONFIG_PATH` environment variable can also be used and takes precedence over flags.

### Running Against Testnets

For running against testnets (Movement Bardock + Base Sepolia), use the provided script:

```bash
./testing-infra/testnet/run-coordinator-local.sh
```

This script automatically uses the `--testnet` flag.
