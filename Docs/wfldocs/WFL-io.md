Great. I’ll design a unified I/O specification for WFL that supports file handling, networking, and database access with consistent natural-language syntax. This specification will also include:

- Support for both synchronous and asynchronous I/O operations.
- Batch operations for all I/O types (e.g., reading multiple files, sending parallel HTTP requests, querying multiple databases).
- Mocking and dependency declaration for test environments.
- Shared abstractions for resource handling (like a `connection` or `resource` object).
- Uniform error handling and retry mechanisms across all I/O types.

I’ll format the result as a technical proposal with examples and pseudocode that align with WFL’s natural-language approach. I’ll let you know when it’s ready for review.

# Unified I/O Specification for WebFirst Language (WFL)

## Introduction and Goals 
WebFirst Language (WFL) aims to simplify web programming with a **unified, natural-language I/O syntax**. This specification defines a single, consistent way to handle file systems, network requests, and database queries. The design follows WFL’s guiding principles of **minimal special characters**, **high readability**, and **clarity** ([wfl-foundation.md](file://file-A3Q4Kynjr6TMEwh12ZuqBY#:~:text=Description%3A%20Embrace%20a%20syntax%20that,like%20constructs)). In practice, this means that whether you're reading a local file, calling a web API, or querying a database, the code will look and read in a similar, English-like way. Key goals include:

- **Consistency:** Use the same sentence-like syntax for all I/O operations (open, read, write, stream, query, close) across files, network, and databases. 
- **Clarity:** I/O code should read like instructions (e.g. *“open file at **X** and read content”*), avoiding cryptic symbols or jargon.
- **Flexibility:** Support both synchronous and asynchronous operations without changing the fundamental syntax.
- **Robustness:** Provide natural error handling (using `try/when/otherwise` blocks) and easy retry logic for any I/O errors.
- **Testability:** Allow declaration of external I/O dependencies (files, URLs, databases) and offer simple **mocking syntax** so that I/O can be simulated or redirected in test environments.

By meeting these goals, WFL’s unified I/O model ensures that developers can learn one intuitive pattern and apply it to any form of input/output. The following sections detail the syntax, features, and examples for file, network, and database I/O, including batch operations and testing scenarios.

## Unified I/O Syntax and Abstractions 
All I/O in WFL revolves around a **common syntax structure**. The language treats files, network connections, and databases as similar “resources” that you can open, use, and close. Instead of distinct APIs for each, WFL uses a shared set of natural-language commands. A resource (sometimes called a **connection** or **handle**) is an object representing an open file, an HTTP connection/response, or a database session. Developers work with these resources using English-like verbs:

- **Open** – Acquire a resource for use (e.g. open a file, open a URL, open a database).  
- **Read** – Get data from the resource (e.g. read file contents, read an HTTP response, read database query results).  
- **Write** – Send or put data to the resource (e.g. write text to a file, send data in an HTTP request, write data or commands to a database).  
- **Stream** – Continuously read/write from the resource in chunks or as events (useful for large files, live network streams, or large query results).  
- **Close** – Release the resource when done (close file, close network connection, close database connection).  
- **Query** – (Database-specific) Send a query to a database and retrieve results, using the same sentence structure as other operations.

Each of these operations is expressed in WFL using a **minimal-symbol, natural syntax**. For example, an open operation looks like `open <resource type> at "<location>" as <name>`. The word “at” introduces the location (file path, URL, or connection string) and `as <name>` gives it a variable name for later use. Similarly, `read ... from <name>` and `write ... to <name>` are used for all resource types. The idea is that if you know how to read from a file, you can read from a web response or a database result in almost the same way.

**Common I/O Syntax Patterns:**  
- **Open a resource:** `open [file|url|database] at "<location>" as <resourceName>`  
  *Examples:* `open file at "docs/report.txt" as reportFile`, `open url at "https://api.example.com/data" as apiResponse`, `open database at "sqlite3://mydata.db" as myDB`.  
- **Close a resource:** `close <resourceName>` – Closes any open resource, regardless of type.  
  *Example:* `close reportFile`. (Closing a resource ensures the file handle or network connection is freed.)  
- **Read from a resource:** `store <varName> as read [content|response|results] from <resourceName>` – Reads data from the resource into a variable. The keyword after `read` can be adapted to context (“content” for file, “response” for web, “results” for database) for clarity, though the structure is consistent.  
  *Examples:* `store textData as read content from reportFile`, `store apiData as read response from apiResponse`. In the case of databases (after sending a query), you might use `store rows as read results from myDB`.  
- **Write to a resource:** `write <data> to <resourceName>` – Writes data to the resource. What “data” means depends on the target: writing to a file means writing text/binary data, writing to a URL means sending an HTTP request body, and writing to a database means sending a command or data (like an SQL statement or record).  
  *Examples:* `write "Log entry\n" to reportFile` (appends a line to a file), `write requestBody to apiResponse` (send data in an HTTP request), or `write newUserRecord to myDB` (insert a record, if `newUserRecord` is a structured data object). In many cases, specialized verbs or actions (like **query**, or HTTP **fetch**) are provided as shortcuts instead of a raw `write` – these are discussed later.  
- **Stream from a resource:** `stream from <resourceName> ...` – Initiates streaming mode to handle data in chunks or as they arrive. The exact syntax for streaming may involve a loop or callback-style block, but it remains wordy and clear.  
  *Example pattern:* 
  ```wfl
  repeat until end of file:
      read chunk from reportFile as part
      perform process chunk with part   // hypothetical action to handle the chunk
  end repeat
  ``` 
  This loop continually reads from the file until EOF, processing each chunk. Similarly, one could stream from a network resource (reading a response in parts as it comes in) or iterate through large query results without loading all at once. (We will see a concrete streaming example in the File I/O section.)

- **Perform a query (database or web):** `store <varName> as perform query "<SQL or query string>" on <resourceName>` – A convenient form to send a query command to a database (or even to a web resource for web queries) and capture the result in one step.  
  *Example:* `store userList as perform query "SELECT * FROM Users" on myDB`. This sends the SQL to `myDB` (which was opened as a database resource) and stores the returned result set into `userList`. Under the hood, this is equivalent to writing the query to the database and then reading the results, but WFL lets you do it in one fluid sentence. (For web APIs, you might not use `query` but rather a dedicated HTTP request action like `fetch` – see Network I/O.)

**Synchronous vs. Asynchronous:** WFL uses the same syntax structure for both blocking (synchronous) and non-blocking (asynchronous) I/O. By default, an `open`/`read`/`write` sequence in an action will run in order (waiting for each step to finish). However, WFL actions can be declared `async` and use the `await` keyword to wait for operations without freezing other tasks ([wfl-vars.md](file://file-EZsTqg3EhLzRxRW4Mbj7si#:~:text=Asynchronous%20Actions)) ([wfl-vars.md](file://file-EZsTqg3EhLzRxRW4Mbj7si#:~:text=When%20calling%20an%20async%20action%2C,if%20you%20need%20the%20result)). Importantly, **the grammar of the I/O operations doesn’t change** – you still write `open`, `read`, `write` the same way. The only difference is you might prepend `await` in front of an operation to indicate the program should pause for it in an async context, or launch multiple operations concurrently and wait for them later (we will demonstrate this in the Batch Operations section). This uniform approach means you don’t have to learn two sets of I/O commands; whether an operation is sync or async depends on the context (if you’re inside an `async action` or using explicit `await`). For example, in an async context you could do: 

```wfl
// Inside an async action or environment
store userData as await read content from reportFile
```

This looks almost the same as the synchronous version, but here `await` ensures other tasks can run while the file is being read. Similarly, `await perform query "SELECT..." on myDB` would asynchronously retrieve database results. If you omit `await` in an async context, the operation is kicked off and the code continues (allowing parallelism), and you can `await` its result later.

## File I/O: Unified Syntax and Examples 
File handling in WFL uses the unified syntax to make working with files straightforward and readable. You can open files, read or write data, stream large files, and close them, all with English-like commands. By default, WFL’s file operations avoid technical symbols like file mode flags or binary markers; instead, they use optional phrases (e.g. *“for writing”*, *“for reading”*) when needed, and sensible defaults.

**Opening and Closing Files:** To work with a file, you first **open** it. This establishes a file resource. WFL might infer the mode (read/write) from context or allow an explicit hint. For example: 

```wfl
open file at "logs/app.log" as logFile
```

This opens the file at the given path and names the handle `logFile`. By default, if the file exists, it could be opened for reading and appending; if it doesn’t exist, WFL will throw a `file not found` error (which you can handle in a `try` block) or, if intended for writing, create it (depending on context or an option). You can also specify the mode explicitly in natural terms:

```wfl
open file at "data/output.txt" for writing as outFile
```

The phrase **“for writing”** clearly indicates we intend to write (and if the file doesn’t exist, it will be created). Other variations could be **“for reading”** (to ensure it’s only read), or **“for append”** (to add to an existing file’s end), etc., all expressed as words. When finished, use `close outFile` to close it. Closing releases the file resource, analogous to ending a with-block in other languages; forgetting to close could be caught by WFL’s runtime, but it’s good practice to close explicitly or use a `finally` clause for it.

**Reading and Writing Files:** Once a file is open, you can **read** from or **write** to it using consistent verbs: 

- `read content from <fileHandle>` – reads the entire content (or next chunk) from the file. If used without qualifiers, WFL will typically read the whole file content into a text variable. You can store the result by prefixing the command with `store ... as`. For example: 

  ```wfl
  store logText as read content from logFile
  display "File has " with length of logText with " characters."
  ```
  Here, `logText` will contain all text from *logs/app.log*. If the file is huge, you might prefer to stream it in parts instead of reading all at once (discussed below).

- `write <data> to <fileHandle>` – writes data to the file at the current position. The data can be text or binary (WFL will handle encoding behind the scenes, possibly by you indicating the data type or by context). For example:
  
  ```wfl
  write "User signed in at " with current time with "\n" to logFile
  ```
  This appends a line to the log file. Notice we can concatenate strings using `with` (WFL’s way to join text without `+` ([wfl-vars.md](file://file-EZsTqg3EhLzRxRW4Mbj7si#:~:text=number.%20,allowed%2C%20but%20WFL%20prefers%20words))). After this operation, the file pointer moves to after the written text. If you need to write at a specific location or overwrite, you might have options like `go to start of file` or `open file for overwrite`, but those are optional details not changing the fundamental syntax.

**Example – Reading from and Writing to a File:**  
Below is a simple example combining these operations. It reads a configuration file and writes a new entry to a log file:

```wfl
try:
    open file at "config.txt" for reading as configFile
    store configData as read content from configFile
    close configFile

    open file at "logs/app.log" as logFile  // open for append by default
    write "Loaded config with length " with length of configData with "\n" to logFile
    close logFile

when file not found:
    // If config.txt is missing, create it with defaults and retry reading
    display "Config file not found. Creating a default config."
    create file at "config.txt" with "defaultSettings=true\n"
    retry  // try opening and reading config.txt again

otherwise:
    display "Failed to read config: " with error message
end try
```

**Explanation:** In this example, we use a `try` block to attempt opening and reading `"config.txt"`. If it’s missing, the `when file not found` clause runs – we handle it by using a high-level `create file` command (another natural phrasing in WFL) to make a new file with some default content, then `retry` the `try` block to attempt the open/read again. The `retry` keyword causes the code in the `try` to run again from the top, which is a clear way to attempt the operation a second time after fixing the issue (in this case by creating the file). This shows how **error handling** and file operations integrate seamlessly: the syntax remains readable (`when file not found:` is much like saying "when the file isn't there, then..."). After successfully reading the config, we open a log file and write a message. All resources are closed properly (even if an error occurred, the `retry` ensures we only proceed when things are resolved; we could also use a `finally` to close files if needed). The `otherwise` clause catches any other errors (like permission issues) and prints a friendly message along with the `error message` (WFL provides an `error message` variable in error handlers, containing a human-readable description of the error). 

**Streaming File Data:** For very large files or continuous data, WFL supports streaming using loops or special constructs, rather than loading everything at once. The syntax still uses `read ... from file` but typically in a loop until an end-of-file condition is met. For instance:

```wfl
open file at "large_data.dat" for reading as bigFile
repeat until end of file:
    store chunk as read next 1024 bytes from bigFile
    perform process data with chunk  // hypothetical action to handle the chunk
end repeat
close bigFile
```

In this pseudo-example, `read next 1024 bytes from bigFile` reads a fixed-size chunk (1024 bytes) on each iteration. We use a `repeat until end of file` loop, which is a natural way of saying "keep reading until there's no more data." WFL might allow variants like `read next line from bigFile` or `read next record from bigFile as entry` to handle text files line by line or structured data record by record. The key point is that **streaming uses the same verbs** (`read from ...`) with slight phrasing adjustments (“next 1024 bytes” or “next line”) and relies on loops to process data incrementally. This pattern could similarly apply to network streams (reading data from a socket until closed) or databases (iterating through a large result set row by row).

**File I/O in Tests (Mocking Files):** When writing tests, you might not want to use real files on disk. WFL’s design allows you to **mock file resources** easily. One approach is to declare file dependencies (see the section on Dependencies) and override them, or simply open a different file path in a test context. For example, suppose our program normally does `open file at "config.txt"`. In a test, we can simulate this by preparing a small temp file or using an in-memory file if WFL supports it. Using WFL’s mocking syntax, it could look like:

```wfl
// Declare that in a testing scenario, use an alternate file
when testing:
    use file "tests/config_test.txt" as "config.txt"
end when
```

In plain language, this means: *“when running in test mode, treat any operation on `"config.txt"` as operating on `"tests/config_test.txt"` instead.”* This way, the rest of the code doesn’t change – it still opens "config.txt" – but actually reads from a controlled test file. Alternatively, one could use a fake file object by implementing the same interface (if WFL allows interface-based injection for files), but the direct substitution approach is simple and clear. We will discuss a more structured way to declare and override dependencies in a later section, but this example shows the spirit of file mocking: **keep the syntax identical, just point it to a safe test resource.**

## Network I/O: Unified Syntax and Examples 
Networking in WFL (such as making HTTP requests or reading from sockets) follows the same philosophy: open something (like a URL or socket), read/write data, optionally stream, and close – all with natural, consistent syntax. Web operations often have a request/response pattern, so WFL provides a straightforward way to express that.

**Opening a Network Connection or URL:** To fetch data from the web or call an API, you typically specify a URL. In WFL you can simply do:

```wfl
open url at "https://api.example.com/data" as apiResponse
```

This single line might *initiate an HTTP GET request* to that URL and set up `apiResponse` as the resource holding the response. The act of “opening” a URL can be understood as “connecting to the server and requesting the resource”. By default, WFL would perform a GET request if no method is specified. (Under the hood it might use something like an HTTP client, but WFL abstracts that away.) If you want to specify a different HTTP method or headers, WFL can allow that through a natural parameter block:

```wfl
open url at "https://api.example.com/data" as apiResponse with:
    method as POST
    body as "{\"query\": \"status\"}"
    header "Content-Type" as "application/json"
end with
```

Here, we’ve extended the `open url` command with a **with-block** to include additional details ([wfl-vars.md](file://file-EZsTqg3EhLzRxRW4Mbj7si#:~:text=%2F%2F%20Not%20in%20cache%2C%20load,store%20load%20item%20with%20id)). We indicated an HTTP POST, provided a JSON body, and set a header, all without using symbols like `{}` or `;` (except the JSON string itself which uses braces as data). This keeps things readable: it’s clear we are opening a URL with a POST request including a body and a header. The result is still that `apiResponse` is a resource representing the pending or completed response from the server.

**Reading Responses:** Once a URL is “opened” (meaning the request is sent), we need to get the response data. We use the unified `read` syntax for this:

```wfl
store resultData as read response from apiResponse
```

This will wait for the HTTP response and store the response body (e.g., text or JSON) into `resultData`. If the response includes status codes or headers that we care about, WFL might provide ways to access them as properties of `apiResponse` (for example, `apiResponse.status` or a specialized read command like `read status from apiResponse`). But at the base level, **reading the response body** is done with the same `read ... from ...` pattern.

For one-off HTTP GET calls, WFL might offer an even higher-level shorthand: a built-in action like `fetch`. For instance:

```wfl
store resultData as perform fetch from url "https://api.example.com/data"
```

This single line performs the common sequence of opening a GET connection, reading the response, and closing it, returning the data. It uses `perform ... from url "..."` in a natural way (here `fetch` is the verb instead of manually writing open/read/close). This is functionally similar to the explicit open+read example above, but more convenient. Both approaches are valid and consistent – one is just more abbreviated. In terms of syntax structure, `perform fetch from url ...` still reads like an English command and fits the WFL style (no weird characters, just words and quotes). 

**Writing/Sending Data:** If you need to send data (for example, an HTTP POST/PUT or sending data over a raw socket), you can use `write ... to <connection>` just as with files. In the earlier `open url ... with method POST`, we included `body as ...` which essentially handles writing the body. If you were using a lower-level socket, you might do:

```wfl
open url at "ws://example.com/socket" as chatSocket  // e.g., open a WebSocket
write "Hello world" to chatSocket
```

This would send a message over a WebSocket connection. After that you could `read response from chatSocket` or `stream from chatSocket` if it’s a continuous connection. The syntax doesn’t change because it’s network – you still `write ... to ...` and `read ... from ...`. The differences (like HTTP vs WebSocket vs raw TCP) are handled by WFL under the same umbrella of “network resource”.

**Closing Connections:** Use `close <resourceName>` for network resources just as you do for files. In HTTP `fetch` scenario, the connection is usually short-lived and closed automatically after reading the response. But for persistent connections (like sockets or if reusing an HTTP keep-alive connection), you should call `close apiResponse` or `close chatSocket` when done. Closing network resources uses the same keyword and is just as important to free resources or end communication politely.

**Example – Fetching an API and Handling Errors:**  
```wfl
define action get user profile:
    needs:
        user id as text
    gives back:
        profile data as text
    do:
        try:
            // Attempt to fetch user profile from remote API
            open url at "https://api.example.com/users/" with user id as apiResponse
            store profileJson as read response from apiResponse
            close apiResponse
            give back profileJson

        when network timeout:
            // If the request timed out, maybe retry once
            display "Timeout fetching user, retrying..."
            retry

        when http error:
            // Handle HTTP errors (non-200 status)
            display "Server returned error " with apiResponse.status
            give back "{}"  // return empty JSON as fallback

        otherwise:
            display "Unexpected network error: " with error message
            give back "{}"
        end try
end action
```

**Explanation:** In this action (which could be async, but here we didn't mark it `async` for simplicity), we build a URL for a user profile by concatenating the base URL with the `user id` (WFL allows string concatenation with *with*, or we could have used a template). We then open that URL, get the response, and return the profile JSON. The `try` block around it catches network-related errors. We used `when network timeout:` to specifically handle a timeout scenario – maybe the server is slow. In that case, we log (display) a message and `retry` the whole block once more. We also included `when http error:` as a placeholder for any HTTP status code indicating failure (WFL could categorize any 4xx/5xx response as an `http error`, and perhaps provide the status code and message via the `apiResponse` or an `error message`). In that handler, we print the status and return an empty JSON object as a safe fallback. The `otherwise` catches anything else (like no internet connection, DNS failure, etc.) with a generic error message. This example highlights that **error handling for network I/O in WFL is declarative and readable** – we describe error conditions with words like “timeout” or “http error” instead of exception classes, and use `retry` just as we did for files. The syntax inside try/when is identical to file error handling, proving the consistency of the approach.

**Parallel Network Requests:** One of the powerful features of WFL’s unified I/O is the ease of doing batch operations. For instance, if you need to fetch multiple URLs in parallel, you can take advantage of WFL’s async capabilities *without changing the fundamental syntax*. Suppose we want to fetch two APIs concurrently:

```wfl
// Launch two requests without waiting (assuming this code is in an async context or action)
store request1 as perform fetch from url "https://api.example.com/data1"
store request2 as perform fetch from url "https://api.example.com/data2"

// ... possibly do other work here while requests are in flight ...

// Now await both results
store result1 as await request1
store result2 as await request2

display "Got results of length " with length of result1 with " and " with length of result2
```

In this snippet, `perform fetch from url` starts the HTTP GET requests. We don’t use `await` immediately, so the action continues running (the requests happen in the background). We then separately `await request1` and `await request2` to get their responses. The syntax is still **perform ... from url** and `await` (which is a natural way to say “wait for it” in English). We did not need to introduce new keywords like Promise, thread, or callback; everything reads as straightforward steps. WFL ensures that under the hood these run in parallel. This style can be used for **batch HTTP calls** easily. We could also wrap such logic in a `try` block to handle if one request fails while others succeed (with appropriate error conditions for each – perhaps identifying which request failed by its resource name or error content).

**Mocking Network I/O in Tests:** Just like with files, you often want to avoid real network calls in a test environment. WFL allows you to **mock network endpoints** or provide stub responses in a natural way. If your code calls `open url at "https://api.example.com/users/123"`, in a test you might not want to hit that URL. You have a few options:
- **Override the URL** to point to a local test server or file. For example: `when testing: use url "https://api.example.com" as "http://localhost:8080/test-api" end when`. This would redirect all calls to the example API to your local test server (which can be programmed to return predictable data).
- **Simulate the response directly.** WFL could let you declare something like:
  ```wfl
  mock url "https://api.example.com/users/123" gives back "{ \"name\": \"Test User\" }"
  ```
  This would mean whenever that exact URL is requested in test mode, WFL will not perform a real network call but instead immediately provide the given JSON string as the response body (with perhaps a default 200 OK status). You might also specify `status as 200` or other metadata if needed. This kind of syntax (`mock url ... gives back ...`) is very readable – it states the intention clearly (we are mocking this web call with a prepared answer).

No matter which method, the idea is to keep the interface the same: your main code still does `open url ... read response ...`, but in testing, the environment is set up such that no real HTTP traffic occurs. The consistent syntax and the `mock`/`use` constructs ensure that your code is testable without modifications, staying true to dependency injection principles but with a much more **declarative, English-like feel**.

## Database I/O: Unified Syntax and Examples 
Database access is another important I/O category that WFL supports out-of-the-box. The language is designed to work with SQLite3 by default (no external drivers needed) and optionally with more powerful systems like PostgreSQL (with perhaps an additional library or configuration). The unified I/O design means interacting with a database looks similar to file or network interactions: you open a connection, you perform queries (which is akin to writing commands and reading results), and you close the connection. The key difference is the inclusion of a **`query`** operation, which is a specialized form of read/write for databases.

**Opening Database Connections:** To start using a database, you use `open database` with a connection string or path. For SQLite, which is file-based, the connection string can simply be the file path (with a prefix to indicate SQLite). For example:

```wfl
open database at "sqlite3://./my_app.db" as myDB
```

This opens (or creates, if not exists) a SQLite3 database stored in the file `my_app.db` in the current directory, and names the connection handle `myDB`. The connection string `"sqlite3://./my_app.db"` is a URL-like notation indicating the type (sqlite3) and path. WFL could allow a shorthand like `open database at "my_app.db" as myDB` and assume SQLite by default (since it’s built in). For PostgreSQL or other DBs, a typical connection string might include credentials and host, e.g., `"postgres://user:pass@localhost:5432/mydb"`. If WFL has the Postgres driver available, you could do:

```wfl
open database at "postgres://user:password@localhost:5432/testdb" as pgDB
```

This would attempt to connect to that Postgres database and assign the handle `pgDB`. Regardless of the database type, the syntax is the same – `open database at "<connection_string>" as <name>` – making it easy to swap databases by changing the string, not the code structure.

**Performing Queries:** Once the database is open, you will want to run queries (SELECTs, INSERTs, etc.). WFL provides a couple of ways to do this, staying consistent with our other I/O patterns:

- Use the generic `write ... to ...` and `read ... from ...` semantics. For instance, one could conceive:
  ```wfl
  write "INSERT INTO users (name,email) VALUES ('Alice','alice@example.com');" to myDB
  ```
  which sends an SQL command to the database, and 
  ```wfl
  write "SELECT id, name FROM users;" to myDB
  store results as read results from myDB
  ```
  which sends a query and then reads the returned rows into `results`. This method makes sense in terms of consistency (we treat the database like a stream you write commands to and read responses from), but writing raw SQL as a string is still necessary. WFL avoids special characters, but SQL by nature contains symbols (`*,;()` etc.). That’s acceptable because the SQL is just a string literal from WFL’s perspective (WFL isn’t redefining SQL syntax, just carrying it as data). The key is WFL surrounds it in a very clear `write ... to myDB` context.

- Use the higher-level **`perform query ... on ...`** syntax. This is both clearer in intent and less verbose. For example:
  ```wfl
  store userList as perform query "SELECT * FROM users WHERE active = 1;" on myDB
  ```
  This line does everything: it sends the query to `myDB` and retrieves the results (perhaps as a list of records or a table) into `userList`. It’s effectively sugar for the two-step write/read above. WFL can determine that this query is a SELECT and thus knows to fetch results. If the query was an UPDATE or INSERT (which doesn’t produce a result set), `perform query` could still be used; WFL would then return a success indicator or number of affected rows. For example:
  ```wfl
  store rowsAdded as perform query "INSERT INTO logs(message) VALUES('Test');" on myDB
  ```
  might set `rowsAdded` to 1 if the insert succeeded. In keeping with natural language, WFL might even allow an alias like `perform update ... on myDB` or `perform insert ...` but that’s not strictly necessary – the word "query" can encompass any SQL command.

- Use containerized actions for common operations. If WFL has an interface or container representing a database, there might be predefined actions like `myDB find users where ...` or `myDB save record ...` which internally use the above mechanisms. However, that would be built on top of this base specification. So for now, we focus on the explicit query commands.

**Reading Query Results:** If you didn’t use the one-liner `perform query ... on ... as ...`, and instead did a `write` followed by a `read`, you would use `read results from myDB` to get the data. The data returned could be a list of records, which WFL might represent as a list of maps or objects, accessible in natural syntax (for example you could then do `for each user in userList:` and treat `user.name` and `user.email` naturally). The exact structure of result is beyond the scope of I/O syntax, but the act of **reading the results** uses the same syntax as reading a file or HTTP response. It’s just that in this case, WFL knows the content is structured (rows and columns). 

**Example – Database Query Sequence:**  
```wfl
open database at "sqlite3://users.db" as userDB

// Write (execute) a command to ensure a table exists
perform query "CREATE TABLE IF NOT EXISTS Users (id INTEGER PRIMARY KEY, name TEXT, email TEXT);" on userDB

// Insert a new user
store inserted as perform query "INSERT INTO Users(name,email) VALUES('Bob','bob@example.com');" on userDB
display "Inserted rows: " with inserted  // 'inserted' might be number of rows added (1)

// Query the table for all users
store allUsers as perform query "SELECT id, name, email FROM Users;" on userDB

// Use the results (allUsers might be a list of records)
for each user in allUsers:
    display "User " with user.id with ": " with user.name with " <" with user.email with ">"
end for

close userDB
```

**Explanation:** We open a SQLite database file `users.db` as `userDB`. We then perform a create-table query. Next, we perform an insert query and store the result in `inserted` – likely the number of affected rows or perhaps the new row ID (this could be defined by WFL; for simplicity we assume number of rows affected). We print that out. Then we perform a select query to get all users, storing the result in `allUsers`. We iterate through the list using a `for each` loop (WFL’s loop syntax, which is also English-like) and display each user’s info. Finally, we close the database. All of these operations use **no special characters except quotes and commas inside the SQL strings**. The WFL code itself is just words like `open database ...`, `perform query ...`, `for each ...`, etc., demonstrating the goal of being **highly readable and clear**. Even someone not fluent in SQL can roughly understand what the code is doing, because the surrounding WFL syntax explains the intent (creating a table, inserting, selecting, iterating).

**Error Handling for Database I/O:** Database operations can raise errors too (e.g., connection failures, SQL syntax errors, constraint violations). WFL handles these through the same `try/when/otherwise` mechanism. For example:

```wfl
try:
    perform query "INSERT INTO Users(name,email) VALUES('Alice','alice@example.com');" on userDB
when database locked:
    // Maybe the database is locked by another process – handle it
    wait 1 second
    retry
when constraint violation:
    display "User already exists or invalid data."
otherwise:
    display "Database error: " with error message
end try
```

In this snippet, we attempt an insert. If the database is locked (SQLite can throw this if two writes happen concurrently), we wait a second and `retry` the operation. If there’s a constraint issue (perhaps a UNIQUE constraint preventing duplicate entries), we handle that separately. The conditions `database locked` and `constraint violation` are expressed in plain terms, which WFL would map to specific internal error codes. The `otherwise` catches anything else (like a syntax error in the SQL or a connection drop). This is exactly the same style as file and network errors, proving the consistency of error handling across all I/O in WFL.

**Batch Operations and Transactions:** The unified approach also extends to doing multiple database operations at once or in a transaction. For instance, WFL might allow grouping queries:

```wfl
perform query """
    BEGIN TRANSACTION;
    INSERT INTO Users(name,email) VALUES('Carol','carol@example.com');
    INSERT INTO Users(name,email) VALUES('Dave','dave@example.com');
    COMMIT;
""" on userDB
```

By sending a multi-line query (notice we used triple quotes `"""` to span multiple lines in this hypothetical example), we execute a batch of inserts in one go. The syntax is still one `perform query ... on userDB`. If any part fails, the entire thing would fail (and if not caught, would throw an error that could be handled with `when`). Alternatively, WFL could offer a more structured way to start a transaction and ensure commit/rollback in a `try/finally`, but that’s more about semantics than syntax. The important part is **consistent syntax**: even a batch of commands is just given as a larger string to the same `query` mechanism.

**Mocking Database I/O in Tests:** Testing database code often involves using an in-memory database or a dummy database so you don’t affect real data. WFL’s dependency declaration and mocking features make this easy:
- If your code uses `open database at "sqlite3://users.db"`, you can override that connection string in tests to use an in-memory SQLite database (which WFL could represent with a special URI like `"sqlite3://:memory:"` or simply `"sqlite3::memory:"`). For example:
  ```wfl
  when testing:
      use database "sqlite3::memory:" as "sqlite3://users.db"
  end when
  ```
  This means any attempt to open the `users.db` file will actually get a fresh in-memory database (which is fast and isolated).
- Alternatively, provide a completely mocked database interface. For example, WFL interfaces could allow a `DummyDB` container that implements the same actions as a real database but just returns preset values. This is more advanced, but since WFL supports interface polymorphism ([wfl-vars.md](file://file-EZsTqg3EhLzRxRW4Mbj7si#:~:text=For%20instance%2C%20you%20might%20have,implement%20it%20in%20different%20containers)) ([wfl-vars.md](file://file-EZsTqg3EhLzRxRW4Mbj7si#:~:text=,store%20base%20path%20as%20text)), one could imagine passing a `DummyDB` to code that expects a Data Store. In fact, the earlier snippet from WFL docs defines a `Data Store` interface and could have implementations like `File Store` or `Memory Store`. In tests, you could use a `Memory Store` that just stores data in a variable. But without going that far, the straightforward way is to use SQLite’s in-memory mode or a test database URL.
- If using Postgres in production, for tests you might use a local test database or a container. You could set the connection string via an environment variable and have WFL pick it up, or explicitly override it with `mock database at "<prod string>" with "<test string>"`.

The key in all these cases is that **the syntax in the main code remains unchanged**. The tests or environment configuration handles the redirection. This separation ensures that your business logic is expressed clearly (open/read/write queries) and isn’t cluttered with conditional logic for tests. It also makes the codebase easier to read because the I/O intentions are declared in one place, and any test-specific details are kept in a designated section or configuration.

## Declaring I/O Dependencies and Test Overrides 
WFL encourages declaring external I/O dependencies explicitly, so it’s clear what outside world interactions a piece of code will perform. This not only improves readability but also makes it easier to substitute those interactions during testing (mocking). The language provides a **declaration block** for I/O requirements and a corresponding **mocking override** syntax.

**Dependency Declarations:** At the top of a WFL program (or within a container/module), you can list required files, URLs, or databases that the code relies on. For example:

```wfl
requires:
    file "config.txt" as configPath
    url "https://api.example.com" as apiEndpoint
    database "sqlite3://users.db" as mainDB
end requires
```

This declares that the program expects a file at `"config.txt"` (perhaps a configuration file path), an API endpoint base URL, and a main database connection string. By giving them names (`configPath`, `apiEndpoint`, `mainDB`), we create aliases that can be used in the rest of the code when opening resources. Think of these as configuration constants that are I/O-related. In the code, instead of hardcoding `"config.txt"`, you could do:

```wfl
open file at configPath as configFile  // uses the path from requires
```

and instead of `"https://api.example.com"`, use `apiEndpoint` when constructing full URLs or making requests. This indirection has two benefits: clarity (up front we see what external resources are used), and testability (we can change these without editing the core logic).

**Mocking and Overrides for Testing:** When running the program in a different environment (like a unit test, or a staging setup), WFL allows you to override the `requires` definitions with alternative values. The syntax for this is designed to be simple and obvious. For instance, you might have a special block or file that runs in testing:

```wfl
test scenario:
    configPath becomes "tests/config_test.txt"
    apiEndpoint becomes "http://localhost:3000/api_mock"
    mainDB becomes "sqlite3::memory:"
end scenario
```

This **test scenario** block redefines the three dependencies for testing. We point `configPath` to a test config file, `apiEndpoint` to a local mock server (perhaps your test harness runs a dummy server on port 3000), and `mainDB` to an in-memory SQLite database (so tests start with a fresh database each time). The use of the word **“becomes”** (or one could use `=` or the phrase **“use ... instead of ...”**) keeps it in natural language. It reads almost like instructions: "In the test scenario, configPath becomes this, apiEndpoint becomes that...". 

Alternatively, WFL might support a shorthand `mock` keyword:

```wfl
mock configPath with "tests/config_test.txt"
mock apiEndpoint with "http://localhost:3000/api_mock"
mock mainDB with "sqlite3::memory:"
```

Either way, it’s clear that we’re substituting those values. Under the hood, when an `open` or `perform fetch` or `perform query` uses one of these dependency names, the language will actually use the overridden value if in test mode. 

This approach to mocking means you **don’t have to modify your actual I/O code to make it testable**. You declare what you will use, and then tests can provide fake or alternate endpoints easily. It also centralizes test configuration in one place. For example, if the API endpoint changes for testing, you change it in one mock line, rather than searching through code.

**Mocking Behavior vs. Just Endpoints:** The above focuses on swapping out file paths, URLs, or DB locations. In some cases, you might want to simulate specific behaviors (like raising an error to test error handling, or returning specific data). WFL’s design could include advanced mocking where you specify a behavior for a resource. For example:

```wfl
// Simulate that any query on mainDB returns a preset result (for testing)
mock perform query on mainDB:
    if query contains "SELECT * FROM Users":
        give back [{ id: 1, name: "Test User", email: "test@example.com" }]
    otherwise:
        give back 0
end mock
```

This hypothetical syntax is more complex, but it illustrates that one could intercept the `perform query` calls and provide fake responses without a real database. Similarly, for a URL, you could specify: 

```wfl
mock fetch from apiEndpoint with:
    result as "{ \"status\": \"ok\", \"data\": [] }"
    status code as 200
end mock
```

which means whenever a `fetch` is performed on `apiEndpoint` (regardless of path or maybe specific path matching could be added), it will immediately return an HTTP 200 response with the provided JSON body. 

Such detailed mocking might be part of a testing library or framework on top of WFL, but the language’s core supports the needed hooks by identifying I/O via those resource names.

**Isolation of Test Code:** Notably, WFL keeps test overrides separate from production code. Using a `when testing:` or `test scenario:` block ensures that test-specific instructions do not accidentally run in a production environment. WFL could determine the mode via a compiler flag or environment variable. In any case, the presence of these blocks does not affect the main program logic except when specifically enabled. This design maintains the **clarity** of the main code (no scattered `if test then ...` conditions throughout), adhering to best practices of separating configuration from logic.

## Batch Operations and Parallel I/O 
Finally, a unified I/O system in WFL enables batch operations – doing many I/O tasks together – with minimal fuss. We’ve touched on parallel network requests and multi-statement database queries; here we summarize how batch and asynchronous operations are handled uniformly:

- **Multiple Files:** Suppose you want to read several files at once (e.g., load a set of configuration files). You can open all of them first and then read from each, or use a loop. WFL’s loop syntax (e.g., `for each fileName in fileList:`) lets you iterate naturally. If you need parallel reads (perhaps to speed up I/O-bound operations), you could start each file read in an async action. For example: 
  ```wfl
  define async action read files:
      needs:
          file paths as list of text
      gives back:
          contents as list of text
      do:
          store tasks as empty list
          for each path in file paths:
              // start reading each file asynchronously
              open file at path as fileHandle
              add perform async read content from fileHandle to tasks
          end for
          // Now wait for all reads to finish and collect results
          store contents as empty list
          for each task in tasks:
              add await task to contents
          end for
          give back contents
  end action
  ```
  While a bit advanced, this shows that we can use the same `open file`, `read content` commands inside an async action to read multiple files in parallel. We even treat the `read content from fileHandle` as an asynchronous operation by prefacing it with `perform async` (which could be implicit if inside an async action). The results are awaited and collected. The syntax is consistent – nowhere did we introduce a new method for “multiple” vs “single” read; we just leveraged loops and async, which themselves use English-like syntax.

- **Multiple Network Calls:** We already saw how to fire off multiple `fetch` operations and await them. WFL could further simplify common cases of parallel HTTP requests with a specialized syntax, but it may not be necessary. The combination of `perform ... from url` and `await` is sufficient and clear. For example, to fetch a list of URLs:
  ```wfl
  for each endpoint in apiEndpoints:
      add perform fetch from url endpoint to tasks
  end for
  // later...
  for each task in tasks:
      add await task to results
  end for
  ```
  This uses the same ideas as the file example. Additionally, WFL might allow a block like:
  ```wfl
  do in parallel:
      store A as perform fetch from url urlA
      store B as perform fetch from url urlB
  end do
  display "Fetched A and B"
  ```
  where the `do in parallel` block automatically runs its contents concurrently and waits for all to complete. This is hypothetical, but it would align with WFL’s goal of sounding like natural instructions ("do these in parallel, then continue"). Whether through explicit syntax or just good use of async/await, **batch network I/O doesn’t require new APIs**, just the same ones used multiple times.

- **Multiple Database Queries:** In some cases, you might want to query two databases (or two different queries on the same DB) concurrently, perhaps to improve throughput or when the results are independent. You can use the same trick: start both queries then await both results. Since WFL’s `perform query` returns a result (or needs an await in async), you can do:
  ```wfl
  store q1 as perform query "SELECT * FROM Orders;" on mainDB
  store q2 as perform query "SELECT * FROM Users;" on mainDB
  // ... other work ...
  store orders as await q1
  store users as await q2
  ```
  If `mainDB` can handle concurrent queries (SQLite cannot handle parallel queries on the same connection by default, but Postgres can, or you could open two connections), WFL allows it. If not, WFL might queue them – but that’s an implementation detail. The code still reads clearly: we performed two queries and then awaited their results. For two different databases, say `mainDB` and `analyticsDB`, it’s the same idea with handles `q1 on mainDB`, `q2 on analyticsDB`. No new syntax needed.  

In all these batch scenarios, an underlying theme is that **asynchronous execution is controlled with the same natural syntax**. We schedule multiple operations in flight and then wait for them. The words `await`, `perform async`, and even a possible `parallel` block all fit within WFL’s no-symbols, English-like style (e.g., `await` is a real word, not a symbol; `perform async action X` reads normally). This uniform approach demystifies concurrency for newcomers: it feels like describing tasks ("do this, and that, then when both done do next") rather than dealing with threads or callbacks.

## Conclusion 
The unified I/O specification for WFL brings together file handling, networking, and database access under one consistent syntactical roof. By using natural language constructs and avoiding special-case operators, WFL makes I/O code intuitive:

- **Minimal symbols, maximal clarity:** Opening a file, sending an HTTP request, or querying a database all involve phrases like *open*, *read*, *write*, *close* that anyone can read. There’s little to no code noise (brackets, ampersands, etc.) obscuring the intent.
- **One mental model for all I/O:** Developers learn one pattern (open/use/close with try/when for errors) and reuse it. This reduces cognitive load and errors, and it aligns with how we might describe the operations in plain English.
- **Robust error handling and retries:** The try/when/otherwise blocks, with readable error condition names and a `retry` mechanism, mean that handling an error from a file or a failed web request is straightforward. The code for error handling remains as legible as the happy path, making it easier to write resilient programs.
- **Asynchronous friendly:** Without introducing different functions or callback styles, the same I/O syntax can be used for asynchronous operations. Keywords like `async` and `await` (or structured parallel blocks) integrate smoothly, preserving readability while enabling modern concurrent programming.
- **Testing and maintenance:** Declaring I/O dependencies and using mocking syntax ensures that code is flexible and testable. It cleanly separates the *what* (e.g., "read config file") from the *where* (e.g., "which config file path in this environment"). This leads to code that is easier to maintain and adapt – e.g., changing a database from SQLite to PostgreSQL only requires changing the connection string in one place, since the usage syntax is the same.

All examples and patterns above follow WFL’s documentation tone – clear, tutorial-like, but precise. By adhering to these unified I/O conventions, WFL programs can handle complex interactions (from local filesystem to remote servers to databases) in a way that feels coherent and accessible. The result is a highly readable codebase where I/O logic is self-explanatory, reducing the gap between a program’s implementation and the way a developer would describe its behavior in plain language.