#!/usr/bin/env bash
# Generate test summary table for all components
# Usage: ./testing-infra/run-all-unit-tests.sh

# Don't use set -e so we can capture all test results even if some fail

# Track overall exit code
OVERALL_EXIT=0

echo "Running Chain-Clients tests..."
CHAIN_CLIENTS_EXIT=0
CHAIN_CLIENTS_TEST_OUTPUT=$(nix develop ./nix -c bash -c "./chain-clients/scripts/test.sh 2>&1") || CHAIN_CLIENTS_EXIT=$?
if [ $CHAIN_CLIENTS_EXIT -ne 0 ]; then
    echo "Chain-Clients tests failed (exit code $CHAIN_CLIENTS_EXIT):"
    echo "$CHAIN_CLIENTS_TEST_OUTPUT"
fi
CHAIN_CLIENTS_PASSED=$(echo "$CHAIN_CLIENTS_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
CHAIN_CLIENTS_FAILED=$(echo "$CHAIN_CLIENTS_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')
if [ $CHAIN_CLIENTS_EXIT -ne 0 ] && [ "$CHAIN_CLIENTS_PASSED" = "0" ] && [ "$CHAIN_CLIENTS_FAILED" = "0" ]; then
    CHAIN_CLIENTS_FAILED="ERR"
    OVERALL_EXIT=1
fi

echo "Running Coordinator tests..."
COORDINATOR_EXIT=0
COORDINATOR_TEST_OUTPUT=$(RUST_LOG=off nix develop ./nix -c bash -c "cd coordinator && cargo test --quiet 2>&1") || COORDINATOR_EXIT=$?
if [ $COORDINATOR_EXIT -ne 0 ]; then
    echo "Coordinator tests failed (exit code $COORDINATOR_EXIT):"
    echo "$COORDINATOR_TEST_OUTPUT"
fi
COORDINATOR_PASSED=$(echo "$COORDINATOR_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
COORDINATOR_FAILED=$(echo "$COORDINATOR_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')
if [ $COORDINATOR_EXIT -ne 0 ] && [ "$COORDINATOR_PASSED" = "0" ] && [ "$COORDINATOR_FAILED" = "0" ]; then
    COORDINATOR_FAILED="ERR"
    OVERALL_EXIT=1
fi

echo "Running Integrated-GMP tests..."
INTEGRATED_GMP_EXIT=0
INTEGRATED_GMP_TEST_OUTPUT=$(RUST_LOG=off nix develop ./nix -c bash -c "cd integrated-gmp && cargo test --quiet 2>&1") || INTEGRATED_GMP_EXIT=$?
if [ $INTEGRATED_GMP_EXIT -ne 0 ]; then
    echo "Integrated-GMP tests failed (exit code $INTEGRATED_GMP_EXIT):"
    echo "$INTEGRATED_GMP_TEST_OUTPUT"
fi
INTEGRATED_GMP_PASSED=$(echo "$INTEGRATED_GMP_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
INTEGRATED_GMP_FAILED=$(echo "$INTEGRATED_GMP_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')
if [ $INTEGRATED_GMP_EXIT -ne 0 ] && [ "$INTEGRATED_GMP_PASSED" = "0" ] && [ "$INTEGRATED_GMP_FAILED" = "0" ]; then
    INTEGRATED_GMP_FAILED="ERR"
    OVERALL_EXIT=1
fi

echo "Running Solver tests..."
SOLVER_EXIT=0
SOLVER_TEST_OUTPUT=$(RUST_LOG=off nix develop ./nix -c bash -c "cd solver && cargo test --quiet 2>&1") || SOLVER_EXIT=$?
if [ $SOLVER_EXIT -ne 0 ]; then
    echo "Solver tests failed (exit code $SOLVER_EXIT):"
    echo "$SOLVER_TEST_OUTPUT"
fi
SOLVER_PASSED=$(echo "$SOLVER_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
SOLVER_FAILED=$(echo "$SOLVER_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')
if [ $SOLVER_EXIT -ne 0 ] && [ "$SOLVER_PASSED" = "0" ] && [ "$SOLVER_FAILED" = "0" ]; then
    SOLVER_FAILED="ERR"
    OVERALL_EXIT=1
fi

echo "Running MVM tests..."
MVM_EXIT=0
MVM_TEST_OUTPUT=$(./intent-frameworks/mvm/scripts/test.sh 2>&1) || MVM_EXIT=$?
if [ $MVM_EXIT -ne 0 ]; then
    echo "MVM tests failed (exit code $MVM_EXIT):"
    echo "$MVM_TEST_OUTPUT"
fi
# Parse "passed: N" from each package output and sum
MOVE_PASSED=$(echo "$MVM_TEST_OUTPUT" | grep -oE "passed: [0-9]+" | awk '{sum += $2} END {print sum+0}')
MOVE_FAILED=$(echo "$MVM_TEST_OUTPUT" | grep -oE "failed: [0-9]+" | awk '{sum += $2} END {print sum+0}')
if [ $MVM_EXIT -ne 0 ] && [ "$MOVE_PASSED" = "0" ] && [ "$MOVE_FAILED" = "0" ]; then
    MOVE_FAILED="ERR"
    OVERALL_EXIT=1
fi

echo "Running EVM tests..."
EVM_EXIT=0
EVM_TEST_OUTPUT=$(nix develop ./nix -c bash -c "cd intent-frameworks/evm && npm install && npm test" 2>&1) || EVM_EXIT=$?
if [ $EVM_EXIT -ne 0 ]; then
    echo "EVM tests failed (exit code $EVM_EXIT):"
    echo "$EVM_TEST_OUTPUT"
fi
EVM_PASSED=$(echo "$EVM_TEST_OUTPUT" | grep -oE "[0-9]+ passing" | awk '{print $1}')
EVM_FAILED=$(echo "$EVM_TEST_OUTPUT" | grep -oE "[0-9]+ failing" | awk '{print $1}' || echo "0")
EVM_PASSED=${EVM_PASSED:-0}
EVM_FAILED=${EVM_FAILED:-0}
if [ $EVM_EXIT -ne 0 ] && [ "$EVM_PASSED" = "0" ] && [ "$EVM_FAILED" = "0" ]; then
    EVM_FAILED="ERR"
    OVERALL_EXIT=1
fi

echo "Running SVM tests..."
# Build and run tests
SVM_EXIT=0
SVM_TEST_OUTPUT=$(cd intent-frameworks/svm && ./scripts/test.sh 2>&1) || SVM_EXIT=$?
if [ $SVM_EXIT -ne 0 ]; then
    echo "SVM tests failed (exit code $SVM_EXIT):"
    echo "$SVM_TEST_OUTPUT"
fi
# Parse cargo test output (e.g., "test result: ok. 3 passed; 0 failed;")
SVM_PASSED=$(echo "$SVM_TEST_OUTPUT" | grep -oE "[0-9]+ passed" | awk '{sum += $1} END {print sum+0}')
SVM_FAILED=$(echo "$SVM_TEST_OUTPUT" | grep -oE "[0-9]+ failed" | awk '{sum += $1} END {print sum+0}')
if [ $SVM_EXIT -ne 0 ] && [ "$SVM_PASSED" = "0" ] && [ "$SVM_FAILED" = "0" ]; then
    SVM_FAILED="ERR"
    OVERALL_EXIT=1
fi

echo "Running Frontend tests..."
FRONTEND_EXIT=0
FRONTEND_TEST_OUTPUT=$(nix develop ./nix -c bash -c "cd frontend && npm install --legacy-peer-deps && npm test" 2>&1) || FRONTEND_EXIT=$?
if [ $FRONTEND_EXIT -ne 0 ]; then
    echo "Frontend tests failed (exit code $FRONTEND_EXIT):"
    echo "$FRONTEND_TEST_OUTPUT"
fi
FRONTEND_PASSED=$(echo "$FRONTEND_TEST_OUTPUT" | grep -oE "Tests[[:space:]]+[0-9]+ passed" | grep -oE "[0-9]+")
FRONTEND_FAILED=$(echo "$FRONTEND_TEST_OUTPUT" | grep -oE "Tests[[:space:]]+[0-9]+ failed" | grep -oE "[0-9]+" || echo "0")
FRONTEND_PASSED=${FRONTEND_PASSED:-0}
FRONTEND_FAILED=${FRONTEND_FAILED:-0}
if [ $FRONTEND_EXIT -ne 0 ] && [ "$FRONTEND_PASSED" = "0" ] && [ "$FRONTEND_FAILED" = "0" ]; then
    FRONTEND_FAILED="ERR"
    OVERALL_EXIT=1
fi

echo "=== Test Summary Table ==="
echo ""
printf "| %-14s | %6s | %6s |\n" "Tests" "Passed" "Failed"
printf "|%-16s|%8s|%8s|\n" "----------------" "--------" "--------"
printf "| %-14s | %6s | %6s |\n" "Chain-Clients" "$CHAIN_CLIENTS_PASSED" "$CHAIN_CLIENTS_FAILED"
printf "| %-14s | %6s | %6s |\n" "Coordinator" "$COORDINATOR_PASSED" "$COORDINATOR_FAILED"
printf "| %-14s | %6s | %6s |\n" "Integrated-GMP" "$INTEGRATED_GMP_PASSED" "$INTEGRATED_GMP_FAILED"
printf "| %-14s | %6s | %6s |\n" "Solver" "$SOLVER_PASSED" "$SOLVER_FAILED"
printf "| %-14s | %6s | %6s |\n" "MVM" "$MOVE_PASSED" "$MOVE_FAILED"
printf "| %-14s | %6s | %6s |\n" "EVM" "$EVM_PASSED" "$EVM_FAILED"
printf "| %-14s | %6s | %6s |\n" "SVM" "$SVM_PASSED" "$SVM_FAILED"
printf "| %-14s | %6s | %6s |\n" "Frontend" "$FRONTEND_PASSED" "$FRONTEND_FAILED"
echo ""

exit $OVERALL_EXIT

