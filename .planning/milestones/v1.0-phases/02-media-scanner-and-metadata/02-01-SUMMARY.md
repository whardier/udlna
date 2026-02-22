---
phase: 02-media-scanner-and-metadata
plan: "01"
subsystem: media
tags: [rust, walkdir, symphonia, mp4, imagesize, uuid, machine-uid, media-types]

# Dependency graph
requires:
  - phase: 01-project-setup-cli
    provides: MediaKind enum from src/media/mime.rs (used as MediaItem.kind field type)
provides:
  - MediaItem struct with id/path/file_size/mime/kind/meta fields
  - MediaMeta struct with optional duration/resolution/bitrate/dlna_profile fields
  - MediaLibrary flat Vec<MediaItem> container with new() constructor
  - All Phase 2 crate dependencies in Cargo.toml (walkdir, symphonia, mp4, imagesize, uuid, machine-uid)
affects:
  - 02-02-media-scanner (uses MediaItem/MediaLibrary to build the library)
  - 02-03-metadata-extractor (uses MediaMeta to populate metadata fields)
  - 03-http-server (uses Arc<RwLock<MediaLibrary>> for thread-safe media access)
  - 05-content-directory (uses MediaItem fields for DIDL-Lite XML generation)

# Tech tracking
tech-stack:
  added:
    - walkdir 2 (recursive directory traversal)
    - symphonia 0.5 with mp3/aac/isomp4/mkv/flac/ogg/wav features (audio metadata extraction)
    - mp4 0.14 (MP4 container metadata extraction)
    - imagesize 0.14 (image dimension extraction)
    - uuid 1 with v5 feature (stable UUIDv5 media item IDs)
    - machine-uid 0.5 (machine-specific UUID namespace seed)
  patterns:
    - "Option<&'static str> for dlna_profile: None means omit DLNA.ORG_PN entirely, never use wildcard"
    - "All MediaMeta fields are Option<T>: extraction failure returns None, not placeholder values"
    - "MediaLibrary.items must never contain Subtitle kind items (filtered at scan time)"

key-files:
  created:
    - src/media/library.rs (MediaItem, MediaMeta, MediaLibrary type definitions)
  modified:
    - Cargo.toml (added 6 Phase 2 crate dependencies)
    - src/media/mod.rs (added pub mod library)

key-decisions:
  - "dlna_profile: Option<&'static str> — never wildcard '*', use None when DLNA profile cannot be determined"
  - "path: PathBuf stores canonicalize() result (resolved symlinks), not raw walkdir path"
  - "MediaLibrary.items is Vec<MediaItem> with NO Subtitle items — filtered at scan time in Phase 2 Plan 02"

patterns-established:
  - "MediaItem.id is UUIDv5: uuid5(machine_namespace, canonical_path_bytes) — stable per machine across restarts"
  - "Arc<RwLock<MediaLibrary>> is the intended sharing pattern for Phase 3+ thread-safe access"

requirements-completed: [CLI-02, INDX-01]

# Metrics
duration: 2min
completed: 2026-02-22
---

# Phase 2 Plan 01: Media Type Definitions and Phase 2 Dependencies Summary

**MediaItem/MediaMeta/MediaLibrary structs defined in Rust with UUIDv5 stable IDs and all 6 Phase 2 crates (walkdir, symphonia, mp4, imagesize, uuid, machine-uid) added to Cargo.toml**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-22T21:33:55Z
- **Completed:** 2026-02-22T21:35:23Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Added all 6 Phase 2 crate dependencies to Cargo.toml — downstream plans compile without incremental edits
- Defined MediaMeta with optional duration/resolution/bitrate/dlna_profile (all Option<T>, extraction failures return None)
- Defined MediaItem with UUIDv5 stable id, canonical PathBuf, file_size, mime &'static str, MediaKind, and MediaMeta
- Defined MediaLibrary as flat Vec<MediaItem> with Default+new() constructors, ready for Arc<RwLock<>> wrapping
- All locked decisions from CONTEXT.md encoded in types: dlna_profile as Option<&'static str> (no wildcard), path as PathBuf (canonical), items as Vec (no Subtitle kinds)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Phase 2 crate dependencies to Cargo.toml** - `ffcbdf0` (chore)
2. **Task 2: Define MediaItem, MediaMeta, and MediaLibrary** - `e2f937a` (feat)

**Plan metadata:** _(docs commit follows)_

## Files Created/Modified
- `src/media/library.rs` - MediaItem, MediaMeta, and MediaLibrary type definitions
- `src/media/mod.rs` - Added `pub mod library` to expose new module
- `Cargo.toml` - Added walkdir, symphonia, mp4, imagesize, uuid, machine-uid dependencies

## Decisions Made
None - followed plan as specified. All type definitions, field types, and locked decisions were pre-specified in the plan from CONTEXT.md decisions.

## Deviations from Plan
None - plan executed exactly as written.

## Issues Encountered
None - cargo fetch and build succeeded without conflicts on first attempt.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- MediaItem/MediaMeta/MediaLibrary types are the contract all downstream code depends on — ready for Plan 02 (scanner) and Plan 03 (metadata extractor)
- All Phase 2 crates are resolved and compiled — no further Cargo.toml edits needed for this phase
- Scanner (Plan 02) can immediately import MediaItem/MediaLibrary and use walkdir to build the library
- Metadata extractor (Plan 03) can populate MediaMeta fields using symphonia, mp4, and imagesize crates

---
*Phase: 02-media-scanner-and-metadata*
*Completed: 2026-02-22*
