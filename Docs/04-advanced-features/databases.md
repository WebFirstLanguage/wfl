# Databases

WFL has built-in database support powered by connection pooling. Three
backends are supported out of the box:

| Backend | Connection URL schemes | Placeholders |
|---------|------------------------|--------------|
| SQLite | `sqlite://path/to/file.db`, `sqlite::memory:` | `?` |
| PostgreSQL | `postgres://...`, `postgresql://...` | `$1`, `$2`, ... |
| MariaDB / MySQL | `mariadb://...`, `mysql://...` | `?` |

## Opening a Connection

Use `open database` (or the equivalent `connect to database`) with a
connection URL, and give the connection a name with `as`:

```wfl
open database at "sqlite://app.db" as db
```

```wfl
connect to database at "postgres://user:password@localhost:5432/mydb" as db
```

Notes:

- SQLite files are created automatically if they do not exist.
- File-backed SQLite uses WAL (`-wal` / `-shm` sidecar files) and a
  five-second busy and acquire wait. A contended file fails with a database
  error instead of waiting long enough to look like a hung program. Closing a
  SQLite pool is bounded to the same five seconds.
- `sqlite::memory:` opens a temporary in-memory database that disappears when
  the connection closes. In-memory pools stay at one connection and do not
  create WAL files.
- `mariadb://` URLs are accepted as an alias for `mysql://` — MariaDB speaks
  the MySQL protocol.
- Connection failures (bad URL, unreachable server, wrong credentials) raise
  catchable errors.

## Querying Rows

`query` runs a statement that returns rows (typically `SELECT`). The result is
a list of objects, one per row, keyed by column name:

```wfl
open database at "sqlite://app.db" as db
store users as query db with "SELECT id, name, age FROM users"

for each user in users:
    display user["name"] with " is " with user["age"]
end for

close database db
```

## Parameterized Queries

Pass values with `and parameters [...]` — never by splicing them into the SQL
text. Parameters are bound by the database driver, so a bound value can never
change the meaning of the SQL statement. Binding protects values only: never
build the SQL text itself — including table or column names — from untrusted
input:

```wfl
store min_age as 21
store adults as query db with "SELECT * FROM users WHERE age > ?" and parameters [min_age]
```

Multiple parameters are separated with `and` (or commas) and bind in order:

```wfl
store found_users as query db with "SELECT * FROM users WHERE age > ? AND name = ?" and parameters [21 and "Alice"]
```

On PostgreSQL, use numbered placeholders instead:

```wfl
store adults as query db with "SELECT * FROM users WHERE age > $1" and parameters [min_age]
```

## Executing Statements

`execute` runs statements that do not return rows — `INSERT`, `UPDATE`,
`DELETE`, and DDL like `CREATE TABLE`. The result is an object with
`affected_rows` and `last_insert_id`:

```wfl
store created as execute db with "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)"
store inserted as execute db with "INSERT INTO users (name, age) VALUES (?, ?)" and parameters ["Alice" and 30]

display "Inserted " with inserted["affected_rows"] with " row(s)"
display "New id: " with inserted["last_insert_id"]
```

Backend notes for `last_insert_id`:

- **SQLite**: the last inserted rowid.
- **MariaDB/MySQL**: the last `AUTO_INCREMENT` id.
- **PostgreSQL**: always `nothing` — use a `RETURNING` clause with `query`
  instead:

```wfl
store rows as query db with "INSERT INTO users (name) VALUES ($1) RETURNING id" and parameters ["Carol"]
store row as rows[0]
store new_id as row["id"]
```

## Transactions

Some changes only make sense together. Moving money between two accounts is two
`UPDATE`s, and stopping halfway is worse than never starting. A transaction
block makes a group of statements all-or-nothing:

```wfl
in transaction on db:
    store debited as execute db with "UPDATE accounts SET balance = balance - 100 WHERE id = 1"
    store credited as execute db with "UPDATE accounts SET balance = balance + 100 WHERE id = 2"
end transaction
```

As everywhere else, `execute` is written as `store <name> as execute ...` — the
result object is always bound, even when you do not need it.

If both statements succeed, the block commits and the changes become permanent
when it reaches `end transaction`. If anything inside fails, everything the
block did is undone — including the statements that had already succeeded — and
the error is reported as usual, so you can catch it:

```wfl
try:
    in transaction on db:
        store claimed as execute db with "INSERT INTO jobs (slug, status) VALUES (?, 'running')" and parameters [slug]
        store counted as execute db with "UPDATE counters SET running = running + 1"
    end transaction
    display "Job claimed."
when error:
    display "Could not claim the job; nothing was changed."
end try
```

### What the block guarantees

- **One connection for the whole block.** `open database` maintains a pool of
  connections, and outside a transaction each statement takes whichever one is
  free. Inside the block, every statement runs on the same connection — that is
  what makes the group atomic. File-backed SQLite pools wait at most five
  seconds to acquire a connection or a write lock.
