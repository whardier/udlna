---
phase: 08-server-identity-customization
plan: 01
subsystem: server-identity
tags: [uuid-v5, hostname, server-name, dlna, ssdp, rust]

# Dependency graph
requires:
  - phase: 07-connectionmanager-service
    provides: CMS SOAP handler with SsdpConfig struct
  - phase: 06-ssdp-discovery
    provides: SSDP service, SsdpConfig, device.xml endpoint
provides:
  - hostname crate (0.4.2) integrated for dynamic server identity
  - Stable UUID v5 derived from hostname (NAMESPACE_DNS) — no state file needed
  - Dynamic default server name "udlna@{hostname}" when --name absent
  - --name flag wired end-to-end: AppState -> device.xml <friendlyName> + SSDP startup log
  - Startup banner: udlna "<name>" (uuid: <uuid>) on port <port>
  - Zero Rust compiler warnings (release build clean)
affects: [all-future-phases, dlna-clients]

# Tech tracking
tech-stack:
  added: [hostname 0.4.2]
  patterns:
    - UUID v5 from hostname using Uuid::NAMESPACE_DNS (mirrors build_machine_namespace pattern)
    - Dynamic default via fn (not const) for hostname-dependent values
    - xml_escape on user-provided server name before embedding in XML

key-files:
  created: []
  modified:
    - Cargo.toml
    - src/config.rs
    - src/http/state.rs
    - src/http/soap.rs
    - src/ssdp/socket.rs
    - src/main.rs
    - src/http/description.rs
    - src/ssdp/service.rs

key-decisions:
  - "UUID v5 uses Uuid::NAMESPACE_DNS with raw hostname bytes — mirrors build_machine_namespace() in media/metadata.rs"
  - "default_name() is a fn not a const — hostname is only available at runtime, not compile time"
  - "xml_escape applied to server_name in description.rs — protects against XML injection from user --name values"
  - "Two SsdpConfig construction sites in main.rs (localhost path + dual-bind path) both updated with server_name"
  - "Elided lifetime in Cow<str> fixed to Cow<'_, str> per compiler suggestion (no logic change)"
  - "#[allow(dead_code)] on IfaceV4.index — field kept for future IPv6 multicast use"

patterns-established:
  - "Server identity: AppState carries both server_uuid (UUID v5) and server_name (user-provided or default)"
  - "Banner format: udlna \"<name>\" (uuid: <uuid>) on port <port>"

requirements-completed: [CLI-08, DISC-05]

# Metrics
duration: 8min
completed: 2026-02-23
---

# Phase 8 Plan 01: Server Identity Customization Summary

**UUID v5 stable identity from hostname + user-customizable --name flag wired end-to-end through device.xml and SSDP, eliminating two compiler warnings; final v1 milestone complete**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-23T05:22:36Z
- **Completed:** 2026-02-23T05:30:00Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Replaced random UUID v4 with stable UUID v5 derived from hostname (same across all restarts)
- Added `--name` flag wired through `AppState.server_name` -> `device.xml <friendlyName>` + SSDP startup log
- Default name is `"udlna@{hostname}"` (e.g., `"udlna@MacBookPro"`) when `--name` not provided
- Fixed xml_escape lifetime annotation warning (`Cow<str>` -> `Cow<'_, str>`)
- Fixed IfaceV4.index unused field warning (`#[allow(dead_code)]`)
- `cargo build --release` produces zero warnings, 81 tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Add hostname crate, update config.rs default name, fix compiler warnings** - `63e1ae6` (feat)
2. **Task 2: Wire UUID v5 and server_name through main.rs, description.rs, and ssdp/service.rs** - `f070117` (feat)

**Plan metadata:** (docs commit hash — recorded after this summary is committed)

## Files Created/Modified
- `Cargo.toml` - Added `hostname = "0.4.2"` dependency
- `Cargo.lock` - Updated with hostname crate resolution
- `src/config.rs` - Replaced `DEFAULT_NAME` const with `default_name()` fn computing `"udlna@{hostname}"`; updated test
- `src/http/state.rs` - Added `server_name: String` field to `AppState`
- `src/http/soap.rs` - Fixed `xml_escape` return type: `Cow<str>` -> `Cow<'_, str>`
- `src/ssdp/socket.rs` - Added `#[allow(dead_code)]` on `IfaceV4.index`
- `src/main.rs` - Added `build_server_uuid()` + `get_hostname()` helpers; replaced UUID v4 with UUID v5; updated banner; updated both SsdpConfig construction sites with `server_name`
- `src/http/description.rs` - `<friendlyName>` now reads from `state.server_name` (xml_escaped)
- `src/ssdp/service.rs` - Added `server_name: String` to `SsdpConfig`; updated startup log

