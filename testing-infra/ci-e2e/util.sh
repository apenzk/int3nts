#!/bin/bash

# Common utilities for testing infrastructure scripts
# Source this file in other scripts with: source "$(dirname "$0")/util.sh" or similar

# Pinned Aptos Docker image for reproducible builds
export APTOS_DOCKER_IMAGE="aptoslabs/tools@sha256:b6d7fc304963929ad89ef74da020ed995da22dc11fd6cb68cf5f17b6bfff0ccf"


# Build only if any of the specified output files are missing.
# Used by --no-build to skip builds when binaries already exist.
# Usage: build_if_missing <build_dir> <build_command> <description> <check_paths...>
#   build_dir: Directory to run the build in (pushd target)
#   build_command: The build command to run (e.g., "cargo build --bin coordinator")
#   description: Human-readable description for log output
#   check_paths: One or more output file paths to check; rebuilds if ANY is missing
build_if_missing() {
    local build_dir="$1"
    local build_command="$2"
    local description="$3"
    shift 3

    local needs_build=false
    for check_path in "$@"; do
        if [ ! -f "$check_path" ]; then
            needs_build=true
            break
        fi
    done

    if [ "$needs_build" = "true" ]; then
        pushd "$build_dir" > /dev/null
        eval "$build_command" 2>&1 | tail -5
        popd > /dev/null
        log_and_echo "   ✅ $description (built)"
    else
        log_and_echo "   ✅ $description (exists)"
    fi
}

# Build the common set of binaries required by all E2E test scripts.
# Checks each binary individually and only builds what's missing.
# Usage: build_common_bins_if_missing
# Requires PROJECT_ROOT to be set.
build_common_bins_if_missing() {
    # Order: coordinator first (fastest independent build ~57s),
    # then generate_keys before integrated-gmp (same crate, warms dep cache)
    build_if_missing "$PROJECT_ROOT/coordinator" "cargo build --bin coordinator" \
        "Coordinator: coordinator" \
        "$PROJECT_ROOT/coordinator/target/debug/coordinator"
    build_if_missing "$PROJECT_ROOT/integrated-gmp" "cargo build --bin generate_keys" \
        "Integrated-GMP: generate_keys" \
        "$PROJECT_ROOT/integrated-gmp/target/debug/generate_keys"
    build_if_missing "$PROJECT_ROOT/integrated-gmp" "cargo build --bin integrated-gmp" \
        "Integrated-GMP: integrated-gmp" \
        "$PROJECT_ROOT/integrated-gmp/target/debug/integrated-gmp"
    build_if_missing "$PROJECT_ROOT/solver" "cargo build --bin solver" \
        "Solver: solver" \
        "$PROJECT_ROOT/solver/target/debug/solver"
}

# Get project root - can be called from any script location
# Usage: Call this function to set PROJECT_ROOT and optionally change to it
# Note: If SCRIPT_DIR is already set by the calling script, use that; otherwise derive from BASH_SOURCE
setup_project_root() {
    local script_dir
    
    # Use SCRIPT_DIR if already set (set by scripts before sourcing)
    if [ -n "$SCRIPT_DIR" ]; then
        script_dir="$SCRIPT_DIR"
    else
        # Get the calling script's path (BASH_SOURCE[1] because [0] is util.sh)
        local script_path="${BASH_SOURCE[1]}"
        if [ -z "$script_path" ]; then
            # Fallback if called differently
            script_path="${BASH_SOURCE[0]}"
        fi
        script_dir="$( cd "$( dirname "$script_path" )" && pwd )"
    fi
    
    # Determine how many levels up to go based on script location
    # Scripts in testing-infra/*/* need to go up 2 levels
    # Scripts in testing-infra/* need to go up 1 level
    if [[ "$script_dir" == *"/testing-infra/"*"/"* ]]; then
        # Script is in a subdirectory (e.g., testing-infra/ci-e2e/e2e-tests-mvm/)
        PROJECT_ROOT="$( cd "$script_dir/../../.." && pwd )"
    else
        # Script is directly in testing-infra/
        PROJECT_ROOT="$( cd "$script_dir/../.." && pwd )"
    fi
    
    export PROJECT_ROOT
}

# Setup logging functions and directory
# Usage: setup_logging "script-name"
# Creates log file: .tmp/e2e-tests/script-name.log
setup_logging() {
    local script_name="${1:-script}"
    
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi
    
    LOG_DIR="$PROJECT_ROOT/.tmp/e2e-tests"
    mkdir -p "$LOG_DIR"
    LOG_FILE="$LOG_DIR/${script_name}.log"
    # Truncate log file for a clean run
    : > "$LOG_FILE"
    
    export LOG_DIR LOG_FILE
}

# Helper function to print important messages to terminal (also logs them)
log_and_echo() {
    echo "$@"
    [ -n "$LOG_FILE" ] && echo "$@" >> "$LOG_FILE"
    return 0  # Prevent set -e from exiting on [ -n "$LOG_FILE" ] returning false
}

# Helper function to write only to log file (not terminal)
log() {
    echo "$@"
    [ -n "$LOG_FILE" ] && echo "$@" >> "$LOG_FILE"
    return 0  # Prevent set -e from exiting on [ -n "$LOG_FILE" ] returning false
}

# Setup solver configuration for E2E/CI testing
# Usage: setup_solver_config
# Always creates config from template (overwrites any existing config)
# This function is used by E2E test scripts
#
# SECURITY: This function ALWAYS creates a fresh config from template for E2E/CI testing.
# Test scripts should populate it with ephemeral/test addresses and keys.
setup_solver_config() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi

    SOLVER_E2E_CI_TESTING_CONFIG="$PROJECT_ROOT/solver/config/solver-e2e-ci-testing.toml"
    SOLVER_TEMPLATE="$PROJECT_ROOT/solver/config/solver.template.toml"

    # Always recreate config from template (ensures fresh config for each test run)
    log_and_echo "   Creating solver-e2e-ci-testing.toml from template..."
    
    if [ ! -f "$SOLVER_TEMPLATE" ]; then
        log_and_echo "❌ ERROR: solver.template.toml not found at $SOLVER_TEMPLATE"
        exit 1
    fi
    
    # Copy template (overwrites any existing config file)
    cp "$SOLVER_TEMPLATE" "$SOLVER_E2E_CI_TESTING_CONFIG"
    
    cd "$PROJECT_ROOT"
    log_and_echo "   ✅ Created solver-e2e-ci-testing.toml from template"

    # Export config path for Rust code to use (absolute path so tests can find it)
    export SOLVER_CONFIG_PATH="$SOLVER_E2E_CI_TESTING_CONFIG"

    log "   ✅ Solver config set: $SOLVER_CONFIG_PATH"
}

# Save intent information to file
# Usage: save_intent_info [intent_id] [hub_intent_addr]
# If arguments are provided, uses them; otherwise uses INTENT_ID and HUB_INTENT_ADDR env vars
# Saves to ${PROJECT_ROOT}/.tmp/intent-info.env
save_intent_info() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi

    local intent_id="${1:-$INTENT_ID}"
    local hub_intent_addr="${2:-$HUB_INTENT_ADDR}"

    if [ -z "$intent_id" ]; then
        log_and_echo "❌ ERROR: save_intent_info() requires INTENT_ID"
        exit 1
    fi

    INTENT_INFO_FILE="${PROJECT_ROOT}/.tmp/intent-info.env"
    mkdir -p "$(dirname "$INTENT_INFO_FILE")"
    
    echo "INTENT_ID=$intent_id" > "$INTENT_INFO_FILE"
    
    if [ -n "$hub_intent_addr" ] && [ "$hub_intent_addr" != "null" ]; then
        echo "HUB_INTENT_ADDR=$hub_intent_addr" >> "$INTENT_INFO_FILE"
    fi
    
    # Save SOLVER_EVM_ADDR if set (for EVM inflow escrow creation)
    if [ -n "$SOLVER_EVM_ADDR" ] && [ "$SOLVER_EVM_ADDR" != "null" ]; then
        echo "SOLVER_EVM_ADDR=$SOLVER_EVM_ADDR" >> "$INTENT_INFO_FILE"
        log "   Saved SOLVER_EVM_ADDR to intent info file"
    else
        log "   SOLVER_EVM_ADDR not set or null, skipping save"
    fi
    
    log "    Intent info saved to: $INTENT_INFO_FILE"
}

