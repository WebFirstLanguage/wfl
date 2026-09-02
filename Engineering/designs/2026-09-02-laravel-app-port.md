# Design: port laravel/laravel (the app) to WFL

**Status:** Implemented as `examples/laravel-app/`
**Source:** [laravel/laravel](https://github.com/laravel/laravel) 13.x
**Risk class:** R2 (example + HTTP contract; no change to the concurrent
listen/respond/stream implementation — §11.3 stays on those primitives)

## What “the actual app” is

`laravel/laravel` is the application skeleton users get from `laravel new`.
It is not the framework. On 13.x the runnable HTTP surface is:

1. `GET /` → `resources/views/welcome.blade.php`
2. `GET /up` → framework health registered in `bootstrap/app.php`
3. `GET /robots.txt` → `User-agent: *` / `Disallow:`
4. Unknown paths → 404
5. `Route::get` routes reject other methods with 405

Plus unused-but-present scaffolding: empty `Controller`, `User` model,
`artisan inspire`, Vite/CSS/JS entrypoints, config, migrations.

## Approach

Port the **observable app**, not the framework.

- Keep WFL idioms (`route`, `include from`, `listen`, containers).
- Extract routing into testable actions (`status_for_path`, `body_for_path`,
  `content_type_for_path`) so `TestPrograms/` can assert without a socket,
  and `tests/laravel_app_http_test.rs` can assert the real binary over HTTP.
- Write original welcome-page copy and CSS. Do not reconstruct Blade,
  Tailwind, or Laravel’s logo/SVG.
- Document every gap in `examples/laravel-app/README.md`. A missing
  framework feature is a documented gap, not a silent pretence.

## Rejected alternatives

1. **Reimplement Laravel in WFL** (service container, Eloquent, Blade,
   Artisan). WFL does not have those primitives; claiming them would
   violate the docs-honesty rule.
2. **Single-file demo that only prints “Hello”.** That is not a port of
   the starter app’s public routes.
3. **Copy `welcome.blade.php` verbatim.** Copyrighted; also fights WFL’s
   no-unlearning / original-example bar.

## Residual gaps (by design)

Eloquent, Blade, Vite, Artisan, queues, cache, mail, sessions,
broadcasting, service providers, and `.env` config beyond bind address.
See the capability map in the example README.
