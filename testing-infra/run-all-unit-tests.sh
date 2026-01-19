#!/usr/bin/env bash
# Generate test summary table for all components
# Usage: ./testing-infra/run-all-unit-tests.sh

# Don't use set -e so we can capture all test results even if some fail

echo "Running Verifier tests..."
VERIFIER_TEST_OUTPUT=$(RUST_LOG=off nix develop -c bash -c "cd trusted-verifier && cargo test --quiet 2>&1") || {
    echo "Verifier tests failed:"
    echo "$VERIFIER_TEST_OUTPUT"
}
VERIFIER_PASSED=$(echo "$VERIFIER_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
VERIFIER_FAILED=$(echo "$VERIFIER_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running Solver tests..."
SOLVER_TEST_OUTPUT=$(RUST_LOG=off nix develop -c bash -c "cd solver && cargo test --quiet 2>&1") || {
    echo "Solver tests failed:"
    echo "$SOLVER_TEST_OUTPUT"
}
SOLVER_PASSED=$(echo "$SOLVER_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
SOLVER_FAILED=$(echo "$SOLVER_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running Move tests..."
MOVE_TEST_OUTPUT=$(nix develop -c bash -c "cd move-intent-framework && movement move test --dev --named-addresses mvmt_intent=0x123" 2>&1) || {
    echo "Move tests failed:"
    echo "$MOVE_TEST_OUTPUT"
}
MOVE_PASSED=$(echo "$MOVE_TEST_OUTPUT" | grep -oE "passed: [0-9]+" | awk '{print $2}' | head -1)
MOVE_FAILED=$(echo "$MOVE_TEST_OUTPUT" | grep -oE "failed: [0-9]+" | awk '{print $2}' | head -1)
MOVE_PASSED=${MOVE_PASSED:-0}
MOVE_FAILED=${MOVE_FAILED:-0}

echo "Running EVM tests..."
EVM_TEST_OUTPUT=$(nix develop -c bash -c "cd evm-intent-framework && npm test" 2>&1) || {
    echo "EVM tests failed:"
    echo "$EVM_TEST_OUTPUT"
}
EVM_PASSED=$(echo "$EVM_TEST_OUTPUT" | grep -oE "[0-9]+ passing" | awk '{print $1}')
EVM_FAILED=$(echo "$EVM_TEST_OUTPUT" | grep -oE "[0-9]+ failing" | awk '{print $1}' || echo "0")
EVM_PASSED=${EVM_PASSED:-0}
EVM_FAILED=${EVM_FAILED:-0}

echo "Running SVM tests..."
# Build and run tests
SVM_TEST_OUTPUT=$(cd svm-intent-framework && ./scripts/test.sh 2>&1) || {
    echo "SVM tests failed:"
    echo "$SVM_TEST_OUTPUT"
}
# Parse cargo test output (e.g., "test result: ok. 3 passed; 0 failed;")
SVM_PASSED=$(echo "$SVM_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
SVM_FAILED=$(echo "$SVM_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running Frontend tests..."
FRONTEND_TEST_OUTPUT=$(nix develop -c bash -c "cd frontend && npm test" 2>&1) || {
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
echo "| Verifier | $VERIFIER_PASSED | $VERIFIER_FAILED |"
echo "| Solver | $SOLVER_PASSED | $SOLVER_FAILED |"
echo "| Move | $MOVE_PASSED | $MOVE_FAILED |"
echo "| EVM | $EVM_PASSED | $EVM_FAILED |"
echo "| SVM | $SVM_PASSED | $SVM_FAILED |"
echo "| Frontend | $FRONTEND_PASSED | $FRONTEND_FAILED |"
echo ""

