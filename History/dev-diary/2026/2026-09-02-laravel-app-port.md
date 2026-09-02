# 2026-09-02 — Port of laravel/laravel (the starter app) to WFL

## What this is

A faithful-as-able port of the official Laravel **application skeleton**
([laravel/laravel](https://github.com/laravel/laravel) 13.x) — the repo you
get from `laravel new`, not `laravel/framework`.

## What mapped

The 13.x skeleton's public HTTP surface is small, and WFL can express it:

- `GET /` welcome HTML (original copy and CSS; not a Blade reconstruction)
- `GET /up` health, the route `bootstrap/app.php` registers
- `GET /robots.txt` with the skeleton's two-line body
- unknown paths → 404
- non-GET/HEAD on registered paths → 405
- a `User` container for `app/Models/User.php`'s name + email fields
- `inspire.wfl` as the `artisan inspire` stand-in

Routing is extracted into actions so
`TestPrograms/laravel_app/laravel_starter_app.test.wfl` can assert without a
socket, and `tests/laravel_app_http_test.rs` drives the real `wfl` binary
over HTTP.

## What did not map

Eloquent, Blade, Vite, Artisan, queues, cache, mail, sessions,
broadcasting, service providers, and most of `.env` / `config/*.php`. Those
are framework features. The example README's capability map lists them as
gaps rather than pretending they shipped.

## Tests

Red: stub router returned 418 / `"stub"`; 15 of 16 feature tests failed on
assertions (`Engineering/evidence/2026-09-02-laravel-app-port-red.md`).
Green: the same file, plus the HTTP e2e suite.