# Load intent information from file
# Usage: load_intent_info [required_vars]
#   required_vars: comma-separated list of required variables (e.g., "INTENT_ID,HUB_INTENT_ADDR")
#   If not provided, only INTENT_ID is required
#   If INTENT_ID is already set, skips loading (allows override via env var)
# Loads from ${PROJECT_ROOT}/.tmp/intent-info.env
load_intent_info() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi

    local required_vars="${1:-INTENT_ID}"
    INTENT_INFO_FILE="${PROJECT_ROOT}/.tmp/intent-info.env"

    # If INTENT_ID is already set and only INTENT_ID is required, skip loading
    if [ "$required_vars" = "INTENT_ID" ] && [ -n "$INTENT_ID" ]; then
        log "   ✅ INTENT_ID already set, skipping load"
        return 0
    fi

    if [ ! -f "$INTENT_INFO_FILE" ]; then
        log_and_echo "❌ ERROR: intent-info.env not found at $INTENT_INFO_FILE"
        if [ "$required_vars" = "INTENT_ID,HUB_INTENT_ADDR" ]; then
            log_and_echo "   Run inflow-submit-hub-intent.sh first, or provide INTENT_ID=<id> and HUB_INTENT_ADDR=<address>"
        else
            log_and_echo "   Run inflow-submit-hub-intent.sh first, or provide INTENT_ID=<id>"
        fi
        exit 1
    fi

    source "$INTENT_INFO_FILE"
    log "   ✅ Loaded intent info from $INTENT_INFO_FILE"

    # Validate required variables
    IFS=',' read -ra VARS <<< "$required_vars"
    for var in "${VARS[@]}"; do
        var=$(echo "$var" | tr -d ' ')
        local value="${!var}"
        if [ -z "$value" ]; then
            log_and_echo "❌ ERROR: $var not found in intent-info.env"
            if [ "$required_vars" = "INTENT_ID,HUB_INTENT_ADDR" ]; then
                log_and_echo "   Run inflow-submit-hub-intent.sh first"
            fi
            exit 1
        fi
    done

    return 0
}

# Check if port is listening
# Usage: check_port_listening [port]
# Returns 0 if port is listening, 1 if not
check_port_listening() {
    local port="${1:-3333}"
    
    # Try different methods depending on what's available
    if command -v ss > /dev/null 2>&1; then
        ss -ln | grep -q ":${port} " && return 0
    elif command -v netstat > /dev/null 2>&1; then
        netstat -ln | grep -q ":${port} " && return 0
    elif command -v lsof > /dev/null 2>&1; then
        lsof -i ":${port}" > /dev/null 2>&1 && return 0
    fi
    
    # Fallback: try to connect to the port
    if command -v nc > /dev/null 2>&1; then
        nc -z 127.0.0.1 "${port}" > /dev/null 2>&1 && return 0
    fi
    
    return 1
}

# Verify solver is running
# Usage: verify_solver_running
# Checks solver process
# Exits with error if solver is not running
verify_solver_running() {
    # Ensure LOG_DIR is set (for reading PID files)
    if [ -z "$LOG_DIR" ] && [ -n "$PROJECT_ROOT" ]; then
        LOG_DIR="$PROJECT_ROOT/.tmp/e2e-tests"
    fi
    
    log ""
    log " Verifying solver is running..."
    
    # Try to load SOLVER_PID from file if not set
    if [ -z "$SOLVER_PID" ] && [ -n "$LOG_DIR" ] && [ -f "$LOG_DIR/solver.pid" ]; then
        SOLVER_PID=$(cat "$LOG_DIR/solver.pid" 2>/dev/null || echo "")
        export SOLVER_PID
    fi
    
    # Check solver
    if [ -z "$SOLVER_PID" ] || ! ps -p "$SOLVER_PID" > /dev/null 2>&1; then
        log_and_echo "❌ ERROR: Solver is not running"
        log_and_echo "   Expected PID: ${SOLVER_PID:-<not set>}"
        log_and_echo "   Please start solver first using start-solver.sh"
        exit 1
    fi
    log "   ✅ Solver is running (PID: $SOLVER_PID)"
}

# Note: verify_solver_registered() is defined in util_mvm.sh with auto-detection
# Both MVM and EVM E2E tests source util_mvm.sh since the hub chain is always MVM

