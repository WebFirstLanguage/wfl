# Interoperability

WFL is designed for web development and integrates with existing web technologies. This page covers current and planned interoperability features.

## Current Interoperability

### 1. **Subprocess Execution**

Run external programs and tools:

```wfl
// Run Node.js scripts
wait for execute command "node script.js" as node_output

// Run Python scripts
wait for execute command "python analyze.py" as result

// Run system commands
wait for execute command "git status" as git_output
```

**[Learn more → Subprocess Execution](subprocess-execution.md)**

### 2. **File System Integration**

Read and write files in any format:

```wfl
// Read JSON file (as text, parse manually)
open file at "data.json" for reading as json_file
wait for store json_text as read content from json_file
close file json_file

// Write CSV files
open file at "export.csv" for writing as csv_file
wait for write content "Name,Age\n" into csv_file
wait for append content "Alice,28\n" into csv_file
close file csv_file
```

### 3. **HTTP Client**

Make HTTP requests to external APIs:

```wfl
try:
    open url at "https://api.example.com/data" and read content as api_response
    display "API response: " with api_response
catch:
    display "API request failed"
end try
```

#### Methods, headers, and request bodies

Real API integrations need more than GET: the `open url` statement accepts
optional `method`, `headers`, and `body` clauses, joined by `and`/`with` in
any order:

```wfl
create map request_headers:
    Authorization is "Bearer sk_test_123"
    "Content-Type" is "application/x-www-form-urlencoded"
end map

open url at "https://api.stripe.com/v1/checkout/sessions"
    with method "POST"
    and headers request_headers
    and body "mode=payment&line_items[0][price]=price_123"
    and read response as resp
```

- `method` — any HTTP method as text: `"GET"` (default), `"POST"`, `"PUT"`, `"PATCH"`, `"DELETE"`, ...
- `headers` — a map of header names to values (string keys like `"Content-Type"` are allowed in `create map`)
- `body` — the request body as text; build form-encoded or JSON bodies with concatenation or `stringify_json`

#### Reading the full response

`read content as` binds only the response body text. `read response as`
binds a response object with the status and headers too:

```wfl
open url at "https://api.example.com/thing" and read response as resp

display "Status: " with resp.status         // e.g. 200 (a number)
display "OK: " with resp.ok                 // yes for 2xx statuses
display "Body: " with resp.body             // response body text
store content_type as resp.headers["content-type"]
```

Non-2xx statuses are not errors — check `resp.ok` or `resp.status`
yourself. Network failures (DNS, connection refused) still raise errors you
can `try`/`catch`.

Outbound responses are streamed and decoded into a bounded buffer. The
`web_server_max_response_size` setting (64 MiB by default) limits the response
body for `read content` and `read response`, both as received and after text
decoding. The limit includes chunked responses with no declared length. Outside
a `main loop`, the connection and body read share the script's remaining
`timeout_seconds`; inside a lifetime-exempt `main loop`, each request gets a
fresh timeout of that duration. Cooperative cancellation also interrupts a
request that is waiting on the remote peer.

**Note:** inside an `open url` statement the words `method`, `headers`, and
`body` introduce clauses, so use different variable names there (e.g.
`request_headers`, `payload`).

#### Streaming a response incrementally

`read content` / `read response` buffer the whole body before returning. For a
large download, or an upstream that emits output progressively (for example a
model endpoint sending newline-delimited JSON), use `stream response` instead.
It returns as soon as the status and headers arrive — **without buffering the
body** — and binds a streaming handle you pull from piece by piece:

```wfl
open url at "https://api.example.com/events"
    with method "POST"
    and headers request_headers
    and body payload
    and stream response as upstream

display "Status: " with upstream.status        // available immediately
store content_type as upstream.headers["content-type"]

// Pull the body one line at a time. Each read returns the next line, or
// `nothing` once the stream ends cleanly.
store done as no
count from 1 to 1000000:
    wait for next line from upstream as line
    check if line is nothing:
        break
    otherwise:
        display line
    end check
end count

close upstream
```

Two incremental reads are available on a streaming handle:

- `wait for next line from <handle> as <name>` — binds the next
  newline-delimited line (the trailing newline, and a paired carriage return,
  are stripped). A final line with no trailing newline is still delivered.
- `wait for next chunk from <handle> as <name>` — binds the next raw byte chunk
  (`binary`) exactly as it arrives off the network, for non-line-oriented
  payloads.

