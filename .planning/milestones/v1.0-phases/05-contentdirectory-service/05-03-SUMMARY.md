---
phase: 05-contentdirectory-service
plan: "03"
subsystem: api
tags: [rust, axum, soap, upnp, dlna, content-directory]

# Dependency graph
requires:
  - phase: 05-01
    provides: soap_response(), soap_fault(), extract_soap_param() utility functions in soap.rs

provides:
  - src/http/content_directory.rs with cds_control handler and action dispatch
  - GetSearchCapabilities returning empty SearchCaps element (HTTP 200)
  - GetSortCapabilities returning empty SortCaps element (HTTP 200)
  - GetSystemUpdateID returning Id element with value 1 (HTTP 200)
  - Unknown SOAPAction returning SOAP fault errorCode 402 (HTTP 500)
  - handle_browse placeholder returning fault 401 until Plan 04
  - /cds/control route wired to content_directory::cds_control (replaces 501 stub)

affects: [05-04, phase-06, phase-07]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ok_xml() inline helper wraps soap_response() string in HTTP 200 + text/xml response tuple"
    - "SOAPAction header parsed with split('#').nth(1) + trim_matches('\"') to extract action name"
    - "Body fallback: if SOAPAction absent/empty, parse <u: prefix in body for action name"
    - "match action.as_deref() dispatch pattern for SOAP action routing"

key-files:
  created:
    - src/http/content_directory.rs
  modified:
    - src/http/mod.rs

key-decisions:
  - "ok_xml() helper avoids repeating (StatusCode::OK, [(CONTENT_TYPE, text/xml)], body) tuple in every action handler"
  - "handle_browse returns fault 401 Action Failed as placeholder — Plan 04 replaces this function entirely"
  - "SOAPAction body fallback searches for <u: prefix (namespace-prefixed element) matching Samsung/Xbox client patterns"

patterns-established:
  - "CDS action handlers are synchronous fn -> Response except handle_browse (async for Plan 04 compatibility)"
  - "Unknown/missing SOAPAction → errorCode 402 InvalidArgs per UPnP spec"

requirements-completed: [CONT-06, CONT-07, CONT-08]

# Metrics
duration: 5min
completed: 2026-02-23
---

# Phase 5 Plan 03: ContentDirectory SOAP Service Summary

**CDS control endpoint with SOAPAction dispatch: GetSearchCapabilities, GetSortCapabilities, GetSystemUpdateID fully implemented; Browse placeholder returns fault 401 until Plan 04**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-23T02:17:02Z
- **Completed:** 2026-02-23T02:22:00Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Created `src/http/content_directory.rs` with `cds_control` axum handler and five functions
- Replaced the inline 501 stub in `mod.rs` with the real `content_directory::cds_control` handler
- Verified all four curl scenarios: GetSearchCapabilities, GetSortCapabilities, GetSystemUpdateID (all HTTP 200), and unknown action (HTTP 500 with errorCode 402)
- All 81 cargo tests pass with no regressions

## Task Commits

Each task was committed atomically:

1. **Task 1: Create content_directory.rs with action dispatch and stub actions** - `4c2c61b` (feat)
2. **Task 2: Wire content_directory into mod.rs and verify stub actions with curl** - `b6181a3` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/http/content_directory.rs` - Main CDS handler with SOAPAction dispatch, three stub action implementations, and Browse placeholder
- `src/http/mod.rs` - Added `pub mod content_directory` and replaced /cds/control 501 stub with real handler

## Decisions Made
- `ok_xml()` inline helper wraps soap_response body into HTTP 200 + `text/xml; charset="utf-8"` response, avoiding tuple repetition in every handler
- `handle_browse` is declared `async fn` so Plan 04 can add `await` calls without changing the dispatch signature
- SOAPAction body fallback scans for `<u:` namespace prefix to extract action name — handles clients that omit the SOAPAction header per RESEARCH.md Pitfall 3

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered
- Server exits with "no media files found" when given an empty directory. Resolved by copying a real MP3 from macOS system frameworks for integration testing. This is expected behavior per the existing scanner implementation, not a regression.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness
- `/cds/control` is fully dispatching. Plan 04 can replace `handle_browse` with real Browse logic.
- All three stub action responses verified with curl — UPnP clients should accept them immediately.
- Plan 04 (Browse) depends on this plan and can start immediately.

## Self-Check: PASSED

- src/http/content_directory.rs: FOUND
- src/http/mod.rs: FOUND
- 05-03-SUMMARY.md: FOUND
- Commit 4c2c61b (Task 1): FOUND
- Commit b6181a3 (Task 2): FOUND

---
*Phase: 05-contentdirectory-service*
*Completed: 2026-02-23*