- **Reads see the block's own writes.** A `query` inside the block sees rows the
  block has inserted but not yet committed. Other connections do not see them
  until the block commits.
- **`break`, `continue` and `return` commit.** They are ordinary ways to leave a
  block, so the work inside finished and is kept.
- **Errors and `exit` roll back.** A failed statement rolls the block back, and
  so does `exit`, which stops the program where it stands rather than finishing
  the block. A transaction still open when the program ends rolls back for the
  same reason — an abrupt stop discards the partial work rather than
  half-committing it.

### Restrictions

**Transaction blocks cannot be nested on the same database.** Starting a second
block on a database that already has one open is an error rather than a silent
flattening of one into the other. Nested transactions require savepoints, which
WFL does not currently expose.

**A database cannot be closed inside its own transaction.** `close database`
during an open block is an error; let the block finish first.

### Do not send BEGIN or COMMIT through `query` or `execute`

Writing transaction control as SQL does not work, and WFL now says so. Both
statements are checked, not just `execute`:

```wfl
store t as execute db with "BEGIN"     // Error, with a pointer to the block syntax
store r as query db with "COMMIT"      // Same error — `query` is checked too
```

The reason is the connection pool. `BEGIN`, the statements after it, and
`COMMIT` would each take a different pooled connection, so the transaction would
not cover the statements it appeared to wrap — a `ROLLBACK` would quietly undo
nothing while the writes it was meant to discard survived. `BEGIN`, `COMMIT`,
`ROLLBACK`, `START TRANSACTION`, `SAVEPOINT` and `RELEASE` are therefore
rejected with an error naming the block syntax above.

Only the first real token of a statement is checked, so ordinary SQL that merely
contains those words — a column named `begin_at`, a value of `'rollback plan'` —
runs normally. Leading comments are skipped before that token is read, so
`-- set up\nBEGIN` is refused rather than slipping past.

> **If this used to work for you, it worked by accident.** An in-memory SQLite
> database (`sqlite::memory:`) only ever has one connection, so hand-written
> transaction control did take effect there — and nowhere else. The same program
> pointed at a file-backed or networked database silently lost the writes it
> meant to roll back. That is why this is now an error everywhere rather than a
> warning: the pattern's failure mode was to pass in development and corrupt data
> in production. Replace it with the block above.

### Transactions and concurrent handlers

Under `main loop concurrently:` a transaction belongs to the handler that opened
it. Two requests can hold their own transactions on the same database handle at
the same time, and a handler that has no transaction of its own keeps taking a
pooled connection as usual — it is never enrolled in someone else's transaction,
and cannot have its writes committed or rolled back by another request.

### Application validation failures

Use `call raise_error with "Saving the record failed: title must be text. Supply a title."`
when an application rule fails. A raised error unwinds actions and `finally`
blocks, rolls back the transaction, and reaches the caller's `when error`.
This is the same error path used by database constraint failures. An ordinary
`return no` is a successful return and still commits; returning a failure flag
does not abort a transaction. Catch an error outside the transaction when the
whole operation must roll back. Catching it inside the block handles the error,
so the block may continue and commit.

Use an operation name and corrective action in the message. When attaching
context to an existing error, `raise_error` can accept the safe context joined
with `error_message`; the result is an ordinary application error containing
that diagnostic text. It does not preserve a caught error's specialized kind.
Do not include passwords, tokens or confidential parameter values.

The executable [application-error suite](../../TestPrograms/application_errors_test.wfl)
demonstrates direct errors, library calls, contextual diagnostics and rollback.

### SQLite schema changes

For SQLite rebuilds, extend the same transaction form to
`in transaction on db for schema changes:` and close it with `end transaction`.
This mode owns one connection before setup, temporarily disables foreign-key
enforcement, and begins an immediate write transaction. It checks all foreign
keys before committing; any violation rolls back the schema, data and ledger
writes together. Enforcement is restored before the connection returns to the
pool. Failed or cancelled cleanup discards the connection instead of pooling
it with enforcement disabled. In-memory databases retain their connection on
normal success and handled failure.

Schema transaction acquisition, including waiting for another writer, is
bounded to five seconds. A timeout reports that the transaction body has not
run and asks the caller to finish the competing operation before retrying.
There are no automatic retries. Two migration runners must read their ledger
and apply their pending work **inside** this scope so the write lock serializes
those decisions. The bound applies to acquisition, not to the duration of the
migration body. Set the program's execution budget for a total runtime limit.

Setting `PRAGMA foreign_keys = OFF` after an ordinary transaction has begun
does not disable enforcement. Deferred foreign-key checking does not defer
`ON DELETE CASCADE` actions. Use the schema transaction mode for a
create/copy/drop/rename rebuild so extension-owned referencing rows survive.
The [schema-transaction suite](../../TestPrograms/database_schema_transaction_test.wfl)
contains that rebuild and rejected relationships. The
[recovery](../../TestPrograms/database_schema_recovery_test.wfl),
[compatibility](../../TestPrograms/database_schema_compatibility_test.wfl) and
[locking](../../TestPrograms/database_schema_lock_test.wfl) suites cover failure
recovery, ordinary returns, names, nesting, in-memory retention and competing
writers.