# Display solver, coordinator, and integrated-gmp logs for debugging
# Usage: display_service_logs [context_message]
# Shows last 100 lines of solver.log, coordinator.log, and integrated-gmp.log if they exist
display_service_logs() {
    local context="${1:-Error occurred}"
    
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi
    
    local log_dir="$PROJECT_ROOT/.tmp/e2e-tests"
    local solver_log="$log_dir/solver.log"
    local coordinator_log="$log_dir/coordinator.log"
    local integrated_gmp_log="$log_dir/integrated-gmp.log"
    
    # Get current timestamp in ISO format (matches Rust log format)
    local error_timestamp=$(date -u +"%Y-%m-%dT%H:%M:%S.%NZ" 2>/dev/null || date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    log_and_echo ""
    log_and_echo " Service Logs ($context)"
    log_and_echo "=========================================="
    log_and_echo "⏰ Error occurred at: $error_timestamp"
    log_and_echo ""
    
    if [ -f "$coordinator_log" ]; then
        log_and_echo ""
        log_and_echo " Coordinator logs:"
        log_and_echo "-----------------------------------"
        tail -100 "$coordinator_log" | sed 's/^/   /'
    else
        log_and_echo ""
        log_and_echo "️  Coordinator log not found: $coordinator_log"
    fi
    
    if [ -f "$integrated_gmp_log" ]; then
        log_and_echo ""
        log_and_echo " Integrated-GMP logs:"
        log_and_echo "-----------------------------------"
        tail -100 "$integrated_gmp_log" | sed 's/^/   /'
    else
        log_and_echo ""
        log_and_echo "️  Integrated-GMP log not found: $integrated_gmp_log"
    fi
    
    if [ -f "$solver_log" ]; then
        # Surface WARN/ERROR lines first for quick diagnosis
        local warn_error_lines
        warn_error_lines=$(grep -E 'WARN|ERROR' "$solver_log" 2>/dev/null | sed 's/^/   /' || true)
        if [ -n "$warn_error_lines" ]; then
            log_and_echo ""
            log_and_echo "⚠ Solver WARN/ERROR lines:"
            log_and_echo "-----------------------------------"
            echo "$warn_error_lines" | while IFS= read -r line; do log_and_echo "$line"; done
            log_and_echo "-----------------------------------"
        fi
        log_and_echo ""
        log_and_echo " Solver logs:"
        log_and_echo "-----------------------------------"
        cat "$solver_log" | sed 's/^/   /'
    else
        log_and_echo ""
        log_and_echo "️  Solver log not found: $solver_log"
    fi

    # Show coordinator events summary
    local coordinator_url="${COORDINATOR_URL:-http://127.0.0.1:3333}"
    local events_response
    events_response=$(curl -s "${coordinator_url}/events" 2>/dev/null)
    if [ $? -eq 0 ]; then
        local escrow_count fulfillment_count intent_count
        escrow_count=$(echo "$events_response" | jq -r '.data.escrow_events | length' 2>/dev/null || echo "0")
        fulfillment_count=$(echo "$events_response" | jq -r '.data.fulfillment_events | length' 2>/dev/null || echo "0")
        intent_count=$(echo "$events_response" | jq -r '.data.intent_events | length' 2>/dev/null || echo "0")

        log_and_echo ""
        log_and_echo " Coordinator events:"
        log_and_echo "   Intent events: $intent_count"
        log_and_echo "   Escrow events: $escrow_count"
        log_and_echo "   Fulfillment events: $fulfillment_count"

        if [ "$escrow_count" != "0" ]; then
            log_and_echo ""
            log_and_echo "   Escrow details:"
            echo "$events_response" | jq -r '.data.escrow_events[] | "      \(.intent_id) - amount: \(.offered_amount)"' 2>/dev/null || log_and_echo "      (parse error)"
        fi
    fi

    log_and_echo ""
}

# Stop solver processes
# Usage: stop_solver
# Stops any running solver processes
stop_solver() {
    log "   Checking for existing solvers..."
    
    if pgrep -f "cargo.*solver" > /dev/null || pgrep -f "target/debug/solver" > /dev/null; then
        log "   ️  Found existing solver processes, stopping them..."
        pkill -f "cargo.*solver" || true
        pkill -f "target/debug/solver" || true
        sleep 2
        log "   ✅ Solver processes stopped"
    else
        log "   ✅ No existing solver processes"
    fi
}

# Check solver health (placeholder - solver service doesn't have health endpoint yet)
# Usage: check_solver_health [port]
# Returns 0 if healthy, 1 if not
# TODO: Implement health check once solver service has health endpoint
check_solver_health() {
    # Placeholder - solver service will have health endpoint in future
    # For now, just check if process is running
    local port="${1:-3334}"
    
    # TODO: Once solver service is implemented, check health endpoint
    # if curl -s -f "http://127.0.0.1:${port}/health" > /dev/null 2>&1; then
    #     return 0
    # else
    #     return 1
    # fi
    
    # For now, return 1 (not implemented)
    return 1
}


# Start solver service
# Usage: start_solver [log_file] [rust_log_level] [config_path]
# Starts solver in background and waits for it to be ready
# Sets SOLVER_PID and SOLVER_LOG global variables
# Exits with error if solver fails to start
# NOTE: This will fail until solver service is implemented (Task 6-7)
start_solver() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi

    local log_file="${1:-$LOG_DIR/solver.log}"
    local rust_log="${2:-info}"
    local config_path="${3:-$PROJECT_ROOT/solver/config/solver.toml}"
    
    # Ensure log directory exists
    mkdir -p "$(dirname "$log_file")"
    
    # Delete existing log file to start fresh for this test run
    if [ -f "$log_file" ]; then
        rm -f "$log_file"
    fi
    
    # Stop any existing solver first
    stop_solver
    
    log "   Starting solver service..."
    log "   Using config: $config_path"
    log "   Log file: $log_file"
    
    # Use pre-built binary (must be built in Step 1)
    local solver_binary="$PROJECT_ROOT/solver/target/debug/solver"
    if [ ! -f "$solver_binary" ]; then
        log_and_echo "   ❌ PANIC: solver not built. Step 1 (build binaries) failed."
        exit 1
    fi
    
    log "   Using binary: $solver_binary"
    SOLVER_CONFIG_PATH="$config_path" RUST_LOG="$rust_log" "$solver_binary" >> "$log_file" 2>&1 &
    SOLVER_PID=$!
    
    # Export PID so it persists across subshells
    export SOLVER_PID
    
    # Save PID to file for cross-script persistence
    if [ -n "$LOG_DIR" ]; then
        echo "$SOLVER_PID" > "$LOG_DIR/solver.pid"
    fi
    
    log "   ✅ Solver started with PID: $SOLVER_PID"
    
    # Wait for solver to be ready (check for "Starting all services" in log)
    # This properly waits for compilation + initialization in CI
    log "   - Waiting for solver to initialize (may take a while if compiling)..."
    RETRY_COUNT=0
    MAX_RETRIES=180  # 3 minutes to allow for compilation in CI
    
    while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
        # Check if process is still running
        if ! ps -p "$SOLVER_PID" > /dev/null 2>&1; then
            log_and_echo "   ❌ Solver process died"
            log_and_echo "   Solver log:"
            log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
            if [ -f "$log_file" ]; then
                cat "$log_file" | while read line; do log_and_echo "   $line"; done
            else
                log_and_echo "   Log file not found at: $log_file"
            fi
            log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
            exit 1
        fi
        
        # Check if solver has initialized by looking for the startup log message
        if [ -f "$log_file" ] && grep -q "Starting all services" "$log_file" 2>/dev/null; then
            log "   ✅ Solver is ready!"
            SOLVER_LOG="$log_file"
            export SOLVER_PID SOLVER_LOG
            return 0
        fi
        
        # Show progress every 10 seconds
        if [ $((RETRY_COUNT % 10)) -eq 0 ] && [ $RETRY_COUNT -gt 0 ]; then
            if [ -f "$log_file" ]; then
                # Show last line of log to indicate progress
                local last_line=$(tail -1 "$log_file" 2>/dev/null || echo "")
                if [ -n "$last_line" ]; then
                    log "   ... still waiting (${RETRY_COUNT}s): $last_line"
                fi
            fi
        fi
        
        sleep 1
        RETRY_COUNT=$((RETRY_COUNT + 1))
    done
    
    # If we get here, solver didn't become ready
    log_and_echo "   ❌ Solver failed to start after $MAX_RETRIES seconds"
    log_and_echo "   Solver log:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    if [ -f "$log_file" ]; then
        log_and_echo "   $(cat "$log_file")"
    else
        log_and_echo "   Log file not found at: $log_file"
    fi
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
}

# ============================================================================
# COORDINATOR SERVICE FUNCTIONS (Phase 0 - No Keys, Read-Only)
# ============================================================================

# Stop coordinator processes
# Usage: stop_coordinator
# Stops any running coordinator processes
stop_coordinator() {
    log "   Checking for existing coordinators..."

    if pgrep -f "cargo.*coordinator" > /dev/null || pgrep -f "target/debug/coordinator" > /dev/null; then
        log "   ️  Found existing coordinator processes, stopping them..."
        pkill -f "cargo.*coordinator" || true
        pkill -f "target/debug/coordinator" || true
        sleep 2
        log "   ✅ Coordinator processes stopped"
    else
        log "   ✅ No existing coordinator processes"
    fi
}