## Decisions Made
- **UUID v5 namespace:** Used `Uuid::NAMESPACE_DNS` with raw hostname bytes — consistent with existing `build_machine_namespace()` in `media/metadata.rs` (UUID v5 with machine-uid). Both use UUID v5 but different namespaces; server UUID is hostname-based so it's consistent across machines with the same name, while media UUIDs are machine-specific.
- **dynamic `default_name()` fn:** Cannot be a `const` because `hostname::get()` is a runtime system call. The fn pattern allows the fallback chain `args.name -> file.name -> default_name()` to work cleanly with `unwrap_or_else`.
- **XML escaping:** User-provided `--name` values flow into `device.xml` XML; `xml_escape()` applied at description.rs boundary to prevent malformed XML from names containing `<`, `>`, `&`, `"`, `'`.

## Deviations from Plan

### Auto-fixed Issue

**1. [Rule 3 - Blocking] Added minimal server_name to AppState construction in Task 1 commit**
- **Found during:** Task 1 (compiling after adding `server_name: String` to AppState)
- **Issue:** Adding `server_name` field to `AppState` required updating all construction sites or cargo build would fail with "missing field `server_name`". main.rs constructs AppState and was Task 2's scope.
- **Fix:** Added `server_name: config.name.clone()` to AppState construction in main.rs as part of Task 1 commit to allow build verification to pass. Task 2 then replaced the surrounding UUID/banner logic completely.
- **Files modified:** `src/main.rs`
- **Verification:** `cargo build` (zero errors, zero warnings) confirmed after Task 1
- **Committed in:** `63e1ae6` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (blocking — required for compilation)
**Impact on plan:** Minimal deviation; main.rs was updated slightly early (Task 1 instead of Task 2) for compilation to succeed. Task 2 completed the full main.rs rewrite. No scope creep.

## Issues Encountered
None beyond the minor compilation order issue above.

## User Setup Required
None - no external service configuration required. The hostname crate is a pure Rust crate with no system dependencies.

## Verification Results

```
$ cargo build --release 2>&1 | grep -c "^warning\["
0

$ cargo test 2>&1 | tail -3
test result: ok. 81 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.16s

$ curl -s http://localhost:8200/device.xml | grep friendlyName
    <friendlyName>udlna@MacBookPro</friendlyName>

$ cargo run -- --name "Living Room" /tmp 2>&1 | grep "Living Room"
udlna "Living Room" (uuid: 1e7aff3c-08bc-5529-aa9a-66c39e30b0c9) on port 8200
SSDP advertising "Living Room" on 192.168.4.111:1900

$ # UUID stability (two runs):
udlna "udlna@MacBookPro" (uuid: 1e7aff3c-08bc-5529-aa9a-66c39e30b0c9) on port 8200
udlna "udlna@MacBookPro" (uuid: 1e7aff3c-08bc-5529-aa9a-66c39e30b0c9) on port 8200
```

## Next Phase Readiness
Phase 8 Plan 01 is the final plan of Phase 8, which is the final phase of v1. The udlna v1 milestone is complete:
- All DLNA/UPnP services implemented (ContentDirectory, ConnectionManager, SSDP)
- Stable UUID v5 identity (no state file needed; consistent across restarts)
- User-customizable friendly name via --name flag
- Zero compiler warnings on release build
- 81 tests passing

---
*Phase: 08-server-identity-customization*
*Completed: 2026-02-23*

## Self-Check: PASSED

- FOUND: .planning/phases/08-server-identity-customization/08-01-SUMMARY.md
- FOUND: commit 63e1ae6 (Task 1)
- FOUND: commit f070117 (Task 2)
- FOUND: commit ed2e0d0 (metadata)
- cargo build --release: 0 warnings
- cargo test: 81 passed, 0 failed
