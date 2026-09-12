# Managed authentication and request identity

WFL now centralizes session lifetime and credential policy in maintained native
helpers. Session stores are opaque, shared capabilities within one program;
session IDs and CSRF tokens come from the operating system random source and
only token digests remain in the store. Rotation replaces both tokens without
extending absolute lifetime, and single-session/account-wide revocation takes
effect before the next handler can use those credentials. Expiry indexes stay
bounded by live entries. Authentication state disappears when its last handle
is dropped or the process ends.

The request CSRF guard validates the original request's cookie and header
metadata, including duplicate physical headers and undecodable header values.
It is explicitly called at the top of a protected handler. A separate account
attempt limiter uses bounded fixed windows and denies new identities when full
without evicting blocked accounts. Both handles retain shared state across
cloned request and module environments.

Trusted proxies are an opt-in IP/CIDR configuration. `client_ip` remains the
socket peer; `originating_ip` validates the complete bounded X-Forwarded-For
chain and walks it from the trusted socket side. Untrusted peers and malformed
or ambiguous forwarding data resolve to the socket identity. Configuration
loading, checking and fixing all use the same parser.

Password policy objects pin bounded Argon2id v19 parameters and are revalidated
on every use. Configured hashing has bounded asynchronous admission and worker
slots; cancellation cannot release a running worker's slot prematurely.
`password_needs_rehash` inspects metadata without running a password KDF and
rejects malformed records or ambiguous cost downgrades. Existing single-argument
hashing and verification remain compatible.

This is R3 work. Failing native and real-binary HTTP/TLS tests were committed
before implementation. Additional tests cover entropy failure, inclusive idle
and absolute expiry, revocation races, simultaneous rate-limit admission,
duplicate headers, spoofed forwarding chains, malformed hash metadata,
blocking-pool saturation and cancellation recovery. Independently instructed
agents reviewed each feature's actual implementation; registration and nested
handle equality findings were corrected. The PR records the Red revisions,
full validation results, and the remaining limitation that these session and
rate-limit stores are process-local rather than a distributed storage service.

The full Windows suite exposed a preexisting file-fixture problem: concurrent
file-I/O tests wrote into the repository and exceeded their unchanged ten-second
deadline on a drive with slow durable flushes. Direct flush measurements and
the unchanged test executable in system temporary storage isolated the cause.
Each test now owns a temporary directory and uses absolute paths for both WFL
and Rust access. Real flushes, deadlines and content assertions remain intact,
and cleanup also occurs on failure.

The same validation exposed repository-relative paths in the performance
fixture. Its unchanged executable also passed from system temporary storage;
it now uses owned temporary directories while retaining all measured
operations, watchdogs and performance thresholds.
