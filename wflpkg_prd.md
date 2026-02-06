# WFLHub — Product Requirements Document

## 1. Overview

**WFLHub** is the official package registry and web portal for WFL packages (`.wflpkg`). It serves as the central hub where WFL developers publish, discover, and manage packages distributed via the `wflpkg` CLI. The default registry URL is already hardcoded as `wflhub.org` in the wflpkg manifest system.

### Goals
- Provide a reliable, secure package registry that the existing `wflpkg` CLI can publish to and install from
- Enable package discovery through search, browsing, and categorization
- Build community trust through verified authors, download stats, and security audits

### Non-Goals (v1)
- Package build/CI pipelines (packages are built locally and uploaded as `.wflpkg` archives)
- Paid/private packages
- Organization/team management beyond basic ownership

---

## 2. System Context

WFLHub must be compatible with the existing `wflpkg` registry client API. The client already expects these endpoints:

| Method | Endpoint | Purpose |
|--------|----------|---------|
| `GET` | `/api/v1/search?q=<query>` | Search packages |
| `GET` | `/api/v1/packages/<name>` | Get package metadata |
| `POST` | `/api/v1/packages` | Publish a package (multipart, Bearer auth) |

### Package Format
- **Archive**: `.wflpkg` (tar.gz)
- **Manifest**: `project.wfl` (natural-language flat file with `is` assignment syntax)
- **Lockfile**: `project.lock` (auto-generated, exact versions + checksums)
- **Versioning**: Calendar-based `YY.MM.BUILD` (e.g., `26.2.1`)
- **Checksums**: SHA-256 with `wflhash:` prefix
- **Auth token storage**: `~/.wfl/auth.json`

---

## 3. User Roles

| Role | Description |
|------|-------------|
| **Visitor** | Unauthenticated user browsing/searching packages |
| **Author** | Authenticated user who can publish and manage their own packages |
| **Admin** | Site operator with moderation, takedown, and user management powers |

---

## 4. Features

### 4.1 Package Registry API

The backend API that the `wflpkg` CLI already communicates with.

#### 4.1.1 Search — `GET /api/v1/search?q=<query>`
- Full-text search across package name, description, and keywords
- Returns: `[{name, description, version, downloads}]`
- Pagination: `?q=<query>&page=1&per_page=20` (default 20, max 100)
- Sort options: `relevance` (default), `downloads`, `recent`

#### 4.1.2 Package Info — `GET /api/v1/packages/<name>`
- Returns full metadata for a package
- Response:
  ```json
  {
    "name": "http-client",
    "description": "A simple HTTP client for WFL",
    "latest_version": "26.2.1",
    "versions": ["26.2.1", "26.1.3", "25.12.8"],
    "author": "alice",
    "license": "MIT",
    "downloads": 1542,
    "repository": "https://github.com/alice/http-client",
    "permissions": ["network-access"],
    "created_at": "2025-12-08T...",
    "updated_at": "2026-02-01T..."
  }
  ```

#### 4.1.3 Publish — `POST /api/v1/packages`
- **Auth**: Bearer token (from `wfl login`)
- **Body**: Multipart form with fields:
  - `name` — package name
  - `version` — version string (YY.MM.BUILD)
  - `checksum` — `wflhash:<sha256hex>`
  - `archive` — binary `.wflpkg` file
- **Validation on upload**:
  1. Name matches `[a-z][a-z0-9-]{0,63}` (enforced by existing manifest parser)
  2. Version is valid calendar version
  3. Version does not already exist for this package (no overwrites)
  4. Checksum matches server-side recomputation of the archive
  5. Archive extracts safely (no symlinks, no path traversal, no absolute paths)
  6. Archive contains a valid `project.wfl` manifest
  7. Manifest `name` and `version` match the form fields
  8. First publish of a name claims ownership; subsequent publishes require same author
- **Size limit**: 50 MB per archive (v1)

#### 4.1.4 Download — `GET /api/v1/packages/<name>/<version>/download`
- Returns the `.wflpkg` binary archive
- Increments download counter
- Supports `Accept-Encoding: gzip` (archive is already gzipped, serve as-is)

