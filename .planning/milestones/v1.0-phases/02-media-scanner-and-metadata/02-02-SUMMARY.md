---
phase: 02-media-scanner-and-metadata
plan: "02"
subsystem: media
tags: [rust, tdd, uuid, machine-uid, upnp, dlna, duration-formatting]

# Dependency graph
requires:
  - phase: 02-media-scanner-and-metadata
    plan: "01"
    provides: MediaItem/MediaMeta/MediaLibrary types, uuid/machine-uid crates already in Cargo.toml
provides:
  - format_upnp_duration(total_seconds, frac) -> String  ("HH:MM:SS.mmm" UPnP format)
  - dlna_profile_for(mime) -> Option<&'static str>  (static match table, 4 profiles)
  - build_machine_namespace() -> Uuid  (deterministic UUIDv5 seeded from machine-uid)
  - media_item_id(namespace, canonical_path) -> Uuid  (stable per-file UUIDv5)
affects:
  - 02-03-metadata-extractor (uses format_upnp_duration to populate MediaMeta.duration)
  - 05-content-directory (uses dlna_profile_for and media_item_id in DIDL-Lite generation)

# Tech tracking
tech-stack:
  added: []  # All crates (uuid, machine-uid) were added in Plan 01
  patterns:
    - "TDD RED-GREEN: stubs return '' and None; 11 tests fail; then implementations make 27/27 pass"
    - "format_upnp_duration uses {:02}/{:03} formatting — hours unbounded but always 2+ digits"
    - "dlna_profile_for is a plain match statement — no regex, no external data, O(1) at compile time"
    - "video/mp4 explicitly returns None in Phase 2 (DLNA profile deferred to Phase 5)"
    - "build_machine_namespace falls back to 'unknown' string if machine_uid::get() fails"

key-files:
  created:
    - src/media/metadata.rs (format_upnp_duration, dlna_profile_for, build_machine_namespace, media_item_id + 27 tests)
  modified:
    - src/media/mod.rs (added pub mod metadata)

key-decisions:
  - "video/mp4 returns None from dlna_profile_for — DLNA profile deferred to Phase 5 per RESEARCH.md"
  - "image/jpeg and image/png use unconditional JPEG_LRG/PNG_LRG — no size-tier logic in Phase 2"
  - "ms = (frac * 1000.0).round() as u32 — edge case frac=0.9995 producing 1000 is acceptable"

patterns-established:
  - "TDD RED-GREEN-REFACTOR applied to pure functions — stubs committed at RED, implementations at GREEN"
  - "media_item_id uses path.as_os_str().as_encoded_bytes() for cross-platform stable hashing"

requirements-completed: [INDX-02, INDX-03, INDX-04]

# Metrics
duration: 3min
completed: 2026-02-22
---

# Phase 2 Plan 02: Pure Metadata Helper Functions Summary

**UPnP duration formatter, DLNA profile lookup table, and UUIDv5 ID helpers implemented via TDD with 27 passing tests — RED phase committed before GREEN**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-22T21:37:25Z
- **Completed:** 2026-02-22T21:40:30Z
- **Tasks:** 1 (TDD: 2 commits — RED then GREEN)
- **Files modified:** 2

## Accomplishments
- Demonstrated RED phase: 11 tests failed against stub implementations (format_upnp_duration returned "", dlna_profile_for returned None for all inputs)
- Implemented format_upnp_duration with correct {:02}/{:03} zero-padding and sub-second rounding
- Implemented dlna_profile_for as a static match table covering all 15 MIME types specified in plan
- Implemented build_machine_namespace (UUIDv5 from machine_uid, fallback "unknown") and media_item_id (UUIDv5 from path bytes) — fully deterministic
- Full test suite: 41/41 passing with 0 failures across the entire project

## Task Commits

Each TDD phase committed atomically:

1. **RED phase: failing tests + stub implementations** - `0231d32` (test)
2. **GREEN phase: real implementations, all 27 tests pass** - `52346db` (feat)

**Plan metadata:** _(docs commit follows)_

_Note: TDD task produces two commits — test (RED) then feat (GREEN). No REFACTOR commit needed; implementations are already clean._

## Files Created/Modified
- `src/media/metadata.rs` - All four pub functions plus 27 test cases in #[cfg(test)] module
- `src/media/mod.rs` - Added `pub mod metadata` to expose new module

## Decisions Made
- `video/mp4` returns None (not a profile) — DLNA profile assignment deferred to Phase 5 per RESEARCH.md recommendation
- `image/jpeg` and `image/png` use unconditional JPEG_LRG/PNG_LRG strings — no resolution-tier logic in Phase 2
- No REFACTOR commit needed — implementations are minimal and clean as written

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
None - all tests passed on first GREEN attempt. No compilation errors, no logic fixes needed.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `format_upnp_duration` is ready for Plan 03 (metadata extractor) to call when populating `MediaMeta.duration`
- `dlna_profile_for` is ready for Plan 03 to call when assigning `MediaMeta.dlna_profile`
- `build_machine_namespace` and `media_item_id` are ready for Plan 02 (scanner) to call when constructing `MediaItem.id`
- Plan 03 (metadata extractor) can now focus solely on file I/O with symphonia/mp4/imagesize — all format helpers are tested and available

---
*Phase: 02-media-scanner-and-metadata*
*Completed: 2026-02-22*

## Self-Check: PASSED

- src/media/metadata.rs: FOUND
- src/media/mod.rs: FOUND
- 02-02-SUMMARY.md: FOUND
- RED commit 0231d32: FOUND
- GREEN commit 52346db: FOUND
