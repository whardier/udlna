# Phase 3: HTTP Server & File Streaming - Research

**Researched:** 2026-02-22
**Domain:** axum 0.8 HTTP server, RFC 7233 Range requests, DLNA response headers, dual-socket binding
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **HTTP framework:** axum (built on hyper) — not raw hyper. Router, extractors, State injection built-in.
- **Async runtime:** `#[tokio::main]` on `main.rs` — single tokio runtime for the whole process.
- **Bind address:** configurable via a `localhost` config flag:
  - `localhost = true` → bind to `127.0.0.1:{port}` only
  - `localhost = false` (default) → dual-bind: both `0.0.0.0:{port}` (IPv4) and `:::{port}` (IPv6) as separate sockets
- **File streaming:** `tokio::fs` (async I/O), not spawn_blocking.
- **Range edge cases (RFC 7233):**
  - Unsatisfied range → 416 Range Not Satisfiable with `Content-Range: bytes */total_size`
  - Multi-part ranges → serve first range only (ignore additional parts)
  - Suffix ranges (`bytes=-500`) → supported (compute from file size)
- **Router scaffold (all routes established in Phase 3):**
  - `/media/:id` — implemented and working (GET + HEAD)
  - `/device.xml` — 501 stub (Phase 4)
  - `/cds/scpd.xml` — 501 stub (Phase 4)
  - `/cms/scpd.xml` — 501 stub (Phase 4)
  - `/cds/control` — 501 stub (Phase 5)
  - `/cms/control` — 501 stub (Phase 5)
