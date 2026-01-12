# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

MicroBin is a self-contained pastebin and URL shortener web application written in Rust using the Actix-web framework. It supports text pastes, file uploads, URL redirects, encryption, expiration, and access control.

## Build & Development Commands

```bash
# Build
cargo build              # Debug build
cargo build --release    # Release build (LTO enabled, stripped)

# Run locally (reads configuration from .env)
cargo run

# Run tests
cargo test

# Lint
cargo clippy

# Install from source
cargo install --path .
```

## Configuration

All configuration is via environment variables (see `.env` for full reference). Key settings:
- `MICROBIN_PORT` / `MICROBIN_BIND` - Server binding
- `MICROBIN_DATA_DIR` - Data storage location (default: `microbin_data`)
- `MICROBIN_JSON_DB` - Use JSON storage instead of SQLite
- `MICROBIN_ADMIN_USERNAME` / `MICROBIN_ADMIN_PASSWORD` - Admin credentials
- `MICROBIN_BASIC_AUTH_USERNAME` / `MICROBIN_BASIC_AUTH_PASSWORD` - Access protection

## Architecture

### Core Components

- **`src/main.rs`** - Entry point, Actix-web server setup, route configuration
- **`src/args.rs`** - CLI/environment config parsing via clap. Exposes `ARGS` lazy_static singleton
- **`src/pasta.rs`** - Core data model (`Pasta` struct with content, files, encryption, expiration)

### Endpoints (`src/endpoints/`)

HTTP handlers organized by function:
- `create.rs` - POST /upload (new pasta creation)
- `pasta.rs` - GET /pasta/{id}, /p/{id}, /raw/{id}, /r/{id} (display/raw views)
- `file.rs` - File upload/download handling
- `edit.rs` - Pasta modification
- `remove.rs` - Deletion
- `admin.rs` - Admin dashboard
- `auth_*.rs` - Authentication gates
- `list.rs`, `qr.rs`, `guide.rs` - Supporting pages

### Utilities (`src/util/`)

- **`db.rs`** - Database abstraction layer
- **`db_sqlite.rs`** / **`db_json.rs`** - Storage backends (SQLite default, JSON optional)
- **`animalnumbers.rs`** - Converts IDs to memorable animal name pairs
- **`misc.rs`** - Encryption helpers, QR generation, expiry logic
- **`syntaxhighlighter.rs`** - Code highlighting via syntect

### Templates

Askama HTML templates in `templates/`. Rendered via derive macros:
```rust
#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate { ... }
```

## Database

Dual-mode storage:
- **SQLite** (default): `microbin_data/microbin.db`
- **JSON** (set `MICROBIN_JSON_DB=true`): `microbin_data/db.json`

Database operations go through `src/util/db.rs` which dispatches to the appropriate backend.

## Feature Flags

- `default` - SQLite + OpenSSL
- `no-c-deps` - Rustls + pure Rust syntect (for environments without C toolchain)

## Application State

Shared state via Actix-web `web::Data<AppState>`:
```rust
pub struct AppState {
    pub pastas: Mutex<Vec<Pasta>>,
}
```

Pastas are loaded into memory at startup; database operations persist changes.
