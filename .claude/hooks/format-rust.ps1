# Auto-format Rust files after Edit/Write operations
# PowerShell hook for Claude Code (Windows/PowerShell Core)

# Read JSON from stdin
$json = [Console]::In.ReadToEnd() | ConvertFrom-Json

# Extract file path
$path = $json.tool_input.file_path

# Check if it's a Rust file
if ($path -like '*.rs') {
    cargo fmt --all
}
