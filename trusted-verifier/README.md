# Trusted Verifier Service

Service that monitors chains and provides approval signatures.

📚 **Full documentation: [docs/trusted-verifier/](../docs/trusted-verifier/README.md)**

## Quick Start

```bash
# Build
cargo build

# Configure (copy template and edit)
cp config/verifier.template.toml config/verifier.toml

# Run (default: uses config/verifier.toml)
cargo run --bin trusted-verifier

# Run with testnet config
cargo run --bin trusted-verifier -- --testnet

# Run with custom config
cargo run --bin trusted-verifier -- --config config/my-config.toml

# Show help
cargo run --bin trusted-verifier -- --help
```

### Command-Line Options

- `--testnet`, `-t` - Use testnet configuration (`config/verifier_testnet.toml`)
- `--config <path>` - Use custom config file path
- `--help`, `-h` - Show help message

**Note:** The `VERIFIER_CONFIG_PATH` environment variable can also be used and takes precedence over flags.

### Running Against Testnets

For running against testnets (Movement Bardock + Base Sepolia), use the provided script:

```bash
./testing-infra/testnet/run-verifier-local.sh
```

This script automatically uses the `--testnet` flag and loads keys from `.testnet-keys.env`.
