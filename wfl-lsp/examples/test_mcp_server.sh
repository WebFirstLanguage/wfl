#!/bin/bash
# Example script to test WFL MCP server

echo "========================================="
echo "WFL MCP Server Test Script"
echo "========================================="
echo ""

# Color codes for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to send JSON-RPC request and pretty-print response
send_request() {
    local name=$1
    local request=$2

    echo -e "${BLUE}[Test] $name${NC}"
    echo "$request" | wfl-lsp --mcp 2>/dev/null
    echo ""
}

# Test 1: Initialize
echo -e "${GREEN}1. Initialize Server${NC}"
send_request "Initialize" \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'

# Test 2: List Tools
echo -e "${GREEN}2. List Available Tools${NC}"
send_request "List Tools" \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'

# Test 3: Parse WFL Code
echo -e "${GREEN}3. Parse WFL Code${NC}"
send_request "Parse Valid Code" \
    '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"parse_wfl","arguments":{"source":"store x as 5\ndisplay x","include_positions":false}}}'

# Test 4: Analyze WFL Code
echo -e "${GREEN}4. Analyze WFL Code${NC}"
send_request "Analyze Code" \
    '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"analyze_wfl","arguments":{"source":"store x as 5\ndisplay x"}}}'

# Test 5: Type Check
echo -e "${GREEN}5. Type Check WFL Code${NC}"
send_request "Type Check" \
    '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"typecheck_wfl","arguments":{"source":"store x as 5"}}}'

# Test 6: Get Completions
echo -e "${GREEN}6. Get Code Completions${NC}"
send_request "Completions" \
    '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"get_completions","arguments":{"source":"store ","line":0,"column":6}}}'

# Test 7: List Resources
echo -e "${GREEN}7. List Available Resources${NC}"
send_request "List Resources" \
    '{"jsonrpc":"2.0","id":7,"method":"resources/list"}'

# Test 8: Read Workspace Files
echo -e "${GREEN}8. Read Workspace Files${NC}"
send_request "Workspace Files" \
    '{"jsonrpc":"2.0","id":8,"method":"resources/read","params":{"uri":"workspace://files"}}'

# Test 9: Parse Error Example
echo -e "${GREEN}9. Parse Invalid Code (Error Handling)${NC}"
send_request "Parse Error" \
    '{"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"parse_wfl","arguments":{"source":"store x as"}}}'

echo "========================================="
echo -e "${GREEN}All tests completed!${NC}"
echo "========================================="
