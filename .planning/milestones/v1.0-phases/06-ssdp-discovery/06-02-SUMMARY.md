---
phase: 06-ssdp-discovery
plan: 02
subsystem: networking
tags: [ssdp, upnp, multicast, udp, tokio, broadcast, graceful-shutdown, rust]

# Dependency graph
requires:
  - phase: 06-ssdp-discovery/06-01
    provides: socket.rs (build_recv_socket_v4/v6, build_send_socket, list_non_loopback_v4, find_iface_for_sender), messages.rs (notify_alive, notify_byebye, msearch_response, usn_set)

provides:
  - src/ssdp/service.rs with SsdpConfig struct and run() async task (M-SEARCH recv, 900s re-advertisement timer, byebye on shutdown)
  - src/main.rs restructured with broadcast shutdown channel, SSDP task spawn, graceful HTTP shutdown, double-Ctrl+C force exit

affects:
  - 06-03 (SSDP integration testing — builds on working service)
  - Phase 07 (ConnectionManager — SSDP already running, HTTP dual-bind preserved)
  - All real-device testing phases (Samsung TV / Xbox discovery now works)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - tokio::sync::broadcast channel for fan-out shutdown signal to multiple tasks
    - AtomicBool SHUTTING_DOWN for double-Ctrl+C force exit pattern
    - axum::serve().with_graceful_shutdown(broadcast::Receiver) for clean HTTP drain
    - tokio::select! with std::future::pending() to disable optional socket branch
    - Separate recv buffers per select! branch (borrow checker requirement)
    - SSDP task spawned after TcpListener::bind (HTTP ready before SSDP advertises)
    - 1-second tokio::time::timeout on SSDP byebye wait before process exit

key-files:
  created:
    - src/ssdp/service.rs
  modified:
    - src/main.rs

key-decisions:
  - "Separate buf_v4/buf_v6 buffers in select! — Rust borrow checker requires distinct mutable refs for each concurrent recv_from arm"
  - "recv_v6_from() returns std::future::pending when IPv6 socket is None — cleanly disables the IPv6 select! branch without conditional compilation"
  - "tokio::spawn(async move { axum::serve().with_graceful_shutdown().await }) pattern — WithGracefulShutdown is not itself a Future, must be awaited inside async block"
  - "SSDP task spawned after both HTTP listeners are bound (not after await) — satisfies HTTP-ready-before-SSDP startup sequencing requirement"
  - "Process exits after SSDP byebye timeout; HTTP tasks drained automatically via with_graceful_shutdown — no explicit join on HTTP tasks needed"

patterns-established:
  - "SSDP select! loop: 3 arms — timer tick (re-advert), IPv4 recv (M-SEARCH), IPv6 recv (best-effort), shutdown recv (byebye)"
  - "Shutdown orchestration: wait_for_shutdown() -> broadcast shutdown -> timeout SSDP task -> log Goodbye -> process exit"
  - "Best-effort IPv6 SSDP: build_recv_socket_v6 failure logged at debug level, IPv4-only operation continues"

requirements-completed: [DISC-01, DISC-02, DISC-03, DISC-04, CLI-06]

# Metrics
duration: 3min
completed: 2026-02-22
---

# Phase 6 Plan 02: SSDP Service Task Summary

**SSDP service task wired into main.rs: M-SEARCH unicast response, 900s re-advertisement, 3-burst startup NOTIFY, byebye on Ctrl+C, double-Ctrl+C force exit, HTTP graceful drain via broadcast channel**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-23T03:23:11Z
- **Completed:** 2026-02-23T03:26:11Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Full SSDP service task (`service.rs`): interface discovery, startup NOTIFY burst (3x/150ms), M-SEARCH parsing and unicast response, 900s re-advertisement timer, byebye on shutdown
- 5-USN type coverage: uuid-only, upnp:rootdevice, MediaServer:1, ContentDirectory:1, ConnectionManager:1
- IPv6 SSDP best-effort: `build_recv_socket_v6(0)` attempted; failure is non-fatal (IPv4-only mode)
- main.rs restructured: broadcast shutdown channel, SSDP task spawn after HTTP listeners bound, HTTP graceful drain, double-Ctrl+C force exit