# Check coordinator health
# Usage: check_coordinator_health [port]
# Checks if coordinator health endpoint responds
# Returns 0 if healthy, 1 if not
check_coordinator_health() {
    local port="${1:-3333}"

    if curl -s -f "http://127.0.0.1:${port}/health" > /dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Verify coordinator is running
# Usage: verify_coordinator_running
# Checks coordinator process, port, and health endpoint
# Exits with error if coordinator is not running
verify_coordinator_running() {
    # Ensure LOG_DIR is set (for reading PID files)
    if [ -z "$LOG_DIR" ] && [ -n "$PROJECT_ROOT" ]; then
        LOG_DIR="$PROJECT_ROOT/.tmp/e2e-tests"
    fi

    log ""
    log " Verifying coordinator is running..."

    # Try to load COORDINATOR_PID from file if not set
    if [ -z "$COORDINATOR_PID" ] && [ -n "$LOG_DIR" ] && [ -f "$LOG_DIR/coordinator.pid" ]; then
        COORDINATOR_PID=$(cat "$LOG_DIR/coordinator.pid" 2>/dev/null || echo "")
        export COORDINATOR_PID
    fi

    # Check coordinator process
    if [ -z "$COORDINATOR_PID" ] || ! ps -p "$COORDINATOR_PID" > /dev/null 2>&1; then
        log_and_echo "❌ ERROR: Coordinator process is not running"
        log_and_echo "   Expected PID: ${COORDINATOR_PID:-<not set>}"
        log_and_echo "   Please start coordinator first using start-coordinator.sh"
        exit 1
    fi

    # Check if coordinator port is listening
    COORDINATOR_PORT="${COORDINATOR_PORT:-3333}"
    if ! check_port_listening "$COORDINATOR_PORT"; then
        log_and_echo "❌ ERROR: Coordinator is not listening on port $COORDINATOR_PORT"
        log_and_echo "   Coordinator PID: $COORDINATOR_PID"
        log_and_echo "   Process exists but port is not accessible"
        log_and_echo "   Check logs: ${COORDINATOR_LOG:-<not set>}"
        exit 1
    fi

    # Check coordinator health endpoint
    if ! check_coordinator_health "$COORDINATOR_PORT"; then
        log_and_echo "❌ ERROR: Coordinator health check failed"
        log_and_echo "   Coordinator PID: $COORDINATOR_PID"
        log_and_echo "   Port $COORDINATOR_PORT is listening but /health endpoint failed"
        log_and_echo "   Check logs: ${COORDINATOR_LOG:-<not set>}"
        exit 1
    fi
    log "   ✅ Coordinator is running and healthy (PID: $COORDINATOR_PID, port: $COORDINATOR_PORT)"
}

# Start coordinator service
# Usage: start_coordinator [log_file] [rust_log_level]
# Starts coordinator in background and waits for it to be ready
# Sets COORDINATOR_PID and COORDINATOR_LOG global variables
# Exits with error if coordinator fails to start
start_coordinator() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi

    if [ -z "$COORDINATOR_CONFIG_PATH" ]; then
        export COORDINATOR_CONFIG_PATH="$PROJECT_ROOT/coordinator/config/coordinator-e2e-ci-testing.toml"
    fi

    local log_file="${1:-$LOG_DIR/coordinator.log}"
    local rust_log="${2:-info}"

    # Ensure log directory exists
    mkdir -p "$(dirname "$log_file")"

    # Delete existing log file to start fresh for this test run
    if [ -f "$log_file" ]; then
        rm -f "$log_file"
    fi

    # Stop any existing coordinator first
    stop_coordinator

    log "   Starting coordinator service..."
    log "   Using config: $COORDINATOR_CONFIG_PATH"
    log "   Log file: $log_file"

    # Use pre-built binary (must be built in Step 1)
    local coordinator_binary="$PROJECT_ROOT/coordinator/target/debug/coordinator"
    if [ ! -f "$coordinator_binary" ]; then
        log_and_echo "   ❌ PANIC: coordinator not built. Step 1 (build binaries) failed."
        exit 1
    fi

    log "   Using binary: $coordinator_binary"
    COORDINATOR_CONFIG_PATH="$COORDINATOR_CONFIG_PATH" RUST_LOG="$rust_log" "$coordinator_binary" >> "$log_file" 2>&1 &
    COORDINATOR_PID=$!

    # Export PID so it persists across subshells
    export COORDINATOR_PID

    # Save PID to file for cross-script persistence
    if [ -n "$LOG_DIR" ]; then
        echo "$COORDINATOR_PID" > "$LOG_DIR/coordinator.pid"
    fi

    log "   ✅ Coordinator started with PID: $COORDINATOR_PID"

    # Wait for coordinator to be ready
    log "   - Waiting for coordinator to initialize..."
    RETRY_COUNT=0
    MAX_RETRIES=180

    while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
        # Check if process is still running
        if ! ps -p "$COORDINATOR_PID" > /dev/null 2>&1; then
            log_and_echo "   ❌ Coordinator process died"
            log_and_echo "   Coordinator log:"
            log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
            if [ -f "$log_file" ]; then
                log_and_echo "   $(cat "$log_file")"
            else
                log_and_echo "   Log file not found at: $log_file"
            fi
            log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
            exit 1
        fi

        # Check health endpoint
        if check_coordinator_health; then
            log "   ✅ Coordinator is ready!"

            # Give coordinator time to start polling and collect initial events
            log "   - Waiting for coordinator to poll and collect events (30 seconds)..."
            sleep 30

            COORDINATOR_LOG="$log_file"
            export COORDINATOR_PID COORDINATOR_LOG
            return 0
        fi

        sleep 1
        RETRY_COUNT=$((RETRY_COUNT + 1))
    done

    # If we get here, coordinator didn't become healthy
    log_and_echo "   ❌ Coordinator failed to start after $MAX_RETRIES seconds"
    log_and_echo "   Coordinator log:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    if [ -f "$log_file" ]; then
        log_and_echo "   $(cat "$log_file")"
    else
        log_and_echo "   Log file not found at: $log_file"
    fi
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
}

# ============================================================================
# INTEGRATED GMP SERVICE FUNCTIONS (Phase 0 - Has Keys, Validation)
# ============================================================================

# Stop integrated-gmp processes
# Usage: stop_integrated_gmp
# Stops any running integrated-gmp processes
stop_integrated_gmp() {
    log "   Checking for existing integrated-gmp..."

    if pgrep -f "cargo.*integrated.gmp" > /dev/null || pgrep -f "target/debug/integrated.gmp" > /dev/null; then
        log "   ️  Found existing integrated-gmp processes, stopping them..."
        pkill -f "cargo.*integrated.gmp" || true
        pkill -f "target/debug/integrated.gmp" || true
        sleep 2
        log "   ✅ Integrated-GMP processes stopped"
    else
        log "   ✅ No existing integrated-gmp processes"
    fi
}

# Check integrated-gmp health
# Usage: check_integrated_gmp_health
# Checks if integrated-gmp process is running and has initialized
# The integrated GMP relay has no HTTP server; we check the log for init message
# Returns 0 if healthy, 1 if not
check_integrated_gmp_health() {
    # Check process is alive
    if [ -n "$INTEGRATED_GMP_PID" ] && ps -p "$INTEGRATED_GMP_PID" > /dev/null 2>&1; then
        # Check log for successful initialization
        local log_file="${INTEGRATED_GMP_LOG:-${LOG_DIR:-/dev/null}/integrated-gmp.log}"
        if [ -f "$log_file" ] && grep -q "Integrated GMP relay initialized successfully" "$log_file" 2>/dev/null; then
            return 0
        fi
    fi
    return 1
}

# Verify integrated-gmp is running
# Usage: verify_integrated_gmp_running
# Checks integrated-gmp process is alive and initialized
# Exits with error if integrated-gmp is not running
verify_integrated_gmp_running() {
    # Ensure LOG_DIR is set (for reading PID files)
    if [ -z "$LOG_DIR" ] && [ -n "$PROJECT_ROOT" ]; then
        LOG_DIR="$PROJECT_ROOT/.tmp/e2e-tests"
    fi

    log ""
    log " Verifying integrated-gmp is running..."

    # Try to load INTEGRATED_GMP_PID from file if not set
    if [ -z "$INTEGRATED_GMP_PID" ] && [ -n "$LOG_DIR" ] && [ -f "$LOG_DIR/integrated-gmp.pid" ]; then
        INTEGRATED_GMP_PID=$(cat "$LOG_DIR/integrated-gmp.pid" 2>/dev/null || echo "")
        export INTEGRATED_GMP_PID
    fi

    # Load log path if not set
    if [ -z "$INTEGRATED_GMP_LOG" ] && [ -n "$LOG_DIR" ]; then
        INTEGRATED_GMP_LOG="$LOG_DIR/integrated-gmp.log"
    fi

    # Check integrated-gmp process
    if [ -z "$INTEGRATED_GMP_PID" ] || ! ps -p "$INTEGRATED_GMP_PID" > /dev/null 2>&1; then
        log_and_echo "❌ ERROR: Integrated-GMP process is not running"
        log_and_echo "   Expected PID: ${INTEGRATED_GMP_PID:-<not set>}"
        log_and_echo "   Please start integrated-gmp first using start-integrated-gmp.sh"
        exit 1
    fi

    # Check integrated-gmp initialized (integrated GMP relay has no HTTP server)
    if ! check_integrated_gmp_health; then
        log_and_echo "❌ ERROR: Integrated-GMP health check failed"
        log_and_echo "   Integrated-GMP PID: $INTEGRATED_GMP_PID"
        log_and_echo "   Process is running but initialization not confirmed in log"
        log_and_echo "   Check logs: ${INTEGRATED_GMP_LOG:-<not set>}"
        exit 1
    fi
    log "   ✅ Integrated-GMP is running and healthy (PID: $INTEGRATED_GMP_PID)"
}

