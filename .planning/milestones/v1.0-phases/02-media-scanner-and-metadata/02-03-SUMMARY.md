---
phase: 02-media-scanner-and-metadata
plan: "03"
subsystem: media-scanning
tags: [symphonia, mp4, imagesize, walkdir, rust, media-metadata]

# Dependency graph
requires:
  - phase: 02-01
    provides: "MediaItem, MediaMeta, MediaLibrary structs in library.rs; MediaKind enum in mime.rs"
  - phase: 02-02
    provides: "format_upnp_duration, dlna_profile_for, build_machine_namespace, media_item_id pure helpers in metadata.rs"
provides:
  - "extract_metadata(path, kind, mime) -> Option<MediaMeta> I/O extraction function in metadata.rs"
  - "scan(paths) -> MediaLibrary in scanner.rs — full recursive walkdir traversal with subtitle filtering and canonicalization"
affects:
  - "03-http-server"
  - "main.rs scan integration"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Audio metadata extraction via symphonia probe + track n_frames * time_base"
    - "MP4 video metadata via mp4::Mp4Reader::read_header + video track width/height"
    - "Image dimensions via imagesize::size() header-only reads (~16 bytes)"
    - "Non-MP4 video: symphonia for duration, resolution left as None (Pitfall 2)"
    - "Walkdir with follow_links(true) for recursive traversal with symlink support"
    - "Canonicalize path before UUID generation for stability (Pitfall 5)"

key-files:
  created:
    - "src/media/scanner.rs — scan() and process_file() functions"
  modified:
    - "src/media/metadata.rs — added extract_metadata() and private extraction helpers"
    - "src/media/mod.rs — added pub mod scanner"

key-decisions:
  - "Non-MP4 video resolution is None — symphonia does not expose video frame dimensions (Pitfall 2); resolution omitted, not a blocking issue"
  - "extract_metadata returns None for Subtitle kind — subtitles filtered before this is reached in scanner"
  - "Audio bitrate derived from bits_per_coded_sample when available; None otherwise"
  - "MP4 duration uses mp4 reader container duration (ms precision), not symphonia audio track"

patterns-established:
  - "Pattern: All metadata extraction paths return Option, never panic — file skip on None"
  - "Pattern: classify() -> subtitle filter -> canonicalize -> stat -> extract_metadata -> push"
  - "Pattern: warn log for skipped/unreadable files; debug log for missing optional data"

requirements-completed: [INDX-01, INDX-02, INDX-03, INDX-04]

# Metrics
duration: 3min
completed: 2026-02-22
---

# Phase 2 Plan 03: Metadata I/O Extraction and Directory Scanner Summary

**extract_metadata() via symphonia/mp4/imagesize plus walkdir scanner building a populated MediaLibrary with subtitle filtering and canonical-path UUID stability**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-02-22T21:40:43Z
- **Completed:** 2026-02-22T21:43:17Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Implemented `extract_metadata(path, kind, mime) -> Option<MediaMeta>` dispatching to three extraction paths: audio via symphonia, MP4 video via mp4 crate, image via imagesize
- Implemented `scan(paths) -> MediaLibrary` with walkdir traversal, subtitle filtering, path canonicalization, and startup summary log line
- All 47 tests pass including 4 new tests for extract_metadata None-on-failure behavior and 2 scanner tests for missing/empty paths

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement extract_metadata() I/O extraction in metadata.rs** - `6577b66` (feat)
2. **Task 2: Implement scanner.rs with walkdir traversal and MediaLibrary construction** - `f264d9d` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified
- `src/media/metadata.rs` - Added extract_metadata() dispatch + extract_audio_meta(), extract_mp4_video_meta(), extract_non_mp4_video_meta(), extract_image_meta() helpers; existing pure-function tests retained
- `src/media/scanner.rs` - New file: scan() and process_file() with walkdir, subtitle filter, canonicalize, UUID generation, MediaLibrary construction
- `src/media/mod.rs` - Added `pub mod scanner`

## Decisions Made
- Non-MP4 video (MKV, AVI, etc.) resolution is `None` — symphonia does not expose video frame dimensions through `CodecParameters`; this is documented in RESEARCH.md Pitfall 2 and is acceptable per plan spec
- Audio bitrate uses `bits_per_coded_sample` as a fallback; this is a rough approximation but is the only field available without decoding
- MP4 duration is taken from `mp4.duration()` (container-level, millisecond precision) rather than audio track for correctness

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None — symphonia and mp4 crate APIs matched the patterns documented in RESEARCH.md exactly. The `mp4::TrackType` is exported via `pub use types::*` in the mp4 crate, so no import disambiguation was needed.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness
- `scan()` and `extract_metadata()` are fully implemented and tested; main.rs can call `media::scanner::scan(&config.paths)` to get a populated `MediaLibrary`
- Phase 3 (HTTP server) can wrap the result in `Arc<RwLock<MediaLibrary>>` and expose DIDL-Lite browse responses using the `duration`, `resolution`, `bitrate`, and `dlna_profile` fields on each `MediaItem`
- Zero-files-found exit guard (Pitfall 6) should be added in main.rs before Phase 3 server startup — out of scope for this plan but noted in RESEARCH.md

## Self-Check: PASSED

- src/media/metadata.rs — FOUND
- src/media/scanner.rs — FOUND
- .planning/phases/02-media-scanner-and-metadata/02-03-SUMMARY.md — FOUND
- Commit 6577b66 (Task 1) — FOUND
- Commit f264d9d (Task 2) — FOUND
- Commit 6131db0 (docs/metadata) — FOUND

---
*Phase: 02-media-scanner-and-metadata*
*Completed: 2026-02-22*