- **Media ID format:** UUID string matching Phase 2 UUIDv5 IDs (e.g., `/media/550e8400-e29b-41d4-a716-446655440000`)
- **App state:** injected via `axum::extract::State<AppState>` — shape is Claude's discretion
- **Error responses:**
  - 404 for unknown media ID: empty body
  - Invalid/malformed UUID in path: 404 (not 400)
  - File missing at serve time: 500 + `tracing::error!` log
  - All requests logged at info level: method, path, status, duration
  - 501 stub routes return minimal body (Claude's discretion — "Not Implemented" text or empty)
- **DLNA response headers (all media responses MUST include):**
  - `Accept-Ranges: bytes`
  - `transferMode.dlna.org: Streaming`
  - `contentFeatures.dlna.org` with `DLNA.ORG_OP=01` and `DLNA.ORG_FLAGS`
- HEAD requests return 200 with all headers but no body.
- `Content-Type` from the `MediaItem.mime` field.

### Claude's Discretion

- Exact `AppState` shape
- Exact `contentFeatures.dlna.org` header value (values for `DLNA.ORG_FLAGS`)
- 501 stub body content ("Not Implemented" text or empty)

### Deferred Ideas (OUT OF SCOPE)

- None — discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| STRM-01 | Serve video, audio, and image files via GET `/media/{id}` with correct MIME types | axum Router + Path extractor + tokio::fs file open + Body::from_stream |
| STRM-02 | Handle HEAD requests on `/media/{id}` (Samsung sends HEAD before every GET) | axum auto-handles HEAD for GET routes; explicit `.head()` override for custom behavior |
| STRM-03 | Return 206 Partial Content with correct `Content-Range: bytes start-end/total` for Range requests (RFC 7233) | axum-extra FileStream.into_range_response + http-range-header crate for parsing |
| STRM-04 | Include `Accept-Ranges: bytes` on all media responses | Add as static header in handler response tuple |
| STRM-05 | Include `contentFeatures.dlna.org` header with DLNA.ORG_OP=01 and DLNA.ORG_FLAGS on all media responses | Static header string construction documented in research |
| STRM-06 | Include `transferMode.dlna.org: Streaming` header on all media responses | Add as static header in handler response tuple |
| STRM-07 | DIDL-Lite `<res>` elements include `size` attribute (file size in bytes) | MediaItem.file_size already stored from Phase 2; exposed via AppState library access |
</phase_requirements>

---

## Summary

Phase 3 adds axum as the HTTP layer on top of the Phase 2 media library. The core work is: (1) transition `main.rs` from a sync `fn main` to `async fn main` with `#[tokio::main]`, (2) build an axum Router with the full route scaffold, (3) implement the `/media/{id}` GET handler with Range-aware streaming, and (4) wire the dual-bind IPv4/IPv6 socket strategy based on the `localhost` config flag.

The key technical challenge is RFC 7233 Range request handling. The `axum-extra` crate (v0.12.5, feature `file-stream`) provides `FileStream::into_range_response()` which handles 206 response construction. The `http-range-header` crate (v0.4.2) handles parsing the `Range:` header string with zero dependencies. Manual parsing of Range headers is complex and error-prone — do not hand-roll it.

For the dual-bind strategy, the pattern is two separate `tokio::net::TcpListener::bind()` calls followed by two `tokio::spawn(axum::serve(...))` calls sharing a cloned `Arc<AppState>`. The main task then `tokio::select!` or joins on both handles. DLNA headers are static strings added to every media response tuple.

**Primary recommendation:** Use axum 0.8 + axum-extra 0.12 (file-stream feature) + http-range-header 0.4.2 + tower-http 0.6 (trace feature). This stack handles all RFC 7233 edge cases and DLNA header requirements with minimal custom code.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8.8 | HTTP routing, State injection, request extraction | Official tokio-rs web framework; built on hyper + tower |
| tokio | 1.x (already in tree) | Async runtime, `TcpListener`, `fs::File` | Already chosen; required by axum |
| axum-extra | 0.12.5 | `FileStream` for file streaming + range responses | Official axum companion; FileStream added Dec 2024 |
| tower-http | 0.6.8 | `TraceLayer` for request/response logging middleware | Standard companion to axum for middleware |
| http-range-header | 0.4.2 | Parse `Range: bytes=X-Y` headers per RFC 7233 | Zero-dep, fuzz-tested, 2M+ monthly downloads |
| tokio-util | 0.7.18 | `ReaderStream` to convert `AsyncRead` to `Stream` | Required by axum-extra file-stream; already transitively present |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| uuid | 1.x (already in tree) | Parse UUID from path param for media lookup | Already in Cargo.toml from Phase 2 |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| axum-extra FileStream | tower-http ServeFile | ServeFile does file serving but is not customizable for DLNA headers; FileStream is lower-level and composable |
| axum-extra FileStream | Manual tokio::fs + Body::from_stream | Manual approach requires hand-rolling 206/416 logic; error-prone per RFC 7233 edge cases |
| http-range-header | Custom Range header parser | Custom parsing misses suffix ranges, multi-range, and validation edge cases |
| TraceLayer | axum::middleware::from_fn | TraceLayer is purpose-built for request/response logging with timing; middleware::from_fn requires more boilerplate |

**Installation (new dependencies only):**
```toml
axum = { version = "0.8", features = ["tokio", "http1", "macros"] }
axum-extra = { version = "0.12", features = ["file-stream"] }
tower-http = { version = "0.6", features = ["trace"] }
http-range-header = "0.4"
tokio = { version = "1", features = ["macros", "rt-multi-thread", "fs", "net", "io-util"] }
tokio-util = { version = "0.7", features = ["io"] }
```

Note: `tokio` is already a transitive dependency via axum but needs to be explicit for `#[tokio::main]` and `tokio::fs`.

---

## Architecture Patterns

### Recommended Project Structure

```
src/
├── main.rs              # #[tokio::main], config load, scan, build router, bind sockets
├── cli.rs               # clap Args (extend with --localhost flag)
├── config.rs            # Config struct (extend with localhost: bool field)
├── http/
│   ├── mod.rs           # pub fn build_router(state: AppState) -> Router
│   ├── media.rs         # serve_media handler, range logic
│   └── state.rs         # AppState struct definition
└── media/               # Phase 2 (unchanged)
    ├── mod.rs
    ├── library.rs
    ├── mime.rs
    ├── metadata.rs
    └── scanner.rs
```

### Pattern 1: AppState Shape

**What:** Shared state containing the media library and config values needed by handlers.
**When to use:** Injected into all route handlers via `State<AppState>`.

```rust
// src/http/state.rs
use std::sync::{Arc, RwLock};
use crate::media::library::MediaLibrary;

#[derive(Clone)]
pub struct AppState {
    pub library: Arc<RwLock<MediaLibrary>>,
}
```

Requirements on AppState: must implement `Clone + Send + Sync`. `Arc<RwLock<MediaLibrary>>` satisfies all three — `Arc` provides cheap clone, `RwLock` provides Send+Sync.

### Pattern 2: Router Construction

**What:** Full route scaffold with all Phase 3–5 routes registered in one place.
**When to use:** Called from `main.rs` after library scan.

```rust
// src/http/mod.rs
// Source: https://docs.rs/axum/latest/axum/
use axum::{routing::get, Router};
use tower_http::trace::TraceLayer;
use crate::http::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Phase 3 — implemented
        .route("/media/{id}", get(media::serve_media))
        // Phase 4 stubs
        .route("/device.xml", get(|| async { (
            axum::http::StatusCode::NOT_IMPLEMENTED,
            "Not Implemented",
        )}))
        .route("/cds/scpd.xml", get(|| async { (
            axum::http::StatusCode::NOT_IMPLEMENTED,
            "Not Implemented",
        )}))
        .route("/cms/scpd.xml", get(|| async { (
            axum::http::StatusCode::NOT_IMPLEMENTED,
            "Not Implemented",
        )}))
        // Phase 5 stubs (POST for SOAP control)
        .route("/cds/control", axum::routing::post(|| async { (
            axum::http::StatusCode::NOT_IMPLEMENTED,
            "Not Implemented",
        )}))
        .route("/cms/control", axum::routing::post(|| async { (
            axum::http::StatusCode::NOT_IMPLEMENTED,
            "Not Implemented",
        )}))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
```

**IMPORTANT:** axum 0.8 uses `{id}` path syntax (not `:id`). This is a breaking change from axum 0.7.

### Pattern 3: main.rs Transition — Sync to Async

**What:** main.rs must become async to call `axum::serve`. The scan can remain synchronous (blocking) but must be called before entering async context or run with `tokio::task::spawn_blocking`.

```rust
// src/main.rs
#[tokio::main]
async fn main() {
    // tracing init, config, path validation (as today)

    // Synchronous scan — blocks the thread but runs before server starts
    // This is acceptable: scan is startup-only, server isn't serving yet
    let library = media::scanner::scan(&config.paths);

    if library.items.is_empty() {
        eprintln!("error: no media files found — exiting");
        std::process::exit(1);
    }

    let state = AppState {
        library: Arc::new(RwLock::new(library)),
    };

    let app = http::build_router(state);

    // Dual-bind or localhost-only based on config
    if config.localhost {
        let listener = tokio::net::TcpListener::bind(
            format!("127.0.0.1:{}", config.port)
        ).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    } else {
        // Dual-bind: separate IPv4 + IPv6 sockets
        let ipv4 = tokio::net::TcpListener::bind(
            format!("0.0.0.0:{}", config.port)
        ).await.unwrap();
        let ipv6 = tokio::net::TcpListener::bind(
            format!("[::]:{}", config.port)
        ).await.unwrap();

        let app_clone = app.clone();
        let h4 = tokio::spawn(async move {
            axum::serve(ipv4, app_clone).await.unwrap();
        });
        let h6 = tokio::spawn(async move {
            axum::serve(ipv6, app).await.unwrap();
        });
        // Both run forever; join either (they never return in Phase 3)
        tokio::join!(h4, h6);
    }
}
```

**Note on Router cloning:** `Router` implements `Clone` in axum 0.8. `AppState` must also be `Clone` (which it is via `Arc`).

### Pattern 4: Media Handler with Range Support

**What:** The `/media/{id}` handler must serve GET (full or range) and HEAD (headers only, no body).

```rust
// src/http/media.rs
// Source: https://docs.rs/axum-extra/latest/axum_extra/response/file_stream/
use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::response::file_stream::FileStream;
use http_range_header::parse_range_header;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

// DLNA header constants
const ACCEPT_RANGES: &str = "bytes";
const TRANSFER_MODE: &str = "Streaming";
// DLNA.ORG_OP=01: byte seek supported, time seek not supported
// DLNA.ORG_FLAGS: 01700000 = streaming transfer mode + background transfer + connection stall + DLNA v1.5
const CONTENT_FEATURES: &str =
    "DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000";

pub async fn serve_media(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> Response {
    // Parse UUID — invalid format → 404
    let id = match Uuid::parse_str(&id_str) {
        Ok(id) => id,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    // Look up item in library — unknown ID → 404
    let item = {
        let lib = state.library.read().unwrap();
        lib.items.iter().find(|i| i.id == id).cloned()
    };
    let item = match item {
        Some(i) => i,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    // Build standard DLNA headers (present on ALL responses including HEAD)
    let mut response_headers = vec![
        ("accept-ranges", ACCEPT_RANGES.to_string()),
        ("transferMode.dlna.org", TRANSFER_MODE.to_string()),
        ("contentFeatures.dlna.org", CONTENT_FEATURES.to_string()),
        ("content-type", item.mime.to_string()),
        ("content-length", item.file_size.to_string()),
    ];

    // Check for Range header
    if let Some(range_header) = headers.get("range") {
        let range_str = range_header.to_str().unwrap_or("");
        // parse_range_header returns parsed ranges validated against file size
        // ... handle range response (see Pattern 5)
    }

    // Full GET response
    let file = match tokio::fs::File::open(&item.path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to open file {}: {}", item.path.display(), e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    // Build response with headers + body
    // (full implementation in task code)
    todo!()
}
```

### Pattern 5: RFC 7233 Range Request Logic

**What:** Parse Range header, compute byte bounds, return 206 or 416.

```rust
// Source: https://docs.rs/http-range-header/latest/http_range_header/
// Source: https://docs.rs/axum-extra/latest/axum_extra/response/file_stream/

use http_range_header::{parse_range_header, StartPosition, EndPosition};

fn handle_range(
    range_str: &str,
    file_path: &std::path::Path,
    file_size: u64,
    mime: &str,
    dlna_headers: Vec<(&str, String)>,
) -> Response {
    let parsed = match parse_range_header(range_str) {
        Ok(p) => p,
        Err(_) => {
            // Syntactically invalid Range header → 416
            return build_416(file_size);
        }
    };

    // Take only the first range (per CONTEXT.md decision)
    let first = match parsed.validate(file_size) {
        Ok(ranges) => ranges.into_iter().next().unwrap(),
        Err(_) => return build_416(file_size),
    };

    let start = first.start();
    let end = first.end();  // inclusive end byte

    // Use FileStream::into_range_response for 206 construction
    // Must open file and seek to `start` position first
    // (async, so this whole fn needs to be async in practice)
    todo!()
}

fn build_416(file_size: u64) -> Response {
    (
        StatusCode::RANGE_NOT_SATISFIABLE,
        [(
            "content-range",
            format!("bytes */{}", file_size),
        )],
    ).into_response()
}
```

**Key detail on http-range-header API:** Call `parse_range_header(header_str)` then `.validate(file_size)` to get concrete byte positions. Suffix ranges (`bytes=-500`) are resolved to absolute positions during `.validate()`.

### Pattern 6: Dual-Bind with Shared Router

**What:** For localhost=false, spawn two independent `axum::serve` tasks sharing the same Router via clone.

```rust
// Source: https://github.com/tokio-rs/axum/discussions/2949
let app = build_router(state);

let ipv4 = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
let ipv6 = tokio::net::TcpListener::bind(format!("[::]:{}", port)).await?;

let app_v4 = app.clone();
let h4 = tokio::spawn(async move {
    axum::serve(ipv4, app_v4).await.unwrap()
});
let h6 = tokio::spawn(async move {
    axum::serve(ipv6, app).await.unwrap()
});

tokio::join!(h4, h6);
```

**OS note:** On Linux, binding `[::]` may also accept IPv4 via IPv4-mapped addresses depending on `IPV6_V6ONLY` socket option (defaults vary by OS). The user explicitly wants separate sockets to guarantee both work regardless of OS configuration. Setting `IPV6_V6ONLY = true` ensures the IPv6 socket is IPv6-only, but that requires `TcpSocket::new_v6()` with `.set_only_v6(true)` before binding. This is MEDIUM confidence — test on target OS.

### Pattern 7: Config Extension for `localhost` Flag

**What:** The `localhost` option must be added to both `FileConfig` and `Config`, plus a `--localhost` CLI flag.

```rust
// Extend src/config.rs
#[derive(Deserialize, Default, Debug)]
pub struct FileConfig {
    pub port: Option<u16>,
    pub name: Option<String>,
    pub localhost: Option<bool>,  // NEW
}

#[derive(Debug)]
pub struct Config {
    pub port: u16,
    pub name: String,
    pub paths: Vec<PathBuf>,
    pub localhost: bool,  // NEW — default: false
}
```

```rust
// Extend src/cli.rs
#[derive(Parser, Debug)]
pub struct Args {
    // existing fields ...
    #[arg(long, help = "Bind to localhost only (127.0.0.1) instead of all interfaces")]
    pub localhost: bool,  // flag presence = true
}
```

### Anti-Patterns to Avoid

- **Using `:id` path syntax:** axum 0.8 uses `{id}` syntax. `:id` is the old axum 0.7 syntax and will NOT match routes.
- **Using `spawn_blocking` for file I/O:** Context.md locked `tokio::fs` (async I/O). Use `tokio::fs::File::open().await`.
- **Returning 400 for malformed UUID:** Decision is locked to 404 for all unresolvable IDs.
- **Hand-rolling Range header parsing:** Suffix ranges (`bytes=-500`), multi-range, and boundary validation are subtle. Use `http-range-header`.
- **Forgetting HEAD request optimization:** axum automatically strips body for HEAD requests on GET routes, but the handler still runs and opens the file. For Samsung compatibility (it sends HEAD before every GET), consider registering an explicit `.head()` handler that skips file open.
- **Blocking during `axum::serve`:** The scan is synchronous and must complete before `axum::serve` is called. Do not attempt to run scan and server concurrently in Phase 3.
- **Cloning MediaLibrary items eagerly:** The library is read-locked, items are cloned out with `cloned()` on the found item before releasing the lock — this is the correct pattern.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Range header parsing | Custom `Range: bytes=X-Y` string parser | `http-range-header` | Suffix ranges, multi-range coalescing, boundary validation, fuzz-tested |
| 206 Partial Content response | Manual Content-Range header + sliced body | `axum_extra::response::file_stream::FileStream::into_range_response()` | Handles correct 206 status, Content-Range format, and chunked delivery |
| Request/response logging | Custom `from_fn` middleware | `tower_http::trace::TraceLayer` | Purpose-built, integrates with tracing spans, includes latency automatically |
| File streaming body | Custom `AsyncRead` → HTTP body adapter | `tokio_util::io::ReaderStream` + `axum::body::Body::from_stream()` | Standard pattern; handles backpressure correctly |

**Key insight:** HTTP range request handling has many RFC-specified edge cases (suffix ranges, unsatisfiable ranges, multi-range requests, validation against content length). The combination of `http-range-header` for parsing and `FileStream::into_range_response()` for response construction eliminates all of this complexity.

---

## Common Pitfalls

### Pitfall 1: axum 0.8 Path Syntax Breaking Change

**What goes wrong:** Routes defined with `:id` syntax don't match any requests; handler is never called; all requests return 404.
**Why it happens:** axum 0.8.0 (released January 2025) changed path parameter syntax from `/:id` (axum 0.7) to `/{id}` (axum 0.8).
**How to avoid:** Always use `/{id}` in route definitions. For the media route: `.route("/media/{id}", get(serve_media))`.
**Warning signs:** Handler function never called; tracing shows no matching route.

### Pitfall 2: HEAD Request Opens File Unnecessarily

**What goes wrong:** Samsung TV sends HEAD before every GET. The GET handler opens the file just to satisfy HEAD, creating unnecessary disk I/O.
**Why it happens:** axum auto-handles HEAD by running the GET handler and stripping the body.
**How to avoid:** Register an explicit `.head()` handler on the same route that returns headers without opening the file. Samsung requires HEAD to work correctly.
**Warning signs:** Double file opens in logs; disk activity before every stream.

```rust
.route("/media/{id}",
    get(serve_media_get).head(serve_media_head)
)
```

### Pitfall 3: IPv6 Dual-Bind "Address Already in Use"

**What goes wrong:** Binding `0.0.0.0:{port}` and `[::]:{port}` on Linux without `IPV6_V6ONLY=true` fails because the IPv6 socket already accepts IPv4 connections, making both binds conflict.
**Why it happens:** Linux kernel by default has `IPV6_V6ONLY=false` (accepts IPv4-mapped IPv6 addresses), so `[::]:{port}` covers IPv4 too.
**How to avoid:** Either (a) set `SO_REUSEPORT` on both sockets, or (b) use `tokio::net::TcpSocket::new_v6()`, call `.set_only_v6(true)` on the raw socket before binding. macOS defaults to `IPV6_V6ONLY=true` so this may only manifest on Linux.
**Warning signs:** Server panics on startup with "Address already in use (os error 98)".

### Pitfall 4: Releasing Lock Before Async File Open

**What goes wrong:** Holding `RwLock` guard across an `.await` causes a compile error ("Future is not Send" or "cannot be held across await point").
**Why it happens:** `std::sync::RwLock` guards are not `Send`; the async executor may move the future between threads at an `.await` point.
**How to avoid:** Clone the needed data out of the locked scope before any `.await`. Pattern: `{ let item = lib.read().unwrap().find(...).cloned() }` — lock is released before the block ends.
**Warning signs:** Compile error mentioning RwLock guard not being Send.

### Pitfall 5: Content-Range Header Format in 416

**What goes wrong:** 416 response has wrong Content-Range format; DLNA clients don't recognize the error.
**Why it happens:** RFC 7233 requires `Content-Range: bytes */total_size` (not `bytes 0-0/total_size`) for 416 responses.
**How to avoid:** Use `format!("bytes */{}", file_size)` for 416 Content-Range header.
**Warning signs:** Clients retry infinitely; curl shows 416 without Content-Range.

### Pitfall 6: tokio features not enabled

**What goes wrong:** `tokio::fs::File::open` or `tokio::net::TcpListener::bind` not found at compile time.
**Why it happens:** tokio requires explicit feature flags; `#[tokio::main]` requires `macros` + `rt-multi-thread`; `tokio::fs` requires `fs`; `tokio::net` requires `net`.
**How to avoid:** In `Cargo.toml` add `tokio = { version = "1", features = ["macros", "rt-multi-thread", "fs", "net", "io-util"] }`.
**Warning signs:** Compile error "no function named `main` in module `tokio`" or "no module named `fs`".

### Pitfall 7: DLNA.ORG_FLAGS Wrong Length

**What goes wrong:** Samsung TV ignores or rejects `contentFeatures.dlna.org` header if `DLNA.ORG_FLAGS` is not exactly 32 hex characters (8 significant + 24 zero-padding).
**Why it happens:** DLNA spec requires DLNA.ORG_FLAGS to be a 32-character hex string.
**How to avoid:** Use the constant: `"01700000000000000000000000000000"` — that is exactly 32 hex chars.
**Warning signs:** Samsung TV shows file but cannot seek; contentFeatures header present but malformed.

---

## Code Examples

### Complete DLNA Header Constants

```rust
// Source: https://github.com/anacrolix/dms/blob/master/dlna/dlna.go (verified pattern)
// DLNA.ORG_OP bits: bit 1 = TimeSeek (0=no), bit 0 = Range (1=yes) → "01"
// DLNA.ORG_CI: 0 = not converted/transcoded
// DLNA.ORG_FLAGS: 32 hex chars
//   01700000 = STREAMING_TRANSFER_MODE | BACKGROUND_TRANSFER_MODE | CONNECTION_STALL | DLNA_V15
//   followed by 24 zero chars
const DLNA_CONTENT_FEATURES: &str =
    "DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000";
const DLNA_TRANSFER_MODE: &str = "Streaming";
const DLNA_ACCEPT_RANGES: &str = "bytes";
```

### File Streaming Body (Full GET)

```rust
// Source: https://github.com/tokio-rs/axum/discussions/608
use tokio_util::io::ReaderStream;
use axum::body::Body;

let file = tokio::fs::File::open(&item.path).await
    .map_err(|e| {
        tracing::error!("Cannot open {}: {}", item.path.display(), e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
let stream = ReaderStream::new(file);
let body = Body::from_stream(stream);
```

### Range Response (206 Partial Content)

```rust
// Source: https://docs.rs/axum-extra/latest/axum_extra/response/file_stream/
// Source: https://docs.rs/http-range-header/latest/http_range_header/
use axum_extra::response::file_stream::FileStream;
use http_range_header::parse_range_header;

async fn range_response(
    range_str: &str,
    path: &std::path::Path,
    file_size: u64,
) -> Response {
    let parsed = match parse_range_header(range_str) {
        Ok(p) => p,
        Err(_) => return (
            StatusCode::RANGE_NOT_SATISFIABLE,
            [("content-range", format!("bytes */{}", file_size))],
        ).into_response(),
    };

    let ranges = match parsed.validate(file_size) {
        Ok(r) => r,
        Err(_) => return (
            StatusCode::RANGE_NOT_SATISFIABLE,
            [("content-range", format!("bytes */{}", file_size))],
        ).into_response(),
    };

    // Take first range only (CONTEXT.md decision)
    let first = ranges.into_iter().next().unwrap();
    let start = first.start();
    let end = first.end(); // inclusive

    // FileStream::try_range_response opens file, seeks, and builds 206
    match FileStream::try_range_response(path, start, end).await {
        Ok(resp) => resp,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
```

### TraceLayer for Request Logging

```rust
// Source: https://docs.rs/tower-http/latest/tower_http/trace/
use tower_http::trace::{TraceLayer, DefaultOnRequest, DefaultOnResponse};
use tracing::Level;

let layer = TraceLayer::new_for_http()
    .on_request(DefaultOnRequest::new().level(Level::INFO))
    .on_response(DefaultOnResponse::new().level(Level::INFO));

// Produces: INFO GET /media/550e8400-... 200 OK in 2ms
```

### UUID Lookup Pattern (Safe Lock Usage)

```rust
// Clone item OUT of lock scope before any .await
let item: Option<MediaItem> = {
    let lib = state.library.read().unwrap();
    lib.items.iter().find(|i| i.id == id).cloned()
};
// Lock is released here; safe to .await below
match item {
    None => return StatusCode::NOT_FOUND.into_response(),
    Some(item) => {
        // safe to .await now
        let file = tokio::fs::File::open(&item.path).await...
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `/:param` route syntax | `/{param}` route syntax | axum 0.8.0 (Jan 2025) | Breaking change — all routes must use new syntax |
| `#[async_trait]` on custom extractors | Native async traits (no macro) | axum 0.8.0 (Jan 2025) | Simpler extractor code |
| `StreamBody::new(stream)` | `Body::from_stream(stream)` | axum 0.7+ | StreamBody deprecated |
| Multiple listeners via `serve_with_incoming` | Multiple `tokio::spawn(axum::serve(...))` | axum 0.8.x | Example removed; spawn pattern is now canonical |
| Custom file streaming | `axum_extra::response::file_stream::FileStream` | axum-extra 0.9+ (Dec 2024) | Official solution; range support built-in |
| `Option<T>` extractor silently converts rejections to None | `Option<T>` requires `OptionalFromRequestParts` | axum 0.8.0 | Relevant if HEAD uses Option<Range> extractor |

**Deprecated/outdated:**
- `axum::extract::extractor_middleware`: removed; use `axum::middleware::from_extractor`
- `StreamBody`: replaced by `Body::from_stream`
- `serve_with_incoming`: removed in axum 0.8.x; use multiple `axum::serve` + `tokio::spawn`

---

## Open Questions

1. **IPv6_V6ONLY behavior on target OS**
   - What we know: macOS defaults to `IPV6_V6ONLY=true` (separate stacks); Linux defaults to `false` (shared stack)
   - What's unclear: Whether the user's deployment target is Linux or macOS; whether dual-bind will conflict on Linux
   - Recommendation: Implement with `TcpSocket::new_v6().set_only_v6(true)` explicitly to ensure portable behavior regardless of OS defaults

2. **FileStream::try_range_response DLNA header inclusion**
   - What we know: `try_range_response` returns a 206 Response with `Content-Range`
   - What's unclear: Whether it automatically includes `Accept-Ranges`, `transferMode.dlna.org`, and `contentFeatures.dlna.org` — it almost certainly does NOT since those are DLNA-specific
   - Recommendation: After calling `try_range_response`, insert DLNA headers into the returned Response's HeaderMap before returning; or build the 206 response manually using `into_range_response` which returns a raw Response that can be modified

3. **http-range-header API: exact method names for start/end**
   - What we know: ParsedRanges has `.validate(file_size)` which returns concrete ranges; each range has start/end accessors
   - What's unclear: Exact method names on the validated range struct (`first.start()`, `first.end()`, etc.)
   - Recommendation: Check docs.rs/http-range-header at implementation time; the API is small and well-documented

---

## Sources

### Primary (HIGH confidence)

- [docs.rs/axum/latest](https://docs.rs/axum/latest/axum/) — State extractor, Router API, serve function, route syntax
- [docs.rs/axum-extra/latest/axum_extra/response/file_stream/](https://docs.rs/axum-extra/latest/axum_extra/response/file_stream/struct.FileStream.html) — FileStream API, into_range_response, try_range_response
- [docs.rs/tower-http/latest/tower_http/trace/](https://docs.rs/tower-http/latest/tower_http/trace/struct.TraceLayer.html) — TraceLayer API
- [docs.rs/http-range-header](https://docs.rs/http-range-header/latest/http_range_header/) — parse_range_header API
- cargo search axum (2026-02-22) — confirmed axum=0.8.8, axum-extra=0.12.5, tower-http=0.6.8, http-range-header=0.4.2, tokio-util=0.7.18

### Secondary (MEDIUM confidence)

- [tokio.rs — Announcing axum 0.8.0](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0) — Breaking changes confirmed: path syntax change, async trait removal
- [github.com/tokio-rs/axum/discussions/2949](https://github.com/tokio-rs/axum/discussions/2949) — Multiple listeners pattern: separate tokio::spawn + axum::serve confirmed as canonical approach
- [github.com/tokio-rs/axum/pull/3047](https://github.com/tokio-rs/axum/pull/3047) — FileStream merged Dec 4, 2024; file-stream feature confirmed
- [anacrolix/dms dlna.go](https://github.com/anacrolix/dms/blob/master/dlna/dlna.go) — DLNA.ORG_FLAGS=01700000... pattern; cross-verified with Samsung TV reports
- [lib.rs/crates/axum-extra](https://lib.rs/crates/axum-extra) — Confirmed axum-extra 0.12.5 released Dec 27, 2025; file-stream is valid feature flag

### Tertiary (LOW confidence)

- Multiple Samsung TV forum posts confirming `DLNA.ORG_FLAGS=01700000000000000000000000000000` — not from official DLNA spec docs; pattern is consistent across multiple independent DLNA server implementations
- IPv6_V6ONLY OS default behavior — documented in Linux kernel docs and macOS man pages but OS-specific runtime behavior should be verified on target platform

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — verified via cargo search (exact versions confirmed 2026-02-22)
- Architecture: HIGH — axum 0.8 State, routing, and serve patterns verified from official docs
- DLNA header values: MEDIUM — consistent across multiple DLNA server implementations but not from official DLNA spec document
- Pitfalls: HIGH — most sourced from official axum changelog, docs, and GitHub discussions
- Dual-bind IPv6_V6ONLY behavior: MEDIUM — OS-specific; needs runtime testing

**Research date:** 2026-02-22
**Valid until:** 2026-03-22 (axum is stable; axum-extra 0.12.x is recent but FileStream API is merged and stable)
