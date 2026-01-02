# Example PowerShell script to test WFL MCP server

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "WFL MCP Server Test Script" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

function Send-McpRequest {
    param(
        [string]$Name,
        [string]$Request
    )

    Write-Host "[Test] $Name" -ForegroundColor Blue
    $Request | wfl-lsp --mcp 2>$null
    Write-Host ""
}

# Test 1: Initialize
Write-Host "1. Initialize Server" -ForegroundColor Green
Send-McpRequest "Initialize" '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}'

# Test 2: List Tools
Write-Host "2. List Available Tools" -ForegroundColor Green
Send-McpRequest "List Tools" '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'

# Test 3: Parse WFL Code
Write-Host "3. Parse WFL Code" -ForegroundColor Green
Send-McpRequest "Parse Valid Code" '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"parse_wfl","arguments":{"source":"store x as 5\ndisplay x","include_positions":false}}}'

# Test 4: Analyze WFL Code
Write-Host "4. Analyze WFL Code" -ForegroundColor Green
Send-McpRequest "Analyze Code" '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"analyze_wfl","arguments":{"source":"store x as 5\ndisplay x"}}}'

# Test 5: Type Check
Write-Host "5. Type Check WFL Code" -ForegroundColor Green
Send-McpRequest "Type Check" '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"typecheck_wfl","arguments":{"source":"store x as 5"}}}'

# Test 6: Get Completions
Write-Host "6. Get Code Completions" -ForegroundColor Green
Send-McpRequest "Completions" '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"get_completions","arguments":{"source":"store ","line":0,"column":6}}}'

# Test 7: List Resources
Write-Host "7. List Available Resources" -ForegroundColor Green
Send-McpRequest "List Resources" '{"jsonrpc":"2.0","id":7,"method":"resources/list"}'

# Test 8: Read Workspace Files
Write-Host "8. Read Workspace Files" -ForegroundColor Green
Send-McpRequest "Workspace Files" '{"jsonrpc":"2.0","id":8,"method":"resources/read","params":{"uri":"workspace://files"}}'

# Test 9: Read Workspace Diagnostics
Write-Host "9. Read Workspace Diagnostics" -ForegroundColor Green
Send-McpRequest "Workspace Diagnostics" '{"jsonrpc":"2.0","id":9,"method":"resources/read","params":{"uri":"workspace://diagnostics"}}'

# Test 10: Parse Error Example
Write-Host "10. Parse Invalid Code (Error Handling)" -ForegroundColor Green
Send-McpRequest "Parse Error" '{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"parse_wfl","arguments":{"source":"store x as"}}}'

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "All tests completed!" -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Cyan
