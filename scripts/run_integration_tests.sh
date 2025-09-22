#!/bin/bash
# WFL Integration Test Runner
# Ensures release binary is built before running integration tests

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if binary exists
check_binary() {
    local binary_path="$1"
    if [ -f "$binary_path" ]; then
        print_success "Binary found: $binary_path"
        return 0
    else
        print_error "Binary not found: $binary_path"
        return 1
    fi
}

# Function to build release binary
build_release() {
    print_status "Building release binary..."
    if cargo build --release --verbose; then
        print_success "Release build completed"
        return 0
    else
        print_error "Release build failed"
        return 1
    fi
}

# Function to run integration tests
run_integration_tests() {
    print_status "Running integration tests..."
    
    # Run split functionality tests specifically
    print_status "Running split functionality tests..."
    if cargo test --test split_functionality --verbose; then
        print_success "Split functionality tests passed"
    else
        print_error "Split functionality tests failed"
        return 1
    fi
    
    # Run all integration tests
    print_status "Running all integration tests..."
    if cargo test --test '*' --verbose; then
        print_success "All integration tests passed"
    else
        print_error "Some integration tests failed"
        return 1
    fi
    
    return 0
}

# Function to run TestPrograms
run_test_programs() {
    print_status "Running WFL test programs..."
    
    # Determine binary path based on OS
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        WFL_BINARY="target/release/wfl.exe"
    else
        WFL_BINARY="target/release/wfl"
    fi
    
    # Check if TestPrograms directory exists
    if [ ! -d "TestPrograms" ]; then
        print_warning "TestPrograms directory not found, skipping WFL program tests"
        return 0
    fi
    
    # Count WFL files
    wfl_files=$(find TestPrograms -name "*.wfl" 2>/dev/null | wc -l)
    if [ "$wfl_files" -eq 0 ]; then
        print_warning "No WFL test programs found in TestPrograms/"
        return 0
    fi
    
    print_status "Found $wfl_files WFL test programs"
    
    # Run each WFL program
    failed_programs=0
    for wfl_file in TestPrograms/*.wfl; do
        if [ -f "$wfl_file" ]; then
            print_status "Testing: $wfl_file"
            if "./$WFL_BINARY" "$wfl_file" > /dev/null 2>&1; then
                print_success "✓ $wfl_file"
            else
                print_error "✗ $wfl_file"
                ((failed_programs++))
            fi
        fi
    done
    
    if [ "$failed_programs" -eq 0 ]; then
        print_success "All WFL test programs passed"
        return 0
    else
        print_error "$failed_programs WFL test programs failed"
        return 1
    fi
}

# Main execution
main() {
    print_status "WFL Integration Test Runner"
    print_status "=========================="
    
    # Check if we're in the right directory
    if [ ! -f "Cargo.toml" ]; then
        print_error "Cargo.toml not found. Please run this script from the WFL project root."
        exit 1
    fi
    
    # Determine binary path based on OS
    if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
        BINARY_PATH="target/release/wfl.exe"
    else
        BINARY_PATH="target/release/wfl"
    fi
    
    # Check if release binary exists, build if not
    if ! check_binary "$BINARY_PATH"; then
        print_status "Release binary not found, building..."
        if ! build_release; then
            print_error "Failed to build release binary"
            exit 1
        fi
        
        # Verify binary was created
        if ! check_binary "$BINARY_PATH"; then
            print_error "Release binary still not found after build"
            exit 1
        fi
    fi
    
    # Run integration tests
    if ! run_integration_tests; then
        print_error "Integration tests failed"
        exit 1
    fi
    
    # Run WFL test programs
    if ! run_test_programs; then
        print_error "WFL test programs failed"
        exit 1
    fi
    
    print_success "All tests completed successfully!"
    print_status "Integration test runner finished"
}

# Parse command line arguments
case "${1:-}" in
    --help|-h)
        echo "WFL Integration Test Runner"
        echo ""
        echo "Usage: $0 [options]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --build-only   Only build release binary, don't run tests"
        echo "  --test-only    Only run tests, assume binary exists"
        echo ""
        echo "This script ensures the WFL release binary is built before running"
        echo "integration tests that depend on it."
        exit 0
        ;;
    --build-only)
        print_status "Build-only mode"
        if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
            BINARY_PATH="target/release/wfl.exe"
        else
            BINARY_PATH="target/release/wfl"
        fi
        
        if ! build_release; then
            exit 1
        fi
        
        check_binary "$BINARY_PATH"
        exit $?
        ;;
    --test-only)
        print_status "Test-only mode"
        if [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
            BINARY_PATH="target/release/wfl.exe"
        else
            BINARY_PATH="target/release/wfl"
        fi
        
        if ! check_binary "$BINARY_PATH"; then
            print_error "Release binary not found. Run without --test-only to build it first."
            exit 1
        fi
        
        if ! run_integration_tests; then
            exit 1
        fi
        
        if ! run_test_programs; then
            exit 1
        fi
        
        print_success "All tests completed successfully!"
        exit 0
        ;;
    "")
        # No arguments, run main function
        main
        ;;
    *)
        print_error "Unknown option: $1"
        print_status "Use --help for usage information"
        exit 1
        ;;
esac