Both bind `nothing` at a clean end of stream, so `check if line is nothing`
ends the loop. `close <handle>` releases the stream early and cancels the
in-flight upstream request; reading from a closed (or fully-drained) handle
raises a catchable error.

The same limits as buffered requests apply: the running total of body bytes is
held under `web_server_max_response_size`, each read is bounded by the request's
timeout, and cooperative cancellation interrupts a read waiting on the peer. A
mid-stream network error surfaces as a catchable error from the `wait for next
...` statement, and every stream is closed when the handle is dropped on any
exit path.

### 4. **Web Standards**

WFL web servers work with standard HTTP:

- **Standard HTTP methods:** GET, POST, PUT, DELETE
- **Standard status codes:** 200, 404, 500, etc.
- **Standard headers:** Content-Type, User-Agent, etc.
- **Standard MIME types:** text/html, application/json, etc.

Any HTTP client can communicate with WFL servers!

### 5. **Database Integration**

WFL has built-in database support for SQLite, PostgreSQL, and MariaDB/MySQL:

```wfl
// Connect to database
connect to database at "postgres://localhost/mydb" as db

// Query
wait for store users as query db with "SELECT * FROM users"

// Close
close database db
```

See the [Databases guide](databases.md) for the complete reference, including
parameterized queries, type mapping, and per-backend notes.

## Planned Interoperability

### JavaScript Integration (Planned)

**Future syntax (conceptual):**

```text
// Import JavaScript library
use javascript library "moment" as moment

// Call JavaScript functions
store formatted_date as moment.format("YYYY-MM-DD")
```

**Status:** Planned for future versions

### JSON Support (Planned)

**Future syntax (conceptual):**

```text
// Parse JSON
store data as parse json from json_text

// Generate JSON
store json as to json of data_object
```

**Current workaround:** Build JSON strings manually

```wfl
store name as "Alice"
store age as 28

store json as "{
    \"name\": \"" with name with "\",
    \"age\": " with age with "
}"

display json
// Output: {"name": "Alice", "age": 28}
```

### HTML/CSS Integration (Planned)

**Future syntax (conceptual):**

```text
// Generate HTML
create html document as page:
    add heading level 1 with "Welcome"
    add paragraph with "Hello, World!"
end html

respond to req with page
```

**Current approach:** Build HTML strings

```wfl
store html as "<!DOCTYPE html>
<html>
<head><title>Page</title></head>
<body>
    <h1>Welcome</h1>
    <p>Hello, World!</p>
</body>
</html>"

respond to req with html and content_type "text/html"
```

## Current Interoperability Patterns

### Calling External Tools

#### Git Integration

```wfl
define action called git_commit with parameters message:
    store cmd as "git add . && git commit -m \"" with message with "\""
    try:
        wait for execute command cmd
        display "✓ Commit successful"
        return yes
    catch:
        display "✗ Commit failed"
        return no
    end try
end action

call git_commit with "Updated documentation"
```

#### NPM Scripts

```wfl
display "Installing dependencies..."
wait for execute command "npm install"

display "Running build..."
wait for execute command "npm run build"

display "Running tests..."
wait for execute command "npm test"
```

#### Python Data Processing

```wfl
// Call Python script for data analysis
display "Analyzing data..."

try:
    wait for execute command "python analyze.py input.csv" as output
    display "Analysis complete"
    display output
catch:
    display "Python script failed"
end try
```

### Data Exchange Formats

#### CSV Files

```wfl
// Write CSV
create list users:
    add "Alice,28,Developer"
    add "Bob,35,Designer"
end list

open file at "users.csv" for writing as csvfile
wait for write content "Name,Age,Role\n" into csvfile
for each user in users:
    wait for append content user with "\n" into csvfile
end for
close file csvfile

// Python can now read this CSV
wait for execute command "python process_users.py users.csv"
```

#### JSON Files

```wfl
// Build JSON manually
store user_json as "{
    \"users\": [
        {\"name\": \"Alice\", \"age\": 28},
        {\"name\": \"Bob\", \"age\": 35}
    ]
}"

open file at "users.json" for writing as jsonfile
wait for write content user_json into jsonfile
close file jsonfile

// JavaScript can now read this JSON
wait for execute command "node process_users.js users.json"
```

## Calling Other Languages

### From WFL to Python

**WFL script:**
```wfl
// data_processor.wfl
display "Preparing data for Python..."

open file at "input.txt" for writing as input_file
wait for write content "data to process" into input_file
close file input_file

display "Calling Python script..."
wait for execute command "python process.py input.txt output.txt" as result

open file at "output.txt" for reading as output_file
wait for store processed as read content from output_file
close file output_file

display "Result: " with processed
```

