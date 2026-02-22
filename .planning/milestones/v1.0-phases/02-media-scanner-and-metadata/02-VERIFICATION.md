---
phase: 02-media-scanner-and-metadata
verified: 2026-02-22T22:00:00Z
status: passed
score: 5/5 must-haves verified
gaps: []
human_verification:
  - test: "Run udlna against a real media directory containing MP3, MP4, JPEG files"
    expected: "Scan summary prints with correct counts; ready log shows item count; metadata fields (duration, resolution, dlna_profile) are populated on items"
    why_human: "All extraction paths (symphonia, mp4 crate, imagesize) require real media files with valid headers — test files are not in the repo"
---

# Phase 2: Media Scanner & Metadata Verification Report

**Phase Goal:** Server scans all provided directories at startup and builds an in-memory media library with extracted metadata (duration, resolution, bitrate) ready for all downstream components
**Verified:** 2026-02-22T22:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths (from ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Server recursively walks all provided media directories and discovers every video, audio, and image file | VERIFIED | `scan()` uses `WalkDir::new(root).follow_links(true)` in `scanner.rs:38`; classifies via `classify()` from `mime.rs`; all three media kinds (Video, Audio, Image) pushed to `library.items` |
| 2 | Each media item has a stable UUID ID, file path, file size, and detected MIME type stored in memory | VERIFIED | `MediaItem` struct has `id: Uuid` (UUIDv5), `path: PathBuf` (canonical), `file_size: u64`, `mime: &'static str` in `library.rs:37-51`. Note: ROADMAP says "integer ID" but CONTEXT.md locked UUIDv5 — implementation correctly follows CONTEXT.md |
| 3 | Audio and video files have duration extracted from container headers and stored in UPnP format (HH:MM:SS.mmm) | VERIFIED | `extract_audio_meta()` uses symphonia `n_frames * time_base`; `extract_mp4_video_meta()` uses `mp4.duration().as_millis()`; both call `format_upnp_duration()` which produces `HH:MM:SS.mmm` — tested by 7 passing unit tests |
| 4 | Video files and images have resolution (WxH) extracted from container/file headers | VERIFIED | `extract_mp4_video_meta()` reads `track.width()/track.height()` via mp4 crate; `extract_image_meta()` reads `imagesize::size(path)` — non-MP4 video resolution is `None` per documented RESEARCH.md Pitfall 2 limitation |
| 5 | Each media item has a DLNA profile name assigned where determinable, with None for unrecognized formats | VERIFIED | `dlna_profile_for()` assigns MP3/AAC_ISO_320/JPEG_LRG/PNG_LRG for known types; returns `None` for all others. Note: ROADMAP says "wildcard fallback" but CONTEXT.md locked "never use wildcard — use None" — implementation correctly follows CONTEXT.md |

**Score:** 5/5 truths verified

---

### Required Artifacts

All artifacts verified at three levels: exists, substantive (not a stub), and wired.

| Artifact | Provides | Exists | Substantive | Wired | Status |
|----------|----------|--------|-------------|-------|--------|
| `src/media/library.rs` | `MediaItem`, `MediaMeta`, `MediaLibrary` type definitions | Yes | Yes — 67 lines, all fields present with correct types | Yes — imported by `scanner.rs:6` and `metadata.rs:5` | VERIFIED |
| `src/media/metadata.rs` | `format_upnp_duration`, `dlna_profile_for`, `build_machine_namespace`, `media_item_id`, `extract_metadata` | Yes | Yes — 433 lines, all 5 functions implemented with full I/O logic, 27 unit tests | Yes — imported by `scanner.rs:7`; called in `process_file()` | VERIFIED |
| `src/media/scanner.rs` | `scan(paths) -> MediaLibrary` with walkdir traversal | Yes | Yes — 160 lines, `scan()` + `process_file()` fully implemented | Yes — called in `main.rs:54` as `media::scanner::scan(&config.paths)` | VERIFIED |
| `src/main.rs` | Scan call, `Arc<RwLock<MediaLibrary>>` construction, zero-file error exit | Yes | Yes — scan wired, zero-file guard present, Arc wrap present | Yes — all Phase 2 modules accessed via `media::` | VERIFIED |
| `src/media/mod.rs` | Re-exports for all four Phase 2 modules | Yes | Yes — exports `library`, `metadata`, `mime`, `scanner` | Yes — `mod media` declared in `main.rs:7` | VERIFIED |
| `Cargo.toml` | `walkdir`, `symphonia`, `mp4`, `imagesize`, `uuid`, `machine-uid` dependencies | Yes | Yes — all 6 crates present with correct versions | Yes — used in source files | VERIFIED |

---

### Key Link Verification

| From | To | Via | Status | Evidence |
|------|----|-----|--------|----------|
| `src/media/library.rs` | `src/media/mime.rs` | `use crate::media::mime::MediaKind` | WIRED | `library.rs:3`: `use crate::media::mime::MediaKind` |
| `src/media/library.rs` | `uuid` crate | `uuid::Uuid` field on `MediaItem` | WIRED | `library.rs:2`: `use uuid::Uuid`; `library.rs:40`: `pub id: Uuid` |
| `src/media/metadata.rs` | `uuid` crate | `Uuid::new_v5` for ID generation | WIRED | `metadata.rs:37,43`: two `Uuid::new_v5(...)` calls |
| `src/media/metadata.rs` | `machine-uid` crate | `machine_uid::get()` for namespace seed | WIRED | `metadata.rs:36`: `machine_uid::get().unwrap_or_else(|_| "unknown".to_string())` |
| `src/media/scanner.rs` | `src/media/metadata.rs` | `extract_metadata()` called per file | WIRED | `scanner.rs:7`: import; `scanner.rs:114`: `extract_metadata(&canonical, kind.clone(), mime)` |
| `src/media/scanner.rs` | `src/media/mime.rs` | `classify()` called per file | WIRED | `scanner.rs:8`: import; `scanner.rs:82`: `classify(path)` |
| `src/media/scanner.rs` | `src/media/library.rs` | `MediaItem`/`MediaLibrary` populated during walk | WIRED | `scanner.rs:6`: import; `scanner.rs:131`: `library.items.push(MediaItem { ... })` |
| `src/media/metadata.rs` | `symphonia` crate | `MediaSourceStream` + `Probe` for audio duration | WIRED | `metadata.rs:61-74`: `use symphonia::...`; `symphonia::default::get_probe()` called |
| `src/media/metadata.rs` | `mp4` crate | `Mp4Reader::read_header` for video metadata | WIRED | `metadata.rs:134`: `mp4::Mp4Reader::read_header(reader, file_len).ok()?` |
| `src/media/metadata.rs` | `imagesize` crate | `imagesize::size()` for image dimensions | WIRED | `metadata.rs:224`: `imagesize::size(path)` |
| `src/main.rs` | `src/media/scanner.rs` | `media::scanner::scan(&config.paths)` | WIRED | `main.rs:54`: `let library = media::scanner::scan(&config.paths)` |
| `src/main.rs` | `src/media/library.rs` | `Arc::new(RwLock::new(library))` | WIRED | `main.rs:65`: `let _library = Arc::new(RwLock::new(library))` |

All 12 key links: WIRED.

---

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CLI-02 | 02-01, 02-04 | Server recursively scans all provided paths at startup and exposes all files as a flat list | SATISFIED | `scan()` in `scanner.rs` uses `WalkDir` recursively; `main.rs:54` calls it at startup; `MediaLibrary.items` is the flat list |
| INDX-01 | 02-01, 02-03 | Server extracts media metadata at scan time by reading container/file headers (no transcoding, no ffmpeg) | SATISFIED | `extract_metadata()` dispatches to symphonia (audio), mp4 crate (MP4 video), imagesize (images) — all pure Rust, header-only reads |
| INDX-02 | 02-02, 02-03 | DIDL-Lite `<res>` elements include `duration` attribute in `HH:MM:SS.mmm` format for audio/video | SATISFIED | `format_upnp_duration()` tested by 7 unit tests; `MediaMeta.duration: Option<String>` populated at scan time; available for Phase 5 DIDL-Lite generation |
| INDX-03 | 02-02, 02-03 | DIDL-Lite `<res>` elements include `resolution` attribute (`WxH`) for video files and images | SATISFIED | `MediaMeta.resolution: Option<String>` populated as `"WxH"` by `extract_mp4_video_meta()` and `extract_image_meta()`; non-MP4 video resolution is `None` (documented limitation) |
| INDX-04 | 02-02, 02-03 | DLNA profile names assigned where determinable; fallback for unrecognized formats | SATISFIED | `dlna_profile_for()` assigns MP3, AAC_ISO_320, JPEG_LRG, PNG_LRG for known types; returns `None` for unknown types (CONTEXT.md LOCKED: never use wildcard `*`) |

No orphaned requirements found. REQUIREMENTS.md traceability table lists CLI-02, INDX-01, INDX-02, INDX-03, INDX-04 all as "Phase 2 | Complete" — consistent with what is implemented.

---

### ROADMAP Success Criteria Discrepancies (Resolved)

Two discrepancies exist between ROADMAP.md wording and the actual implementation. Both are **correctly resolved in favor of CONTEXT.md locked decisions**:

**SC-2 "stable integer ID":** ROADMAP says "stable integer ID" but the implementation uses `uuid::Uuid` (UUIDv5). CONTEXT.md explicitly locked UUIDv5 as the ID strategy. The ROADMAP wording is a loose description; the implementation is correct per design intent.

**SC-5 "wildcard fallback":** ROADMAP says "wildcard fallback for unrecognized formats." CONTEXT.md locked: "Do not use wildcard (`*`) — don't assign a profile we can't determine." The implementation returns `None` for unknown MIME types, which correctly omits the DLNA.ORG_PN field from protocolInfo entirely. The CONTEXT.md locked decision takes precedence; the implementation is correct.

Neither discrepancy represents a gap — the implementation is consistent with the authoritative locked decisions.

---

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|------|---------|----------|--------|
| `src/media/library.rs:8,35` | `#[allow(dead_code)]` on `MediaMeta` and `MediaItem` structs | Info | Expected — fields consumed by Phase 3 HTTP server, not yet read at end of Phase 2. Targeted (struct-level), not file-level |
| `src/media/scanner.rs:12` | `#[allow(dead_code)]` on `ScanStats` | Info | Expected — `ScanStats` is a reserved Phase 3+ API struct not yet called by any consumer |
| `src/main.rs:65` | `_library` underscore prefix | Info | Expected — suppresses unused-variable warning until Phase 3 passes `Arc::clone(&library)` to HTTP server |

No blockers. No stubs. No placeholders. All `#[allow(dead_code)]` attributes are targeted (struct-level) and expected — the file-level suppressor from Phase 1 was correctly removed.

---

### Build and Test Verification

- `cargo build` exits 0 with no errors — confirmed
- `cargo test` exits 0 — 47 tests pass, 0 fail — confirmed
- Zero-file guard confirmed: `cargo run -- /tmp/udlna_empty_test` prints `error: no media files found in the provided paths — exiting` and exits with code 1
- Startup banner before scan, summary after scan — confirmed in live run output

---

### Human Verification Required

#### 1. Real media file metadata extraction

**Test:** Create a directory with at least one MP3, one MP4, and one JPEG. Run `RUST_LOG=debug cargo run -- /path/to/that/dir`.

**Expected:** Scan summary prints `"Scanned N files (X video, Y audio, Z image) in T.Ts"` with non-zero counts. Ready log shows `"Serving N media items on port 8200"`. Debug logs show no "metadata extraction failed" warnings for valid files.

**Why human:** No media test fixtures are in the repo. The extraction code paths (symphonia probe, mp4::Mp4Reader, imagesize::size) cannot be verified without actual binary media files with valid headers.

#### 2. Non-MP4 video resolution limitation

**Test:** Point the server at a directory with `.mkv` files. Observe metadata extraction output.

**Expected:** MKV files are indexed (duration extracted via symphonia audio track), but `resolution` is `None`. A debug log `"No video resolution for {path} (non-MP4)"` appears for each MKV file.

**Why human:** Requires actual MKV files with audio tracks; verifying the symphonia probe succeeds on a real MKV cannot be done statically.

---

### Gaps Summary

None. All must-haves verified. Phase goal is achieved: the server scans directories at startup, builds an in-memory `MediaLibrary` wrapped in `Arc<RwLock<>>`, extracts metadata (duration via symphonia/mp4, resolution via mp4/imagesize, DLNA profiles via static lookup), generates stable UUIDv5 IDs, filters subtitles, handles missing paths gracefully, and exits with error when no media is found. All downstream phases can access the populated library via `Arc::clone`.

---

_Verified: 2026-02-22T22:00:00Z_
_Verifier: Claude (gsd-verifier)_