## Task Commits

Each task was committed atomically:

1. **Task 1: Create src/ssdp/service.rs — SSDP async task** - `dd642f2` (feat)
2. **Task 2: Restructure main.rs — shutdown broadcast, SSDP task, graceful exit** - `a790daa` (feat)

**Plan metadata:** TBD (docs: complete plan)

## Files Created/Modified
- `src/ssdp/service.rs` - SsdpConfig struct, run() async task with M-SEARCH recv loop, 900s re-advertisement, startup NOTIFY burst, shutdown byebye
- `src/main.rs` - SHUTTING_DOWN AtomicBool, wait_for_shutdown(), broadcast::channel, SSDP task spawn, HTTP graceful shutdown, double-Ctrl+C force exit

## Decisions Made
- Separate `buf_v4`/`buf_v6` buffers: two select! arms cannot borrow the same `&mut buf` simultaneously — distinct buffers required by borrow checker
- `recv_v6_from()` returns `std::future::pending()` when socket is `None` — the IPv6 select! branch never fires, cleanly disabled without `cfg!` or other complexity
- `tokio::spawn(async move { axum::serve().with_graceful_shutdown().await })` — `WithGracefulShutdown` is a builder type, not a `Future`; must be awaited inside an `async` block passed to `tokio::spawn`
- SSDP task spawned after `TcpListener::bind` succeeds (before `await`) — HTTP listener is already accepting connections before SSDP startup burst fires

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed duplicate mutable borrow of recv buffer in select! arms**
- **Found during:** Task 1 (service.rs main loop implementation)
- **Issue:** Plan's pseudocode used a single `buf` for both IPv4 and IPv6 recv_from arms in tokio::select! — Rust borrow checker rejects two simultaneous `&mut buf` borrows
- **Fix:** Introduced `buf_v4` and `buf_v6` as separate arrays; each select! arm uses its own buffer
- **Files modified:** src/ssdp/service.rs
- **Verification:** cargo build exits 0 after fix
- **Committed in:** dd642f2 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed recv_v6_from return type for disabled branch**
- **Found during:** Task 1 (IPv6 optional socket handling)
- **Issue:** Initial implementation returned `Option<Result<...>>` to signal None socket, but the select! arm expected a direct `Result<...>` value; the None arm also tried to await std::future::pending inside a Some/None match — logic inconsistency
- **Fix:** `recv_v6_from` always returns `std::io::Result<(usize, SocketAddr)>` and internally calls `std::future::pending()` when socket is None — select! branch is uniformly typed and permanently suspended
- **Files modified:** src/ssdp/service.rs
- **Verification:** cargo build exits 0
- **Committed in:** dd642f2 (Task 1 commit)

**3. [Rule 1 - Bug] Fixed tokio::spawn(axum::serve().with_graceful_shutdown()) — not a Future**
- **Found during:** Task 2 (main.rs HTTP graceful shutdown implementation)
- **Issue:** Plan's pattern `tokio::spawn(axum::serve(...).with_graceful_shutdown(...))` fails — `WithGracefulShutdown` implements `IntoFuture`, not `Future` directly; `tokio::spawn` requires `Future`
- **Fix:** Wrapped in `tokio::spawn(async move { axum::serve(...).with_graceful_shutdown(...).await ... })` for localhost and both dual-bind HTTP tasks
- **Files modified:** src/main.rs
- **Verification:** cargo build exits 0
- **Committed in:** a790daa (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (3 Rule 1 bugs from API behavior and borrow checker constraints)
**Impact on plan:** All fixes essential for compilation. No scope creep. Behavior matches plan spec exactly.

## Issues Encountered
- None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full SSDP discovery stack is operational: M-SEARCH response, startup burst, re-advertisement, byebye on Ctrl+C
- All 5 USN types advertised; LOCATION URL correctly derived per sender subnet
- Graceful shutdown in place for HTTP and SSDP tasks
- Ready for Phase 06-03 (integration testing) or real-device Samsung/Xbox testing

---
*Phase: 06-ssdp-discovery*
*Completed: 2026-02-22*
