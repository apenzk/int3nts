# Testing Infrastructure

Infrastructure setup for running chains for development and testing.

## Resources

- [Testing Guide](../../testing-infra/ci-e2e/testing-guide.md) - Testing and validation commands
- [Framework Extension Guide](../intent-frameworks/framework-extension-guide.md) - How to add new blockchain frameworks while maintaining test alignment

## E2E Tests

- **[MVM E2E Tests](../../testing-infra/ci-e2e/e2e-tests-mvm/README.md)** - Tests MVM-only cross-chain intents (Chain 1 → Chain 2)
- **[EVM E2E Tests](../../testing-infra/ci-e2e/e2e-tests-evm/README.md)** - Tests mixed-chain intents (MVM Chain 1 → EVM Chain 3)
- **[SVM E2E Tests](../../testing-infra/ci-e2e/e2e-tests-svm/README.md)** - Tests mixed-chain intents (MVM Chain 1 → SVM Chain 4)

## Testnet

Scripts for deploying to and interacting with public testnets (Movement Bardock, Base Sepolia). See the **[testnet README](../../testing-infra/testnet/README.md)** for deployment and interaction scripts.
