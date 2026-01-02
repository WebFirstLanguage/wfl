# WFL MCP Server Examples

This directory contains example scripts and clients for testing and using the WFL MCP server.

## Available Examples

### 1. test_mcp_server.sh (Bash)

Shell script for testing all MCP server features on Linux/macOS.

**Usage:**
```bash
cd wfl-lsp/examples
chmod +x test_mcp_server.sh
./test_mcp_server.sh
```

**What it tests:**
- Server initialization
- Tool listing and execution
- Resource listing and reading
- Error handling

### 2. test_mcp_server.ps1 (PowerShell)

PowerShell script for testing all MCP server features on Windows.

**Usage:**
```powershell
cd wfl-lsp\examples
.\test_mcp_server.ps1
```

**What it tests:**
- All 6 tools (parse, analyze, typecheck, lint, completions, symbol info)
- All 5 resources (files, symbols, diagnostics, config, file:///)
- Error handling scenarios

### 3. simple_mcp_client.rs (Rust)

Example Rust program demonstrating programmatic MCP client implementation.

**Build and run:**
```bash
cd wfl-lsp
cargo run --example simple_mcp_client
```

**Features:**
- Shows how to spawn wfl-lsp in MCP mode
- Demonstrates JSON-RPC request/response handling
- Example tool and resource usage
- Error handling patterns

## Quick Tests

### Test Parse Tool

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"parse_wfl","arguments":{"source":"store x as 5"}}}' | wfl-lsp --mcp 2>/dev/null
```

### Test Analyze Tool

```bash
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"analyze_wfl","arguments":{"source":"store x as 5\ndisplay x"}}}' | wfl-lsp --mcp 2>/dev/null
```

### Test Resources

```bash
echo '{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"workspace://files"}}' | wfl-lsp --mcp 2>/dev/null
```

## Building Custom Clients

### Minimal Example (Python)

```python
import subprocess
import json

# Start MCP server
proc = subprocess.Popen(
    ['wfl-lsp', '--mcp'],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True
)

# Send request
request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
        "name": "parse_wfl",
        "arguments": {
            "source": "store x as 5"
        }
    }
}

proc.stdin.write(json.dumps(request) + '\n')
proc.stdin.flush()

# Read response
response = json.loads(proc.stdout.readline())
print(response)
```

### Minimal Example (Node.js)

```javascript
const { spawn } = require('child_process');

const server = spawn('wfl-lsp', ['--mcp']);

// Send request
const request = {
  jsonrpc: '2.0',
  id: 1,
  method: 'tools/call',
  params: {
    name: 'parse_wfl',
    arguments: {
      source: 'store x as 5'
    }
  }
};

server.stdin.write(JSON.stringify(request) + '\n');

// Read response
server.stdout.on('data', (data) => {
  const response = JSON.parse(data.toString());
  console.log(response);
});
```

## Testing Checklist

When testing the MCP server, verify:

- [ ] Server initializes successfully
- [ ] Tools are listed correctly (6 tools)
- [ ] Resources are listed correctly (4-5 resources)
- [ ] parse_wfl works with valid code
- [ ] parse_wfl reports errors for invalid code
- [ ] analyze_wfl finds semantic errors
- [ ] typecheck_wfl catches type errors
- [ ] lint_wfl suggests improvements
- [ ] get_completions returns keywords
- [ ] get_symbol_info provides information
- [ ] workspace://files lists WFL files
- [ ] workspace://symbols extracts symbols
- [ ] workspace://diagnostics aggregates errors
- [ ] workspace://config reads .wflcfg
- [ ] file:/// resources read file contents
- [ ] Error responses are properly formatted

## Troubleshooting Examples

### Debug Mode

Run with error output to see server logs:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | wfl-lsp --mcp
# Server logs appear on stderr
```

### Validate JSON-RPC

Test with minimal request:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | wfl-lsp --mcp 2>/dev/null
```

Should return valid JSON-RPC response with server info.

### Check Workspace

Verify workspace resources work:

```bash
cd /your/wfl/project
echo '{"jsonrpc":"2.0","id":1,"method":"resources/read","params":{"uri":"workspace://files"}}' | wfl-lsp --mcp 2>/dev/null
```

Should list all .wfl files in current directory.

## See Also

- [WFL MCP User Guide](../../Docs/guides/wfl-mcp-guide.md)
- [WFL MCP API Reference](../../Docs/guides/wfl-mcp-api-reference.md)
- [MCP Specification](https://modelcontextprotocol.io/specification)

---

**Note:** These examples are for testing and learning. For production use, consider error handling, timeouts, and proper resource cleanup.
