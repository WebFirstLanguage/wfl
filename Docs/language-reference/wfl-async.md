Great! I’ll put together a detailed implementation plan for adding file, HTTP, and database I/O support to WFL using Tokio, with unified natural-language syntax and safe async execution. I’ll also design the `.wflsec` file format for optional security constraints, suggest changes to the AST and grammar, and cover error handling through WFL's `try/when/otherwise` construct. I’ll get back to you shortly with a full breakdown.

# Design and Implementation Plan for Asynchronous I/O in WFL

## Introduction and Goals

WebFirst Language (WFL) is a scripting language that emphasizes **natural-language syntax** for web programming tasks. To extend WFL’s capabilities, we plan to add **asynchronous I/O support** for file operations, HTTP requests, and database queries. The goal is to enable non-blocking, high-performance I/O while preserving WFL’s English-like coding style and ensuring safety. Key objectives include:

- **Unified Syntax:** Provide a consistent, sentence-like syntax for all I/O operations (file, network, database) so code reads intuitively (e.g. *“open file at **foo.txt** and read content”*).
- **Tokio Runtime Integration:** Leverage Rust’s Tokio runtime for asynchronous execution, ensuring that I/O doesn’t block the interpreter. Tokio is a proven, widely-used async runtime in Rust ([Futures and the Async Syntax - The Rust Programming Language](https://doc.rust-lang.org/book/ch17-01-futures-and-syntax.html#:~:text=Tokio%20is%20the%20most%20widely,well%20tested%20and%20widely%20used)) that will give WFL safe concurrency.
- **Support for Multiple I/O Types:** Implement asynchronous **file** access (using `tokio::fs`), **HTTP** requests (using `reqwest`), and **database** queries (using an async library like `sqlx`). All should follow the unified syntax.
- **“wait for” Keyword for Async:** Introduce a natural-language `wait for` syntax to pause execution until an async operation completes. This behaves like an English equivalent of `await`, allowing straightforward sequencing of dependent operations.
- **Security and Permissions:** Implement a sandboxing mechanism via an optional **`.wflsec` permissions file**. This file will declare which files, URLs, or databases the script is allowed to access. The interpreter will enforce these restrictions at runtime to prevent unauthorized access.
- **Exception-based Error Handling:** All I/O errors (file not found, network failure, query error, permission denied, etc.) will raise exceptions in WFL (to be caught with `try/when/otherwise` blocks) rather than returning error codes. This keeps error handling consistent and in line with WFL’s natural syntax error management.
- **Parser and AST Extensions:** Extend the WFL grammar (likely using Pest) and AST to recognize the new async I/O constructs (e.g., the phrase “open file at *X* and read content”) as first-class language elements. We will provide examples of the new syntax and how it maps to internal function calls or AST nodes.
- **Interpreter Modifications:** Refactor the interpreter’s execution engine to support asynchronous evaluation. Functions like expression evaluation and statement execution (`eval_expr`, `execute_statement`) will become `async` functions so they can `await` I/O futures. The interpreter will run inside a Tokio runtime for scheduling.
- **Testing and Security Review:** Develop a test plan covering each I/O type (files, HTTP, DB) in both success and failure scenarios, including permission enforcement. We will also address security concerns like file path traversal, SQL injection, enforcing HTTPS for network calls, and verifying permissions from the `.wflsec` file.

By achieving these, WFL will allow developers to perform I/O in an intuitive manner without worrying about blocking behavior. The following sections detail the design for each aspect of this implementation.

## Runtime Integration with Tokio for Async Execution

To implement asynchronous I/O, WFL’s interpreter must run on an async runtime. We choose **Tokio** as it is a well-tested, widely used async runtime in Rust ([Futures and the Async Syntax - The Rust Programming Language](https://doc.rust-lang.org/book/ch17-01-futures-and-syntax.html#:~:text=Tokio%20is%20the%20most%20widely,well%20tested%20and%20widely%20used)). Tokio provides an event loop and executors that allow multiple tasks (futures) to progress without blocking each other. Key integration points:

- **Tokio Dependency:** We will add Tokio as a dependency (using the full feature set or at least the `runtime` and I/O features). The interpreter’s main entry point (for example, the `main()` of the WFL CLI or the evaluation function in a host application) will be marked with `#[tokio::main]` or otherwise ensured to run inside a Tokio runtime. This macro starts the Tokio reactor and thread pool, allowing us to use `.await` inside.
- **Async Functions:** Core interpreter functions will be converted to `async fn` so that within them we can call `await` on I/O futures. For example, `Interpreter::execute_statement(&Stmt)` becomes `async fn execute_statement(&mut self, stmt: &Stmt)`, and similarly for expression evaluation. This change will propagate through the call stack, ultimately requiring the top-level evaluation to be awaited.
- **Non-Blocking Execution:** Using Tokio means that while WFL awaits an I/O operation, other asynchronous tasks (if any) can run. Tokio ensures that operations like file reads, network calls, and DB queries do not stall the entire thread. For instance, if WFL launches multiple operations, Tokio can drive them concurrently. Even file I/O, which isn’t truly async at the OS level, is handled by Tokio using background threads to prevent blocking the main executor ([tokio::fs - Rust](https://doc.servo.org/tokio/fs/index.html#:~:text=Be%20aware%20that%20most%20operating,run%20them%20in%20the%20background)). In short, **no thread-blocking calls** will be made on the main interpreter thread; everything goes through futures and the Tokio scheduler.
- **Tokio Features Utilized:** We will specifically use:
  - `tokio::fs` for file operations (provides async versions of file open, read, write, etc.).
  - `reqwest` (which uses Tokio under the hood) for async HTTP. Reqwest’s default client is async and uses Tokio’s reactor for network I/O ([reqwest - Rust](https://docs.rs/reqwest/#:~:text=,Cookies)).
  - `sqlx` with the Tokio runtime feature for database operations. SQLx is designed to be runtime-agnostic and will operate asynchronously on Tokio ([GitHub - launchbadge/sqlx:  The Rust SQL Toolkit. An async, pure Rust SQL crate featuring compile-time checked queries without a DSL. Supports PostgreSQL, MySQL, and SQLite.](https://github.com/launchbadge/sqlx#:~:text=,using%20async%2Fawait%20for%20maximum%20concurrency)).
- **Single vs Multi-threaded Runtime:** By default, `#[tokio::main]` uses a multi-threaded executor. This is fine for our needs, as it will allow, for example, file I/O to run on a thread pool and multiple DB queries to parallelize. All WFL interpreter tasks (executing the user’s script) will reside on one of these threads. We must ensure any futures we spawn (if we allow concurrency) are `Send` or use `spawn_blocking` appropriately. Alternatively, we could use Tokio’s current-thread runtime if we want stricter control, but the multi-threaded runtime provides more flexibility and performance.
- **Example:** When WFL code says `wait for open file at "data.txt" and read content`, the interpreter will call an async Rust function (from `tokio::fs`) to open and read the file. The `.await` on that future yields control back to Tokio. Tokio may schedule other tasks (like another WFL I/O or simply idle) while the file is being read. Once the OS operation completes (on a background thread, since file I/O is blocking at OS level ([tokio::fs - Rust](https://doc.servo.org/tokio/fs/index.html#:~:text=Be%20aware%20that%20most%20operating,run%20them%20in%20the%20background))), Tokio wakes the interpreter task to resume with the file’s content. This way, WFL achieves concurrency without explicit threading in the language.
- **Error Propagation:** Tokio integration also means we need to handle the Results returned by these async calls. We will use Rust’s `?` operator or manual `match` to catch errors from futures and turn them into WFL exceptions (discussed later).

Overall, integrating Tokio ensures **safe and efficient asynchronous execution**. The WFL developer does not see Tokio at all – they simply use `wait for` and other syntax – but under the hood we rely on Tokio to manage timers, sockets, threads, etc., for non-blocking behavior. This approach aligns with Rust’s best practices for async: using futures and an executor rather than blocking calls in threads.

## Asynchronous File I/O Support (Using `tokio::fs`)

**Syntax Design:** WFL will support natural-language commands to interact with files. The basic pattern is to **open a file resource and then perform an action** on it (read or write). We propose the following syntax for common operations:
- **Read a File:** `open file at "<path>" and read content [as <var>]`. This command opens the file at the given path and reads its entire content. If an `as <var>` clause is provided, the content is stored in that variable; otherwise the content is returned as the expression’s value.
- **Write to a File:** `open file at "<path>" and write <dataExpr>`. This opens/creates the file at the path and writes the given data (which could be a string expression). Optionally, we might allow `and write <data> and close` or use a similar natural phrasing. The result of a write operation could be a success indicator (or we might just return nothing and rely on exceptions for errors).
- (In the future, more actions could be added, like **append**, but for now read/write suffice.)

**Examples:**
```wfl
// Reading a file
wait for open file at "config.txt" and read content as configText

// Writing to a file
wait for open file at "output.log" and write logMessage
```
In the first example, the content of *config.txt* is read asynchronously and stored in `configText`. In the second, the string in `logMessage` is written to *output.log* (creating or overwriting it). The `wait for` keyword (explained later) ensures these operations complete before proceeding.

**Underlying Implementation:** These file operations will be implemented using the `tokio::fs` module:
- To **open and read** the entire file, we will likely use the convenient function `tokio::fs::read_to_string(path)`. This returns a `Future` that reads the complete file into a `String` ([tokio::fs - Rust](https://doc.servo.org/tokio/fs/index.html#:~:text=For%20example%2C%20to%20read%20the,file)). For binary data, `tokio::fs::read(path)` returns a `Vec<u8>` future; but since WFL is text-focused, we’ll treat file contents as strings unless specified otherwise.
- To **write** data to a file, we can use `tokio::fs::write(path, data)`, which asynchronously writes the entire provided data (creating the file if needed, truncating if it exists) ([tokio::fs - Rust](https://doc.servo.org/tokio/fs/index.html#:~:text=For%20example%2C%20to%20read%20the,file)). This returns a future that resolves when the write is done.
- Both of these functions actually use blocking OS calls internally, but Tokio ensures they run on a threadpool via `spawn_blocking` so as not to block the async runtime ([tokio::fs - Rust](https://doc.servo.org/tokio/fs/index.html#:~:text=Be%20aware%20that%20most%20operating,run%20them%20in%20the%20background)). Thus, WFL’s thread is free while large files are loading or being written.
- If a more granular control is needed (like streaming a very large file), we could use `tokio::fs::File` and `AsyncReadExt`/`AsyncWriteExt` traits, but initially we aim for simplicity (whole-file operations).

**Mapping to AST/Calls:** The WFL parser will recognize the pattern `"open file at" STRING "and read content"` as a single operation (perhaps creating an AST node like `AstNode::FileRead(path)`), and similarly `"open file at" STRING "and write" <expr>` as `AstNode::FileWrite(path, data)`. During interpretation:
  - If it’s a read operation, the interpreter will call `tokio::fs::read_to_string(path).await` to get the file content. On success, it returns a WFL string value. On failure, it raises an exception (e.g., `FileError`).
  - If it’s a write operation, the interpreter evaluates the data expression to get a string or byte array, then calls `tokio::fs::write(path, dataBytes).await`. On success, perhaps return a success message or a special `None` value; on error, raise an exception.

**Error Handling:** File I/O can fail in various ways (file not found, permission denied by OS, etc.). Our design says **no error is silently absorbed**; instead, any `Err` from Tokio will translate into a WFL exception. For example:
- *File not found:* `open file at "nosuch.txt" and read content` will raise a `FileError` exception indicating the file wasn’t found.
- *Permission denied:* If the OS prevents access, we raise a `FileError` with a permission message.
- *Write error:* If writing fails (e.g., disk full), an exception is raised.
These exceptions can be caught in WFL using a `try/when/otherwise` block (see Error Handling section).

**Security – Path Restrictions:** A major security concern is **path traversal** (a script trying to escape allowed directories using `../`). We will mitigate this using the `.wflsec` file’s permissions and path normalization:
  - Before opening a file, the interpreter will **canonicalize the requested path** (resolve `..`, symlinks, etc.) and then check it against the allowed paths from `.wflsec`. If the path is not within an allowed directory or does not match an allowed pattern, the operation will be blocked. This approach (canonicalize then check prefix) is a known strategy to prevent traversal attacks ([RUSTSEC-2021-0126: rust-embed: RustEmbed generated `get` method allows for directory traversal when reading files from disk › RustSec Advisory Database](https://rustsec.org/advisories/RUSTSEC-2021-0126.html#:~:text=The%20flaw%20was%20corrected%20by,with%20the%20canonicalized%20folder%20path)).
  - For example, if `.wflsec` permits `"/app/data"` directory, and the script tries to `open file at "../secret.txt"`, the canonical path might be `/app/secret.txt` which is outside `/app/data`, so the interpreter will refuse and throw a security exception.
  - We will also ensure that relative paths in the script are resolved relative to a known base (perhaps the script’s directory if allowed by policy) and then checked.
  - If no `.wflsec` is present or it doesn’t allow the requested file, the attempt will be denied (we prefer safe default: no file access unless explicitly allowed).

**Return Types:** The content read from a file will be represented as a WFL string value (since WFL likely supports strings natively). If binary data is ever needed, we might introduce a byte array type or base64 encoding, but initially assume text. For writes, WFL might not need to use the result, but we could return an integer count of bytes written or simply nothing. It may be simplest to return nothing and rely on exceptions for errors – reflecting that the intent was achieved or an error thrown.

**Additional Considerations:**
- *File Handles:* In the unified model, we are often combining open-and-read in one command. If needed, WFL could also allow separate steps: e.g., `open file at "x.txt" as f` (getting a file handle object) and then later `wait for read content from f as data`. This would mirror how one might open a file and then read incrementally. For now, our plan focuses on the one-liner for simplicity. Under the hood, however, it might still call `File::open` and then `read_to_end`, but we treat it as atomic from the language perspective.
- *Writing modes:* The syntax “open file and write” by default will **overwrite** the file (as `tokio::fs::write` does). We might later introduce a keyword for appending or ensure `.wflsec` can specify if writes are allowed. For now, it overwrites existing content.
- *Closing files:* Tokio will close file handles when they drop out of scope. In our case, since we use the convenience functions, the handle is opened and closed internally during the awaited call. If we did use persistent handles, we’d likely close on drop or provide a language command to close, but that might not be needed unless we hold files open across operations.

By using `tokio::fs` we ensure file operations are asynchronous and do not block the interpreter. This design abstracts away the complexity (the user doesn’t see `await` or futures), but under the hood, for example, reading a file yields to Tokio’s runtime which may utilize a threadpool to fetch the data ([tokio::fs - Rust](https://doc.servo.org/tokio/fs/index.html#:~:text=Be%20aware%20that%20most%20operating,run%20them%20in%20the%20background)). The result is that WFL can handle file reads/writes efficiently and safely, in a style like “English instructions.”

## Asynchronous HTTP Operations (Using `reqwest`)

**Syntax Design:** We want WFL to be able to make web requests in a simple, declarative way. The syntax should feel like instructing the language to fetch or send data. Proposed constructs:
- **HTTP GET (fetch):** `open url at "<URL>" and read content [as <var>]`. This will perform an HTTP GET request to the specified URL and retrieve the response body. The content (e.g., HTML or JSON text) can be stored in a variable if `as <var>` is provided.
- **HTTP POST/PUT (sending data):** We can introduce a variation such as: `open url at "<URL>" with method POST and write <bodyData> [and read content]`. Another phrasing could be `send <dataExpr> to url "<URL>" [as <var>]`. To keep consistent structure, perhaps using “open url” for all, but including method:
  - E.g., `open url at "https://api.example.com/resource" with method POST and write requestBody and read content as responseData`.
- **Headers or advanced options:** Initially, we may not expose setting custom headers or handling complex responses in natural syntax (to avoid too much complexity). We might default to common behavior (like JSON content type if the body is a JSON object, etc., though that’s speculative). Advanced usage can be expanded later or via built-in library calls.

**Examples:**
```wfl
// Simple GET request
wait for open url at "https://api.example.com/data" and read content as resultData

// POST request with a payload
wait for open url at "https://api.example.com/items" with method POST and write newItemJson and read content as response
```
In the first example, the content from the GET request (e.g., a JSON string from the API) is stored in `resultData`. In the second, we POST `newItemJson` to the server and read the response (perhaps confirmation or created item data) into `response`.

**Underlying Implementation:** We will use the **Reqwest** HTTP client crate for networking:
- **GET requests:** We can use the high-level `reqwest::get(URL).await` for simple GETs. This returns a `Response` which we can then `.text().await` to get the response body as a string ([reqwest - Rust](https://docs.rs/reqwest/#:~:text=%C2%A7Making%20a%20GET%20request)). Reqwest automatically handles many details: DNS resolution, establishing connections, following redirects (by default), etc. The client also **uses TLS by default** for HTTPS URLs ([reqwest - Rust](https://docs.rs/reqwest/#:~:text=,Cookies)), meaning connections are secure (and it will verify certificates).
- **Custom methods/requests:** For anything beyond GET, we’ll create a `reqwest::Client` (which we might reuse across calls for efficiency). For example, for a POST: 
  ```rust
  let client = reqwest::Client::new();
  let resp = client.post(url)
      .body(bodyData)
      .send().await?;
  let text = resp.text().await?;
  ```
  We will map the WFL “with method POST and write X” to such a call. Similarly for other methods (PUT, DELETE, etc.) if needed. Using a single `Client` for multiple requests is wise to utilize connection pooling ([reqwest - Rust](https://docs.rs/reqwest/#:~:text=)), but since WFL scripts likely won’t do dozens of calls in tight loops, even using separate calls is fine initially. We could have a lazy static Client to reuse.
- **Response handling:** By default, we will treat the response body as the content the user wants (similar to how a browser fetch API often gives you the body text or JSON). So our implementation will call `.text().await` on the response to get the full body as a string. This is convenient for typical use (fetching an HTML page or a JSON API response). If the user needs headers or status code, we might later provide properties or methods on the response object; but in this design, `read content` implies we are just giving the body. We will implicitly trust that the body is text (if it’s binary, it might get garbled in a string; advanced usage could allow binary but out of scope now).
- **Async streaming:** If needed, reqwest supports streaming bodies, but given WFL’s likely use-cases, reading the whole content into a string is acceptable. Large downloads might be an issue, but that’s an edge case.

**Mapping to AST/Calls:** The grammar will catch `"open url at" <URL>` with optional method and body. We might have an AST variant like `AstNode::HttpRequest { method, url, bodyExpr, captureVar, readResponse }`. For instance:
  - `open url at "http://example.com" and read content` would be method=GET, bodyExpr=None, and readResponse=true.
  - `open url at "http://api.com" with method POST and write dataExpr and read content` would set method=POST, bodyExpr=dataExpr, readResponse=true.
  - If we ever allow an operation that doesn’t need the response (like sending data without caring for the response body), we might allow omitting `and read content`, but usually one wants some result or at least confirmation.

At runtime, the interpreter will:
  - Construct the appropriate reqwest request (GET or POST with given body, etc.).
  - Await the future of sending the request and receiving the response.
  - If `read content` was requested, await the `response.text()` future to get the body string.
  - If the WFL code provided an `as var`, store the result string in that variable; otherwise use it as an expression result.
  - If the HTTP request fails (network error, DNS failure, timeout, etc.), throw an exception (`NetworkError` or specifically `HttpError`). If the server responds with a non-200 status, we have a design choice: we can either treat that as a normal result (perhaps the content could be an error message/HTML from server), or we could choose to raise an exception for certain HTTP error codes. **Design decision:** It’s probably better to **not** throw exceptions on HTTP error statuses by default – it should be up to the script to check the content or a status if we expose it. We will reserve exceptions for *technical failures* (no connection, invalid URL, disallowed by policy, etc.). So a 404 or 500 response will still result in a body (which might be an error page) being delivered as `response`.
  - Status codes and headers might not be directly accessible in this first implementation. We could in future provide something like `response.status` or have `open url ... as response` where `response` is an object with fields. But initially, we assume the main interest is in the content.

**Security – Domain Restrictions & HTTPS:** We must ensure network access is controlled:
  - The `.wflsec` file can list allowed URLs or domains. The interpreter will parse the target URL and check that the hostname (and optionally port) match an allowed pattern. For example, if `.wflsec` allows `"api.example.com"`, then a request to `https://api.example.com/anything` is permitted, but a request to `evil.com` is not. If not allowed, the interpreter will raise a security exception **before** making the request.
  - By default, we will **enforce HTTPS** for external URLs unless the user explicitly allows an HTTP URL. It’s best practice for any API or web fetch to use TLS ([Best practices for REST API security: Authentication and authorization - Stack Overflow](https://stackoverflow.blog/2021/10/06/best-practices-for-authentication-and-authorization-for-rest-apis/#:~:text=Always%20use%20TLS)). If a script tries `open url at "http://example.com"` and the permissions only allow the host but not with HTTP, we either: (a) deny it outright, or (b) warn/throw suggesting to use https. We can have `.wflsec` specify if plain HTTP is allowed. Likely, the default stance is *HTTPS required*. (For local addresses like `http://localhost` for testing, the user can allow that specifically in `.wflsec`.)
  - **Redirects:** Reqwest will automatically follow redirects by default. This can be a security issue if a permitted URL redirects to an unpermitted domain. To handle this, we should either disable automatic redirects and handle them manually, or check each redirect target against the allowed list. Simpler: we can instruct reqwest’s client to not follow redirects (or to have a redirect policy that only allows same-domain redirects). If not, a malicious redirect could bypass domain restrictions. We will aim to configure a redirect policy: e.g., `.redirect(reqwest::redirect::Policy::limited(10))` but with a check. If not easily configurable, we might just document that only redirects to allowed domains will be followed, and implement a check loop if necessary.
  - **Timeouts:** It might be prudent to set a reasonable timeout on requests so a misbehaving server doesn’t hang the WFL script indefinitely. We could use reqwest’s timeout API or Tokio’s `timeout` function around the await.

**Error Handling:** As noted, network failures will raise exceptions. We’ll likely have a generic `NetworkError` or `HttpError`. For instance:
  - DNS resolution failure, connection refused, timeout – all result in an exception with a message (e.g., “NetworkError: could not connect to host”).
  - TLS errors (invalid certificate) – exception (unless we allow ignoring TLS verification via config, but by default we won’t).
  - If `.wflsec` disallows the domain – a **security exception** (distinct from a network error).
  - HTTP error status (4xx, 5xx) – as decided, not an exception by default. But if needed, the script can interpret the `response` content or we can in future provide a way to check status.

**Return Data:** The response content will be returned as a string. If the script expects JSON, it can parse it (in the future, WFL might have a JSON parse function or automatically detect JSON and provide an object – but that’s beyond this plan). For binary data (like an image), returning raw bytes in a string is not ideal. We might simply state that WFL’s `read content` is intended for text responses. If binary download is needed, we could provide a different command (like `download file from URL to path` – which could reuse file I/O and HTTP together). For now, focus on typical text-based web APIs.

With reqwest integrated, WFL web requests will be efficient and straightforward. **Reqwest’s async client** is robust and covers HTTP/1.1 and HTTP/2, HTTPS, proxies, cookies, etc., much of which we get for free ([reqwest - Rust](https://docs.rs/reqwest/#:~:text=,Cookies)). Importantly, using reqwest means **no manual socket handling** in WFL – we rely on the library for networking, which reduces bugs. We will ensure to keep security tight by filtering URLs via `.wflsec` and encouraging TLS.

## Asynchronous Database Access (Using `sqlx`)

**Syntax Design:** Database operations are a bit more complex because they involve connecting to a database and executing queries. WFL should allow these in a natural way. We outline a two-step approach (though we can also allow a one-liner for simple cases):
- **Open a Database Connection:** `open database at "<connection_string or name>" [as <connVar>]`. This establishes a connection (or connection pool) to the database. The connection string might be a DSN (like `"postgres://user:pass@host/dbname"`) or an alias defined in `.wflsec` for a sensitive credential. If an `as` variable is provided, the connection handle is stored for reuse; otherwise it could implicitly become a default connection.
- **Query the Database:** There are a few natural-language patterns we can use:
  - `wait for perform query "<SQL query>" on <connVar> [as <resultVar>]`
  - or simply `wait for query database <connVar> with "<SQL query>" as <resultVar>`
  - or even treat it as reading from the database: `wait for open database at "<connStr>" and read content "<SQL>" as <resultVar>`. However, *“read content”* is less intuitive for databases. It might be clearer to use the word **“query”** in the syntax to denote executing an SQL statement.
- We choose a phrasing like: **`perform query "<SQL>" on <database>`**. This sounds like an instruction and fits English (“perform this query on that database”). The `<database>` could be a connection variable obtained from the earlier open command, or possibly an alias name if the connection was named via `open database at "X" as mainDB`. We will allow either the var or some identifier.
- For write operations (INSERT/UPDATE/DELETE), it’s still “perform query” – the SQL itself dictates whether data is returned or not. If no result rows, the operation could return an affected-row count or just success.

**Examples:**
```wfl
// Open a database connection (using an alias defined in .wflsec for security)
open database at "mainDB" as dbConn

// Execute a SELECT query
wait for perform query "SELECT id, name FROM users WHERE active = true" on dbConn as activeUsers

// Execute an INSERT (no result data expected)
wait for perform query "INSERT INTO logs(msg) VALUES('Started')" on dbConn
```
In the above, `"mainDB"` might correspond to a connection string in the `.wflsec` file (for example, an alias for a Postgres database). We store the connection in `dbConn`. Then we perform a SELECT; the results are awaited and stored in `activeUsers`. The last example performs an INSERT; we `wait for` it to complete but didn’t specify an `as` variable – we might not need to capture anything, though we could return the row count if desired.

**Underlying Implementation with SQLx:** We will use **SQLx** crate for asynchronous database interactions. SQLx supports PostgreSQL, MySQL/MariaDB, SQLite, and others, all asynchronously and without needing a separate database driver thread ([GitHub - launchbadge/sqlx:  The Rust SQL Toolkit. An async, pure Rust SQL crate featuring compile-time checked queries without a DSL. Supports PostgreSQL, MySQL, and SQLite.](https://github.com/launchbadge/sqlx#:~:text=,using%20async%2Fawait%20for%20maximum%20concurrency)):
- **Connection Management:** On `open database at "<conn_str>"`, the interpreter will create a connection or a connection pool. SQLx provides `Pool<DB>` types which are efficient and can manage multiple connections. For simplicity, we might create a single connection if concurrency on the DB isn’t needed, or a pool to allow parallel queries (should WFL ever do that). Using `sqlx::PgPool::connect(conn_str).await` (or MySql/Sqlite equivalent based on prefix in conn_str) will establish the connection. This returns a Pool object.
  - We will likely store this Pool in the interpreter’s state, keyed by either the variable name or some identifier. The WFL variable `dbConn` will refer to this pool handle internally.
  - If the connection fails (bad credentials, unreachable host), this await will error, and we raise an exception (e.g., `DatabaseError: could not connect`).
  - We will also enforce that the `conn_str` is allowed by `.wflsec` (under a “databases” section). More on that in security.
- **Executing Queries:** For the `perform query "<SQL>" on <conn>` command:
  - We will use SQLx’s query API. SQLx allows two main ways: a high-level macro `query!` (which is compile-time checked, but our queries are dynamic from scripts, so we cannot use compile-time checking), or the runtime API `sqlx::query(sql)` which returns a query object where we can `.execute(&pool)` or `.fetch_all(&pool)`, etc.
  - We will parse the SQL string from the WFL source. The interpreter will decide whether to use `.execute` or `.fetch_all` based on the query type. We can do a simple check: if the SQL starts with "SELECT" (case-insensitive), we assume it returns rows and use `.fetch_all` to get a collection of rows. Otherwise (INSERT, UPDATE, DELETE, DDL statements), we use `.execute` which returns the number of affected rows.
  - The returned futures from SQLx are awaited:
    - For `.execute`, SQLx returns a result containing an `Ok(<DB::Result>)` where that might include rows affected. We might capture that number if needed.
    - For `.fetch_all`, it returns `Vec<sqlx::Row>` on success.
  - We then need to convert the result into a WFL value:
    - If it’s a set of rows (Vec<Row>): We will likely convert this into a WFL list of records. WFL might represent objects/dictionaries, so each row can be a dictionary mapping column names to values. For example, a row with columns id and name might become something like `{"id": 5, "name": "Alice"}` in WFL. We’ll gather those in a list for the query result.
    - If it’s a non-query (like INSERT), the result could be just an integer count of affected rows. We can return that as a number. Or we might return a special `None` (if WFL has a concept of null) just to indicate completion. However, returning the count is useful, so the script can check if an update actually changed something.
  - SQLx handles retrieving column values by type; it can give them as Rust types (i32, String, etc.). We’ll map those to WFL’s native types (WFL likely has number and string types at least).
  - If the query fails (SQL syntax error, constraint violation, etc.), the `await` returns an Err, and we raise a `DatabaseError` with details (maybe including the DB error message).
- **Prepared Statements & Injection:** SQLx by default uses prepared statements for `query()` calls, which helps against SQL injection if parameters are bound instead of concatenated ([Raw SQL in Rust with SQLx | Shuttle](https://www.shuttle.dev/blog/2023/10/04/sql-in-rust#:~:text=By%20default%2C%20SQLx%20promotes%20using,find%20more%20about%20this%20here)). In our initial implementation, if the WFL user just writes an SQL string that includes unsanitized input, it’s like constructing raw SQL – subject to injection. **Mitigation:** We can encourage or enforce parameter binding in WFL:
  - For example, WFL could allow placeholders like `?` or `$1` in the query and allow the user to pass separate values. A possible syntax: `perform query "SELECT * FROM users WHERE id = ?" with  userId on dbConn as result`. Then the interpreter would call `sqlx::query(...).bind(userId_value)`.
  - However, designing a natural syntax for binding parameters might be tricky. Initially, we might not implement it, but we will strongly note in documentation that user data should be handled carefully. If WFL scripts are mostly under the control of the developer (and not end-users), injection is a risk if they incorporate external input unsafely. Using `.wflsec` and safe coding practices can mitigate it.
  - In the future, we could allow binding by having multiple expressions after the query string that correspond to `?` placeholders in order. This would leverage SQLx’s ability to bind values and **prevent SQL injection by separating the query from data** ([Raw SQL in Rust with SQLx | Shuttle](https://www.shuttle.dev/blog/2023/10/04/sql-in-rust#:~:text=By%20default%2C%20SQLx%20promotes%20using,find%20more%20about%20this%20here)). For now, we assume the queries are either static or the script writer knows to sanitize input if concatenating.
- **Supported Databases:** Because SQLx is DB-agnostic ([GitHub - launchbadge/sqlx:  The Rust SQL Toolkit. An async, pure Rust SQL crate featuring compile-time checked queries without a DSL. Supports PostgreSQL, MySQL, and SQLite.](https://github.com/launchbadge/sqlx#:~:text=%2A%20Compile,SQLx%20is%20not%20an%20ORM)), WFL can theoretically support any of Postgres, MySQL/MariaDB, SQLite (and possibly others like MSSQL in the future). The connection string (URI) will determine the driver. We need to compile WFL with the features for the databases we want (e.g., `features = ["postgres", "mysql", "sqlite"]` in sqlx). Initially, we might allow SQLite and Postgres (common cases for local or server DB). 
  - *Note:* If supporting SQLite, there is a nuance: SQLx’s SQLite driver uses `libsqlite3` (C library) which is synchronous. However, SQLx runs it in an async fashion by spawning threads if necessary (similar to tokio::fs approach). It’s still safe to use as async, but heavy concurrent queries on SQLite might block each other due to the global interpreter lock on the database file. That’s acceptable given SQLite’s nature.

**Mapping in Parser/AST:** Grammar rules will be added for:
- `"open database at" <STRING> ["as" <Ident>]` – produces an AST node to initiate a DB connection.
- `"perform query" <STRING> "on" <Ident>` – AST node for executing a query on a given database handle.
  - Possibly allow the variant `"perform query" <STRING> "on database" <STRING>` if using a named alias directly from `.wflsec`. But it’s more consistent to require an opened handle (the `.wflsec` alias could be used in the open step).
- We might also allow shorthand: If the user doesn’t call `open database` separately, perhaps `perform query "<SQL>" on "<connStr>"` could implicitly open a connection, execute, then close. This is convenient but less efficient if multiple queries. We can implement that by parsing a string after “on” as either a connection variable or a direct connection string. However, encouraging the explicit open is better for clarity and reusability, so we’ll focus on that.

At runtime:
  - For an `open database` AST node, the interpreter checks the `.wflsec` permissions (to ensure that connection is allowed). Then it calls the appropriate SQLx connect function. This returns a Pool or Connection which we store in a map of active DB connections in the interpreter.
  - The WFL variable (if given with `as`) is mapped to this connection handle. If no `as` was given, we might still store it in a special slot (like a default connection) or simply not allow omission in practice. (Likely we require `as` to use it later; otherwise the only point would be to do a one-liner query with it.)
  - For a `perform query` AST node, the interpreter finds the corresponding connection (via the variable or name), prepares and executes the query as described, and constructs the result value.

**Error Handling:** Database operations can raise exceptions for:
  - Connection failure (e.g., wrong credentials, network issues connecting to a remote DB) – exception on `open database`.
  - Disallowed connection (if `.wflsec` doesn’t permit that DB) – a security exception.
  - Query failure – exception with the database’s error message if available. For example, if the SQL syntax is wrong or violates a constraint, we throw, so the user can catch it and perhaps handle (like using a `when` clause for `DatabaseError`).
  - Also, using an invalid connection handle (e.g., `perform query ... on dbConn` when `dbConn` isn’t opened or has been closed) would be a runtime error. We’ll assume the script is written correctly such that `dbConn` exists from a prior open, otherwise an exception is thrown (like an “Unbound variable” or we can specifically say “Database connection not available”).

**Security – Permissions for Databases:** The `.wflsec` file will include a section to control DB access:
  - It might list approved connection strings or aliases. For instance, the `.wflsec` could contain:
    ```toml
    [databases]
    mainDB = "postgres://user:pass@dbserver/mydb"
    reportingDB = "sqlite:./reports.db"
    ```
    This means the script is allowed to access those databases (and it provides the actual connection strings securely, so the script might use `open database at "mainDB"` and the interpreter looks up the actual DSN).
  - If the script tries to open a database not listed, the interpreter denies it. This prevents a malicious script from connecting to arbitrary databases or using credentials not provided.
  - For remote databases (Postgres/MySQL), the host in the connection string can be checked against allowed hosts as well. For SQLite (file-based), the path can be checked via file permissions (essentially, opening a SQLite database file is like opening a file; we should ensure that path is allowed under the file rules or the DB rules).
  - **No plaintext credentials in script:** By using aliases in `.wflsec`, we avoid having to put actual passwords in the WFL code. The code can just refer to a logical name. This is safer and also allows switching connections (for example, using a test DB vs production DB by changing `.wflsec`).

**Returning Query Results:** Representing the results in WFL:
  - We expect WFL has composite data types (like list and maybe map). We will return query results as a list of rows. Each row can be a map with keys as column names (as strings) and values as the column values (converted to WFL types).
  - Example: The query `SELECT id, name FROM users` might return `[ {id: 1, name: "Alice"}, {id: 2, name: "Bob"} ]` in WFL terms.
  - If the query is an INSERT/UPDATE that returns no rows, we can return an integer (affected row count). In many cases, the script might not need the count; if not, they can ignore it. But it’s useful to provide (and we can document that for non-select queries, the result is the number of affected rows).
  - If a query returns a single value (e.g., `SELECT COUNT(*)`), it would still come as a list of one row with one field, unless we detect that pattern and simplify it. Possibly we keep it uniform: always a list for consistency, even if one element.
  - We should also decide if the list is a regular WFL list or a special “ResultSet” type. Likely just a list.

**Closing connections:** If a connection is opened in a long-running script and not needed later, we might allow `close database <connVar>` in the language (for completeness). Not mentioned in requirements, but it’s a possible addition. If not, the connections will be closed when the interpreter terminates or the pool is dropped. For a short script, that’s fine. For a persistent environment, we might keep them open (pool will keep connections open for reuse, which is good).

By using **SQLx**, we ensure the DB interactions are asynchronous and efficient. SQLx uses async/await thoroughly ([GitHub - launchbadge/sqlx:  The Rust SQL Toolkit. An async, pure Rust SQL crate featuring compile-time checked queries without a DSL. Supports PostgreSQL, MySQL, and SQLite.](https://github.com/launchbadge/sqlx#:~:text=,using%20async%2Fawait%20for%20maximum%20concurrency)) and even supports features like compile-time query checking (though we can’t utilize that for dynamic code) and connection pooling. It’s also safe (no unsafe code for Postgres/MySQL drivers ([GitHub - launchbadge/sqlx:  The Rust SQL Toolkit. An async, pure Rust SQL crate featuring compile-time checked queries without a DSL. Supports PostgreSQL, MySQL, and SQLite.](https://github.com/launchbadge/sqlx#:~:text=,zero%20unsafe%20%20%20code))). This aligns with WFL’s safety goals. 

One more security aspect: **SQL Injection**. As mentioned, if the WFL script concatenates user input into an SQL string, it could cause SQL injection. Our design’s main line of defense is to encourage parameter binding. In a future update, we can extend WFL’s syntax to support parameters (e.g., using `?` placeholders and passing separate values). That would allow us to call `query(sql).bind(val)` as recommended (bound parameters are *“very important for preventing SQL injection”* ([Raw SQL in Rust with SQLx | Shuttle](https://www.shuttle.dev/blog/2023/10/04/sql-in-rust#:~:text=By%20default%2C%20SQLx%20promotes%20using,find%20more%20about%20this%20here))). For now, script authors need to be cautious. The `.wflsec` won’t directly prevent injection since it’s not about the content of queries – it’s more about which DB can be touched. We will note this in documentation and possibly provide some helper to sanitize inputs if needed.

## The `wait for` Keyword and Async Execution Semantics

To integrate asynchronous calls seamlessly into WFL, we introduce the **`wait for`** syntax. This is conceptually similar to using `await` in other languages, but phrased as natural language. The `wait for` keyword is placed before an asynchronous operation to indicate that the script should pause until that operation completes and yield its result.

**Semantics:**
- **Immediate Await:** When a WFL statement or expression is prefixed with `wait for`, the interpreter will execute the operation asynchronously and **wait for its completion** before moving on. During this wait, other tasks (if any) could proceed on the Tokio runtime, but in WFL’s single-threaded script context, it simply acts as a block until the result is ready.
- **Use in Assignments:** Typically, `wait for` will be used in a context of assignment or output. For example: `wait for open file at "data.txt" and read content as fileData`. Here the whole I/O action is awaited, and once done, `fileData` is assigned. If instead the syntax is used in an expression position (like `print (wait for open url at "...")`), the interpreter would still perform the wait and then supply the value to the print function.
- **Asynchronous vs Synchronous Context:** WFL is designed such that the same I/O command can be used with or without `wait for`. If used without, it might initiate the action asynchronously without pausing (effectively returning a task handle/future that can be waited on later). If used with `wait for`, it behaves synchronously from the script’s perspective (though not blocking the system thread). This means the fundamental syntax for I/O doesn’t change, just the presence of `wait for` decides if we pause for the result or not. In practice, for now we will encourage always using `wait for` unless you plan to do multiple operations in parallel.
- **Parallel/Deferred Execution:** One advantage of separating the initiation of an async operation from waiting is that we can allow *concurrent operations*. For example, WFL could do:
  ```wfl
  task1 is open url at "https://api.github.com" and read content
  task2 is open url at "https://api.gitlab.com" and read content
  // launched two requests in parallel (no 'wait for' yet)
  wait for task1 as result1
  wait for task2 as result2
  ```
  Here, `task1` and `task2` would immediately start their HTTP requests (the interpreter would fire them off and store a handle). Later, using `wait for task1` will await their completion. This pattern allows simple concurrency (batching requests) without introducing threads or complex promises to the user – it reads like “do X and Y, then wait for X, then wait for Y.” We intend to support this usage. Under the hood, if `wait for` is not used, the interpreter will **spawn the async operation as a separate task** on Tokio and return a handle (which WFL could represent as a opaque “task” value).
- **Interpreter Implementation:** When the parser encounters a `wait for` prefix, it will likely create an AST node like `AstNode::Await(expr)`. The interpreter, when evaluating this node, will evaluate the inner `expr` which should produce a future or task, then await it. However, since our interpreter itself is running in an async context, we can simply call `.await` on the Rust future representing the inner operation.
  - For example, if inner expr is a `FileRead(path)` that we implement by calling `tokio::fs::read_to_string`, then `eval_expr(FileRead(path))` returns a Rust `Future<String>` (or rather, an `impl Future` since it’s an async fn). The `Await` node’s interpreter code does `let result = eval_expr(FileRead(path)).await;`.
  - Because Rust requires `.await` to happen inside an async function, converting `eval_expr` to async (as discussed) is necessary.
- **Safety and Simplicity:** We will implement `wait for` by directly using `.await` on the underlying future, as opposed to manual polling or callback chaining. This leverages Rust’s async/await to manage the state machine. It’s safe because any panics or errors in the future will be caught as Rust `Result` errors that we then turn into WFL exceptions. There is no risk of forgetting to wait (the compiler enforces it if we use `await` keyword).
- **No Busy-Wait:** Because this is true async awaiting, the OS thread is not blocked. Tokio will schedule other tasks or put the thread to sleep while waiting. So `wait for` does not equate to a CPU spin; it’s cooperative multitasking.
- **Analogy:** In essence, `wait for X` in WFL is analogous to `let res = X.await;` in Rust or `await X` in JavaScript/Python, except phrased to fit natural language flow.

**Grammar and Usage:**
- We will treat `wait for` likely as a **keyword** (or two-word keyword). The grammar might allow it before any expression that yields a future/task. For instance:
  ```
  WaitStmt = "wait for" ( IoOperation | Identifier )
  ```
  This means you can wait for a direct I/O operation (`wait for open file at "x" ...`) or wait on an identifier that represents a pending task (`wait for task1`).
- We might allow `wait for ... as <var>` as a combined form for convenience, though semantically it’s the same as doing `temp = wait for ...;` then assigning temp. For readability, we can integrate `as <var>` into the wait syntax for single step usage. E.g., `wait for open url at "..." and read content as html` is parsed such that `as html` applies to the result of the awaited operation.
- If an I/O operation is used without `wait for` in a context that expects a value (like assignment `x is open file at "foo.txt" and read content`), we will interpret that as launching the async operation and assigning a task handle to `x`. The type of `x` in WFL would be a “pending task” which can’t be directly used as the content until awaited. We will document that such a task can only be used by waiting on it. If you try to use it otherwise, it might implicitly convert or error. (This is analogous to having a Promise in JavaScript that you must await.)
- **Under the Hood (Task Handles):** When an async operation is initiated without waiting:
  - The interpreter could call `tokio::spawn` on the future to run it concurrently, yielding a `JoinHandle` future immediately. We then wrap that handle in a WFL value (a Task object).
  - Alternatively, we don’t spawn, but simply store the `Future` in a WFL value. Storing raw futures is tricky because they are not `Send` unless static; better to use spawn or a custom Future that can be polled later. Using `tokio::spawn` is straightforward and also detaches the execution (so even if WFL doesn’t await, it’s still running; though any result not awaited might be lost unless we handle that).
  - We likely will go with `spawn` to truly run it in parallel, and record the JoinHandle. When `wait for <task>` is called, we `.await` the JoinHandle. Tokio’s JoinHandle yields `Result<Output, JoinError>`; if the task panicked or was cancelled, that’s JoinError (we’d convert to exception), otherwise we get the output which is the actual I/O result.
  - This approach requires the futures to be `Send` because spawn on a multi-thread runtime needs that. Most of our futures (from tokio::fs, reqwest, sqlx) are Send, so it should be fine. We should avoid capturing non-Send things in those futures’ environment.
  - If we choose not to spawn, we could keep the future in memory and poll it later, but we’d need to manually manage waking etc. The complexity isn’t worth it given `spawn` exists and overhead is low.

**Example of Concurrency:**
Suppose a WFL script wants to fetch two APIs simultaneously (as shown earlier). Without `wait for` on each immediately, the interpreter will start both:
```wfl
api1 is open url at "https://service1.com/data" and read content
api2 is open url at "https://service2.com/data" and read content
// At this point, api1 and api2 are task handles (requests in flight concurrently)
wait for api1 as data1
wait for api2 as data2
```
This will be achieved by, internally, spawning two reqwest futures. They run in parallel on Tokio’s executor. The `wait for api1` will await the completion of the first, which may already be done by the time we reach it (or not, but either way the result will be obtained). Then `wait for api2` gets the second. The net effect is both HTTP requests were done in overlapping time, rather than sequentially. This is a powerful feature enabled by our async design (and it **“ensures that under the hood these run in parallel”** as envisioned ([docs.md](file://file-BiN8duHSK36KqfPrVJXAhm#:~:text=,We%20could%20also%20wrap%20such))). From the user’s perspective, the code is still simple and does not explicitly mention threading or promises.

**Relation to Try/Catch:** If an awaited operation fails and throws an exception, that exception will propagate out of the `wait for` statement. Users can catch it with `try/when`. For example:
```wfl
try:
    wait for open url at "https://example.com" and read content as webpage
when NetworkError:
    print "Could not fetch webpage."
otherwise:
    print "Some other error occurred."
end
```
If the network request fails, the `wait for` will throw `NetworkError`, which is caught by the `when NetworkError` branch.

In summary, `wait for` is the mechanism that makes asynchronous calls *appear synchronous* in WFL. It aligns with WFL’s philosophy of being beginner-friendly (“wait for the server’s response, then show it” reads like plain English). It avoids explicit callback or promise syntax. The interpreter’s job is to orchestrate these awaits properly. By implementing `wait for` with direct `await` under the hood, we keep things simple and safe, leveraging Rust’s language support to handle waking and resuming the WFL code when the operation completes.

## Parser and Grammar Changes for Async I/O

To support the new asynchronous I/O constructs and the `wait for` syntax, we will extend WFL’s grammar (which is implemented with Pest) and adjust the AST. The goal is to incorporate the new keywords and sentence structures without ambiguity and while keeping the grammar natural-language-oriented.

**New Grammar Rules:**
We outline the additions to the Pest grammar (in pseudo-code form for clarity):

```pest
// Existing basic rules...
WHITESPACE = _{ " " | "\t" | "\n" }

// New Keyword Tokens (we'll treat multi-word keywords as literals in rules)
KEY_OPEN    = ${ "open" }        // might already exist for other uses
KEY_FILE    = ${ "file" }
KEY_URL     = ${ "url" }
KEY_DATABASE= ${ "database" }
KEY_AND     = ${ "and" }
KEY_READ    = ${ "read" }
KEY_CONTENT = ${ "content" }
KEY_WRITE   = ${ "write" }
KEY_WAIT    = ${ "wait for" }    // treat "wait for" as one phrase in grammar
KEY_WITH    = ${ "with" }
KEY_METHOD  = ${ "method" }
KEY_PERFORM = ${ "perform" }
KEY_QUERY   = ${ "query" }
KEY_ON      = ${ "on" }

// Our grammar might have a high-level rule for statements:
statement  = _{ io_statement | wait_statement | <other existing statements> }

// Asynchronous I/O statements:
io_statement = _{ file_stmt | http_stmt | db_stmt }

// File operations grammar:
file_stmt  = ${ KEY_OPEN ~ KEY_FILE ~ "at" ~ string_literal ~ file_action ~ file_optional_as }
file_action = _{ KEY_AND ~ (file_read_action | file_write_action) }
file_read_action = ${ KEY_READ ~ KEY_CONTENT }        // "and read content"
file_write_action = ${ KEY_WRITE ~ expr }             // "and write <expr>" 
file_optional_as = _{ [ "as" ~ identifier ] }         // optional variable assignment

// HTTP operations grammar:
http_stmt = ${ KEY_OPEN ~ KEY_URL ~ "at" ~ string_literal ~ http_option? ~ http_optional_as }
http_option = _{ KEY_WITH ~ KEY_METHOD ~ ident_or_string ~ ( KEY_AND ~ KEY_WRITE ~ expr )? ~ ( KEY_AND ~ KEY_READ ~ KEY_CONTENT )? }
// Explanation: "with method POST and write expr and read content" 
// ident_or_string would allow method name either as an identifier (e.g., GET) or string literal "GET".
http_optional_as = _{ [ "as" ~ identifier ] }

// Database operations grammar:
db_stmt = ${ KEY_OPEN ~ KEY_DATABASE ~ "at" ~ string_literal ~ db_optional_as }
        | ${ KEY_PERFORM ~ KEY_QUERY ~ string_literal ~ KEY_ON ~ (identifier|string_literal) ~ db_optional_as }
db_optional_as = _{ [ "as" ~ identifier ] }
```

*(The above is a rough sketch; the actual Pest grammar would need careful tuning, and multi-word keywords might need to be handled as silent rules or combined tokens.)*

Some points on the grammar:
- We introduced literal phrases like `"wait for"`, `"open file at"`, etc. Pest will try to match them exactly. We need to ensure these don’t conflict with other rules. For instance, if WFL already had an `open` for something else, we refine it in context with `file` or `url`.
- The `wait_statement` would be something like:
  ```pest
  wait_statement = ${ KEY_WAIT ~ ( io_statement_no_wait | identifier ~ optional_as ) }
  ```
  Where `io_statement_no_wait` is similar to `io_statement` but not including the `wait` (to avoid recursion). Actually, we can parse `wait for X as Y` in one go: if after `wait for` comes an `open file/url/database` etc., we parse it as we would an io_statement, including its trailing `as` if any, and just mark it to be awaited. If after `wait for` comes an identifier (a task handle), we allow that and an optional `as` for assigning the result.
- We add `perform query` for database to clearly delineate that it’s an action to run SQL.

**AST Structure Changes:**
We will introduce new AST node variants to represent these operations. Likely our AST is an enum of expressions or statements. For example:
```rust
enum Statement {
    // ... other statements (assignments, etc.)
    Io(IoStatement),            // a general wrapper for I/O statements
    Await(Box<Statement>),      // for "wait for ..." usage on a statement (like an inlined await)
}
enum IoStatement {
    FileRead { path: Expr },
    FileWrite { path: Expr, data: Expr },
    HttpRequest { method: HttpMethod, url: Expr, body: Option<Expr>, capture: Option<Identifier> },
    DbOpen { conn_str: String, alias: Option<Identifier> },
    DbQuery { query: String, conn: ConnHandleExpr, capture: Option<Identifier> }
}
// ConnHandleExpr might be either an identifier (for a var) or a direct string (for an alias name or DSN)
```
And possibly an expression variant if we allow Io operations to appear in expression context.

However, another approach is that we treat these operations as statements that can also yield values. In WFL, assignment like `x is <expr>` might be a separate AST node or just syntactic sugar for a statement with as-clause.

We likely will implement a construct where `open file ... and read content as X` becomes something like:
- AST node: `Io(IoStatement::FileRead{path})` and separately a wrapper that assigns its result to X.
- But since we included the `as` in grammar, we could directly produce an AST node that includes the target variable, or produce a compound AST (one node for action, one for assignment).

To simplify interpreter logic:
- We might parse `... as var` as an assignment statement whose right-hand side is the Io operation. For example, grammar could instead yield:
  ```pest
  file_stmt = ${ KEY_OPEN ~ KEY_FILE ~ "at" ~ string_literal ~ file_action }   // no 'as' here
  assignment_stmt = ${ (io_statement | other_expr) ~ "as" ~ identifier }
  ```
  So the `'as identifier'` part is handled at a higher level, not inside each statement rule. The docs example used `open file ... as var` which looks like a single statement, but we can treat it as an assignment statement where the expression is `open file ...` (which returns a value).
  In any case, the AST has to capture both the action and the variable.

We also need to add support for `wait for`:
- We can have an AST node `Await(expr)` which indicates the need to await on the inner expression’s result.
- If `wait for` wraps a statement that has an assignment (`as var`), semantically it means await the result then assign. We can parse it as either:
  - A special case: `wait for X as Y` yields an AST that is essentially `Y = await X`.
  - Or parse `wait for (something)` as an expression, then rely on our normal assignment parsing after. But “wait for X as Y” is more naturally parsed as one construct.
- Possibly simplest: the grammar could produce a `WaitForStatement(inner_stmt)` or `WaitForExpr(inner_expr)`. If inner_stmt is an assignment, then we handle that in interpreter: execute inner I/O, await it, get result, assign.
- Another perspective: We may implement `wait for` at the interpreter level by translating it: whenever we see it, we mark the inner IoStatement as needing await.

**Examples Integrated:**
- *File:* `"open file at "config.txt" and read content"` would produce something like `IoStatement::FileRead{path="config.txt"}`. If it had `as configText`, we produce a Statement::Assign(name="configText", value= IoStatement::FileRead{...}).
- *HTTP:* `"open url at "http://example.com" and read content as resultData"` might produce an AST for an HTTP GET (method default GET, url expr, capture “resultData”).
- *HTTP with method:* `"open url at "http://api" with method POST and write payload and read content as resp"` yields IoStatement::HttpRequest{ method=POST, url, body=payload_expr, capture="resp" }.
- *DB:* `"open database at "mainDB" as dbConn"` yields IoStatement::DbOpen{ conn_str="mainDB", alias=dbConn }. 
- `"perform query "SELECT * FROM X" on dbConn as result"` yields IoStatement::DbQuery{ query="SELECT * FROM X", conn=dbConn (as an identifier reference), capture=result }.
- *Wait:* `"wait for open file at "data.txt" and read content as fileData"` could be parsed as WaitForStatement( Assign(fileData, IoStatement::FileRead{"data.txt"}) ). The AST can reflect that structure. The interpreter then knows to await that Io operation’s future.

**Modifications to Existing Grammar:** 
We have to ensure these new patterns don’t conflict with existing ones. WFL’s grammar likely didn’t have these specific constructs before. We must mark the new keywords (open, file, url, database, perform, query, etc.) as reserved so they aren’t misinterpreted as identifiers in other contexts. Pest allows us to prioritize certain rules or use atomic patterns to avoid ambiguity.

We should also add these to the **lexer** (if Pest) so that e.g. `wait` and `for` when adjacent are treated correctly. Possibly easier is to treat `wait for` as two separate tokens in sequence in the rule, since `for` might otherwise be a normal word too. But because `wait for` is a fixed phrase we want, making it a single literal might be fine.

**Interpreter Changes for AST:** Once we have these AST nodes:
- The interpreter’s `execute_statement` (or expression evaluator) will get new match arms for `IoStatement` variants and for `Await`:
  - If `Await(inner)` – interpreter will evaluate `inner` which should produce a future or a task handle, then `.await` it to get the result.
  - If `IoStatement::FileRead{path}` – interpreter calls the async file read (returns a future of string). If this node is being executed under an `Await`, then that future will be awaited immediately. If not (meaning it’s being executed without wait), we either automatically await it anyway (making it effectively sync, which we might do if we decide all I/O must be awaited unless explicitly doing concurrency), or we spawn it and return a Task handle.
    - Design: We likely let the `eval_expr` of `FileRead` always return a future (since it's an async fn, returning the result when awaited). If `Await` wraps it, fine. If not, maybe we automatically spawn. Alternatively, we make `eval_expr` for `FileRead` itself perform the await (effectively not allowing it to run without waiting). But then we lose concurrency ability.
    - Better: `eval_expr` for `FileRead` does *not* await inside; it might either return a special Value like `PendingTask` that holds the future. But Rust futures can't be stored easily unless we box them. We could box the future in a trait object and store in Value::Task.
    - So: If `wait for` is present, we do the await in interpreter. If not, we wrap the operation in a spawn:
      - Pseudocode: 
        ```rust
        match node {
          IoStatement::FileRead{path} => {
             let future = tokio::fs::read_to_string(path_val);
             if auto_spawn {
                let handle = tokio::spawn(future);
                return Value::Task(handle);
             } else {
                let result = future.await?;  // only if in an async context that chooses to await now
                return Value::Str(result);
             }
          }
        }
        ```
      - The decision `auto_spawn` or not could depend on whether this call is under a `wait for`. But our interpreter might not easily know that at the node evaluation level. Another approach: the parser could disallow certain contexts. Simpler: we always spawn if not explicitly awaited, and always await if `wait for` given.
      - Implementation trick: If we have `Await(AstNode)`, we handle awaiting explicitly there. If we have the node without await, we spawn. That means `eval_expr` needs to know context or we make two different evaluation functions: one for getting a future vs one for immediate.
      - Instead of complicating eval_expr, we can implement it like:
        - If `Await(node)` encountered: evaluate `node` with a flag that indicates "don’t spawn, just produce future and await it".
        - If an Io node is evaluated normally in an expression context: then by default spawn and return a task.
      - Or simply, if `Await` exists as an AST node, we write interpreter logic specifically for it that doesn’t use the generic eval but calls internal function to get future and await it. For example:
        ```rust
        fn eval_statement(stmt) -> Result<Value> {
            match stmt {
              Statement::Await(inner) => {
                  match inner {
                     Statement::Io(io) => { 
                        // directly await the io operation
                        return eval_io_statement(io).await;
                     }
                     Statement::Assign(var, io) if io is IoStatement => {
                        let val = eval_io_statement(io).await?;
                        env[var] = val.clone();
                        return Ok(val);
                     }
                     Statement::Await(_) => { /* probably not possible nested */ }
                     _ => { /* if someone wrote wait for on a non-async, just eval normally */ }
                  }
              }
              Statement::Io(io) => {
                  // not awaited explicitly, spawn
                  let handle = spawn_io_statement(io);
                  return Ok(Value::Task(handle));
              }
              ...
            }
        }
        ```
      - The helper `eval_io_statement(io).await` would perform the actual future `.await` calls for file, http, db as described in their sections. `spawn_io_statement` would similarly spawn them.
  - The assignment handling with `as var` might either be already in AST or done via an `Assign` node we create, as in code above.

**Integrating with Existing Language Features:**
- If WFL had a concept of functions or actions that could themselves be async, the grammar might allow `async action ...` etc. Not sure if that exists; not needed for initial design, since top-level usage covers a lot. We can add an ability to define an action that does I/O, then call it and use `wait for` on the call if needed, but that’s beyond the immediate scope.
- Ensure that none of the new keywords conflict with variable names or strings. We should treat them as reserved words in the grammar. For example, `open`, `file`, `url`, `database`, `wait`, `perform`, `query` should not be allowed as variable names or should be distinguished by context. Pest typically can handle by ordering rules and not tokenizing those words as identifiers when in these constructs.

**Examples (AST)**:
Consider the WFL snippet:
```wfl
open file at "data.txt" and read content as data
wait for perform query "SELECT * FROM items" on dbConn as items
```
The parser might produce:
- Statement::Assign(name="data", value=Io(FileRead(path="data.txt"))).
- Statement::Await( inner = Statement::Assign(name="items", value=Io(DbQuery(query="SELECT *...", connVar="dbConn"))) ).

The interpreter then:
- For the first line, sees an Assign of an Io. It will spawn the file read if not implicitly awaited. However, since no `wait for`, maybe by default it should spawn? But in this case, if it's top-level and not awaited later, that would mean the script moves on without reading finishing, which probably is not intended here. Perhaps we decide that an `open file ... as data` without wait is implicitly awaited (so essentially synchronous). But that contradicts the idea of requiring `wait for`.
- This is a design decision: We might actually require that any I/O that returns a value must be awaited (by syntax). Perhaps we should enforce `wait for` for read operations, otherwise they return a Task and if you assign that to `data`, `data` is not the file content but a task. That could confuse a beginner. Possibly, yes, we treat it such that if you forget `wait for`, you end up with a task object, which is likely not what you want. The user will realize they need `wait for`.
- So it's okay if in the first line, because they omitted `wait for`, `data` ends up being a Task. If they try to use `data` later expecting content, the interpreter might error ("cannot use Task as string") prompting them to add `wait for`. We can document that.
- Alternatively, we could implicitly wait in such contexts to be forgiving, but that breaks the model of explicit concurrency.
- We lean towards: The presence of `wait for` is the indicator. Without it, it’s asynchronous and you have a handle, which you must explicitly wait on later to get the value.

Thus the AST handling is consistent:
- If a user writes it without `wait for`, we do not auto-wait. We give a Task handle. If they never wait on it, that operation may either complete in background and get dropped (wasted) or still running even after script ends (Tokio may cancel on drop of handle if not awaited, unless we `.detach()` the task).
- To avoid resource leak, we might decide to `spawn` and then detach tasks if their handle isn’t awaited (Tokio’s JoinHandle has a `.detach()` method to let it run to completion without needing join).
- But that means if user doesn’t wait, they can’t get result later. So probably better not to detach by default – if they don’t await and drop handle, Tokio might cancel it. Actually, in Tokio 1.0, dropping a JoinHandle does *not* cancel the task by default (it keeps running in background, unless you specifically use abort). It just means you’re not waiting for it. So detach vs drop, there's subtlety but likely dropping is effectively detaching in behavior (except you lose error propagation).
- We should confirm: In Tokio, dropping JoinHandle *will* drop the task if not awaited? Actually, need to check: In futures it doesn’t necessarily kill it. Actually, I recall that dropping JoinHandle *does not* abort the task; the task keeps running detached. (Yes, by default tasks are not cancelled when handle is dropped, one must call handle.abort() to cancel).
- That means if script doesn’t wait, the task still runs to finish in the background. That could be fine (if e.g. we fired off something intentionally and don’t care).
- But if it was accidental (likely), that’s just wasted work or unexpected behavior. At least it’s safe (not blocking).
- We might not complicate by auto-cancelling; we can let it run or attach a warning.

**Conclusion on Grammar:** We will implement grammar rules for:
- `open file/url/database` constructs.
- `perform query ... on ...` construct.
- `wait for` prefix.

We will ensure these integrate with assignment and expression usage. We will update the AST to handle these, introducing either new Statement variants or expression forms.

These changes allow WFL code to be written in the desired style. The grammar remains clean and avoids symbols. For example, instead of writing `file_open("foo.txt")` or `await fetch("url")`, the user writes **“open file at *foo.txt* and read content”** and **“wait for open url at *...* and read content”** which reads like instructions rather than code.

## Interpreter Modifications for Async Support

To execute the new asynchronous I/O operations and `wait for` semantics, we must modify the WFL interpreter. The interpreter (which is written in Rust) will be refactored to operate asynchronously itself and to handle the new AST nodes.

**1. Making the Interpreter Async:**  
All evaluation functions will become `async` functions:
- The main function that runs a WFL script (say `exec_program`) will be `async fn` so it can await on internal steps.
- Functions like `eval_expr(&Expr) -> Value` change to `async fn eval_expr(&Expr) -> Value` (or perhaps `-> Result<Value>` if using error result for exceptions).
- Similarly, `execute_statement(&Stmt)` becomes `async fn execute_statement(&Stmt)`.
- This change propagates: any place these are called must `.await` them. Essentially, the interpreter now works within the Tokio runtime’s async context.
- We will likely wrap the entire script execution in a `tokio::spawn` or use `block_on` in non-async contexts, but since our `main` uses `#[tokio::main]`, we can simply `.await` the top-level future.

By doing this, the interpreter can use `.await` whenever it encounters an I/O operation. For example, the code handling `IoStatement::FileRead` can call `tokio::fs::read_to_string().await` directly inside the interpreter logic, because `execute_statement` is async. Without making it async, we would have had to block the thread or store futures; making it async avoids that complexity and keeps it safe and straightforward. Rust’s async/await effectively transforms our interpreter into a state machine that can pause at `await` points.

**2. Handling New AST Nodes:**  
We add match arms in the interpreter for each new AST variant:
- **File I/O:** When encountering a file read/write node:
  - If we decide that file operations always produce a Future or value depending on context: The interpreter might have a function `handle_file_io(io_node) -> impl Future<Output=Value>`. For reads, it does the `tokio::fs::read_to_string` call. For writes, `tokio::fs::write`.
  - If not awaiting here (because no `wait for`), we could spawn the future in a task and return a Task handle Value, as discussed.
  - Concretely, we might implement:
    ```rust
    async fn eval_io_statement(io: IoStatement) -> Value {
       match io {
           IoStatement::FileRead{pathExpr} => {
               let path = eval_expr(pathExpr).await?.to_string();  // get path string
               let content = tokio::fs::read_to_string(path).await;
               match content {
                   Ok(text) => Value::Str(text),
                   Err(e) => throw(FileError(e)),  // convert to WFL exception
               }
           }
           IoStatement::FileWrite{pathExpr, dataExpr} => {
               let path = eval_expr(pathExpr).await?.to_string();
               let data = eval_expr(dataExpr).await?.to_string();
               let result = tokio::fs::write(path, data.into_bytes()).await;
               match result {
                   Ok(_) => Value::Nil,  // or a success indicator
                   Err(e) => throw(FileError(e)),
               }
           }
           // ... other match arms for HTTP, DB ...
       }
    }
    ```
    This function awaits the completion. We would call it inside an `eval_statement` if we are in an awaited context.
  - For spawning, we could have a separate helper:
    ```rust
    fn spawn_io_statement(io: IoStatement) -> Value {
       let task_handle = tokio::spawn(async move {
           eval_io_statement(io).await  // capture output or error
       });
       Value::Task(task_handle)  // wrap join handle
    }
    ```
    Now, in `execute_statement`, if we see `Statement::Io(io)` with no wait, we do `spawn_io_statement(io)`. If we see `Statement::Await(inner)`, we do `let val = execute_statement(inner).await` but ensure that inside it doesn’t spawn unnecessarily (we might bypass spawn and directly call `eval_io_statement` for the inner).
- **HTTP I/O:** Similar approach:
  - For awaited scenario: use reqwest to send and await response as described. If an error occurs (like network fail), catch it and throw as WFL exception. For example, if `reqwest::get(url).await` returns Err, throw `NetworkError`.
  - For spawn scenario: spawn an async task that does the above and returns the result Value or exception.
- **Database I/O:** 
  - We will maintain a map (in the interpreter state) for open DB connections. Possibly a HashMap<String, Pool>.
  - On `DbOpen` node: we perform `sqlx::AnyPool::connect(conn_str).await` (using AnyPool since the DSN can indicate which DB; or we can match prefix and call specific connect like PgPool::connect). Store the pool in the map, keyed by either the provided alias or some internal id. If alias (var name) is given, link that var name to the pool.
  - On `DbQuery` node: find the pool by name or var. Then depending on query, call `.fetch_all` or `.execute` as discussed. Await result, transform to WFL value (list of maps or number). 
  - On exceptions: if `.fetch_all` returns Err (like SQL error), throw `DatabaseError` with message. If the connection name is not found in map (programmer error), throw some runtime error.
  - Spawn version: if not waited, we spawn the whole query future similarly, returning a Task handle.
- **Exceptions mechanism:** The interpreter likely has a way to throw exceptions. Perhaps `throw(FileError(e))` in pseudocode above would actually either return a special `Err` up the call stack, or call some exception handling routine. One approach is to make `eval_expr` return `Result<Value, WflError>` and propagate errors via `?`. We might adopt that: all these `await` calls using `?` would naturally propagate errors up to a surrounding `try` block logic.
  - If not already, we should incorporate that. The `try/when` in WFL can be implemented by the interpreter catching a `Result::Err` and matching the error type against the when clauses.
  - So modifying interpreter to be async also means likely using `Result` for error flows. That can be done concurrently.
  - For instance, in the code above `throw(FileError(e))` would likely be implemented as `return Err(WflError::FileError(e.into()))` or similar.

**3. Managing `.wflsec` Permissions:**  
We integrate permission checks in the interpreter:
- **Loading .wflsec:** When a script is loaded, the interpreter will look for a `.wflsec` file in the script’s directory (or a provided path). It will parse it (likely at startup, synchronously or using serde if JSON/TOML).
- The parsed permissions (allowed files, allowed URLs, allowed DBs) are stored in the interpreter (e.g., in a struct `Permissions`).
- For each I/O operation, before executing it, we consult these permissions:
  - **File:** On `IoStatement::FileRead/Write`, after evaluating the path string, run a check function like `permissions.is_file_allowed(path)`. This function will canonicalize the path and see if it matches any allowed path or is inside an allowed directory. If not allowed, the interpreter will not proceed with `tokio::fs` call; instead it will throw a SecurityError (or PermissionError) immediately. If allowed, proceed.
  - **HTTP:** On `IoStatement::HttpRequest`, after evaluating the URL string, use perhaps the `url` crate to parse it, or a simple regex to extract scheme and host. Check that host (and port if specified) is in allowed list (`permissions.is_net_allowed(host, scheme)`). Also enforce that scheme is "https" unless an exception in config. If not allowed, throw SecurityError. If allowed, proceed with reqwest.
  - Additionally, we might ensure that if the scheme is http (not https) and maybe a flag in `.wflsec` says `allow_insecure = false` by default, we throw saying “HTTPS required by policy” (unless they explicitly allowed that host with http).
  - **Database:** On `IoStatement::DbOpen`, check the connection string or alias against allowed DB entries. Possibly `.wflsec` will map an alias to a DSN, and the script might actually use the alias name. If so, `open database at "mainDB"` would be resolved by interpreter: find "mainDB" in .wflsec’s DB list, get actual DSN. If not found or not allowed, throw SecurityError. If found, use that DSN for connecting.
    - If script provides a full DSN in code (which might be discouraged), we then compare it to allowed patterns. Perhaps we allow it only if exactly matches an entry, or we parse out host and database name and see if host is allowed.
    - It's simpler to encourage alias usage. We can implement: if the given connection string appears to be an alias (matches a key in .wflsec), use the corresponding real DSN. If it looks like a raw DSN (like it contains `://` or a file path), then require that exact string to appear in allowed list (or disallow entirely if we want to force using aliases).
  - On `IoStatement::DbQuery`, presumably the connection is already opened and allowed, so no new check needed except maybe if using a raw string for connection name (which we likely wouldn’t).
- **Permission Error Handling:** If a permission check fails, we throw a distinct exception, say `PermissionError` or `SecurityError`, which can be caught separately if needed. It might be useful to differentiate from regular I/O errors (like FileError vs a file access denied by policy – although OS permission denied vs policy denied are both kind of permission issues; we might unify them or separate by message).
- This layer ensures that **code cannot escalate privileges** or access things not intended by the user running it ([Security and permissions](https://docs.deno.com/runtime/fundamentals/security/#:~:text=same%20thread.%20,to%20any%20dynamic%20module%20imports)). We follow the principle of default-deny: if `.wflsec` isn’t present or has no entry for a resource, the code can’t use it ([Security and permissions](https://docs.deno.com/runtime/fundamentals/security/#:~:text=,network%2C%20npm%2C%20JSR%2C%20etc)). This sandboxing in the interpreter is crucial for safety.

**4. Exception Handling Implementation:**  
We touched on using `Result` for error propagation. To integrate with WFL’s `try/when`:
- Likely, `try` blocks in WFL are represented in AST and interpreter as a structure that catches errors. For example:
  ```rust
  enum Statement {
      Try(Box<StatementBlock>, Vec<(ErrorPattern, StatementBlock)>, Option<StatementBlock>)
  }
  ```
  where you have a main block, some when clauses with patterns (like error type matching), and an otherwise block.
- The interpreter executing a Try will do something like:
  ```rust
  if let Err(err) = execute_block(main_block).await {
      if let Some(branch) = find_matching_when(err, when_clauses) {
         execute_block(branch).await?;
         // perhaps if the branch executes normally or returns, we consider it handled: so return Ok or break out.
         return Ok(Value::Nil);
      } else {
         // no matching when, if there's an otherwise:
         if let Some(otherwise_block) = otherwise {
             execute_block(otherwise_block).await?;
             return Ok(Value::Nil);
         } else {
             // rethrow if no handler
             return Err(err);
         }
      }
  }
  ```
  Something like that, to propagate errors appropriately.
- We will integrate our I/O errors into this by defining distinct error types: e.g., `WflError::FileError`, `NetworkError`, `DbError`, `PermissionError`. The `when` clauses likely specify a type or category. If WFL uses names like `FileError` in the script, we need to map that to our error types.
- Possibly, `when FileError` catches any file-related error. We might have a hierarchy or at least categories. We might decide to treat OS file not found as `FileError`, and our policy permission denied as also a `FileError` or a subtype. It could be useful to differentiate (maybe `FileError(kind)` where kind could be `NotFound` vs `AccessDeniedByPolicy` vs `AccessDeniedOs` etc.), but for the language maybe not necessary to expose that granularity unless needed in logic.
- We will include likely minimal error info accessible to script (like message or maybe code). The user can always print or log the exception which we can format accordingly.

**5. Maintaining Backwards Compatibility:**  
If prior WFL code existed without async, these changes shouldn’t break anything except performance improvements. If `.wflsec` wasn’t used before, by adding it we might restrict code that previously had free access. We may choose to run in an *unrestricted mode* if `.wflsec` is absent, for compatibility (but that’s less secure). We could require `.wflsec` explicitly in contexts where scripts are untrusted. For now, we assume new development with `.wflsec` in mind.

**6. Testing and Debugging Support:**  
We will want to be able to test these interpreter changes. For that, we might implement debug logging in the interpreter at points like when an I/O operation starts and ends, printing thread or task IDs to verify concurrency behavior. Also, ensure that if two tasks run concurrently, no mutable state conflict occurs. Our interpreter’s state (variable environment, etc.) is mostly accessed in a single-threaded manner except when tasks running in parallel. If a task tries to set a variable while another is running, but WFL is single-thread at top-level, that scenario doesn’t occur unless explicitly using parallel tasks and then maybe merging results. We likely are safe because each spawned task will encapsulate needed data and then we only assign result once awaited sequentially.

**7. Refactoring Summary:**  
- Convert evaluation functions to async.
- Introduce internal helpers to execute I/O futures and optionally spawn them.
- Introduce error types for different errors (file, network, etc.).
- Add permission checks before performing I/O.
- Ensure `.wflsec` is read and parsed (e.g., using Serde to map JSON/TOML to our `Permissions` struct).
- Possibly update the environment model to store DB pools and Task handles (for tasks, maybe store in a generic `Value::Task(JoinHandle<Value>)`).
- Update the drop behavior: for any leftover tasks at end of script, maybe detach them or let them finish. But ideally, script should await what it needs. We can decide that any still-pending tasks when script ends are simply ignored (they’ll finish in background or be dropped).
- The main runtime will call `interpreter.run()` which is now async, so from `tokio::main` we do `interpreter.run().await`. If an uncaught exception left, we handle it (print error).
- Also, since our interpreter is now async, writing tests for it becomes easier with Tokio's testing or by driving futures.

This async interpreter design ensures that **all blocking operations are contained**. For example, reading a file uses Tokio’s thread pool behind scenes ([tokio::fs - Rust](https://doc.servo.org/tokio/fs/index.html#:~:text=Be%20aware%20that%20most%20operating,run%20them%20in%20the%20background)), but from interpreter’s view it’s just an await. Database and network operations are fully async. The interpreter can still do CPU-bound tasks (like computations in the script) inline, but while waiting for I/O it doesn’t consume CPU. This makes WFL scale better when performing multiple I/O operations or waiting on slow resources.

## Test Plan

To validate the asynchronous I/O features and their safety, we will create a comprehensive test suite. The tests will cover normal (happy path) usage for each I/O type, error conditions, permission enforcement, and interaction with WFL’s error handling. We outline the key tests:

### File I/O Tests
- **Read Existing File (Happy Path):** Create a file `test.txt` with known content. Run a WFL snippet: `wait for open file at "test.txt" and read content as data`. Verify that `data` equals the file content. Also ensure the operation is non-blocking by possibly timing it with a known delay (though that’s more an integration timing test).
- **Write File (Happy Path):** Run WFL: `wait for open file at "output.txt" and write "Hello World!"`. After execution, verify the file `output.txt` exists and contains "Hello World!". Then do `wait for open file at "output.txt" and read content as content` and verify `content == "Hello World!"`.
- **File Not Found (Error):** Attempt to read a non-existent file: `wait for open file at "no_such_file.txt" and read content as data`. The interpreter should throw a `FileError`. Write a WFL `try/when` block to catch it:
  ```wfl
  try:
      wait for open file at "no_such_file.txt" and read content as data
  when FileError:
      store "caught" as flag
  end
  ```
  Verify that `flag == "caught"` afterwards, meaning the error was caught, and ensure that the error message indicates file not found.
- **OS Permission Denied (Error):** If possible, attempt to open a file that the OS user isn’t allowed to (this can be tricky to simulate in tests without special setup, but one way is to create a file and make it read-protected). Expect a `FileError` with permission message.
- **Path Traversal Attempt (Security):** Set `.wflsec` to allow only a specific directory (e.g., `./allowed/`). Create `./allowed/data.txt` and `./notallowed/secret.txt`. In WFL, attempt: `wait for open file at "notallowed/secret.txt" and read content as secret`. Expect a `PermissionError` (or similar) to be thrown because that path is outside the allowed directory. Also test something like `open file at "allowed/../notallowed/secret.txt"` to ensure normalization catches it. The outcome should be the same denial.
- **No .wflsec for Files (Default Policy):** If `.wflsec` is absent or has no file section, try to open a file. If our policy is default-deny, it should error. If default-allow (which is less likely by our design), then it would succeed. We need to decide expected behavior; likely default-deny: thus test that without .wflsec, file access raises an error indicating not permitted.
- **Concurrent File Reads:** (Advanced test) If WFL allows concurrency, test spawning multiple file reads. E.g., have two files with content, run:
  ```wfl
  task1 is open file at "file1.txt" and read content
  task2 is open file at "file2.txt" and read content
  wait for task1 as data1
  wait for task2 as data2
  ```
  Verify that `data1` and `data2` match the respective file contents. This ensures that tasks ran and results can be collected. Also possibly measure that the overall time is roughly max(read1, read2) not sum, to ensure parallelism (though unit tests might not precisely measure time without flakiness).

### HTTP I/O Tests
- **Simple GET (Happy Path):** Use a known URL. For a reliable test, we might use httpbin or a local test server. (We can include a tiny HTTP server in test code to respond). For example, start a local server that returns "Hello" on path `/hello`. In .wflsec allow `http://localhost:PORT`. Then WFL: `wait for open url at "http://localhost:PORT/hello" and read content as resp`. Verify `resp == "Hello"`. This confirms basic GET.
- **HTTPS GET (Happy Path):** If internet access allowed in test, try an HTTPS URL such as `https://httpbin.org/get`. In .wflsec allow `https://httpbin.org`. The response is JSON; verify we got a non-empty string (since content will be JSON text). Or use a secure local server. This test ensures TLS works (reqwest by default will do it, we just ensure our `.wflsec` didn't block it and result is received).
- **HTTP POST (Happy Path):** Set up local server to accept POST and echo something. Or use httpbin’s `/post` endpoint. WFL:
  ```wfl
  wait for open url at "http://localhost:PORT/echo" with method POST and write "TestData" and read content as resp
  ```
  Verify that `resp` contains "TestData" (if the server echoes it in response).
- **Disallowed Domain (Security):** Set .wflsec to allow only `example.com`. In WFL, do `wait for open url at "http://notallowed.com/path" and read content as data`. Expect a `PermissionError` exception immediately. Wrap in try/when to catch and confirm it’s indeed a permission-related error, not a generic network error. (If our interpreter first tries and then fails DNS, that would be wrong; it should block before trying).
- **HTTP -> HTTPS enforcement:** If .wflsec lists only `example.com` without specifying scheme, our policy might require HTTPS by default. So if code does `open url at "http://example.com"` (note http), we expect either a security exception or at least a warning. Test that it indeed fails, and ideally the error message suggests use of https or enabling http in policy. Conversely, if .wflsec explicitly allows `http://example.com`, then the request should be permitted. Write a test toggling that and verifying behavior.
- **HTTP Error Status (404):** Use a known URL that returns 404 (like `http://httpbin.org/status/404`). WFL: 
  ```wfl
  try:
      wait for open url at "http://httpbin.org/status/404" and read content as body
      store body as gotBody
  when HttpError:
      store "http error" as flag
  end
  ```
  We expect that 404 **should not** throw an HttpError in our design (since it's a valid response). So the `try` block would complete normally and put some content (maybe empty) in `gotBody`. Thus `flag` should not be set. And perhaps `gotBody` might contain a short description or empty string (depending on httpbin’s response for 404). We can assert that `flag` is not defined and `gotBody` length is >=0. Conversely, we can test a truly unreachable URL (like `http://nonexistent.domain`) which should throw a NetworkError that can be caught with `when NetworkError`.
- **Timeout and Large Response:** (Optional) For stress, test that a large response (e.g., fetch a large file) works and doesn’t freeze. Also possibly configure a low timeout in .wflsec or global and test a slow server triggers a timeout exception. Those might be advanced and depending on our implementation if we set timeouts.
- **Parallel HTTP requests:** If allowed, similar to file concurrency test. Fire two different URL fetches concurrently and verify both results. Ensure that total time is shorter than sequential sum. This validates that tasks truly run concurrently (reqwest should allow it, and Tokio will multiplex).

### Database Access Tests
(We will need a test database. E.g., use SQLite for simplicity as it requires no external service, or spin up a temporary PostgreSQL if available. SQLite is easiest.)

- **Open SQLite DB (Happy Path):** Prepare a SQLite file `test.db` with a table and data (or use `:memory:`). .wflsec allow that path or alias. WFL:
  ```wfl
  open database at "sqlite://test.db" as myDB
  wait for perform query "SELECT 1+1 as two" on myDB as result
  ```
  Verify that `result` is a list with one row and that row’s `two` column is 2. This tests connecting and a simple query.
- **Query with Results (Happy Path):** In the test DB, create a table `users(id INT, name TEXT)`, insert some data. Then WFL:
  ```wfl
  wait for perform query "SELECT id, name FROM users WHERE id = 1" on myDB as user1
  ```
  Verify `user1 == [{id: 1, name: "Alice"}]` if that's the data.
  Also test retrieving multiple rows:
  ```wfl
  wait for perform query "SELECT * FROM users" on myDB as allUsers
  ```
  Verify `allUsers` is a list of maps of all rows.
- **Insert/Update Query:** WFL:
  ```wfl
  wait for perform query "INSERT INTO users(name) VALUES('Zoe')" on myDB as res
  ```
  Verify that `res` is an integer equal to 1 (one row inserted). Then do a select to confirm the row was inserted. Similarly for an update.
- **SQL Error Handling:** Try an invalid query:
  ```wfl
  try:
      wait for perform query "SELECT * FROM non_existing_table" on myDB as data
  when DatabaseError:
      store "caught" as flag
  end
  ```
  Verify that `flag == "caught"`, indicating the error was caught. Also ensure the error didn’t propagate. Optionally check that `data` was not set.
- **Disallowed DB (Security):** .wflsec defines only a certain DB alias. If WFL tries to open some other DB (e.g., a connection string not listed), it should throw a permission exception. Test: .wflsec allows only "mainDB". In code do `open database at "otherDB" as db2`. Expect security error. Similarly if code tries a raw connection string not in allow list.
- **SQL Injection Concern:** This is more of a code review item than runtime test since we aren’t implementing dynamic user input here, but we can simulate:
  - Construct a query using string concatenation in WFL if possible (like if WFL has string operations). If WFL supports something like:
    ```wfl
    store "Alice" as name
    wait for perform query ("SELECT * FROM users WHERE name = '" + name + "'") on myDB as results
    ```
    Then verify it returns correct results and did not break. (For injection, one would need malicious input).
  - It’s difficult to test injection automatically; we rely on design (like if one passes `name = "Alice'; DROP TABLE users; --"`, what happens? Likely it would just be part of string and if run, it could drop table if allowed. Without a parameterization feature, the language can’t stop that except by user caution. We might not have an automated test for that beyond verifying that our allowed queries concept doesn’t filter content of query (which it doesn’t).
  - We could include a test where an attempt at injection is made and see that the DB operations indeed execute it if allowed (meaning the language didn’t sanitize, as expected). That just underscores the need for user caution or future improvements.

- **Parallel Queries:** If supported, open two different DB connections or same connection (pool allows concurrency) and perform two queries concurrently. For example:
  ```wfl
  task1 is perform query "SELECT * FROM users WHERE id <= 50" on myDB
  task2 is perform query "SELECT * FROM users WHERE id > 50" on myDB
  wait for task1 as part1
  wait for task2 as part2
  ```
  Verify combined results equal the whole table. And that it runs faster than doing sequentially (if measuring is possible).
  This also tests that our use of connection pool allows concurrent queries (SQLx’s pool will handle scheduling queries sequentially if single connection or use multiple connections if configured).

### `.wflsec` Permissions Tests
- **File Permissions Format:** Provide a `.wflsec` (JSON/TOML) with a file allow list, e.g.:
  ```toml
  [files]
  allow = ["./data/"]
  ```
  Attempt to open a file inside `./data/` (should succeed) and outside (should fail). Verify accordingly.
- **Network Permissions Format:** `.wflsec` with:
  ```toml
  [network]
  allow = ["example.com", "api.example.com:443"]
  ```
  Test that `example.com` is accessible, and another domain is not. If port is specified, ensure it respects it (port 443 for https).
- **Database Permissions Format:** `.wflsec` with:
  ```toml
  [databases]
  main = "sqlite://test.db"
  ```
  In WFL, do `open database at "main" as db`, it should map to that DSN. If WFL provides DSN directly, e.g. `"sqlite://other.db"` which isn’t in config, ensure it fails.

- **Missing .wflsec (Security):** Start interpreter without any .wflsec for an untrusted scenario. Try any I/O (file, net, db) and verify they are all blocked. This ensures the default is secure (if that’s our choice).
- **Malformed .wflsec:** Provide an incorrectly formatted JSON/TOML and ensure the interpreter either refuses to run script (with a clear error about malformed config) or falls back to safe defaults (denying access). We should test that scenario and expected outcome (likely an error at start).

### Exception Handling Tests
- **Catching Specific Exceptions:** Already covered in above tests (FileError, NetworkError, DatabaseError in try/when).
- **Unhandled Exceptions Propagation:** Write a script that triggers an exception without a try, and ensure the interpreter surfaces it as an error (for example, reading a non-existent file should cause the program to terminate with an error). In a test harness, that might mean catching a Result from `run()` and checking it’s Err of the right type.
- **Multiple `when` clauses:** If WFL supports multiple when, test a scenario:
  ```wfl
  try:
      wait for open file at "maybe.txt" and read content as data
  when FileError:
      store "file problem" as msg
  when PermissionError:
      store "permission problem" as msg
  otherwise:
      store "other problem" as msg
  end
  ```
  Try it with a scenario that triggers each branch: (a) file not exist (FileError), (b) not allowed by .wflsec (PermissionError), (c) maybe cause a different error if any (or skip to otherwise by making no error). Verify the correct branch executes and sets `msg`.
- **Ensure Normal Execution Continues:** After handling an error in a try block, ensure subsequent code runs. For example:
  ```wfl
  try:
      wait for perform query "SELECT * FROM badtable" on db as out
  when DatabaseError:
      // handle
  end
  store "done" as status
  ```
  Verify that `status` gets set to "done", meaning the script continued after the try block gracefully.

### Concurrency and Order Tests
- We should test that launching multiple tasks without waiting and then waiting in different order yields correct results for each. This ensures tasks are properly identified:
  ```wfl
  t1 is open file at "a.txt" and read content
  t2 is open file at "b.txt" and read content
  wait for t2 as bContent
  wait for t1 as aContent
  ```
  Verify `aContent` corresponds to A’s file, `bContent` to B’s file even though waited out of order (this implies our tasks are independent and results not confused).
- Also test that re-using a task after waiting yields either an error or some defined behavior. Ideally, once a task is awaited, it’s consumed (like awaiting a future). If WFL user tries `wait for t1` twice, the second time might throw an error that the task is already completed or invalid. We should define that. Possibly disallow second await (could store a flag in Task handle after first await). We can test for now that doing so either yields no second result or an error.

### Performance/Stress (if feasible)
- Read a large file (tens of MB) and ensure it completes and does not block other async tasks. Hard to automate assert on "non-blocking", but we can at least measure that doing two large file reads concurrently is faster than sequential, implying they indeed overlapped on multiple threads thanks to Tokio.
- Many concurrent operations: e.g., spawn 10 file reads and waits, to ensure our interpreter and runtime handle multiple tasks correctly.

Each of these tests will be run in an environment where we control `.wflsec` and the resources. We will use both unit tests for smaller pieces (like permission check function canonicalization) and integration tests running the interpreter on sample scripts.

By executing this test plan, we will verify:
- Correct functionality of asynchronous I/O (getting expected results).
- Proper error raising and catching for exceptional conditions.
- Security enforcement through `.wflsec` in all relevant scenarios.
- The interpreter’s stability when dealing with concurrent tasks and multiple awaits.

## Security Considerations

Designing asynchronous I/O for WFL requires a strong focus on security, as it introduces the ability to interact with the file system, network, and databases. We address the following key security concerns:

- **Path Traversal Attacks:** When accessing files, it’s crucial to prevent malicious or accidental access to files outside permitted directories. Our solution is to **canonicalize file paths and enforce allowed prefixes**. By resolving `..` and symlinks, then checking that the resulting path starts with an allowed directory, we mitigate directory traversal ([RUSTSEC-2021-0126: rust-embed: RustEmbed generated `get` method allows for directory traversal when reading files from disk › RustSec Advisory Database](https://rustsec.org/advisories/RUSTSEC-2021-0126.html#:~:text=The%20flaw%20was%20corrected%20by,with%20the%20canonicalized%20folder%20path)). For example, if only `/app/data` is allowed, any path that doesn’t begin with `/app/data` after normalization will be rejected. This prevents inputs like `"/app/data/../secret.txt"` from escaping the allowed area. We also recommend running the WFL interpreter with minimal OS permissions (e.g., as a user that only has access to intended directories) as a defense in depth.

- **SQL Injection:** WFL’s natural syntax might tempt users to construct SQL queries via string concatenation, which can lead to SQL injection if those strings include untrusted data. While the ultimate responsibility lies with the script author, our design encourages safer practices. We support and recommend **prepared statements with parameter binding** in queries. SQLx inherently supports binding parameters (e.g., using `.bind()` on a query) which is critical for preventing injection ([Raw SQL in Rust with SQLx | Shuttle](https://www.shuttle.dev/blog/2023/10/04/sql-in-rust#:~:text=By%20default%2C%20SQLx%20promotes%20using,find%20more%20about%20this%20here)). In future iterations, we plan to allow WFL syntax for parameterized queries (so that user inputs are passed separately from the query string). For now, if a WFL script must include external data in a query, we advise using placeholders and binding via an API function (or at least carefully sanitizing inputs). Additionally, the `.wflsec` file can restrict which queries or tables are accessible (in a coarse way, e.g., by restricting which database the script can connect to), thereby limiting the damage potential. All database queries run with the privileges of the provided connection credentials, so using least-privilege database users (e.g., no DROP privileges if not needed) is recommended to minimize impact even if an injection were to occur.

- **HTTPS Enforcement for Network Calls:** To protect data in transit and avoid man-in-the-middle attacks, **all external HTTP requests should use TLS (HTTPS)** ([Best practices for REST API security: Authentication and authorization - Stack Overflow](https://stackoverflow.blog/2021/10/06/best-practices-for-authentication-and-authorization-for-rest-apis/#:~:text=Always%20use%20TLS)). Our `.wflsec` format and runtime enforcement reflect this: by default, we do not allow plain `http://` URLs unless explicitly specified. This means that if a script tries to access a URL with an insecure scheme, it will be blocked or require an override in `.wflsec` (which should be granted only for trusted internal networks or testing scenarios). By requiring HTTPS, we ensure that API tokens, user data, and other sensitive information fetched or sent by WFL are encrypted on the wire. Reqwest also verifies TLS certificates by default, so WFL will not fall prey to trivial TLS-stripping or invalid certs unless the user consciously disables verification (which we do not expose at the language level by default). This aligns with industry best practices: *“Every web API should use TLS… Without TLS, a third party could intercept and read sensitive information in transit, undermining any authentication measures.”* ([Best practices for REST API security: Authentication and authorization - Stack Overflow](https://stackoverflow.blog/2021/10/06/best-practices-for-authentication-and-authorization-for-rest-apis/#:~:text=Always%20use%20TLS)).

- **.wflsec Permission Checking:** The introduction of the `.wflsec` security file is a cornerstone of our security model. It implements a **sandbox** for WFL scripts. By default (in secure mode), code has *no access* to files, network, or databases unless permitted in `.wflsec` – following the principle of least privilege (similar to how Deno denies all I/O by default ([Security and permissions](https://docs.deno.com/runtime/fundamentals/security/#:~:text=,network%2C%20npm%2C%20JSR%2C%20etc))). This means even if an attacker is able to execute arbitrary WFL code, they cannot read arbitrary server files or call random URLs unless the host explicitly allowed those actions. The `.wflsec` file itself should be kept out of reach of the script (e.g., placed in a directory the script can’t write to) to prevent tampering. Our format is designed to be clear and human-auditable (JSON or TOML). For example, an admin could specify:
  ```toml
  [files]
  allow = ["./public_data/"]

  [network]
  allow = ["api.payments.com", "payments.internal.local"]

  [databases]
  analytics = "postgres://readonly:pass@dbserver/analytics"
  ```
  This would confine file access to the `public_data` directory, allow network calls only to the specified API hosts (perhaps the application’s payment service, enforcing even specific domains), and provide a read-only database connection for analytics queries. The interpreter will check each operation against these rules and **deny anything not explicitly allowed**. This approach ensures a compromised or errant script can’t, for instance, send requests to an attacker’s server or exfiltrate data from sensitive files.

- **Credential Handling:** With asynchronous I/O, WFL might interface with credentials (API keys, DB passwords). We ensure that such secrets are not hard-coded in the script but rather provided via `.wflsec` or environment variables (which WFL can be extended to read securely). For instance, database DSNs in `.wflsec` keep passwords out of the code. We also ensure that when printing or logging exceptions, we do not accidentally expose sensitive info (e.g., if a DB connection fails, we might sanitize the DSN in the error message to not show the password).

- **Resource Cleanup and Limits:** As a safety measure, we consider what happens if scripts create many tasks or hold resources:
  - File handles are closed promptly after use (because we either use the high-level read/write which closes internally, or we drop the File object after done).
  - HTTP connections are managed by reqwest’s Client, which reuses and closes as needed. We might set limits on the number of parallel connections via reqwest’s config if necessary.
  - Database connections in pools will be closed when the interpreter exits or the pool is dropped. We should ensure to drop (or `.close().await`) the pool at script end to not leave connections open. Also, to prevent abuse, we could cap the pool size (e.g., max 5 connections unless configured) to avoid DoS by exhausting DB connections.
  - We might implement a limit on the number of parallel tasks (to avoid a user spawning thousands of tasks that overwhelm the system). This could be configured in `.wflsec` (e.g., `max_tasks = 100`).
  - Tokio’s runtime also has safeguards and the ability to tune thread pools. For file threadpool, it uses a default global blocking pool with an upper limit (512 threads by default). In pathological cases (mass file ops), that could be an issue, but it’s unlikely in typical scripts. If needed, we can adjust or instruct users to be mindful of unbounded concurrency.

- **Denial of Service & Timeouts:** A malicious script could attempt long-running operations or infinite loops. While that’s beyond I/O specifically, the async design helps in that at least the runtime remains responsive to other tasks. We could integrate a watchdog for overall script execution time or set timeouts for I/O operations via config (for example, `.wflsec` could allow specifying a max duration for network calls). For now, we rely on external oversight (or wrapping the interpreter call in a timeout at the host application level if needed).

- **Safe Defaults and Explicit Escalation:** Summarizing the approach: by default, WFL runs with no external access, and the `.wflsec` file (or runtime flags) must explicitly grant permissions, similar to how Deno’s flags work ([Security and permissions](https://docs.deno.com/runtime/fundamentals/security/#:~:text=To%20enable%20these%20operations%2C%20the,command)) ([Security and permissions](https://docs.deno.com/runtime/fundamentals/security/#:~:text=Users%20can%20also%20explicitly%20disallow,deny%20flag%20will%20take%20precedence)). Code cannot escalate privileges at runtime – it can only use what was given. Even within a script, if it tries to import or run another WFL code, it will still be under the same sandbox unless explicitly loosened (future work might consider module-specific permissions, but that’s beyond scope). 

- **Auditability:** Using a text-based policy file means security reviewers or system administrators can easily see what a script is allowed to do, without reading the script’s code logic. This separation of policy and code is a security win, as it prevents privilege creeping into the code and allows central management of permissions.

In conclusion, the asynchronous I/O features are designed with multiple layers of security:
1. The **language runtime checks** (backed by `.wflsec`) ensure the script doesn’t go beyond intended boundaries.
2. We leverage **secure defaults** (deny by default, TLS by default, etc.) so that common cases are safe out of the box.
3. We incorporate **best practices** from industry (like using prepared statements to mitigate SQL injection and enforcing TLS for network comms).
4. We encourage running WFL in a constrained environment and using least-privilege accounts for any external systems it touches, minimizing impact if something does go wrong.

By addressing path traversal, injection, encryption, and explicit permissions, we aim to make WFL’s new async capabilities as safe as possible for both script authors and the systems they interact with.