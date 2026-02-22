---
phase: 03-http-server-file-streaming
plan: "01"
subsystem: http
tags: [axum, tower-http, tokio, rust, dlna, http-server]

# Dependency graph
requires:
  - phase: 02-media-scanner-and-metadata
    provides: MediaLibrary struct with items, MediaItem with file_size, Arc<RwLock<MediaLibrary>> pattern
provides:
  - axum 0.8 HTTP server dependencies (axum, axum-extra, tower-http, http-range-header, tokio, tokio-util)
  - AppState struct wrapping Arc<RwLock<MediaLibrary>> for thread-safe access in handlers
  - build_router function with all 6 routes registered using axum 0.8 {id} syntax
  - localhost: bool config/CLI flag for dual-bind vs localhost-only mode
  - Stub media handlers (serve_media_get, serve_media_head) returning 501 until Plan 02
affects:
  - 03-02 (media file handler — fills in serve_media_get/serve_media_head stubs)
  - 03-03 (server bind — calls build_router and binds sockets)
  - 04-upnp-device-description (replaces /device.xml 501 stub)
  - 05-content-directory-service (replaces /cds/scpd.xml and /cds/control stubs)

# Tech tracking
tech-stack:
  added:
    - "axum 0.8 (HTTP router and extractors, {id} path syntax)"
    - "axum-extra 0.12 with file-stream feature (efficient file streaming)"
    - "tower-http 0.6 with trace feature (request/response tracing layer)"
    - "http-range-header 0.4 (Range request parsing for STRM-07)"
    - "tokio 1 with macros+rt-multi-thread+fs+net+io-util features"
    - "tokio-util 0.7 with io feature (ReaderStream for body streaming)"
  patterns:
    - "AppState pattern: Arc<RwLock<T>> wrapped in Clone struct for axum State extractor"
    - "Router scaffold pattern: all routes declared up front, stubs return 501 until implemented"
    - "axum 0.8 path syntax: {id} not :id (breaking change from 0.7)"

key-files:
  created:
    - src/http/mod.rs
    - src/http/state.rs
    - src/http/media.rs
  modified:
    - Cargo.toml
    - src/cli.rs
    - src/config.rs
    - src/main.rs

key-decisions:
  - "AppState uses std::sync::RwLock (not tokio::sync::RwLock) — library is write-once at startup, read-only thereafter; switch to tokio if Phase 6 SIGHUP rescan is implemented"
  - "localhost flag is a plain bool in Args (not Option<bool>) — flag presence = true, absence = false, merged with TOML Option<bool>"
  - "Phase 4/5 stub routes return StatusCode::NOT_IMPLEMENTED (501) inline closures — replaced in later plans without touching router registration"
  - "axum 0.8 {id} path syntax confirmed in router — not :id which is 0.7 syntax"

patterns-established:
  - "All routes are declared in build_router once — subsequent plans implement handlers without touching route registration"
  - "AppState::Clone is derived — cheap Arc clone per request with no data duplication"

requirements-completed:
  - STRM-07

# Metrics
duration: 3min
completed: 2026-02-22
---

# Phase 3 Plan 01: HTTP Foundation Summary

**axum 0.8 router scaffold with AppState(Arc<RwLock<MediaLibrary>>), all 6 DLNA routes registered, localhost bind flag, and HTTP dep additions to Cargo.toml**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-02-22T23:32:19Z
- **Completed:** 2026-02-22T23:34:39Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added all HTTP dependencies (axum 0.8, axum-extra 0.12, tower-http 0.6, http-range-header 0.4, tokio 1, tokio-util 0.7) to Cargo.toml
- Created AppState wrapping Arc<RwLock<MediaLibrary>> — injectable shared state for all route handlers via axum::extract::State
- Created build_router with all 6 DLNA routes registered (axum 0.8 {id} syntax), Phase 4/5 stubs returning 501
- Extended Config and CLI with localhost: bool flag for single-interface vs dual-stack binding
- All 48 tests pass after auto-fixing FileConfig struct literal initialization

## Task Commits

Each task was committed atomically:

1. **Task 1: Add HTTP dependencies and localhost config/CLI flag** - `650a7bd` (feat)
2. **Task 2: Create http module with AppState, router scaffold, media stubs** - `67a1089` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `Cargo.toml` - Added 6 HTTP dependencies
- `src/cli.rs` - Added --localhost bool flag to Args struct
- `src/config.rs` - Added localhost: Option<bool> in FileConfig, localhost: bool in Config, resolve merge logic, test helper update, new test
- `src/http/mod.rs` - build_router with 6 routes (2 media + 4 stubs), TraceLayer, with_state(state)
- `src/http/state.rs` - AppState with Arc<RwLock<MediaLibrary>>, derives Clone
- `src/http/media.rs` - serve_media_get and serve_media_head stubs returning 501
- `src/main.rs` - Added mod http; declaration

## Decisions Made

- AppState uses std::sync::RwLock: library is write-once at startup, read-only for server lifetime; if SIGHUP rescan is implemented in Phase 6+, switch to tokio::sync::RwLock
- localhost is a plain bool in Args (flag presence = true): merged with TOML Option<bool> via `args.localhost || file.localhost.unwrap_or(false)`
- All route stubs return inline 501 closures rather than named handlers — named handlers will replace them in later plans without touching route registration
- axum 0.8 {id} path syntax used throughout — critical difference from 0.7 :id syntax

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed FileConfig struct literal initialization missing localhost field**
- **Found during:** Task 2 (post-task test run)
- **Issue:** Two test cases in config.rs constructed FileConfig using struct literal syntax `{ port: Some(7777), name: None }` — after adding `localhost: Option<bool>` to FileConfig, these literals caused compile errors `missing field 'localhost'`
- **Fix:** Added `localhost: None` to both FileConfig struct literals in the test functions `test_toml_overrides_default` and `test_cli_overrides_toml`
- **Files modified:** src/config.rs
- **Verification:** `cargo test` — all 48 tests pass with no errors
- **Committed in:** 67a1089 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - bug)
**Impact on plan:** Necessary correctness fix — struct literal initialization must include all fields. No scope creep.

## Issues Encountered

None beyond the auto-fixed FileConfig literal issue above.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- HTTP foundation complete: all routes declared, AppState injectable, TraceLayer wired
- Plan 02 (media handler) can implement serve_media_get and serve_media_head without touching router or state
- Plan 03 (server bind) can call build_router and bind sockets using config.localhost and config.port
- No blockers

## Self-Check: PASSED

- src/http/mod.rs: FOUND
- src/http/state.rs: FOUND
- src/http/media.rs: FOUND
- 03-01-SUMMARY.md: FOUND
- Commit 650a7bd: FOUND
- Commit 67a1089: FOUND
- Commit 41ee84f (docs): FOUND

---
*Phase: 03-http-server-file-streaming*
*Completed: 2026-02-22*