# Generate integrated-gmp keys for E2E/CI testing
# Usage: generate_integrated_gmp_keys
# Generates fresh ephemeral keys for E2E/CI testing and exports them as env vars.
generate_integrated_gmp_keys() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi

    INTEGRATED_GMP_KEYS_FILE="$PROJECT_ROOT/testing-infra/ci-e2e/.integrated-gmp-keys.env"
    INTEGRATED_GMP_CONFIG_FILE="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp-e2e-ci-testing.toml"

    # If keys already exist, just load them
    if [ -f "$INTEGRATED_GMP_KEYS_FILE" ]; then
        source "$INTEGRATED_GMP_KEYS_FILE"
        export E2E_INTEGRATED_GMP_PRIVATE_KEY
        export E2E_INTEGRATED_GMP_PUBLIC_KEY
        export E2E_INTEGRATED_GMP_MOVE_ADDRESS
        log_and_echo "   ✅ Loaded existing integrated-gmp ephemeral keys"
        return
    fi

    log_and_echo "   Generating integrated-gmp ephemeral test keys..."
    cd "$PROJECT_ROOT/integrated-gmp"

    # Use pre-built binary (must be built in Step 1)
    local generate_keys_bin="$PROJECT_ROOT/integrated-gmp/target/debug/generate_keys"
    if [ ! -x "$generate_keys_bin" ]; then
        log_and_echo "❌ PANIC: integrated-gmp generate_keys not built. Step 1 (build binaries) failed."
        exit 1
    fi
    KEYS_OUTPUT=$("$generate_keys_bin" 2>/dev/null)

    # Extract keys and addresses from output (format: KEY=value).
    # Use cut -d= -f2- to split on the first '=' only (base64 values contain '=' padding).
    PRIVATE_KEY=$(echo "$KEYS_OUTPUT" | grep "INTEGRATED_GMP_PRIVATE_KEY=" | cut -d= -f2-)
    PUBLIC_KEY=$(echo "$KEYS_OUTPUT" | grep "INTEGRATED_GMP_PUBLIC_KEY=" | cut -d= -f2-)
    MOVE_ADDRESS=$(echo "$KEYS_OUTPUT" | grep "INTEGRATED_GMP_MVM_ADDR=" | cut -d= -f2-)

    if [ -z "$PRIVATE_KEY" ] || [ -z "$PUBLIC_KEY" ]; then
        log_and_echo "❌ ERROR: Failed to generate integrated-gmp test keys"
        exit 1
    fi

    # Export keys as environment variables (E2E prefix to avoid collision)
    export E2E_INTEGRATED_GMP_PRIVATE_KEY="$PRIVATE_KEY"
    export E2E_INTEGRATED_GMP_PUBLIC_KEY="$PUBLIC_KEY"
    export E2E_INTEGRATED_GMP_MOVE_ADDRESS="$MOVE_ADDRESS"

    # Save keys to file for reuse within the same test run
    cat > "$INTEGRATED_GMP_KEYS_FILE" << EOF
# Ephemeral integrated-gmp keys for E2E/CI testing
# Generated at: $(date)
# WARNING: These keys are for testing only. Do not use in production.
E2E_INTEGRATED_GMP_PRIVATE_KEY="$PRIVATE_KEY"
E2E_INTEGRATED_GMP_PUBLIC_KEY="$PUBLIC_KEY"
E2E_INTEGRATED_GMP_MOVE_ADDRESS="$MOVE_ADDRESS"
EOF

    cd "$PROJECT_ROOT"
    log_and_echo "   ✅ Generated fresh integrated-gmp ephemeral keys"
}

# Load integrated-gmp keys
# Usage: load_integrated_gmp_keys
# Loads previously generated keys from the keys file.
load_integrated_gmp_keys() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi

    INTEGRATED_GMP_KEYS_FILE="$PROJECT_ROOT/testing-infra/ci-e2e/.integrated-gmp-keys.env"

    if [ -f "$INTEGRATED_GMP_KEYS_FILE" ]; then
        source "$INTEGRATED_GMP_KEYS_FILE"
        export E2E_INTEGRATED_GMP_PRIVATE_KEY
        export E2E_INTEGRATED_GMP_PUBLIC_KEY
        export E2E_INTEGRATED_GMP_MOVE_ADDRESS
    else
        log_and_echo "❌ ERROR: Integrated-GMP keys file not found at $INTEGRATED_GMP_KEYS_FILE"
        log_and_echo "   Run generate_integrated_gmp_keys first."
        exit 1
    fi
}

# Start integrated-gmp service
# Usage: start_integrated_gmp [log_file] [rust_log_level]
# Starts integrated-gmp in background and waits for it to be ready
# Sets INTEGRATED_GMP_PID and INTEGRATED_GMP_LOG global variables
# Exits with error if integrated-gmp fails to start
start_integrated_gmp() {
    if [ -z "$PROJECT_ROOT" ]; then
        setup_project_root
    fi

    if [ -z "$INTEGRATED_GMP_CONFIG_PATH" ]; then
        export INTEGRATED_GMP_CONFIG_PATH="$PROJECT_ROOT/integrated-gmp/config/integrated-gmp-e2e-ci-testing.toml"
    fi

    # Load keys
    load_integrated_gmp_keys

    local log_file="${1:-$LOG_DIR/integrated-gmp.log}"
    local rust_log="${2:-info}"

    # Ensure log directory exists
    mkdir -p "$(dirname "$log_file")"

    # Delete existing log file to start fresh for this test run
    if [ -f "$log_file" ]; then
        rm -f "$log_file"
    fi

    # Stop any existing integrated-gmp first
    stop_integrated_gmp

    log "   Starting integrated-gmp service..."
    log "   Using config: $INTEGRATED_GMP_CONFIG_PATH"
    log "   Log file: $log_file"

    # Use pre-built binary (must be built in Step 1)
    local integrated_gmp_binary="$PROJECT_ROOT/integrated-gmp/target/debug/integrated-gmp"
    if [ ! -f "$integrated_gmp_binary" ]; then
        log_and_echo "   ❌ PANIC: integrated-gmp not built. Step 1 (build binaries) failed."
        exit 1
    fi

    log "   Using binary: $integrated_gmp_binary"
    INTEGRATED_GMP_CONFIG_PATH="$INTEGRATED_GMP_CONFIG_PATH" RUST_LOG="$rust_log" "$integrated_gmp_binary" >> "$log_file" 2>&1 &
    INTEGRATED_GMP_PID=$!

    # Export PID so it persists across subshells
    export INTEGRATED_GMP_PID

    # Save PID to file for cross-script persistence
    if [ -n "$LOG_DIR" ]; then
        echo "$INTEGRATED_GMP_PID" > "$LOG_DIR/integrated-gmp.pid"
    fi

    log "   ✅ Integrated-GMP started with PID: $INTEGRATED_GMP_PID"

    # Wait for integrated-gmp to be ready
    log "   - Waiting for integrated-gmp to initialize..."
    INTEGRATED_GMP_LOG="$log_file"
    export INTEGRATED_GMP_PID INTEGRATED_GMP_LOG
    RETRY_COUNT=0
    MAX_RETRIES=30

    while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
        # Check if process is still running
        if ! ps -p "$INTEGRATED_GMP_PID" > /dev/null 2>&1; then
            log_and_echo "   ❌ Integrated-GMP process died"
            log_and_echo "   Integrated-GMP log:"
            log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
            if [ -f "$log_file" ]; then
                log_and_echo "   $(cat "$log_file")"
            else
                log_and_echo "   Log file not found at: $log_file"
            fi
            log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
            exit 1
        fi

        # Check health (process running + init log message present)
        if check_integrated_gmp_health; then
            log "   ✅ Integrated-GMP is ready!"

            # Give integrated-gmp time to start polling and collect initial events
            log "   - Waiting for integrated-gmp to poll and collect events (10 seconds)..."
            sleep 10

            return 0
        fi

        sleep 1
        RETRY_COUNT=$((RETRY_COUNT + 1))
    done

    # If we get here, integrated-gmp didn't become healthy
    log_and_echo "   ❌ Integrated-GMP failed to start after $MAX_RETRIES seconds"
    log_and_echo "   Integrated-GMP log:"
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    if [ -f "$log_file" ]; then
        log_and_echo "   $(cat "$log_file")"
    else
        log_and_echo "   Log file not found at: $log_file"
    fi
    log_and_echo "   + + + + + + + + + + + + + + + + + + + +"
    exit 1
}

