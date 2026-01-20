# int3nts

> **⚠️ EXPERIMENTAL - NOT PRODUCTION READY**  
> This framework is currently in active development and is **not ready for production use**. Use at your own risk. APIs, interfaces, and implementations may change without notice.

A framework for creating cross-chain intents with the following components

- [intent-frameworks](docs/intent-frameworks/README.md)
- [verifier](docs/verifier/README.md)
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
nix develop ./nix -c bash -c "cd intent-frameworks/mvm && movement move test --dev --named-addresses mvmt_intent=0x123"
nix develop ./nix -c bash -c "cd intent-frameworks/evm && npm test"
nix develop ./nix -c bash -c "cd intent-frameworks/svm && ./scripts/test.sh"
RUST_LOG=off nix develop ./nix -c bash -c "cd verifier && cargo test --quiet"
RUST_LOG=off nix develop ./nix -c bash -c "cd solver && cargo test --quiet"
nix develop ./nix -c bash -c "cd frontend && npm test"
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

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.