The [lifecycle suite](../../TestPrograms/database_schema_lifecycle_test.wfl)
checks abrupt program exit, a killed migration process, recovery and source
fixing. The [cancellation suite](../../TestPrograms/schema_cancellation/cancellation.test.wfl)
stops a concurrent server loop while a schema transaction is suspended, then
proves rollback and foreign-key restoration on the same in-memory connection.
The transaction's registry reservation belongs to its executing block: dropping
that future removes its reservation even if the interpreter keeps running.

This mode currently supports SQLite only and rejects nesting on the same
handle before changing any connection settings. Normal transaction syntax and
return behavior remain unchanged. `schema` and `changes` are ordinary names
outside this header; neither becomes a reserved word.

The runtime supplies transaction safety, not a migration ledger or schema
planner. Applications still own version definitions, checksums, drift checks,
backup/recovery procedures, and preservation of indexes, triggers and sequence
values during a rebuild. A preexisting foreign-key violation also prevents a
schema transaction from committing; repair it explicitly before upgrading.

> `transaction` is not a reserved word. It is recognized only in
> `in transaction on ...` and `end transaction`, so existing programs that use
> `transaction` as a variable name keep working.

## Returning Results from Actions

`query` and `execute` — with or without `and parameters [...]` — can be
returned directly from an action, which keeps small data-access helpers to
one line:

```wfl
define action called ages_for_name with parameters conn and who:
    return query conn with "SELECT age FROM users WHERE name = ?" and parameters [who]
end action

store rows as call ages_for_name with db and "Alice"
```

## Waiting Explicitly

Database statements run asynchronously inside WFL's runtime; you can make the
wait explicit with `wait for`:

```wfl
wait for store users as query db with "SELECT * FROM users"
```

## Type Mapping

Column values convert to WFL values automatically:

| SQL type | WFL value |
|----------|-----------|
| `INTEGER`, `BIGINT`, `SERIAL`, ... | number |
| `REAL`, `FLOAT`, `DOUBLE`, `NUMERIC`, `DECIMAL` | number |
| `TEXT`, `VARCHAR`, `CHAR`, ... | text |
| `BOOLEAN` (and MySQL `TINYINT(1)`) | boolean |
| `BLOB`, `BYTEA`, `VARBINARY` | binary data |
| `DATE` | date |
| `TIME` | time |
| `TIMESTAMP`, `DATETIME` | datetime |
| SQL `NULL` | nothing |

`NULL` values compare equal to the `nothing` literal:

```wfl
store rows as query db with "SELECT age FROM users WHERE name = ?" and parameters ["Ghost"]
store row as rows[0]
check if row["age"] is nothing:
    display "age not recorded"
end check
```

Bind parameters convert the other way: whole numbers bind as integers,
fractional numbers as floats, plus text, booleans, binary data, dates, times,
datetimes, and `nothing` (binds as SQL `NULL`).

## Error Handling

Database errors — bad SQL, constraint violations, connection problems, or use
of a closed handle — raise runtime errors that `try` blocks can catch:

```wfl
try:
    store rows as query db with "SELECT * FROM missing_table"
when error:
    display "query failed: " with error
end try
```

## Closing Connections

Close a connection when you are done with it:

```wfl
close database db
```

Querying a closed (or never-opened) handle raises a catchable error. Pools
left open when a program ends are cleaned up with the runtime.

## Complete Example

```wfl
open database at "sqlite://todo.db" as db

store created as execute db with "CREATE TABLE IF NOT EXISTS tasks (id INTEGER PRIMARY KEY, title TEXT, done BOOLEAN)"

store added as execute db with "INSERT INTO tasks (title, done) VALUES (?, ?)" and parameters ["Write docs" and no]
display "Created task #" with added["last_insert_id"]

store open_tasks as query db with "SELECT id, title FROM tasks WHERE done = ?" and parameters [no]
store task_count as length of open_tasks
display task_count with " open task(s)"

for each task in open_tasks:
    display "- [" with task["id"] with "] " with task["title"]
end for

store finished as execute db with "UPDATE tasks SET done = ? WHERE title = ?" and parameters [yes and "Write docs"]
display "Completed " with finished["affected_rows"] with " task(s)"

close database db
```

## Testing

- SQLite needs no external services; `sqlite::memory:` is ideal for tests.
- The repository's CI runs the full suite against live PostgreSQL 16 and
  MariaDB 11 containers. Locally, set `WFL_TEST_POSTGRES_URL` and/or
  `WFL_TEST_MYSQL_URL` and run `cargo test --test database_test` to exercise
  those backends; the tests skip quietly when the variables are unset.
