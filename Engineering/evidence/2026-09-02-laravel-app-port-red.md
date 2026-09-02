# Red evidence — Laravel starter-app port

**Date:** 2026-09-02
**Risk class:** R2 (new user-facing example + HTTP contract; no runtime/concurrency change)
**Command:** `./target/release/wfl --test TestPrograms/laravel_app/laravel_starter_app.test.wfl`
**Exit:** 1
**Result:** 16 tests, 1 passed (`that true is true`), 15 failed for the intended reason.

Stub router returns `418` / `"stub"` / `"application/octet-stream"` so the failures are assertion mismatches, not missing files or parse errors:

- `GET /` / `HEAD /` / `GET /up` / `GET /robots.txt` expected 200, got 418
- unknown path expected 404, got 418
- `POST /` expected 405, got 418
- welcome / health / robots / 404 / 405 bodies did not contain the required markers
- HTML and `text/plain` content types were `application/octet-stream`
- `User.display_name()` returned `""` instead of the stored name
- `inspire_quote` length was 0

This Red run is an ancestor of the Green implementation commit on `cursor/laravel-app-port-b108`.
