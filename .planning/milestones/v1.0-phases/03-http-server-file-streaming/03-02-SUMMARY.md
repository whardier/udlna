---
phase: 03-http-server-file-streaming
plan: "02"
subsystem: api
tags: [rust, axum, dlna, http-range-header, tokio, streaming, rfc7233]

# Dependency graph
requires:
  - phase: 03-http-server-file-streaming
    provides: AppState with Arc<RwLock<MediaLibrary>>, axum 0.8 router scaffold, stub serve_media_get/serve_media_head
  - phase: 02-media-scanner-and-metadata
    provides: MediaItem struct with id (UUIDv5), path (PathBuf), file_size (u64), mime (&'static str), kind, meta
provides:
  - "serve_media_get: 200 full GET (streaming), 206 Range GET, 416 unsatisfiable, 404 unknown/malformed, 500 missing file"
  - "serve_media_head: 200 with all DLNA headers, no body, no file open"
  - "DLNA headers on all responses: Accept-Ranges, transferMode.dlna.org, contentFeatures.dlna.org with 32-char FLAGS"
  - "lookup_item() helper: UUID parse + RwLock read, releases lock before any .await"
  - "dlna_headers() helper: builds standard HeaderMap for all media responses"
affects:
  - 03-03-main-wiring
  - 04-device-description-xml
  - 05-contentdirectory-soap

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Lock-before-await: RwLock guard released inside sync helper fn before any .await to avoid Send issues"
    - "First-range-only: multi-part Range requests validated by http-range-header, first RangeInclusive taken"
    - "Manual seek+take for 206: File::open -> seek(SeekFrom::Start) -> take(length) -> ReaderStream -> Body::from_stream"
    - "DLNA headers merged: dlna_headers() builds HeaderMap, Content-Range/Content-Length overridden for 206"

key-files:
  created: []
  modified:
    - src/http/media.rs

key-decisions:
  - "FileStream::try_range_response not used — manual seek+take pattern gives full control over DLNA header injection on 206 responses"
  - "http-range-header validates overlapping ranges at validate() — all validation errors uniformly return 416"
  - "MediaItem.mime is &'static str — HeaderValue::from_static(item.mime) used directly (no allocation)"
  - "Content-Length overridden to partial length (end - start + 1) on 206 responses for correct transfer"

patterns-established:
  - "Pattern: Lock-before-await — lookup_item() drops RwLock guard before returning, enabling safe use before async operations"
  - "Pattern: DLNA header injection — dlna_headers() returns HeaderMap, 206 responses clone and extend with Content-Range"

requirements-completed: [STRM-01, STRM-02, STRM-03, STRM-04, STRM-05, STRM-06]

# Metrics
duration: 2min
completed: 2026-02-22
---

# Phase 3 Plan 02: Media Handler Summary

**RFC 7233 Range-aware GET and no-disk-I/O HEAD handlers with DLNA.ORG_FLAGS/OP/CI headers for Samsung TV streaming**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-22T23:37:37Z
- **Completed:** 2026-02-22T23:39:10Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- `serve_media_head`: returns 200 + 5 DLNA headers with zero file I/O — eliminates unnecessary disk open on Samsung TV pre-flight requests
- `serve_media_get`: RFC 7233 compliant — full 200 streaming, 206 partial with correct Content-Range, 416 for unsatisfiable, 404 for bad/missing ID, 500 + tracing::error! for missing file
- All 6 DLNA streaming requirements (STRM-01 through STRM-06) satisfied in a single 189-line file

## Task Commits

Each task was committed atomically:

1. **Task 1: serve_media_head with full DLNA headers** - `5cf0029` (feat)
2. **Task 2: serve_media_get with Range support** - `119daea` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified
- `src/http/media.rs` - Complete media handler: lookup_item(), dlna_headers(), serve_media_head(), serve_media_get(), range_response()

## Decisions Made
- **FileStream::try_range_response not used**: The axum-extra helper double-validates end >= total_size and sets Content-Type to application/octet-stream. Manual seek+take pattern avoids these issues and allows direct DLNA header injection on 206 responses.
- **First-range-only for multi-part**: `http-range-header` `validate()` rejects overlapping ranges as errors; for non-overlapping multi-part, we take the first `RangeInclusive<u64>` via `into_iter().next()` per CONTEXT.md locked decision.
- **Content-Length override on 206**: `dlna_headers()` sets Content-Length to `item.file_size`; range_response() overrides it to `end - start + 1` for correct partial transfer signaling.

## Deviations from Plan

None — plan executed exactly as written. The plan's noted alternative (manual seek+take instead of FileStream::try_range_response) was the implementation choice made after inspecting the actual API.

## Issues Encountered
None. The `http-range-header` API inspection confirmed `validate()` returns `Vec<RangeInclusive<u64>>`, so `*first.start()` / `*first.end()` (not `.start()` / `.end()` as method calls on integers) were correct.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `serve_media_get` and `serve_media_head` are production-ready and wired into the router in `src/http/mod.rs`
- Plan 03 (main.rs wiring) will connect AppState with actual library, bind the server, and enable end-to-end integration testing
- No blockers — all 6 streaming requirements satisfied

---
*Phase: 03-http-server-file-streaming*
*Completed: 2026-02-22*
