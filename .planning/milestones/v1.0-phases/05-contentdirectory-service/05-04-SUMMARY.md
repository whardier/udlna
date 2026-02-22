---
phase: 05-contentdirectory-service
plan: "04"
subsystem: api
tags: [rust, axum, soap, upnp, dlna, content-directory, didl-lite, browse]

# Dependency graph
requires:
  - phase: 05-03
    provides: cds_control handler with handle_browse placeholder, ok_xml() helper, action dispatch
  - phase: 05-02
    provides: soap.rs utilities (extract_soap_param, apply_pagination, build_protocol_info, container_uuid, format_dc_date, build_res_url, xml_escape)
  - phase: 05-01
    provides: soap_response(), soap_fault() builders
  - phase: 02-media-scanner-and-metadata
    provides: MediaItem, MediaLibrary, MediaKind, MediaMeta with duration/resolution/bitrate/dlna_profile

provides:
  - Complete Browse action handler with BrowseDirectChildren and BrowseMetadata
  - Four-container hierarchy: Videos, Music, Photos, All Media with UUIDv5 container IDs
  - DIDL-Lite item elements with file_stem title, dc:date, protocolInfo, res URL
  - Pagination support via apply_pagination (RequestedCount=0 = return all)
  - SOAP fault errorCode 701 for unknown ObjectIDs
  - XML-escaped DIDL-Lite in Result element (& -> &amp;, < -> &lt;)
  - All four DIDL-Lite namespaces including xmlns:dlna

affects: [phase-06, phase-07, phase-08]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "didl_lite_wrap() emits all four namespaces in single string format (no XML builder needed)"
    - "container_element() / item_element() are private helpers returning String, keeping handle_browse readable"
    - "item_element() uses file_stem() not file_name() for dc:title (no extension)"
    - "xml_escape() called on complete DIDL-Lite string before embedding in Result element"
    - "BrowseMetadata on media item UUID: search lib.items.iter().find() then determine parent container from item.kind"
    - "Pre-compute filtered Vec<&MediaItem> for each kind before match dispatch (avoids repeated filter in arms)"

key-files:
  created: []
  modified:
    - src/http/content_directory.rs

key-decisions:
  - "didl_lite_wrap() outputs single-line DIDL-Lite with all four namespaces inline — avoids whitespace issues in some DLNA parsers"
  - "BrowseMetadata on item UUID: parent container determined from item.kind (Video→videos_id, Audio→music_id, Image→photos_id)"
  - "Pre-compute all four filtered item lists before match dispatch — clean, avoids re-filtering in each match arm"
  - "Task 2 (curl verification) required no code changes — all 5 checks passed on first implementation"

patterns-established:
  - "DIDL-Lite generation: wrap -> escape -> embed pattern (never embed raw XML in SOAP Result)"
  - "Unknown ObjectID returns soap_fault(701, 'No such object').into_response() in both browse modes"

requirements-completed: [CONT-01, CONT-02, CONT-03, CONT-04, CONT-05]

# Metrics
duration: 4min
completed: 2026-02-23
---

# Phase 5 Plan 04: ContentDirectory SOAP Service Summary

**Browse action fully implemented: BrowseDirectChildren and BrowseMetadata with four-container DIDL-Lite hierarchy, XML escaping, pagination, and 701 faults — DLNA clients can now enumerate and play the complete media library**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-23T02:25:08Z
- **Completed:** 2026-02-23T02:29:38Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- Replaced `handle_browse` placeholder with complete implementation (306 lines, replacing 8)
- BrowseDirectChildren on ObjectID=0 returns four containers (Videos, Music, Photos, All Media) in XML-escaped DIDL-Lite
- BrowseMetadata on root, containers, and individual media items all return correct elements
- Unknown ObjectIDs return HTTP 500 with SOAP fault errorCode 701
- `dc:title` uses `file_stem()` (no extension), `dc:date` always present, all four DIDL-Lite namespaces included
- All 5 curl checks passed on first build; all 81 cargo tests still pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement full Browse handler — BrowseDirectChildren and BrowseMetadata** - `3d4539a` (feat)
2. **Task 2: End-to-end curl verification of all Browse cases** - no code changes needed; all 5 checks passed

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/http/content_directory.rs` - Complete Browse handler replacing placeholder; added `didl_lite_wrap()`, `container_element()`, `item_element()` private helpers

## Decisions Made
- `didl_lite_wrap()` outputs inline single-line DIDL-Lite (avoids whitespace issues in some DLNA XML parsers)
- BrowseMetadata on item UUID determines parent container ID from `item.kind` (Video → videos container, Audio → music container, Image → photos container)
- Pre-compute all four filtered `Vec<&MediaItem>` lists before `match browse_flag` dispatch — cleaner than re-filtering in each arm
- Task 2 verification confirmed all implementation details correct on first build: file_stem titles, dc:date presence, correct UPnP class strings, protocolInfo format, res URL construction

## Deviations from Plan

None — plan executed exactly as written. All 5 curl checks passed without requiring any fixes.

## Issues Encountered
- None.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness
- Phase 5 ContentDirectory SOAP Service is complete. `/cds/control` Browse action is fully functional.
- DLNA clients can discover the server (Phase 4), request content directory (Phase 5), and stream media (Phase 3).
- Phase 6 (SSDP Discovery) is next — the server needs to announce itself via multicast UDP for clients to auto-discover it on the network.
- Phase 7 (ConnectionManager) and Phase 8 (Integration) follow.

## Self-Check: PASSED

- src/http/content_directory.rs: FOUND (306 insertions in commit 3d4539a)
- 05-04-SUMMARY.md: FOUND
- Commit 3d4539a (Task 1): verified
- cargo build: 0 errors
- cargo test: 81 passed, 0 failed
- All 5 curl checks: PASSED

---
*Phase: 05-contentdirectory-service*
*Completed: 2026-02-23*
