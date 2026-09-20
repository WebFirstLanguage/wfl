# Application errors and SQLite schema transactions

The Scriptorium ORM capability audit found two missing runtime operations:
library validation could not raise a meaningful error, and a SQLite rebuild
could not configure the transaction's connection before BEGIN. Returning a
failure flag committed earlier writes, while disabling foreign keys inside
the existing block did nothing and a parent-table drop cascaded into
extension-owned rows.

The additive API is `raise_error of message` (also `call raise_error with
message`) and `in transaction on db for schema changes:`. Beginners and
experienced callers use the same error and transaction forms. Ordinary
transactions, Boolean returns, and identifier names retain their meanings.

Schema mode owns a pooled connection through setup, immediate write-lock
acquisition, foreign-key validation, commit or rollback, and enforcement
restoration. Acquisition has a five-second limit. Cancellation cleanup keeps
the connection out of the pool until it is safe, closing it if cleanup itself
fails or is cancelled. Successful cleanup preserves in-memory databases.

New executable scenarios and fixtures are WFL, discovered by the existing
Linux/Windows WFL program gates. The test-only predecessor retains the real
file-backed child-row loss and identifies the absent application-error builtin
without claiming an unavailable-builtin diagnostic is behavioral Red evidence.
Technical review and full validation remain requirements of the change record.

Independent review found a lifecycle gap in the existing transaction registry:
dropping a concurrent handler left its transaction in the surviving interpreter.
A real WFL server regression reproduced this by stopping a loop while a schema
scope was suspended. A block guard now removes its exact registry slot on every
dropped future, including acquisition cancellation. The short registry lock is
synchronous so Drop can perform that removal reliably; database operations
retain their separate asynchronous slot locks. This also protects ordinary
transactions without changing their commit rules.

The WFL source-fixer regression caught a separate contextual-marker collision.
When a variable is named `schema changes`, fixing all matching identifier tokens
would rewrite the transaction header. The existing conservative name protection
now covers this marker, while unrelated variable spelling fixes still apply.