# ============================================================================
# COORDINATOR NEGOTIATION ROUTING HELPERS
# ============================================================================

# Get coordinator API base URL (drafts, negotiation - port 3333)
# Usage: get_coordinator_url [port]
# Returns the base URL for coordinator API calls
get_coordinator_url() {
    local port="${1:-3333}"
    echo "http://127.0.0.1:${port}"
}

# Submit draft intent to coordinator
# Usage: submit_draft_intent <requester_addr> <draft_data_json> <expiry_time> [coordinator_port]
# Returns the draft_id on success, exits on error
# draft_data_json should be a JSON object with intent details
# Note: Cannot use log/log_and_echo for success path because this function's output
# is captured via command substitution, and log functions write to stdout.
submit_draft_intent() {
    local requester_addr="$1"
    local draft_data_json="$2"
    local expiry_time="$3"
    local coordinator_port="${4:-3333}"
    
    if [ -z "$requester_addr" ] || [ -z "$draft_data_json" ] || [ -z "$expiry_time" ]; then
        log_and_echo "❌ ERROR: submit_draft_intent() requires requester_addr, draft_data_json, and expiry_time"
        exit 1
    fi
    
    local coordinator_url=$(get_coordinator_url "$coordinator_port")
    
    # Log to stderr so it doesn't contaminate the return value
    echo "   Submitting draft intent to coordinator..." >&2
    echo "     Requester: $requester_addr" >&2
    [ -n "$LOG_FILE" ] && echo "   Submitting draft intent to coordinator..." >> "$LOG_FILE"
    [ -n "$LOG_FILE" ] && echo "     Requester: $requester_addr" >> "$LOG_FILE"
    
    # Build request body using jq to ensure valid JSON
    local request_body
    request_body=$(jq -n \
        --arg ra "$requester_addr" \
        --argjson dd "$draft_data_json" \
        --argjson et "$expiry_time" \
        '{
            requester_addr: $ra,
            draft_data: $dd,
            expiry_time: $et
        }')
    
    # Log the request for debugging (to stderr)
    echo "     DEBUG: Request body:" >&2
    echo "$request_body" >&2
    [ -n "$LOG_FILE" ] && echo "     DEBUG: Request body:" >> "$LOG_FILE"
    [ -n "$LOG_FILE" ] && echo "$request_body" >> "$LOG_FILE"
    
    local response
    response=$(curl -s -X POST "${coordinator_url}/draftintent" \
        -H "Content-Type: application/json" \
        -d "$request_body" 2>&1)
    
    local curl_exit=$?
    if [ $curl_exit -ne 0 ]; then
        log_and_echo "❌ ERROR: Failed to connect to coordinator at ${coordinator_url}"
        log_and_echo "   curl exit code: $curl_exit"
        exit 1
    fi
    
    # Check for success
    local success=$(echo "$response" | jq -r '.success // false')
    if [ "$success" != "true" ]; then
        local error=$(echo "$response" | jq -r '.error // "Unknown error"')
        log_and_echo "❌ ERROR: Failed to submit draft intent"
        log_and_echo "   Error: $error"
        log_and_echo "   Response: $response"
        exit 1
    fi
    
    local draft_id=$(echo "$response" | jq -r '.data.draft_id')
    if [ -z "$draft_id" ] || [ "$draft_id" = "null" ]; then
        log_and_echo "❌ ERROR: No draft_id in response"
        log_and_echo "   Response: $response"
        exit 1
    fi
    
    # Log to stderr so it doesn't contaminate the return value (stdout is captured by caller)
    echo "     ✅ Draft submitted with ID: $draft_id" >&2
    [ -n "$LOG_FILE" ] && echo "     ✅ Draft submitted with ID: $draft_id" >> "$LOG_FILE"
    echo "$draft_id"
}

# Poll coordinator for pending drafts (solver perspective)
# Usage: poll_pending_drafts [coordinator_port]
# Returns JSON array of pending drafts
# Note: Cannot use log/log_and_echo for success path because this function's output
# is captured via command substitution (e.g., PENDING_DRAFTS=$(poll_pending_drafts)),
# and log functions write to stdout which would contaminate the JSON output.
poll_pending_drafts() {
    local coordinator_port="${1:-3333}"
    local coordinator_url=$(get_coordinator_url "$coordinator_port")
    
    local response
    response=$(curl -s -X GET "${coordinator_url}/draftintents/pending" 2>&1)
    
    local curl_exit=$?
    if [ $curl_exit -ne 0 ]; then
        log_and_echo "❌ ERROR: Failed to connect to coordinator at ${coordinator_url}"
        exit 1
    fi
    
    local success=$(echo "$response" | jq -r '.success // false')
    if [ "$success" != "true" ]; then
        local error=$(echo "$response" | jq -r '.error // "Unknown error"')
        log_and_echo "❌ ERROR: Failed to poll pending drafts"
        log_and_echo "   Error: $error"
        exit 1
    fi
    
    local drafts=$(echo "$response" | jq -r '.data')
    echo "$drafts"
}

# Get draft intent by ID
# Usage: get_draft_intent <draft_id> [coordinator_port]
# Returns the draft data JSON
get_draft_intent() {
    local draft_id="$1"
    local coordinator_port="${2:-3333}"
    
    if [ -z "$draft_id" ]; then
        log_and_echo "❌ ERROR: get_draft_intent() requires draft_id"
        exit 1
    fi
    
    local coordinator_url=$(get_coordinator_url "$coordinator_port")
    
    local response
    response=$(curl -s -X GET "${coordinator_url}/draftintent/${draft_id}" 2>&1)
    
    local curl_exit=$?
    if [ $curl_exit -ne 0 ]; then
        log_and_echo "❌ ERROR: Failed to connect to coordinator at ${coordinator_url}"
        exit 1
    fi
    
    local success=$(echo "$response" | jq -r '.success // false')
    if [ "$success" != "true" ]; then
        local error=$(echo "$response" | jq -r '.error // "Unknown error"')
        log_and_echo "❌ ERROR: Failed to get draft intent"
        log_and_echo "   Error: $error"
        exit 1
    fi
    
    echo "$response" | jq -r '.data'
}

