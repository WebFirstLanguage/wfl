# TLS 1.3 handshake record boundaries

Raised the rustls dependency floor to 0.23.45 and refreshed the root and fuzz
lockfiles for RUSTSEC-2026-0285. The compatible update also requires newer
aws-lc-rs, aws-lc-sys, and rustls-webpki. No language syntax or TLS configuration
was changed.

The test-first reproduction feeds a genuine ServerHello and a complete
plaintext EncryptedExtensions through rustls's record parser. Version 0.23.42
accepted the malformed record; 0.23.45 rejects it at the key-change boundary.
Coverage also verifies a valid authenticated TLS 1.3 exchange, every two-chunk
partition of the malformed record, and rejection by a WFL HTTPS request over a
local TCP socket. Existing TLS listener tests retain protocol, ALPN, timeout,
shutdown, invalid-key, and unrelated-client coverage.

The assessment and validation record is in
`Engineering/evidence/2026-09-21-rustls-handshake-boundaries.md`.
