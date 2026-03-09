# int3nts

> **⚠️ EXPERIMENTAL - NOT PRODUCTION READY**  
> This framework is currently in active development and is **not ready for production use**. Use at your own risk. APIs, interfaces, and implementations may change without notice.

A framework for creating cross-chain intents with the following components

- [intent-frameworks](docs/intent-frameworks/README.md)
- [coordinator](docs/coordinator/README.md)
- [integrated-gmp](docs/integrated-gmp/README.md)
- [frontend](docs/frontend/README.md)
- [solver tools](docs/solver/README.md)
- [testing infrastructure](docs/testing-infra/README.md)

For complete documentation, see [docs/](docs/README.md).

For contributing guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).

## Quick start

- Enter dev shell with pinned toolchain (Rust, Movement CLI, Aptos CLI):

```text
nix develop ./nix
```

### Testing

#### Unit Tests (no Docker required)

Run from project root:

```bash
# MVM (Movement)
./intent-frameworks/mvm/scripts/test.sh
# EVM (Ethereum)
nix develop ./nix -c bash -c "cd intent-frameworks/evm && npm install && npm test"
# SVM (Solana)
./intent-frameworks/svm/scripts/test.sh
# Chain clients
nix develop ./nix -c bash -c "./chain-clients/scripts/test.sh"
# Rust services
RUST_LOG=off nix develop ./nix -c bash -c "cd coordinator && cargo test --quiet"
RUST_LOG=off nix develop ./nix -c bash -c "cd integrated-gmp && cargo test --quiet"
RUST_LOG=off nix develop ./nix -c bash -c "cd solver && cargo test --quiet"
# Frontend
nix develop ./nix -c bash -c "cd frontend && npm install --legacy-peer-deps && npm test"
```

#### E2E Integration Tests (requires Docker)

Run from project root:

```bash
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-inflow.sh"
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-outflow.sh"
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-evm/run-tests-inflow.sh"
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-evm/run-tests-outflow.sh"
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-svm/run-tests-inflow.sh"
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-svm/run-tests-outflow.sh"
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-rust-integration.sh"
```

Pass `--no-build` to skip Rust binary compilation (uses previously built binaries):

```bash
nix develop ./nix -c bash -c "./testing-infra/ci-e2e/e2e-tests-mvm/run-tests-inflow.sh --no-build"
```

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
