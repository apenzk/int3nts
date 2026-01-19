# SVM E2E Tests

Scripts for running SVM connected-chain E2E flows with the hub chain.

## Usage

```bash
./testing-infra/ci-e2e/e2e-tests-svm/run-tests-outflow.sh
./testing-infra/ci-e2e/e2e-tests-svm/run-tests-inflow.sh
```

Notes:

- These scripts assume `nix develop` is available on the host.
- The inflow flow currently leaves the SVM escrow locked after fulfillment.
