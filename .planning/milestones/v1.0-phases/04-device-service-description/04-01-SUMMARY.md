---
phase: 04-device-service-description
plan: "01"
subsystem: api
tags: [upnp, dlna, axum, uuid, xml, media-server]

# Dependency graph
requires:
  - phase: 03-http-server-file-streaming
    provides: AppState, build_router, axum HTTP server foundation with 501 stubs for description routes
provides:
  - UPnP device description XML handler at /device.xml (MediaServer:1, DLNA X_DLNADOC)
  - CDS SCPD XML handler at /cds/scpd.xml (Browse + 3 capability actions, full serviceStateTable)
  - CMS SCPD XML handler at /cms/scpd.xml (GetProtocolInfo, GetCurrentConnectionIDs, GetCurrentConnectionInfo)
  - AppState.server_uuid field populated with UUID v4 at startup
affects:
  - 05-content-directory-soap (consumes /cds/control, reads server_uuid)
  - 06-ssdp-discovery (announces UDN built from server_uuid)
  - 07-connection-manager-soap (consumes /cms/control)

# Tech tracking
tech-stack:
  added:
    - uuid v4 feature (added to existing uuid = { version = "1", features = ["v4", "v5"] })
  patterns:
    - Static XML as const &str for fully-static SCPD documents (no XML builder crate)
    - UUID interpolation via format!() in serve_device_xml handler
    - State<AppState> extraction in axum handlers for UUID access
    - Content-Type: text/xml; charset="utf-8" on all description responses

key-files:
  created:
    - src/http/description.rs
  modified:
    - src/http/state.rs
    - src/http/mod.rs
    - src/main.rs
    - Cargo.toml

key-decisions:
  - "uuid crate v4 feature added explicitly — uuid 1.x requires opt-in feature for new_v4()"
  - "serviceId uses urn:upnp-org:serviceId (not urn:schemas-upnp-org) — avoids known PyMedS-ng bug"
  - "URLBase omitted from device.xml — deprecated in UPnP 1.1 and complex with dual-stack (RESEARCH.md Pitfall 3)"
  - "eventSubURL element present but empty — required for client compatibility even though eventing not yet implemented"
  - "Static const &str for SCPD XML — no XML builder crate needed for fully-static documents"
  - "server_uuid is random UUID v4 per restart — acceptable since SSDP (Phase 6) not yet active"

patterns-established:
  - "Static XML pattern: const &str raw string literals for spec-compliant XML without builder crates"
  - "Handler extraction pattern: State(state): State<AppState> for accessing shared state in axum"

requirements-completed: [DESC-01, DESC-02, DESC-03]

# Metrics
duration: 3min
completed: 2026-02-22
---

# Phase 4 Plan 01: Device and Service Description Summary

**UPnP device.xml (MediaServer:1 + DLNA), CDS SCPD (4 actions), and CMS SCPD (3 actions) served as spec-compliant XML via axum, replacing Phase 3 501 stubs**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-23T00:35:04Z
- **Completed:** 2026-02-23T00:38:00Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments
- AppState extended with `server_uuid: String` field, populated from `uuid::Uuid::new_v4()` before server startup
- `src/http/description.rs` created with three spec-compliant axum handlers (serve_device_xml, serve_cds_scpd, serve_cms_scpd)
- Phase 3 501 stubs at /device.xml, /cds/scpd.xml, /cms/scpd.xml replaced with real handlers; Phase 5 stubs at /cds/control and /cms/control preserved
- `cargo build --release` succeeds with no errors

## Task Commits

Each task was committed atomically:

1. **Task 1: Extend AppState with server_uuid and generate UUID v4 at startup** - `420b54b` (feat)
2. **Task 2: Create description.rs with device.xml, cds/scpd.xml, and cms/scpd.xml handlers** - `160175b` (feat)
3. **Task 3: Wire description handlers into build_router, replacing Phase 4 stubs** - `9ed25af` (feat)

## Files Created/Modified
- `src/http/description.rs` - Three axum handlers: serve_device_xml (UUID-interpolated), serve_cds_scpd (static), serve_cms_scpd (static)
- `src/http/state.rs` - AppState struct extended with `pub server_uuid: String` field
- `src/main.rs` - UUID v4 generated before AppState construction, passed as `server_uuid` field
- `src/http/mod.rs` - `pub mod description` declared, three description routes wired, 501 stubs replaced
- `Cargo.toml` - uuid crate features updated from `["v5"]` to `["v4", "v5"]`

## Decisions Made
- Added `v4` feature explicitly to the uuid crate — uuid 1.x requires opt-in features for all version generators
- serviceId values use `urn:upnp-org:serviceId:*` (correct UPnP namespace), not `urn:schemas-upnp-org:serviceId:*` which is a known PyMedS-ng bug
- URLBase omitted from device.xml per RESEARCH.md Pitfall 3 (deprecated in UPnP 1.1, complex with IPv4+IPv6 dual-stack)
- Static `const &str` for CDS and CMS SCPD documents — no XML builder crate needed since content is fully static

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added v4 feature to uuid crate**
- **Found during:** Task 1 (AppState + UUID generation)
- **Issue:** Cargo.toml had uuid with only `features = ["v5"]`; uuid 1.x requires explicit `v4` feature for `Uuid::new_v4()`
- **Fix:** Updated Cargo.toml to `features = ["v4", "v5"]`
- **Files modified:** Cargo.toml, Cargo.lock
- **Verification:** `cargo build` succeeded after feature addition
- **Committed in:** `420b54b` (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 missing critical dependency feature)
**Impact on plan:** Required for compilation — no scope creep.

## Issues Encountered

- Pre-existing clippy warnings in `src/media/metadata.rs` (derivable_impls, unnecessary_cast) — out of scope per deviation rules, not caused by Phase 4 changes. Deferred to future cleanup pass.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `/device.xml`, `/cds/scpd.xml`, `/cms/scpd.xml` all return spec-compliant XML — DLNA clients can now discover service action signatures
- Phase 5 (ContentDirectory SOAP) can proceed: `/cds/control` POST stub is in place, Browse action is declared in CDS SCPD
- `server_uuid` is in AppState and will be accessible to Phase 5 handlers for SOAP response construction
- Pre-existing clippy warnings in metadata.rs do not block Phase 5

## Self-Check: PASSED

- FOUND: src/http/description.rs
- FOUND: src/http/state.rs (with server_uuid field)
- FOUND: src/http/mod.rs (with description:: routes)
- FOUND: src/main.rs (with uuid::Uuid::new_v4())
- FOUND: .planning/phases/04-device-service-description/04-01-SUMMARY.md
- FOUND commit: 420b54b (Task 1 - AppState + UUID)
- FOUND commit: 160175b (Task 2 - description.rs handlers)
- FOUND commit: 9ed25af (Task 3 - router wiring)

---
*Phase: 04-device-service-description*
*Completed: 2026-02-22*
