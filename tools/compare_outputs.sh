#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PYTHON_COMBINER="$REPO_ROOT/Tools/wfl_md_combiner.py"
WFL_COMBINER="$REPO_ROOT/Tools/combiner.wfl"
WFL_BINARY="$REPO_ROOT/target/debug/wfl"
FIXTURES_DIR="$REPO_ROOT/tests/fixtures/combiner"
TEMP_DIR="/tmp/wfl_combiner_test_$$"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT

mkdir -p "$TEMP_DIR"

echo "WFL Combiner Output Comparison Test"
echo "==================================="
echo "Python combiner: $PYTHON_COMBINER"
echo "WFL combiner: $WFL_COMBINER"
echo "Test fixtures: $FIXTURES_DIR"
echo "Temp directory: $TEMP_DIR"
echo ""

if [[ ! -f "$PYTHON_COMBINER" ]]; then
    echo -e "${RED}ERROR: Python combiner not found at $PYTHON_COMBINER${NC}"
    exit 1
fi

if [[ ! -f "$WFL_COMBINER" ]]; then
    echo -e "${RED}ERROR: WFL combiner not found at $WFL_COMBINER${NC}"
    exit 1
fi

if [[ ! -x "$WFL_BINARY" ]]; then
    echo -e "${RED}ERROR: WFL binary not found or not executable at $WFL_BINARY${NC}"
    echo "Run 'cargo build' to build the WFL interpreter"
    exit 1
fi

if [[ ! -d "$FIXTURES_DIR" ]]; then
    echo -e "${RED}ERROR: Test fixtures directory not found at $FIXTURES_DIR${NC}"
    exit 1
fi

normalize_newlines() {
    local file="$1"
    if [[ -f "$file" ]]; then
        sed -i 's/\r$//' "$file"
        sed -i -e '$a\' "$file"
        sed -i -e :a -e '/^\s*$/{$d;N;ba' -e '}' "$file"
    fi
}

run_python_combiner() {
    local args="$1"
    local output_prefix="$2"
    
    echo "Running Python combiner with args: $args"
    cd "$REPO_ROOT"
    python3 "$PYTHON_COMBINER" $args --output "$TEMP_DIR/${output_prefix}_python.md"
    
    if [[ -f "$TEMP_DIR/${output_prefix}_python.md" ]]; then
        normalize_newlines "$TEMP_DIR/${output_prefix}_python.md"
    fi
    if [[ -f "$TEMP_DIR/${output_prefix}_python.txt" ]]; then
        normalize_newlines "$TEMP_DIR/${output_prefix}_python.txt"
    fi
}

run_wfl_combiner() {
    local args="$1"
    local output_prefix="$2"
    
    echo "Running WFL combiner (proof-of-concept)"
    cd "$REPO_ROOT"
    
    "$WFL_BINARY" "$WFL_COMBINER" > "$TEMP_DIR/${output_prefix}_wfl_output.txt" 2>&1
    
    echo "# WFL Combiner Output (Proof of Concept)" > "$TEMP_DIR/${output_prefix}_wfl.md"
    echo "This is a placeholder output from the WFL combiner proof-of-concept." >> "$TEMP_DIR/${output_prefix}_wfl.md"
    echo "Full implementation blocked by WFL parser limitations." >> "$TEMP_DIR/${output_prefix}_wfl.md"
    echo "" >> "$TEMP_DIR/${output_prefix}_wfl.md"
    
    echo "WFL Combiner Output (Proof of Concept)" > "$TEMP_DIR/${output_prefix}_wfl.txt"
    echo "This is a placeholder output from the WFL combiner proof-of-concept." >> "$TEMP_DIR/${output_prefix}_wfl.txt"
    echo "Full implementation blocked by WFL parser limitations." >> "$TEMP_DIR/${output_prefix}_wfl.txt"
    echo "" >> "$TEMP_DIR/${output_prefix}_wfl.txt"
    
    normalize_newlines "$TEMP_DIR/${output_prefix}_wfl.md"
    normalize_newlines "$TEMP_DIR/${output_prefix}_wfl.txt"
}

compare_files() {
    local python_file="$1"
    local wfl_file="$2"
    local test_name="$3"
    
    if [[ ! -f "$python_file" ]]; then
        echo -e "${RED}FAIL: $test_name - Python output file missing: $python_file${NC}"
        return 1
    fi
    
    if [[ ! -f "$wfl_file" ]]; then
        echo -e "${RED}FAIL: $test_name - WFL output file missing: $wfl_file${NC}"
        return 1
    fi
    
    if diff -q "$python_file" "$wfl_file" > /dev/null; then
        echo -e "${GREEN}PASS: $test_name - Files are identical${NC}"
        return 0
    else
        echo -e "${RED}FAIL: $test_name - Files differ${NC}"
        echo "Differences:"
        diff -u "$python_file" "$wfl_file" | head -20
        echo ""
        return 1
    fi
}

TESTS_PASSED=0
TESTS_FAILED=0

echo "Test 1: Basic fixture directory processing"
echo "----------------------------------------"
if run_python_combiner "--input $FIXTURES_DIR --type docs --all-files" "test1"; then
    if run_wfl_combiner "--input $FIXTURES_DIR --type docs" "test1"; then
        if compare_files "$TEMP_DIR/test1_python.md" "$TEMP_DIR/test1_wfl.md" "Test1 Markdown"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
        
        if compare_files "$TEMP_DIR/test1_python.txt" "$TEMP_DIR/test1_wfl.txt" "Test1 Text"; then
            ((TESTS_PASSED++))
        else
            ((TESTS_FAILED++))
        fi
    else
        echo -e "${RED}FAIL: WFL combiner execution failed${NC}"
        ((TESTS_FAILED+=2))
    fi
else
    echo -e "${RED}FAIL: Python combiner execution failed${NC}"
    ((TESTS_FAILED+=2))
fi

echo ""
echo "Test 2: Help text comparison"
echo "---------------------------"
python3 "$PYTHON_COMBINER" --help > "$TEMP_DIR/python_help.txt" 2>&1 || true
echo "WFL Combiner Help (Proof of Concept)" > "$TEMP_DIR/wfl_help.txt"
echo "Full help text implementation blocked by WFL parser limitations." >> "$TEMP_DIR/wfl_help.txt"

normalize_newlines "$TEMP_DIR/python_help.txt"
normalize_newlines "$TEMP_DIR/wfl_help.txt"

if compare_files "$TEMP_DIR/python_help.txt" "$TEMP_DIR/wfl_help.txt" "Help Text"; then
    ((TESTS_PASSED++))
else
    ((TESTS_FAILED++))
fi

echo ""
echo "Summary"
echo "======="
echo -e "Tests passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests failed: ${RED}$TESTS_FAILED${NC}"

if [[ $TESTS_FAILED -eq 0 ]]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed. WFL combiner implementation needs completion.${NC}"
    echo ""
    echo "Current Status:"
    echo "- Python combiner: Fully functional"
    echo "- WFL combiner: Proof-of-concept only (blocked by parser limitations)"
    echo "- Stdlib I/O functions: Implemented and tested"
    echo "- Comparison framework: Established"
    echo ""
    echo "Next Steps:"
    echo "1. Enhance WFL parser to support complex syntax"
    echo "2. Implement full WFL combiner using stdlib I/O functions"
    echo "3. Achieve byte-for-byte output parity"
    exit 1
fi
