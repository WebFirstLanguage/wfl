# Green evidence — Laravel starter-app port

**Date:** 2026-09-02
**Risk class:** R2
**Red ancestor:** `06b3368` (`test: add failing Laravel starter-app port coverage`)

## Layers

| Layer | Command | Result |
|---|---|---|
| WFL feature tests | `./target/release/wfl --test TestPrograms/laravel_app/feature_example.test.wfl` | 16 passed, 0 failed, exit 0 |
| Rust HTTP e2e | `cargo test --test laravel_app_http_test` | 5 passed, 0 failed |
| Inspire CLI | `./target/release/wfl examples/laravel-app/inspire.wfl` | printed `Readability is a feature, not a luxury.` |
| Hygiene | `python3 scripts/check_repo_hygiene.py --mode static` | exit 0 |

HTTP e2e (real `wfl` binary, real sockets): `GET /` 200, `GET /up` 200 + “Application up”, `GET /robots.txt` skeleton body, unknown path 404, `POST /` 405.