#### 4.1.5 Version List — `GET /api/v1/packages/<name>/versions`
- Returns all published versions with metadata:
  ```json
  [
    {"version": "26.2.1", "checksum": "wflhash:abc...", "published_at": "...", "yanked": false},
    {"version": "26.1.3", "checksum": "wflhash:def...", "published_at": "...", "yanked": false}
  ]
  ```

#### 4.1.6 Yank — `DELETE /api/v1/packages/<name>/<version>`
- Marks a version as yanked (not deleted — existing lockfiles still resolve)
- Yanked versions don't appear in "latest" or constraint resolution for new installs
- Requires author or admin auth

---

### 4.2 Authentication

#### 4.2.1 Registration
- **Fields**: username, email, password
- Username rules: lowercase alphanumeric + hyphens, 3–39 chars, must start with letter
- Email verification required before publishing
- Password: minimum 10 characters

#### 4.2.2 Login — `POST /api/v1/login`
- **Request**: `{username, password}` or `{email, password}`
- **Response**: `{token, expires_at}`
- Token is a signed JWT or opaque token with 90-day expiry
- The `wflpkg login` CLI command calls this and stores the token in `~/.wfl/auth.json`

#### 4.2.3 Token Management
- Users can revoke tokens from the web dashboard
- Users can generate scoped API tokens (publish-only, read-only) from dashboard
- All tokens visible on dashboard with last-used timestamp

#### 4.2.4 Web Login
- Standard session-based auth for the web portal
- Optional: GitHub OAuth for convenience (link to WFLHub account)

---

### 4.3 Web Portal

#### 4.3.1 Homepage
- Search bar (prominent)
- Featured/trending packages (by recent downloads)
- Recently published packages
- Total package count and community stats

#### 4.3.2 Package Page (`/packages/<name>`)
- Package name, description, latest version
- Install command: `wflpkg add <name>`
- Version selector dropdown
- Tabs:
  - **README** — rendered from `README.md` in the archive (if present)
  - **Versions** — all versions with dates, checksums, yank status
  - **Dependencies** — parsed from `project.wfl` manifest
  - **Dependents** — packages that depend on this one (reverse lookup)
  - **Permissions** — declared permissions (`file-access`, `network-access`, `system-access`) with explanations
- Sidebar:
  - Author (link to profile)
  - License
  - Repository link
  - Download count
  - Published date / last updated
  - Install size

#### 4.3.3 Search Results (`/search?q=<query>`)
- Package cards with name, description, version, downloads, author
- Filter by: license, permissions required, recently updated
- Sort by: relevance, downloads, recently published

#### 4.3.4 User Profile (`/users/<username>`)
- Username, avatar, bio, joined date
- List of published packages
- Total downloads across all packages

#### 4.3.5 Dashboard (`/dashboard`) — Authenticated
- My packages with quick stats (downloads, versions, last publish date)
- API token management (create, revoke, view last-used)
- Account settings (email, password, avatar, bio)
- Notification preferences

---

### 4.4 Security

#### 4.4.1 Upload Validation
- Server-side archive extraction in a sandboxed temp directory
- Reject symlinks, hard links, absolute paths, `..` traversal (mirrors existing `wflpkg` client checks)
- Recompute checksum server-side and compare with submitted checksum
- Virus/malware scan on uploaded archives (v2 — flag for review)

#### 4.4.2 Package Integrity
- Store original checksum alongside each version
- Serve `Content-Digest` header on downloads for client-side verification
- Immutable versions — once published, a version's archive cannot be replaced (only yanked)

#### 4.4.3 Rate Limiting
- Publish: 10 per hour per user
- Search: 60 per minute per IP
- Download: 300 per minute per IP
- Login attempts: 5 per minute per IP (with exponential backoff)

#### 4.4.4 Abuse Prevention
- Package name squatting policy: names unused for 6 months with 0 downloads can be reclaimed
- Typosquatting detection: flag new packages with names similar to popular ones for admin review
- Report button on package pages for policy violations

---

### 4.5 Data Model