# Submit signature to coordinator (solver submits after signing)
# Usage: submit_signature_to_coordinator <draft_id> <solver_addr> <signature_hex> <public_key_hex> [coordinator_port]
# Returns success/failure, exits on error
submit_signature_to_coordinator() {
    local draft_id="$1"
    local solver_addr="$2"
    local signature_hex="$3"
    local public_key_hex="$4"
    local coordinator_port="${5:-3333}"
    
    if [ -z "$draft_id" ] || [ -z "$solver_addr" ] || [ -z "$signature_hex" ] || [ -z "$public_key_hex" ]; then
        log_and_echo "❌ ERROR: submit_signature_to_coordinator() requires draft_id, solver_addr, signature_hex, public_key_hex"
        exit 1
    fi
    
    # Normalize solver address: ensure 0x prefix (aptos config returns addresses without prefix)
    local normalized_solver_addr
    if [ "${solver_addr#0x}" != "$solver_addr" ]; then
        # Already has 0x prefix
        normalized_solver_addr="$solver_addr"
    else
        # Add 0x prefix
        normalized_solver_addr="0x$solver_addr"
    fi
    
    local coordinator_url=$(get_coordinator_url "$coordinator_port")
    
    log "   Submitting signature to coordinator..."
    log "     Draft ID: $draft_id"
    log "     Solver: $normalized_solver_addr"
    
    local response
    response=$(curl -s -X POST "${coordinator_url}/draftintent/${draft_id}/signature" \
        -H "Content-Type: application/json" \
        -d "{
            \"solver_hub_addr\": \"$normalized_solver_addr\",
            \"signature\": \"$signature_hex\",
            \"public_key\": \"$public_key_hex\"
        }" 2>&1)
    
    local curl_exit=$?
    if [ $curl_exit -ne 0 ]; then
        log_and_echo "❌ ERROR: Failed to connect to coordinator at ${coordinator_url}"
        exit 1
    fi
    
    local success=$(echo "$response" | jq -r '.success // false')
    if [ "$success" != "true" ]; then
        local error=$(echo "$response" | jq -r '.error // "Unknown error"')
        # Check if it's a 409 Conflict (already signed)
        if echo "$error" | grep -qi "already signed\|conflict"; then
            log "     ️  Draft already signed by another solver (FCFS)"
            return 1
        fi
        log_and_echo "❌ ERROR: Failed to submit signature"
        log_and_echo "   Error: $error"
        log_and_echo "   Response: $response"
        exit 1
    fi
    
    log "     ✅ Signature submitted successfully"
    return 0
}

# Poll coordinator for signature (requester polls after submitting draft)
# Usage: poll_for_signature <draft_id> [max_attempts] [sleep_seconds] [coordinator_port]
# Returns signature JSON on success, exits on timeout
poll_for_signature() {
    local draft_id="$1"
    local max_attempts="${2:-60}"
    local sleep_seconds="${3:-2}"
    local coordinator_port="${4:-3333}"
    
    if [ -z "$draft_id" ]; then
        log_and_echo "❌ ERROR: poll_for_signature() requires draft_id"
        exit 1
    fi
    
    local coordinator_url=$(get_coordinator_url "$coordinator_port")
    
    # Use >&2 for all logs to avoid capturing them in command substitution
    echo "   Polling coordinator for signature..." >&2
    echo "     Draft ID: $draft_id" >&2
    echo "     Max attempts: $max_attempts, interval: ${sleep_seconds}s" >&2
    [ -n "$LOG_FILE" ] && echo "   Polling coordinator for signature..." >> "$LOG_FILE"
    [ -n "$LOG_FILE" ] && echo "     Draft ID: $draft_id" >> "$LOG_FILE"
    [ -n "$LOG_FILE" ] && echo "     Max attempts: $max_attempts, interval: ${sleep_seconds}s" >> "$LOG_FILE"
    
    local attempt=0
    while [ $attempt -lt $max_attempts ]; do
        local response
        response=$(curl -s -X GET "${coordinator_url}/draftintent/${draft_id}/signature" 2>/dev/null)
        
        local curl_exit=$?
        if [ $curl_exit -ne 0 ] || [ -z "$response" ]; then
            echo "     Attempt $((attempt+1)): Connection failed, retrying..." >&2
            [ -n "$LOG_FILE" ] && echo "     Attempt $((attempt+1)): Connection failed, retrying..." >> "$LOG_FILE"
            sleep "$sleep_seconds"
            attempt=$((attempt + 1))
            continue
        fi
        
        # Debug: show response
        echo "     Attempt $((attempt+1)): Response: $response" >&2
        [ -n "$LOG_FILE" ] && echo "     Attempt $((attempt+1)): Response: $response" >> "$LOG_FILE"
        
        local success=$(echo "$response" | jq -r '.success // false' 2>/dev/null)
        if [ "$success" = "true" ]; then
            local signature=$(echo "$response" | jq -r '.data.signature // empty' 2>/dev/null)
            local solver=$(echo "$response" | jq -r '.data.solver_hub_addr // empty' 2>/dev/null)
            
            if [ -n "$signature" ] && [ "$signature" != "null" ]; then
                echo "     ✅ Signature received from solver: $solver" >&2
                [ -n "$LOG_FILE" ] && echo "     ✅ Signature received from solver: $solver" >> "$LOG_FILE"
                echo "$response" | jq -r '.data'
                return 0
            fi
        fi
        
        sleep "$sleep_seconds"
        attempt=$((attempt + 1))
    done
    
    # Return empty on timeout instead of exiting (let caller handle)
    echo ""
    return 1
}

# Build draft data JSON for intent
# Usage: build_draft_data <offered_metadata> <offered_amount> <offered_chain_id> <desired_metadata> <desired_amount> <desired_chain_id> <expiry_time> <intent_id> <issuer> [extra_fields_json]
# Returns JSON object suitable for submit_draft_intent
build_draft_data() {
    local offered_metadata="$1"
    local offered_amount="$2"
    local offered_chain_id="$3"
    local desired_metadata="$4"
    local desired_amount="$5"
    local desired_chain_id="$6"
    local expiry_time="$7"
    local intent_id="$8"
    local issuer="$9"
    local extra_fields="${10:-{}}"
    
    # Validate extra_fields is valid JSON, default to {} if not
    local validated_extra
    if ! validated_extra=$(echo "$extra_fields" | jq . 2>/dev/null); then
        # Redirect warning to stderr so it doesn't contaminate JSON output
        echo "   Warning: extra_fields is not valid JSON, using empty object" >&2
        [ -n "$LOG_FILE" ] && echo "   Warning: extra_fields is not valid JSON, using empty object" >> "$LOG_FILE"
        validated_extra="{}"
    fi
    
    # Build the JSON object (redirect any warnings to stderr)
    local json
    json=$(jq -n \
        --arg om "$offered_metadata" \
        --arg oa "$offered_amount" \
        --arg oci "$offered_chain_id" \
        --arg dm "$desired_metadata" \
        --arg da "$desired_amount" \
        --arg dci "$desired_chain_id" \
        --arg et "$expiry_time" \
        --arg ii "$intent_id" \
        --arg is "$issuer" \
        --argjson extra "$validated_extra" \
        '{
            offered_metadata: $om,
            offered_amount: $oa,
            offered_chain_id: $oci,
            desired_metadata: $dm,
            desired_amount: $da,
            desired_chain_id: $dci,
            expiry_time: $et,
            intent_id: $ii,
            issuer: $is
        } + $extra' 2>&1)
    
    local jq_exit=$?
    if [ $jq_exit -ne 0 ]; then
        log "   ERROR: build_draft_data jq failed with exit code $jq_exit"
        log "   jq output: $json"
        log "   Inputs: om=$offered_metadata, oa=$offered_amount, oci=$offered_chain_id"
        log "   Inputs: dm=$desired_metadata, da=$desired_amount, dci=$desired_chain_id"
        log "   Inputs: et=$expiry_time, ii=$intent_id, is=$issuer"
        log "   Inputs: extra=$validated_extra"
        echo "{}"
        return 1
    fi
    
    echo "$json"
}

