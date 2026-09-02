# WFL port of the Laravel application skeleton

This directory is a WFL port of the official
[laravel/laravel](https://github.com/laravel/laravel) **starter app** (13.x),
not the Laravel framework.

Laravel's application repo is intentionally thin: a welcome page, a health
endpoint, a `robots.txt`, an empty controller, a User model, and an `inspire`
console command. Everything else (Eloquent, Blade, Artisan, Vite, queues,
mail, sessions) lives in `laravel/framework` and is **not** claimed here.

## Run

```bash
wfl examples/laravel-app/app.wfl
```

Then open `http://127.0.0.1:8000`.

```bash
wfl examples/laravel-app/inspire.wfl   # artisan inspire equivalent
```

`.wflcfg` binds the server to `127.0.0.1`. Change `web_server_bind_address` to
`0.0.0.0` only when you intend to expose it.

## Capability map

| Laravel 13.x skeleton | WFL port | Status |
|---|---|---|
| `GET /` → `view('welcome')` | `welcome_page` HTML | Mapped (original copy/CSS) |
| `GET /up` health (`bootstrap/app.php`) | `/up` → 200 + “Application up” | Mapped |
| `public/robots.txt` | same two-line body | Mapped |
| Unknown path | 404 HTML | Mapped |
| `Route::get` (GET/HEAD only) | other methods → 405 | Mapped |
| `app/Models/User.php` | `User` container (`name`, `email`) | Partial — no Eloquent, casts, or auth |
| `artisan inspire` | `inspire.wfl` / `inspire_quote` | Partial — original quote, not the framework catalog |
| `app/Http/Controllers/Controller.php` | (empty in Laravel) | Nothing to port |
| `app/Providers/AppServiceProvider.php` | — | Not ported (no service container) |
| Blade / Vite / Tailwind | inline HTML in `views.wfl` | Not ported |
| Eloquent, migrations, factories, seeders | — | Not ported |
| Artisan, queues, cache, mail, sessions, broadcasting | — | Not ported |
| `.env` / `config/*.php` | `.wflcfg` bind address only | Partial |
| PHPUnit Feature/Unit example tests | `TestPrograms/laravel_app/feature_example.test.wfl` | Mapped + expanded |

## Layout

```
app.wfl              front controller (public/index.php + listen)
routes.wfl           route table (routes/web.php + /up)
views.wfl            welcome, health, 404, robots bodies
user.wfl             User container
quotes.wfl           inspire_quote
inspire.wfl          console inspire command
public/robots.txt    crawler policy
.wflcfg              loopback bind
```

## Tests

```bash
wfl --test TestPrograms/laravel_app/feature_example.test.wfl
cargo test --test laravel_app_http_test
```