```
users
├── id (UUID)
├── username (unique, indexed)
├── email (unique)
├── password_hash (argon2)
├── avatar_url
├── bio
├── email_verified (bool)
├── created_at
└── updated_at

packages
├── id (UUID)
├── name (unique, indexed)
├── owner_id → users.id
├── description
├── repository
├── license
├── downloads_total (counter)
├── created_at
└── updated_at

versions
├── id (UUID)
├── package_id → packages.id
├── version_string (YY.MM.BUILD)
├── checksum (wflhash:...)
├── archive_path (object storage key)
├── archive_size_bytes
├── permissions (text[], from manifest)
├── dependencies (jsonb, from manifest)
├── yanked (bool, default false)
├── published_at
└── (unique constraint: package_id + version_string)

api_tokens
├── id (UUID)
├── user_id → users.id
├── token_hash
├── name (user-provided label)
├── scope (publish | read-only | full)
├── last_used_at
├── expires_at
├── created_at
└── revoked_at
```

---

## 5. Tech Stack Recommendations

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| **Backend API** | Rust (Axum or Actix-Web) | Matches WFL ecosystem, high performance, type safety |
| **Database** | PostgreSQL | Reliable, JSON support for dependencies, full-text search |
| **Object Storage** | S3-compatible (MinIO for self-host, AWS S3 for prod) | Scalable binary storage for `.wflpkg` archives |
| **Frontend** | Server-rendered HTML + minimal JS (Askama/Tera templates) | Simple, fast, accessible; avoid SPA complexity for v1 |
| **Auth** | Argon2 password hashing, JWT or opaque tokens | Industry standard |
| **Search** | PostgreSQL `tsvector` full-text search | Good enough for v1; upgrade to MeiliSearch/Tantivy later |
| **Deployment** | Docker + single VPS or small k8s cluster | Start simple, scale later |
| **CDN** | Cloudflare or similar | Cache package downloads at edge |

---

## 6. API Versioning

All API endpoints are prefixed with `/api/v1/`. Future breaking changes will use `/api/v2/` with a deprecation period. The `wflpkg` CLI client already uses the `/api/v1/` prefix.

---

## 7. Milestones

### Phase 1 — MVP (Core Registry)
- [ ] API: publish, download, search, package info
- [ ] Auth: registration, login, token generation
- [ ] Upload validation (checksum, manifest, safety)
- [ ] Web: homepage, package page, search results
- [ ] Basic rate limiting
- [ ] Deploy to `wflhub.org`

### Phase 2 — Community
- [ ] User profiles
- [ ] Dashboard with package management
- [ ] Yank support
- [ ] Dependents (reverse dependency) tracking
- [ ] README rendering from archives
- [ ] Scoped API tokens

### Phase 3 — Trust & Safety
- [ ] Typosquatting detection
- [ ] Package report/flag system
- [ ] Admin moderation panel
- [ ] Malware scanning on upload
- [ ] Audit log for publish/yank actions

### Phase 4 — Scale
- [ ] CDN for package downloads
- [ ] Advanced search (MeiliSearch/Tantivy)
- [ ] GitHub OAuth login
- [ ] Package categories/tags
- [ ] Organization accounts
- [ ] Private packages (paid tier)

---

## 8. Success Metrics

| Metric | Target (6 months post-launch) |
|--------|-------------------------------|
| Published packages | 100+ |
| Registered authors | 50+ |
| Monthly downloads | 1,000+ |
| API uptime | 99.5% |
| Median search latency | < 200ms |
| Median download latency | < 500ms |

---

## 9. Open Questions

1. **Domain**: Is `wflhub.org` confirmed and registered? The manifest parser defaults to this.
2. **Email provider**: What service for verification emails? (SendGrid, SES, self-hosted?)
3. **Terms of Service**: Need legal review for package hosting liability, DMCA takedown process.
4. **Namespace policy**: Should we reserve common names (e.g., `std`, `core`, `wfl-*`) for official packages?
5. **Archive size limit**: 50 MB reasonable for v1? Some packages with bundled assets may be larger.
6. **Token format**: JWT (stateless, can't revoke without blocklist) vs opaque tokens (requires DB lookup but simpler revocation)?
