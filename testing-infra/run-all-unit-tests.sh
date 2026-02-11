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

echo "Running Integrated-GMP tests..."
INTEGRATED_GMP_TEST_OUTPUT=$(RUST_LOG=off nix develop ./nix -c bash -c "cd integrated-gmp && cargo test --quiet 2>&1") || {
    echo "Integrated-GMP tests failed:"
    echo "$INTEGRATED_GMP_TEST_OUTPUT"
}
INTEGRATED_GMP_PASSED=$(echo "$INTEGRATED_GMP_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
INTEGRATED_GMP_FAILED=$(echo "$INTEGRATED_GMP_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running Solver tests..."
SOLVER_TEST_OUTPUT=$(RUST_LOG=off nix develop ./nix -c bash -c "cd solver && cargo test --quiet 2>&1") || {
    echo "Solver tests failed:"
    echo "$SOLVER_TEST_OUTPUT"
}
SOLVER_PASSED=$(echo "$SOLVER_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
SOLVER_FAILED=$(echo "$SOLVER_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')

echo "Running MVM tests..."
MVM_TEST_OUTPUT=$(./intent-frameworks/mvm/scripts/test.sh 2>&1) || {
    echo "MVM tests failed:"
    echo "$MVM_TEST_OUTPUT"
}
# Parse "passed: N" from each package output and sum
MOVE_PASSED=$(echo "$MVM_TEST_OUTPUT" | grep -oE "passed: [0-9]+" | awk '{sum += $2} END {print sum+0}')
MOVE_FAILED=$(echo "$MVM_TEST_OUTPUT" | grep -oE "failed: [0-9]+" | awk '{sum += $2} END {print sum+0}')

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
printf "| %-14s | %6s | %6s |\n" "Tests" "Passed" "Failed"
printf "|%-16s|%8s|%8s|\n" "----------------" "--------" "--------"
printf "| %-14s | %6s | %6s |\n" "Coordinator" "$COORDINATOR_PASSED" "$COORDINATOR_FAILED"
printf "| %-14s | %6s | %6s |\n" "Integrated-GMP" "$INTEGRATED_GMP_PASSED" "$INTEGRATED_GMP_FAILED"
printf "| %-14s | %6s | %6s |\n" "Solver" "$SOLVER_PASSED" "$SOLVER_FAILED"
printf "| %-14s | %6s | %6s |\n" "MVM" "$MOVE_PASSED" "$MOVE_FAILED"
printf "| %-14s | %6s | %6s |\n" "EVM" "$EVM_PASSED" "$EVM_FAILED"
printf "| %-14s | %6s | %6s |\n" "SVM" "$SVM_PASSED" "$SVM_FAILED"
printf "| %-14s | %6s | %6s |\n" "Frontend" "$FRONTEND_PASSED" "$FRONTEND_FAILED"
echo ""

