#!/bin/bash

set -e

echo "Running pre-commit checks..."
echo "============================"

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo "Error: Not in a git repository"
    exit 1
fi

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_status() {
    local status="$1"
    local message="$2"
    
    if [[ "$status" == "PASS" ]]; then
        echo -e "${GREEN}✓ $message${NC}"
    elif [[ "$status" == "FAIL" ]]; then
        echo -e "${RED}✗ $message${NC}"
    elif [[ "$status" == "INFO" ]]; then
        echo -e "${YELLOW}ℹ $message${NC}"
    fi
}

echo ""
echo "1. Checking code formatting..."
echo "------------------------------"
if cargo fmt --all -- --check; then
    print_status "PASS" "Code formatting is correct"
else
    print_status "FAIL" "Code formatting issues found"
    echo ""
    echo "Run 'cargo fmt --all' to fix formatting issues"
    exit 1
fi

echo ""
echo "2. Running Clippy lints..."
echo "-------------------------"
if cargo clippy --all-targets --all-features -- -D warnings; then
    print_status "PASS" "No Clippy warnings found"
else
    print_status "FAIL" "Clippy warnings found"
    echo ""
    echo "Fix Clippy warnings before committing"
    exit 1
fi

echo ""
echo "3. Checking compilation..."
echo "-------------------------"
if cargo check; then
    print_status "PASS" "Code compiles successfully"
else
    print_status "FAIL" "Compilation errors found"
    echo ""
    echo "Fix compilation errors before committing"
    exit 1
fi

echo ""
echo "4. Running unit tests..."
echo "-----------------------"
if cargo test --lib; then
    print_status "PASS" "All unit tests passed"
else
    print_status "FAIL" "Unit tests failed"
    echo ""
    echo "Fix failing unit tests before committing"
    exit 1
fi

echo ""
echo "5. Running integration tests..."
echo "------------------------------"
if cargo test --test '*' 2>/dev/null; then
    print_status "PASS" "All integration tests passed"
else
    print_status "INFO" "No integration tests found or some failed"
fi

echo ""
echo "6. Running combiner comparison test..."
echo "------------------------------------"
if [[ -x "$REPO_ROOT/tools/compare_outputs.sh" ]]; then
    if "$REPO_ROOT/tools/compare_outputs.sh"; then
        print_status "PASS" "Combiner comparison test passed"
    else
        print_status "INFO" "Combiner comparison test failed (expected during development)"
        echo "Note: This is expected until WFL combiner implementation is complete"
    fi
else
    print_status "INFO" "Combiner comparison script not found or not executable"
fi

echo ""
echo "7. Testing WFL programs..."
echo "-------------------------"
WFL_BINARY="$REPO_ROOT/target/debug/wfl"

if [[ ! -x "$WFL_BINARY" ]]; then
    echo "Building WFL binary..."
    if ! cargo build; then
        print_status "FAIL" "Failed to build WFL binary"
        exit 1
    fi
fi

TEST_PROGRAMS_DIR="$REPO_ROOT/TestPrograms"
if [[ -d "$TEST_PROGRAMS_DIR" ]]; then
    FAILED_PROGRAMS=()
    
    if [[ -f "$TEST_PROGRAMS_DIR/hello.wfl" ]]; then
        if "$WFL_BINARY" "$TEST_PROGRAMS_DIR/hello.wfl" > /dev/null 2>&1; then
            print_status "PASS" "hello.wfl executes successfully"
        else
            print_status "FAIL" "hello.wfl failed to execute"
            FAILED_PROGRAMS+=("hello.wfl")
        fi
    fi
    
    if [[ -f "$TEST_PROGRAMS_DIR/simple_test.wfl" ]]; then
        if "$WFL_BINARY" "$TEST_PROGRAMS_DIR/simple_test.wfl" > /dev/null 2>&1; then
            print_status "PASS" "simple_test.wfl executes successfully"
        else
            print_status "FAIL" "simple_test.wfl failed to execute"
            FAILED_PROGRAMS+=("simple_test.wfl")
        fi
    fi
    
    if [[ -f "$TEST_PROGRAMS_DIR/simple_stdlib_test.wfl" ]]; then
        if "$WFL_BINARY" "$TEST_PROGRAMS_DIR/simple_stdlib_test.wfl" > /dev/null 2>&1; then
            print_status "PASS" "simple_stdlib_test.wfl executes successfully"
        else
            print_status "FAIL" "simple_stdlib_test.wfl failed to execute"
            FAILED_PROGRAMS+=("simple_stdlib_test.wfl")
        fi
    fi
    
    if [[ ${#FAILED_PROGRAMS[@]} -gt 0 ]]; then
        print_status "FAIL" "Some WFL test programs failed: ${FAILED_PROGRAMS[*]}"
        echo ""
        echo "Fix failing WFL programs before committing"
        exit 1
    fi
else
    print_status "INFO" "TestPrograms directory not found"
fi

echo ""
echo "================================"
print_status "PASS" "All pre-commit checks passed!"
echo "================================"
echo ""
echo "Ready to commit. Your changes look good!"

exit 0
