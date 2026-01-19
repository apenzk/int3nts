# Testing Infrastructure

Infrastructure setup for running chains for development and testing.

## Resources

- [Testing Guide](./testing-guide.md) - Testing and validation commands

## Verifier API

- API: `http://127.0.0.1:3333`
- Port: 3333 (configurable in `trusted-verifier/config/verifier-e2e-ci-testing.toml`)

## E2E Tests

- **[MVM E2E Tests](../../testing-infra/ci-e2e/e2e-tests-mvm/README.md)** - Tests MVM-only cross-chain intents (Chain 1 → Chain 2)
- **[EVM E2E Tests](../../testing-infra/ci-e2e/e2e-tests-evm/README.md)** - Tests mixed-chain intents (MVM Chain 1 → EVM Chain 3)
- **[SVM E2E Tests](../../testing-infra/ci-e2e/e2e-tests-svm/README.md)** - Tests mixed-chain intents (MVM Chain 1 → SVM Chain 4)
