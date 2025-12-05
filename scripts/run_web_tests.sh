#!/bin/bash
# WFL Web Server Integration Test Runner
# Tests WFL web server functionality by starting servers and sending HTTP requests

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
GRAY='\033[0;90m'
NC='\033[0m' # No Color

# Default timeout
TIMEOUT=10

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help|-h)
            echo "WFL Web Server Test Runner"
            echo ""
            echo "Usage: $0 [options]"
            echo ""
            echo "Options:"
            echo "  --help, -h       Show this help message"
            echo "  --timeout <sec>  Timeout for each test (default: 10)"
            echo ""
            echo "This script tests WFL web server functionality by:"
            echo "  1. Starting the WFL web server in background"
            echo "  2. Sending HTTP requests to verify it works"
            echo "  3. Checking responses and cleaning up"
            exit 0
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}[ERROR]${NC} Unknown option: $1"
            exit 1
            ;;
    esac
done

echo -e "${BLUE}[INFO]${NC} WFL Web Server Test Runner"
echo -e "${BLUE}[INFO]${NC} ============================"

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}[ERROR]${NC} Cargo.toml not found. Please run this script from the WFL project root."
    exit 1
fi

# Determine binary path
if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    BINARY_PATH="target/release/wfl.exe"
else
    BINARY_PATH="target/release/wfl"
fi

# Check if release binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}[ERROR]${NC} Release binary not found. Run 'cargo build --release' first."
    exit 1
fi
echo -e "${GREEN}[SUCCESS]${NC} Binary found: $BINARY_PATH"

# Function to test a web server
test_wfl_webserver() {
    local test_file="$1"
    local port="$2"
    local expected_response="$3"
    local timeout_seconds="$4"

    local test_name=$(basename "$test_file")
    echo ""
    echo -e "${BLUE}[INFO]${NC} Testing: $test_name on port $port"

    # Start the WFL server in background
    "./$BINARY_PATH" "$test_file" > /dev/null 2>&1 &
    local server_pid=$!

    # Cleanup function
    cleanup() {
        if kill -0 $server_pid 2>/dev/null; then
            kill $server_pid 2>/dev/null || true
            echo -e "${GRAY}[INFO] Server process terminated${NC}"
        fi
    }
    trap cleanup EXIT

    # Wait for server to start (with retries)
    local server_ready=false
    local retries=0
    local max_retries=$((timeout_seconds * 2))  # Check every 500ms

    while [ "$server_ready" = false ] && [ $retries -lt $max_retries ]; do
        sleep 0.5
        ((retries++))

        # Try to connect using curl
        if response=$(curl -s --max-time 2 "http://localhost:$port/" 2>/dev/null); then
            server_ready=true
        fi
    done

    if [ "$server_ready" = false ]; then
        echo -e "${RED}[ERROR]${NC} TIMEOUT: Server did not start within ${timeout_seconds}s"
        cleanup
        return 1
    fi

    # Server is ready, check response
    if [[ "$response" == *"$expected_response"* ]]; then
        echo -e "${GREEN}[SUCCESS]${NC} PASS: Got expected response"
        cleanup
        return 0
    else
        echo -e "${RED}[ERROR]${NC} FAIL: Unexpected response"
        echo -e "${GRAY}  Expected: $expected_response${NC}"
        echo -e "${GRAY}  Got: $response${NC}"
        cleanup
        return 1
    fi
}

# Run web server tests
total_tests=0
passed_tests=0

# Test 1: simple_web_test.wfl
if [ -f "TestPrograms/simple_web_test.wfl" ]; then
    ((total_tests++))
    if test_wfl_webserver "TestPrograms/simple_web_test.wfl" 8095 "Hello from WFL" "$TIMEOUT"; then
        ((passed_tests++))
    fi
fi

# Test 2: web_server_test.wfl (if exists)
if [ -f "TestPrograms/web_server_test.wfl" ]; then
    ((total_tests++))
    # Read the file to find the port
    port=$(grep -oP 'port\s+\K\d+' "TestPrograms/web_server_test.wfl" 2>/dev/null || echo "")
    if [ -n "$port" ]; then
        if test_wfl_webserver "TestPrograms/web_server_test.wfl" "$port" "" "$TIMEOUT"; then
            ((passed_tests++))
        fi
    else
        echo -e "${YELLOW}[SKIP]${NC} web_server_test.wfl - could not determine port"
        ((total_tests--))
    fi
fi

# Summary
echo ""
echo -e "${BLUE}[INFO]${NC} ============================"
echo -e "${BLUE}[INFO]${NC} Results: $passed_tests/$total_tests tests passed"

if [ "$passed_tests" -eq "$total_tests" ]; then
    echo -e "${GREEN}[SUCCESS]${NC} All web server tests passed!"
    exit 0
else
    echo -e "${RED}[ERROR]${NC} Some web server tests failed"
    exit 1
fi