# Wait for solver to automatically fulfill an intent
# Polls the coordinator's /events endpoint for fulfillment events matching the intent
# Usage: wait_for_solver_fulfillment <intent_id> <flow_type> [timeout_seconds]
#   intent_id: The intent ID to wait for
#   flow_type: "inflow" or "outflow"
#   timeout_seconds: Maximum wait time (default: 60)
# Returns: 0 on success, 1 on timeout
wait_for_solver_fulfillment() {
    local intent_id="$1"
    local flow_type="$2"
    local timeout_seconds="${3:-60}"
    local poll_interval=3
    local elapsed=0
    
    local coordinator_url="${COORDINATOR_URL:-http://127.0.0.1:3333}"
    
    log ""
    log "⏳ Waiting for solver to automatically fulfill $flow_type intent..."
    log "   Intent ID: $intent_id"
    log "   Timeout: ${timeout_seconds}s (polling every ${poll_interval}s)"
    log "   The solver service should detect the escrow/intent and fulfill automatically."
    log ""
    
    # Normalize intent_id for comparison (remove 0x prefix and leading zeros)
    local normalized_intent_id
    normalized_intent_id=$(echo "$intent_id" | tr '[:upper:]' '[:lower:]' | sed 's/^0x//' | sed 's/^0*//')
    
    local solver_log_file="${LOG_DIR:-$PROJECT_ROOT/.tmp/e2e-tests}/solver.log"
    
    while [ $elapsed -lt $timeout_seconds ]; do
        # Check for fulfillment event in coordinator (works for inflow)
        local events_response
        events_response=$(curl -s "${coordinator_url}/events" 2>/dev/null)
        
        if [ $? -eq 0 ]; then
            # Check if fulfillment event exists for this intent
            local fulfillment_found
            fulfillment_found=$(echo "$events_response" | jq -r --arg nid "$normalized_intent_id" \
                '.data.fulfillment_events[]? | select(.intent_id | ascii_downcase | gsub("^0x"; "") | gsub("^0+"; "") == $nid) | .intent_id' \
                2>/dev/null | head -1)
            
            if [ -n "$fulfillment_found" ]; then
                log "   ✅ Solver fulfilled the intent! (detected via coordinator after ${elapsed}s)"
                log "   Fulfillment event found for intent: $fulfillment_found"
                return 0
            fi
        fi
        
        # Also check solver logs for successful fulfillment (works for outflow)
        # Note: fa_intent_with_oracle doesn't emit LimitOrderFulfillmentEvent, so we check solver logs
        # Use normalized_intent_id (without leading zeros) since solver logs may strip them
        if [ -f "$solver_log_file" ]; then
            # Search for intent ID with or without leading zeros (0xd86... or 0x000d86...)
            local intent_id_no_zeros="0x${normalized_intent_id}"
            if grep -qi "Successfully fulfilled.*${intent_id_no_zeros}" "$solver_log_file" 2>/dev/null; then
                log "   ✅ Solver fulfilled the intent! (detected via solver logs after ${elapsed}s)"
                return 0
            fi
        fi
        
        printf "   Waiting for solver... (%ds/%ds)\r" $elapsed $timeout_seconds
        sleep $poll_interval
        elapsed=$((elapsed + poll_interval))
    done
    
    # Timeout - callers handle detailed log display via display_service_logs
    log ""
    log_and_echo "⏰ Timeout waiting for solver fulfillment after ${timeout_seconds}s"

    return 1
}

# Assert that the solver rejects a draft intent due to insufficient liquidity.
#
# After a successful intent, the solver's balance is depleted. Submitting a second
# draft with the same amount should be rejected by the liquidity monitor.
#
# This function:
#   1. Submits a new draft to the coordinator
#   2. Polls for a signature (expecting none)
#   3. Verifies the solver log contains an "insufficient budget" rejection
#
# Usage: assert_solver_rejects_draft <requester_addr> <draft_data_json> <expiry_time>
# Returns: 0 on expected rejection, exits 1 on unexpected acceptance
assert_solver_rejects_draft() {
    local requester_addr="$1"
    local draft_data_json="$2"
    local expiry_time="$3"

    if [ -z "$requester_addr" ] || [ -z "$draft_data_json" ] || [ -z "$expiry_time" ]; then
        log_and_echo "❌ ERROR: assert_solver_rejects_draft() requires requester_addr, draft_data_json, and expiry_time"
        exit 1
    fi

    local solver_log_file="${LOG_DIR:-$PROJECT_ROOT/.tmp/e2e-tests}/solver.log"

    # Record current solver log line count so we only check NEW lines
    local log_lines_before=0
    if [ -f "$solver_log_file" ]; then
        log_lines_before=$(wc -l < "$solver_log_file")
    fi

    log ""
    log "   Submitting second draft intent (expecting rejection)..."

    # Submit the draft to the coordinator
    local draft_id
    draft_id=$(submit_draft_intent "$requester_addr" "$draft_data_json" "$expiry_time")
    if [ -z "$draft_id" ] || [ "$draft_id" = "null" ]; then
        log_and_echo "❌ ERROR: Failed to submit draft intent for liquidity rejection test"
        exit 1
    fi
    log "   Draft ID: $draft_id"

    # Poll for signature with short timeout — we expect NO signature
    log "   Polling for signature (expecting timeout = no signature)..."
    local signature_data
    signature_data=$(poll_for_signature "$draft_id" 5 2) || true

    local signature=""
    if [ -n "$signature_data" ]; then
        signature=$(echo "$signature_data" | jq -r '.signature // empty' 2>/dev/null)
    fi

    if [ -n "$signature" ] && [ "$signature" != "null" ]; then
        log_and_echo "❌ ERROR: Solver unexpectedly SIGNED the draft!"
        log_and_echo "   The solver should have rejected this draft due to insufficient liquidity."
        log_and_echo "   Signature: $signature"
        display_service_logs "Unexpected signature on depleted balance"
        exit 1
    fi

    log "   ✅ No signature received (as expected)"

    # Verify that the solver logged the rejection for the right reason
    if [ -f "$solver_log_file" ]; then
        local new_lines
        new_lines=$(tail -n +"$((log_lines_before + 1))" "$solver_log_file")

        if echo "$new_lines" | grep -q "rejected: insufficient budget"; then
            log "   ✅ Solver log confirms: draft rejected due to insufficient budget"
        elif echo "$new_lines" | grep -q "rejected:"; then
            # Rejected for another reason (e.g., gas token below threshold) — still a valid liquidity guard
            local rejection_reason
            rejection_reason=$(echo "$new_lines" | grep "rejected:" | head -1)
            log "   ✅ Solver log confirms rejection: $rejection_reason"
        else
            log_and_echo "❌ ERROR: No rejection message found in solver logs"
            log_and_echo "   Expected solver to log 'rejected: insufficient budget' or similar"
            log_and_echo "   New solver log lines since draft submission:"
            echo "$new_lines" | tail -30 | while IFS= read -r line; do log_and_echo "   $line"; done
            exit 1
        fi
    else
        log_and_echo "❌ ERROR: Solver log file not found at $solver_log_file"
        exit 1
    fi

    return 0
}

