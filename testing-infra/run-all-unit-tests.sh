#!/usr/bin/env bash
# Generate test summary table for all components
# Usage: ./testing-infra/run-all-unit-tests.sh

# Don't use set -e so we can capture all test results even if some fail

echo "Running Coordinator tests..."
COORDINATOR_TEST_OUTPUT=$(RUST_LOG=off nix develop ./nix -c bash -c "cd coordinator && cargo test --quiet 2>&1") || {
    echo "Coordinator tests failed:"
    echo "$COORDINATOR_TEST_OUTPUT"
}
COORDINATOR_PASSED=$(echo "$COORDINATOR_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
COORDINATOR_FAILED=$(echo "$COORDINATOR_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running Trusted-GMP tests..."
TRUSTED_GMP_TEST_OUTPUT=$(RUST_LOG=off nix develop ./nix -c bash -c "cd trusted-gmp && cargo test --quiet 2>&1") || {
    echo "Trusted-GMP tests failed:"
    echo "$TRUSTED_GMP_TEST_OUTPUT"
}
TRUSTED_GMP_PASSED=$(echo "$TRUSTED_GMP_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
TRUSTED_GMP_FAILED=$(echo "$TRUSTED_GMP_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running Solver tests..."
SOLVER_TEST_OUTPUT=$(RUST_LOG=off nix develop ./nix -c bash -c "cd solver && cargo test --quiet 2>&1") || {
    echo "Solver tests failed:"
    echo "$SOLVER_TEST_OUTPUT"
}
SOLVER_PASSED=$(echo "$SOLVER_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
SOLVER_FAILED=$(echo "$SOLVER_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running Move tests..."
MOVE_TEST_OUTPUT=$(nix develop ./nix -c bash -c "cd intent-frameworks/mvm && movement move test --dev --named-addresses mvmt_intent=0x123" 2>&1) || {
    echo "Move tests failed:"
    echo "$MOVE_TEST_OUTPUT"
}
MOVE_PASSED=$(echo "$MOVE_TEST_OUTPUT" | grep -oE "passed: [0-9]+" | awk '{print $2}' | head -1)
MOVE_FAILED=$(echo "$MOVE_TEST_OUTPUT" | grep -oE "failed: [0-9]+" | awk '{print $2}' | head -1)
MOVE_PASSED=${MOVE_PASSED:-0}
MOVE_FAILED=${MOVE_FAILED:-0}

echo "Running EVM tests..."
EVM_TEST_OUTPUT=$(nix develop ./nix -c bash -c "cd intent-frameworks/evm && npm install && npm test" 2>&1) || {
    echo "EVM tests failed:"
    echo "$EVM_TEST_OUTPUT"
}
EVM_PASSED=$(echo "$EVM_TEST_OUTPUT" | grep -oE "[0-9]+ passing" | awk '{print $1}')
EVM_FAILED=$(echo "$EVM_TEST_OUTPUT" | grep -oE "[0-9]+ failing" | awk '{print $1}' || echo "0")
EVM_PASSED=${EVM_PASSED:-0}
EVM_FAILED=${EVM_FAILED:-0}

echo "Running SVM tests..."
# Build and run tests
SVM_TEST_OUTPUT=$(cd intent-frameworks/svm && ./scripts/test.sh 2>&1) || {
    echo "SVM tests failed:"
    echo "$SVM_TEST_OUTPUT"
}
# Parse cargo test output (e.g., "test result: ok. 3 passed; 0 failed;")
SVM_PASSED=$(echo "$SVM_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
SVM_FAILED=$(echo "$SVM_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running Frontend tests..."
FRONTEND_TEST_OUTPUT=$(nix develop ./nix -c bash -c "cd frontend && npm install --legacy-peer-deps && npm test" 2>&1) || {
    echo "Frontend tests failed:"
    echo "$FRONTEND_TEST_OUTPUT"
}
FRONTEND_PASSED=$(echo "$FRONTEND_TEST_OUTPUT" | grep -oE "Tests[[:space:]]+[0-9]+ passed" | grep -oE "[0-9]+")
FRONTEND_FAILED=$(echo "$FRONTEND_TEST_OUTPUT" | grep -oE "Tests[[:space:]]+[0-9]+ failed" | grep -oE "[0-9]+" || echo "0")
FRONTEND_PASSED=${FRONTEND_PASSED:-0}
FRONTEND_FAILED=${FRONTEND_FAILED:-0}

echo "=== Test Summary Table ==="
echo ""
echo "| Tests | Passed | Failed |"
echo "|-------|--------|--------|"
echo "| Coordinator | $COORDINATOR_PASSED | $COORDINATOR_FAILED |"
echo "| Trusted-GMP | $TRUSTED_GMP_PASSED | $TRUSTED_GMP_FAILED |"
echo "| Solver | $SOLVER_PASSED | $SOLVER_FAILED |"
echo "| MVM | $MOVE_PASSED | $MOVE_FAILED |"
echo "| EVM | $EVM_PASSED | $EVM_FAILED |"
echo "| SVM | $SVM_PASSED | $SVM_FAILED |"
echo "| Frontend | $FRONTEND_PASSED | $FRONTEND_FAILED |"
echo ""

