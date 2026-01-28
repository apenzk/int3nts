# Trusted GMP Service

Service that monitors chains and provides approval signatures (validation and signing).

**Full documentation: [docs/trusted-gmp/](../docs/trusted-gmp/README.md)**

## Quick Start

```bash
# Build
cargo build

# Configure (copy template and edit)
cp config/trusted-gmp.template.toml config/trusted-gmp.toml

# Run (default: uses config/trusted-gmp.toml)
cargo run --bin trusted-gmp

# Run with testnet config
cargo run --bin trusted-gmp -- --testnet

# Run with custom config
cargo run --bin trusted-gmp -- --config config/my-config.toml

# Show help
cargo run --bin trusted-gmp -- --help
```

### Command-Line Options

- `--testnet`, `-t` - Use testnet configuration (`config/trusted-gmp_testnet.toml`)
- `--config <path>` - Use custom config file path
- `--help`, `-h` - Show help message

**Note:** The `TRUSTED_GMP_CONFIG_PATH` environment variable can also be used and takes precedence over flags.

### Running Against Testnets

For running against testnets (Movement Bardock + Base Sepolia), use the provided script:

```bash
./testing-infra/testnet/run-trusted-gmp-local.sh
```

This script automatically uses the `--testnet` flag and loads keys from `testing-infra/testnet/.env.testnet`.
