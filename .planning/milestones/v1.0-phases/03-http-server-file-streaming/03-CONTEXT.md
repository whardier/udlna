# Phase 3: HTTP Server & File Streaming - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Stand up an axum HTTP server that streams media files from the Phase 2 library with full Range request support and all DLNA-required response headers. Any HTTP client (curl, browser, media player) can fetch or seek into any file in the library. This phase also scaffolds the full router for Phases 4–5 routes. ContentDirectory SOAP, device XML, and SSDP are later phases.

</domain>

<decisions>
## Implementation Decisions

### HTTP framework
- **axum** (built on hyper) — not raw hyper. Router, extractors, State injection built-in.
- `#[tokio::main]` on `main.rs` — single tokio runtime for the whole process
- Bind address is **configurable** via a `localhost` config flag/option:
  - `localhost = true` → bind to `127.0.0.1:{port}` only
  - `localhost = false` (default) → **dual-bind**: both `0.0.0.0:{port}` (IPv4) and `:::{port}` (IPv6) as separate sockets
- File streaming uses **tokio::fs** (async I/O), not spawn_blocking

### Range request handling
- Claude's discretion for edge cases (RFC 7233):
  - Unsatisfied range → 416 Range Not Satisfiable with `Content-Range: bytes */total_size`
  - Multi-part ranges → serve first range only (ignore additional parts)
  - Suffix ranges (bytes=-500) → supported (compute from file size)
- Byte streaming: `tokio::fs::File` + axum `Body::from_stream` with chunked reads

### Router structure
- Phase 3 establishes the **full router scaffold**:
  - `/media/:id` — implemented and working (GET + HEAD)
  - `/device.xml` — 501 stub (Phase 4)
  - `/cds/scpd.xml` — 501 stub (Phase 4)
  - `/cms/scpd.xml` — 501 stub (Phase 4)
  - `/cds/control` — 501 stub (Phase 5)
  - `/cms/control` — 501 stub (Phase 5)
- Media ID in URL is the **UUID string** (matches Phase 2 UUIDv5 IDs) — e.g., `/media/550e8400-e29b-41d4-a716-446655440000`
- App state injected via `axum::extract::State<AppState>` — Claude decides exact AppState shape

### Error responses
- 404 for unknown media ID: **empty body** (no text, no HTML)
- Invalid/malformed UUID in path: **404** (not 400 — treat all unresolvable IDs as 404)
- File missing at serve time (in library but gone from disk): **500 Internal Server Error + tracing::error! log**
- All requests logged at **info level**: method, path, status, duration
- 501 stub routes return minimal body (Claude decides — "Not Implemented" text or empty)

### DLNA response headers (Claude's discretion for values)
- All media responses MUST include:
  - `Accept-Ranges: bytes`
  - `transferMode.dlna.org: Streaming`
  - `contentFeatures.dlna.org` with `DLNA.ORG_OP=01` and `DLNA.ORG_FLAGS`
- HEAD requests return 200 with all headers but no body
- `Content-Type` from the MediaItem's mime field

</decisions>

<specifics>
## Specific Ideas

- Dual-bind (IPv4 + IPv6 explicit sockets) is a hard requirement, not a suggestion — the user was specific about this
- The `localhost` flag needs to land in the config struct from Phase 1 (extend `Config` and CLI args)
- 501 stubs are intentional insertion points — Phase 4 replaces `/device.xml`, `/cds/scpd.xml`, `/cms/scpd.xml`; Phase 5 replaces control endpoints

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-http-server-file-streaming*
*Context gathered: 2026-02-22*