**Python script (process.py):**
```python
import sys
with open(sys.argv[1]) as f:
    data = f.read()
processed = data.upper()
with open(sys.argv[2], 'w') as f:
    f.write(processed)
```

### From WFL to Node.js

**WFL script:**
```wfl
// Call Node.js for async operations
wait for execute command "node fetch_api.js"

open file at "api_result.json" for reading as result_file
wait for store api_data as read content from result_file
close file result_file

display "API data: " with api_data
```

**Node.js script (fetch_api.js):**
```javascript
const fs = require('fs');
fetch('https://api.example.com/data')
    .then(r => r.json())
    .then(data => fs.writeFileSync('api_result.json', JSON.stringify(data)));
```

## Web Server Interoperability

WFL web servers are standard HTTP servers that work with:

### Any HTTP Client

```bash
# curl
curl http://localhost:8080/api/status

# wget
wget http://localhost:8080/data.json

# Browser
# Just visit http://localhost:8080
```

### JavaScript Fetch API

```javascript
fetch('http://localhost:8080/api/users')
    .then(response => response.json())
    .then(data => console.log(data));
```

### Python Requests

```python
import requests
response = requests.get('http://localhost:8080/api/status')
print(response.json())
```

## Reverse Interoperability

Other languages can call WFL:

### From Bash

```bash
#!/bin/bash
# Build script using WFL
wfl build.wfl
if [ $? -eq 0 ]; then
    echo "Build successful"
else
    echo "Build failed"
fi
```

### From Python

```python
import subprocess
result = subprocess.run(['wfl', 'script.wfl'], capture_output=True, text=True)
print(result.stdout)
```

### From Node.js

```javascript
const { execSync } = require('child_process');
const output = execSync('wfl script.wfl').toString();
console.log(output);
```

## Future Directions

### Planned Features

**1. Native JSON Support**
```text
// Planned
store data as parse json from json_string
store json as to json of object
```

**2. JavaScript Interop**
```text
// Planned
use javascript library "lodash" as _
store result as _.chunk(array, 2)
```

**3. FFI (Foreign Function Interface)**
```text
// Planned
import c library "libcustom.so" as custom
store result as custom.function(args)
```

**4. WebAssembly Compilation**
```text
// Planned
// Compile WFL to WASM for browser execution
```

**5. Database Adapters**
```text
// Planned
connect to postgres at "localhost/mydb" as db
wait for store users as query db with "SELECT * FROM users"
```

## Current Best Practices

✅ **Use subprocess for external tools** - Works today

✅ **Exchange data via files** - Universal format

✅ **Build JSON/CSV manually** - Interoperable formats

✅ **Use standard HTTP** - Web servers work with anything

✅ **Validate external input** - Always check data from external sources

❌ **Don't expect native JSON** - Build strings manually

❌ **Don't assume JavaScript interop** - Not yet available

❌ **Don't use WFL-specific features for data exchange** - Use standard formats

## What You've Learned

In this section, you learned:

✅ **Current interop** - Subprocess, files, HTTP
✅ **Calling external tools** - Git, npm, Python, Node.js
✅ **Data exchange** - CSV, JSON (manual), text files
✅ **Web server integration** - Standard HTTP works with everything
✅ **Security** - Command injection risks
✅ **Future plans** - Native JSON, JavaScript interop, databases
✅ **Workarounds** - How to integrate today

---

## Advanced Features Complete!

Congratulations! You've completed the Advanced Features section. You now understand:

✅ Async programming with `wait for`
✅ Web servers with built-in HTTP support
✅ File I/O for data persistence
✅ Pattern matching for validation
✅ Containers for object-oriented programming
✅ Subprocess execution for external integration
✅ Interoperability with other technologies

**You can now build real applications with WFL!**

## What's Next?

### Learn All Built-in Functions

**[Standard Library →](../05-standard-library/index.md)**
Explore 181+ built-in functions across 11 modules.

### Write Better Code

**[Best Practices →](../06-best-practices/index.md)**
Code style, security, performance, testing strategies.

### Build Something

**[Cookbook →](../guides/cookbook.md)**
Recipes for common tasks.

**[WFL by Example →](../guides/wfl-by-example.md)**
Complete working examples.

---

**Previous:** [← Subprocess Execution](subprocess-execution.md) | **Next:** [Standard Library →](../05-standard-library/index.md)
