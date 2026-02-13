# Integrated GMP Service

GMP message relay that watches for `MessageSent` events on source chains and delivers messages to destination chains.

**Full documentation: [docs/integrated-gmp/](../docs/integrated-gmp/README.md)**

## Quick Start

```bash
# Build
cargo build

# Configure (copy template and edit)
cp config/integrated-gmp.template.toml config/integrated-gmp.toml

# Run (default: uses config/integrated-gmp.toml)
cargo run --bin integrated-gmp

# Run with testnet config
cargo run --bin integrated-gmp -- --testnet

# Run with custom config
cargo run --bin integrated-gmp -- --config config/my-config.toml

# Show help
cargo run --bin integrated-gmp -- --help
```

### Command-Line Options

- `--testnet`, `-t` - Use testnet configuration (`config/integrated-gmp_testnet.toml`)
- `--config <path>` - Use custom config file path
- `--help`, `-h` - Show help message

**Note:** The `INTEGRATED_GMP_CONFIG_PATH` environment variable can also be used and takes precedence over flags.

### Running Against Testnets

For running against testnets (Movement Bardock + Base Sepolia), use the provided script:

```bash
./testing-infra/testnet/run-integrated-gmp-local.sh
```

This script automatically uses the `--testnet` flag and loads keys from `testing-infra/testnet/.env.testnet`.
