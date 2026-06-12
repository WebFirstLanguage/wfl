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
- `sqlite::memory:` opens a temporary in-memory database that disappears when
  the connection closes.
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
