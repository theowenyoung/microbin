
![Screenshot](.github/index.png)

# MicroBin (Fork)

A fork of [MicroBin](https://github.com/szabodanika/microbin) - a super tiny, feature-rich, configurable, self-contained and self-hosted paste bin web application.

## What's New in This Fork

- **S3 Storage** - Store file attachments in S3-compatible storage (AWS S3, Backblaze B2, MinIO, etc.) instead of local filesystem
- **Markdown Rendering** - GitHub-style Markdown rendering with tables, code blocks, task lists, footnotes, and more
- **HTML Rendering** - Sandboxed iframe display for HTML content
- **Auto Content Detection** - Automatically detects Markdown, HTML, or code syntax and renders accordingly

## Quick Start

### Docker (recommended)

```bash
docker run -d --name microbin \
  -p 8080:8080 \
  -v microbin-data:/app/microbin_data \
  owenyoung/microbin:latest
```

Or use Docker Compose:

```bash
# Download config files
curl -O https://raw.githubusercontent.com/theowenyoung/microbin/master/.env.example
curl -O https://raw.githubusercontent.com/theowenyoung/microbin/master/compose.yaml
cp .env.example .env
# Edit .env with your preferred settings
docker compose --env-file .env up -d
```

### From Source

1. Copy the example env file and edit it with your settings:

```bash
cp .env.example .env
# Edit .env with your preferred settings
```

2. Run the development server:

```bash
make dev
```

For release builds:

```bash
make release
```

## Configuration

All configuration is via environment variables in `.env`. See `.env.example` for all available options with documentation.

### S3 Storage

To store file attachments in S3-compatible storage, configure these variables in your `.env`:

```bash
export MICROBIN_S3_ENDPOINT=https://s3.us-west-000.backblazeb2.com
export MICROBIN_S3_BUCKET=your-bucket-name
export MICROBIN_S3_ACCESS_KEY=your-access-key
export MICROBIN_S3_SECRET_KEY=your-secret-key
export MICROBIN_S3_REGION=us-west-000  # optional
```

All four settings (endpoint, bucket, access key, secret key) must be provided to enable S3 storage. When not configured, files are stored on the local filesystem.

### Content Rendering

| Variable | Default | Description |
|----------|---------|-------------|
| `MICROBIN_DEFAULT_SYNTAX` | `auto` | Default syntax selection. Use `auto` for automatic detection, `none` for plain text, or a language extension (e.g., `py`, `js`, `rs`) |
| `MICROBIN_RENDER_MARKDOWN` | `false` | Enable Markdown rendering with GitHub-style formatting |
| `MICROBIN_RENDER_HTML` | `false` | Enable HTML rendering in sandboxed iframe |

When `MICROBIN_DEFAULT_SYNTAX=auto`:
- **Markdown** content (headers, code blocks, lists, tables) is rendered like GitHub READMEs
- **HTML** content (with DOCTYPE or multiple block elements) is displayed in a secure sandboxed iframe
- **Code** is syntax-highlighted using highlight.js

## Features

- Entirely self-contained executable, MicroBin is a single file!
- S3-compatible object storage for file attachments
- Markdown rendering with GitHub-style formatting
- HTML rendering in sandboxed iframe
- Automatic content type detection
- Server-side and client-side encryption
- File uploads (e.g. `server.com/file/pig-dog-cat`)
- Raw text serving (e.g. `server.com/raw/pig-dog-cat`)
- QR code support
- URL shortening and redirection
- Animal names instead of random numbers for upload identifiers (64 animals)
- SQLite and JSON database support
- Private and public, editable and uneditable, automatically and never expiring uploads
- Automatic dark mode and custom styling support with very little CSS and only vanilla JS (see [`water.css`](https://github.com/kognise/water.css))

## Build Commands

```bash
make dev       # Run dev server (loads .env)
make build     # Debug build
make release   # Release build (LTO enabled, stripped)
make test      # Run tests
make clean     # Clean build artifacts
```

## License

MicroBin and MicroBin.eu are available under the [BSD 3-Clause License](LICENSE).
