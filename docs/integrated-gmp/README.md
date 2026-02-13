# Integrated GMP Service

The Integrated GMP service is a pure **message relay** that delivers GMP (Generic Message Passing) messages between chains. It watches for `MessageSent` events on source chains and calls `deliver_message` on destination chains.

## Key Characteristics

- **Relay only** -- no off-chain validation, no approval signatures
- **Operator wallet keys** for gas payment on each chain (not approval authority)
- **GMP interfaces** for cross-chain message sending and receiving
- **Single `/health` endpoint** for operational monitoring; no external API

## Documentation

- **[Architecture](architecture.md)** -- How the relay works, message flow, configuration
- **[Solver Guide](solver-guide.md)** -- How solvers interact with GMP-based flows
- **[Troubleshooting](troubleshooting.md)** -- Common issues and error patterns

## Quick Start

### Configuration

The relay reads from a TOML config file:

```bash
# Via environment variable
export INTEGRATED_GMP_CONFIG_PATH=config/integrated-gmp.toml

# Via CLI flag
cargo run -- --config config/integrated-gmp.toml

# Testnet shorthand
cargo run -- --testnet
```

### Running

```bash
# From project root
nix develop ./nix -c bash -c "cd integrated-gmp && cargo run"

# With testnet config
nix develop ./nix -c bash -c "cd integrated-gmp && cargo run -- --testnet"
```

### Testing

```bash
RUST_LOG=off nix develop ./nix -c bash -c "cd integrated-gmp && cargo test --quiet"
```

## Related Documentation

- [GMP Message Types](../architecture/data-models.md#gmp-message-types) -- Wire format and struct definitions
- [Architecture Component Mapping](../architecture/architecture-component-mapping.md) -- Where integrated-gmp fits in the system
- [Protocol Overview](../protocol.md) -- End-to-end cross-chain flows
