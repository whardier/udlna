# Phase 2: Media Scanner & Metadata - Research

**Researched:** 2026-02-22
**Domain:** Rust filesystem traversal, container metadata extraction, UUIDv5 ID generation, DLNA profile assignment
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Metadata extraction strategy**
- Pure Rust only — no ffmpeg subprocess, no FFI bindings
- Use whatever formats pure-Rust crates cover well (symphonia for audio, mp4parse for MP4, matroska/nom for MKV, image crate for images)
- Don't chase obscure formats — support what the crate ecosystem handles cleanly
- If metadata extraction fails for a file: skip the file entirely (do not include with null fields)

**DLNA profile assignment**
- When a DLNA profile cannot be determined (unknown codec/container): omit the DLNA profile field entirely
- Do not use wildcard (`*`) — don't assign a profile we can't determine

**Failure & error handling**
- Missing CLI/config directory: warn and continue scanning the rest — do not hard-fail on missing directories
- Unreadable files (permission denied, broken symlinks): log at warn level so user sees what was skipped
- Zero media files found after scanning: exit with error — refuse to start with nothing to serve
- Symlinks: follow symlinks (traverse symlinked directories, include symlinked files)

**Scan progress & logging**
- Scan happens synchronously — scan completes before server starts accepting connections
- Startup banner prints before scan begins; summary line prints after scan completes
- Summary format: `"Scanned N files (X video, Y audio, Z image) in T.Ts"`

**Media item IDs & state**
- IDs use UUIDv5: namespace derived from machine ID, name is the canonical file path
- Same file on same machine always gets the same ID across restarts
- Shared server state structure: `Arc<RwLock<MediaLibrary>>` — built in Phase 2, ready for thread-safe access in Phase 3+
- Scan at startup only — no filesystem watching, no periodic re-scan; restart server to pick up new files

### Claude's Discretion

No discretion areas listed — all implementation choices are locked.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-02 | Server recursively scans all provided paths at startup and exposes all files as a flat list | walkdir 2.5.0 with follow_links(true) covers recursive traversal; MediaLibrary flat Vec covers the "flat list" requirement |
| INDX-01 | Extract media metadata at scan time by reading container/file headers (no transcoding, no ffmpeg dependency) — duration for audio/video, dimensions for video/images, bitrate where available | symphonia 0.5.5 handles audio duration; mp4 0.14.0 handles MP4 video width/height/duration; imagesize 0.14.0 handles image dimensions; all pure Rust, no external processes |
| INDX-02 | DIDL-Lite `<res>` elements include `duration` attribute (UPnP format: `HH:MM:SS.mmm`) for audio and video files | Duration extraction pattern documented; UPnP formatting is manual from symphonia's `Time { seconds, frac }` struct |
| INDX-03 | DIDL-Lite `<res>` elements include `resolution` attribute (`WxH`) for video files and images | mp4 crate provides width()/height() for MP4; imagesize provides dimensions for images; MKV resolution is an open question (see below) |
| INDX-04 | DIDL-Lite `<res protocolInfo>` uses media-format-aware DLNA profile names where determinable from container/codec metadata; omits profile for unrecognized formats | DLNA profile table documented from readymedia reference implementation; profile is extension+codec based |
</phase_requirements>

---

## Summary

Phase 2 adds the media scanning and metadata extraction layer between the CLI/config foundation (Phase 1) and the HTTP server (Phase 3). The output is a single `Arc<RwLock<MediaLibrary>>` containing a flat `Vec<MediaItem>` that all downstream phases clone and read. The scan is synchronous at startup — no async runtime is needed yet.

The Rust ecosystem provides purpose-built crates for every required operation: `walkdir` for recursive directory traversal with symlink support, `symphonia` for audio duration extraction across all major formats (MP3, FLAC, OGG, WAV, AAC, M4A), the `mp4` crate for MP4 video width/height/duration, `imagesize` for image dimensions without full decode, and `uuid` + `machine-uid` for stable per-machine UUIDv5 IDs. All are pure Rust with no C bindings.

The main design challenge is that **Symphonia is audio-focused** — it can demux MP4 and MKV containers for audio track duration but does not expose video frame dimensions through its `CodecParameters`. Separate crates (`mp4` for MP4, and manual EBML parsing or `symphonia-format-mkv` for MKV) handle video resolution. This means the scanning logic has distinct paths for audio-only files vs. video-container files. DLNA profile assignment is a static lookup table (container + codec → profile string) based on publicly documented profiles from reference implementations.

**Primary recommendation:** Use `walkdir` + `symphonia` + `mp4` + `imagesize` + `uuid` + `machine-uid`. Structure as `src/media/scanner.rs` (walk + classify), `src/media/metadata.rs` (per-format extraction), `src/media/library.rs` (MediaItem + MediaLibrary + Arc wrapping).

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| walkdir | 2.5.0 | Recursive directory traversal with symlink control, error-per-entry recovery | BurntSushi's canonical solution; used by ripgrep, cargo; comparable perf to `find` on 3M+ entries |
| symphonia | 0.5.5 | Audio format demuxing + duration/bitrate extraction: MP3, FLAC, WAV, OGG, AAC, M4A, MKV audio, OPUS | Pure Rust; covers all audio formats in the MIME table from Phase 1; `mp3` + `aac` features needed |
| mp4 (alfg) | 0.14.0 | MP4 container reading: video width/height, duration, bitrate, codec info | Dedicated MP4 library with clean Mp4Reader API; `read_header()` is metadata-only (no full decode) |
| imagesize | 0.14.0 | Image dimension extraction without full decode — reads only header bytes | Reads ≤16 bytes for most formats; avoids loading 10MB JPEGs to get 1920x1080 |
| uuid | 1.21.0 | UUIDv5 generation: `Uuid::new_v5(namespace, name_bytes)` | Standard Rust UUID crate; `v5` feature enables SHA1-based name UUIDs |
| machine-uid | 0.5.4 | Cross-platform machine ID (Linux: /etc/machine-id, macOS: IORegistry, Windows: registry) | Returns plain String; supports all target platforms; no root required |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::sync::{Arc, RwLock} | stdlib | Wrap MediaLibrary for thread-safe read sharing in Phase 3+ | Always — no external crate needed, scan is synchronous so no deadlock risk |
| std::time::Instant | stdlib | Timing the scan for the startup summary line | Always — for "Scanned N files in T.Ts" |
| tracing | 0.1 (already added) | Log warnings for skipped files, scan summary | Already in Cargo.toml |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| walkdir | std::fs::read_dir (recursive) | Manual recursion is error-prone; no built-in symlink loop detection; walkdir handles all edge cases |
| symphonia | ffmpeg-sys / ffmpeg-next | FFI to C; requires system ffmpeg; violates pure-Rust requirement (LOCKED) |
| mp4 crate | mp4parse (Mozilla) | mp4parse is Firefox-internal, C API wrapper; API less ergonomic for metadata-only reads |
| imagesize | image crate reader.into_dimensions() | image crate would also work; imagesize is lighter (~16 byte reads vs. image crate's larger parse) |
| machine-uid | machineid-rs | machine-uid is simpler API (returns String); machineid-rs adds encryption not needed here |

**Installation (additions to existing Cargo.toml):**

```toml
[dependencies]
# Phase 2 additions
walkdir = "2"
symphonia = { version = "0.5", features = ["mp3", "aac", "isomp4", "mkv", "flac", "ogg", "wav"] }
mp4 = "0.14"
imagesize = "0.14"
uuid = { version = "1", features = ["v5"] }
machine-uid = "0.5"
```

---

## Architecture Patterns

### Recommended Project Structure

```
src/
├── main.rs              # Calls scanner::scan(), wraps in Arc<RwLock<>>, prints summary
├── cli.rs               # (Phase 1 — unchanged)
├── config.rs            # (Phase 1 — unchanged)
└── media/
    ├── mod.rs           # pub use scanner, library, metadata, mime
    ├── mime.rs          # (Phase 1 — unchanged) classify() returns (MediaKind, &'static str)
    ├── scanner.rs       # scan(paths, follow_symlinks) -> (Vec<MediaItem>, ScanStats)
    ├── metadata.rs      # extract_metadata(path, kind, mime) -> Option<MediaMeta>
    └── library.rs       # MediaItem, MediaMeta, MediaLibrary structs
```

### Pattern 1: MediaItem and MediaLibrary Structs

**What:** The core data types shared by all downstream phases.
**When to use:** Define these first; all other code in this phase populates them.

```rust
// Source: CONTEXT.md decisions + UPnP spec for duration format
use std::path::PathBuf;
use uuid::Uuid;
use crate::media::mime::MediaKind;

/// Metadata extracted from file headers at scan time.
#[derive(Debug, Clone)]
pub struct MediaMeta {
    /// HH:MM:SS.mmm — UPnP duration format (INDX-02)
    pub duration: Option<String>,
    /// "WxH" — pixel dimensions (INDX-03)
    pub resolution: Option<String>,
    /// Bitrate in bits per second (INDX-01)
    pub bitrate: Option<u32>,
    /// DLNA profile name (e.g., "AVC_MP4_MP_HD_720p_AAC") — None means omit from DIDL (INDX-04)
    pub dlna_profile: Option<&'static str>,
}

/// A single discovered media file with its extracted metadata.
#[derive(Debug, Clone)]
pub struct MediaItem {
    /// Stable UUIDv5: uuid5(machine_namespace, canonical_path_bytes)
    pub id: Uuid,
    pub path: PathBuf,
    pub file_size: u64,
    pub mime: &'static str,
    pub kind: MediaKind,
    pub meta: MediaMeta,
}

/// Flat in-memory library built at startup.
#[derive(Debug, Default)]
pub struct MediaLibrary {
    pub items: Vec<MediaItem>,
}
```

### Pattern 2: Scanner — walkdir with Per-Entry Error Handling

**What:** Walk all configured directories, classify by extension, skip non-media and failed files.
**When to use:** Core of `scanner.rs`.

```rust
// Source: docs.rs/walkdir/latest/walkdir/ (verified v2.5.0)
use walkdir::WalkDir;
use crate::media::mime::classify;

pub fn scan_paths(paths: &[std::path::PathBuf]) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for root in paths {
        // Warn and continue if directory is missing — LOCKED DECISION
        if !root.exists() {
            tracing::warn!("Scan path does not exist, skipping: {}", root.display());
            continue;
        }
        for entry in WalkDir::new(root).follow_links(true) {
            match entry {
                Err(e) => {
                    // Unreadable files / broken symlinks — LOCKED: log warn, continue
                    tracing::warn!("Cannot access: {}", e);
                }
                Ok(entry) if entry.file_type().is_file() => {
                    if classify(entry.path()).is_some() {
                        found.push(entry.path().to_owned());
                    }
                }
                Ok(_) => {} // directories — skip, walkdir handles recursion
            }
        }
    }
    found
}
```

### Pattern 3: Audio Duration via Symphonia

**What:** Extract duration (and bitrate) from audio files (and audio tracks in video containers) using Symphonia's demuxer.
**When to use:** For MediaKind::Audio and MediaKind::Video (duration of audio track in video).

```rust
// Source: github.com/pdeljanov/Symphonia issue #223 + docs.rs/symphonia-core Time struct
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;

pub fn extract_audio_duration(path: &Path) -> Option<std::time::Duration> {
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .ok()?;
    let format = probed.format;
    // Select first non-null track
    let track = format.tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)?;
    // Duration via n_frames × time_base (works for FLAC, WAV, OGG, M4A with known frame count)
    let time = track.codec_params.time_base?
        .calc_time(track.codec_params.n_frames?);
    // time.seconds: u64, time.frac: f64
    let total_ms = time.seconds * 1000 + (time.frac * 1000.0) as u64;
    Some(std::time::Duration::from_millis(total_ms))
}
```

**Note on MP3 without Xing tag:** `n_frames` may be `None` for variable-bitrate MP3 without an Xing/Info header. In that case, Symphonia may estimate duration from bitrate+file size — check that `n_frames` is available before using, and log a warning and skip duration if both are None.

### Pattern 4: Video Metadata via mp4 Crate

**What:** Extract width, height, duration for MP4/M4V files using `Mp4Reader`.
**When to use:** Only for `.mp4` and `.m4v` files (MIME `video/mp4`).

```rust
// Source: github.com/alfg/mp4-rust/blob/master/examples/mp4info.rs (verified API)
use std::io::BufReader;
use mp4::{Mp4Reader, TrackType};

pub fn extract_mp4_meta(path: &Path) -> Option<(u32, u32, u64, Option<u32>)> {
    // returns (width, height, duration_ms, bitrate)
    let f = std::fs::File::open(path).ok()?;
    let size = f.metadata().ok()?.len();
    let reader = BufReader::new(f);
    let mp4 = Mp4Reader::read_header(reader, size).ok()?;
    // Duration in milliseconds from the container
    let dur_ms = mp4.duration().as_millis() as u64;
    // Find the first video track
    for track in mp4.tracks().values() {
        if track.track_type().ok()? == TrackType::Video {
            return Some((
                track.width() as u32,
                track.height() as u32,
                dur_ms,
                Some(track.bitrate()),
            ));
        }
    }
    None
}
```

### Pattern 5: Image Dimensions via imagesize

**What:** Extract width and height from image header without full decode.
**When to use:** For MediaKind::Image files.

```rust
// Source: docs.rs/imagesize/latest/imagesize/ (verified v0.14.0)
use imagesize;

pub fn extract_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    match imagesize::size(path) {
        Ok(dim) => Some((dim.width as u32, dim.height as u32)),
        Err(e) => {
            tracing::warn!("Cannot read image dimensions for {}: {}", path.display(), e);
            None
        }
    }
}
```

### Pattern 6: UUIDv5 from Machine ID + File Path

**What:** Generate stable per-machine IDs for media items.
**When to use:** For each MediaItem at scan time.

```rust
// Source: docs.rs/uuid/latest (Uuid::new_v5 verified), docs.rs/machine-uid/latest
use uuid::Uuid;

pub fn build_machine_namespace() -> Uuid {
    // machine-uid returns the OS native machine ID string
    let machine_id = machine_uid::get().unwrap_or_else(|_| "unknown".to_string());
    // CONTEXT.md: namespace = uuid5(MACHINE_ID_NAMESPACE, machine_id)
    // Use a fixed application namespace UUID as outer namespace
    const APP_NS: Uuid = Uuid::from_bytes([
        0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1,
        0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
    ]); // NAMESPACE_DNS — a stable public namespace
    Uuid::new_v5(&APP_NS, machine_id.as_bytes())
}

pub fn media_item_id(machine_namespace: &Uuid, canonical_path: &Path) -> Uuid {
    Uuid::new_v5(machine_namespace, canonical_path.to_string_lossy().as_bytes())
}
```

### Pattern 7: UPnP Duration Formatting

**What:** Convert seconds + fractional seconds into `HH:MM:SS.mmm` format required by UPnP (INDX-02).
**When to use:** Whenever a duration is extracted, before storing in MediaMeta.

```rust
// Source: UPnP AV spec, UPnP duration format = "H+:MM:SS[.F+]" or "H+:MM:SS.mmm"
pub fn format_upnp_duration(total_seconds: u64, frac: f64) -> String {
    let h = total_seconds / 3600;
    let m = (total_seconds % 3600) / 60;
    let s = total_seconds % 60;
    let ms = (frac * 1000.0).round() as u32;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}
// Examples: "00:04:32.841", "01:23:45.000"
```

### Pattern 8: Arc<RwLock<MediaLibrary>> Construction in main.rs

**What:** Build the shared state in main.rs and pass Arc clone to each server component.
**When to use:** After scan completes, before server starts.

```rust
// Source: Tokio tutorial "Shared state" pattern (CONTEXT.md: LOCKED decision)
use std::sync::{Arc, RwLock};

// In main():
let library = scanner::scan(&config.paths)?;
let library = Arc::new(RwLock::new(library));
// Phase 3+ will: let lib = Arc::clone(&library);
```

### Anti-Patterns to Avoid

- **Storing MediaMeta fields as Option<String> with Some(""):** An empty string is not the same as "no data." Use None when metadata is unavailable.
- **Panicking on extraction failure:** Every metadata extraction path must return Option/Result; the scanner wraps all extraction in `match` and logs warn on None.
- **Loading full image data for dimensions:** Use `imagesize::size()`, not `image::open()`. Opening a 20MB TIFF to get 6000x4000 will stall the startup scan.
- **Using std::fs::read_dir recursively with manual impl:** Misses symlink loop detection, has less ergonomic error handling. Always use walkdir.
- **Generating UUID from path alone:** The path is only stable per-machine if the machine namespace is incorporated. Omitting machine-uid makes IDs collide across machines (wrong DLNA behavior).

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Recursive directory walk | Custom read_dir recursion | walkdir 2.5.0 | Symlink loop detection, proper error-per-entry semantics, controlled fd count |
| MP3/FLAC/OGG duration parsing | Parse Xing/ID3/Vorbis headers manually | symphonia 0.5.5 | Symphonia handles VBR estimation, gapless padding, chained OGG streams |
| MP4 box tree parsing | Hand-parse moov/trak/mdia atoms | mp4 0.14.0 | MP4 has many optional box variants; Mp4Reader::read_header() is tested against real-world files |
| Image dimension reading | Parse PNG IHDR / JPEG SOF0 manually | imagesize 0.14.0 | Handles EXIF rotation, progressive JPEGs, TIFF orientation |
| UUIDv5 generation | SHA1 + bit-twiddling | uuid 1.21.0 v5 feature | RFC 4122 compliant; one-liner `Uuid::new_v5()` |
| Machine ID lookup | Read /etc/machine-id directly | machine-uid 0.5.4 | Cross-platform: works on macOS (IORegistry) and Windows (registry) too |

**Key insight:** Container format headers are complex, versioned, and have many real-world edge cases (malformed atoms, partial writes, encoding errors). All of these crates are tested against thousands of real files. Hand-rolled parsers will fail on files from actual cameras, phones, and media players.

---

## Common Pitfalls

### Pitfall 1: Symphonia n_frames = None for Variable-Bitrate MP3

**What goes wrong:** `track.codec_params.n_frames` is `None` for MP3 files without an Xing/Info header. Duration calculation via `time_base.calc_time(n_frames?)` silently returns `None`, and the file gets no duration. This affects a significant portion of old MP3 files.
**Why it happens:** VBR MP3 requires the Xing header to know frame count without scanning the entire file. CBR MP3 uses bitrate + file size estimation.
**How to avoid:** When `n_frames` is None, attempt duration from `FormatReader::metadata()` or from packet accumulation. Per issue #223, "You don't actually need to decode. You can accumulate the value of Packet::dur." As a fallback, log a warn-level message and skip duration for that file (per LOCKED decision: skip rather than include with null fields).
**Warning signs:** All MP3 files returning no duration in tests.

### Pitfall 2: Symphonia Does Not Expose Video Resolution

**What goes wrong:** Trying to extract video width/height from `track.codec_params` in Symphonia. The struct has no width/height fields — it is audio-only.
**Why it happens:** Symphonia is an audio demuxing library. It can extract the audio track from an MP4 or MKV file but does not model video stream properties.
**How to avoid:** Use the `mp4` crate for MP4 video resolution. For MKV, either use `symphonia-format-mkv` API inspection (underdocumented) or skip resolution for MKV entirely since MKV is primarily audio-accompanying video for DLNA. The INDX-03 requirement covers video files and images — confirm MKV resolution approach during implementation.
**Warning signs:** Video files showing no resolution in MediaItem despite successful duration extraction.

### Pitfall 3: Subtitle Files Included as "Media Items" in the Flat List

**What goes wrong:** The Phase 1 `classify()` function returns `Some((MediaKind::Subtitle, "text/srt"))` for `.srt` files. If the scanner passes all classified files to metadata extraction and stores them as `MediaItem`, subtitles appear in the media library as first-class items. They should not be served via DLNA Browse.
**Why it happens:** The LOCKED decision says `.srt` must NOT be skipped (it must be recognized), but Phase 2's flat list is for DLNA media items only. Subtitle handling is Phase 3/5.
**How to avoid:** In `scanner.rs`, filter out `MediaKind::Subtitle` entries from the MediaLibrary `items` Vec. Store subtitle paths separately (or log and skip) pending Phase 3's subtitle association logic.
**Warning signs:** Samsung TV shows `.srt` files as playable items in the browse list.

### Pitfall 4: Arc<RwLock> vs Arc<Mutex> — RwLock Can Deadlock Under Write Starvation

**What goes wrong:** Using `std::sync::RwLock` in async context (Phase 3+ with Tokio) can cause deadlocks if a writer is held across an `.await` point.
**Why it happens:** The scan in Phase 2 is synchronous and happens before any async code. The `Arc<RwLock<MediaLibrary>>` will only be read (never written) after the scan completes. However, if Phase 3+ introduces writes, `std::sync::RwLock` is not safe to hold across await points.
**How to avoid:** Since the library is read-only after scan, `std::sync::RwLock` is safe. Document this constraint: "Library is written once at startup, then read-only." If future phases need writes (e.g., OPER-01 SIGHUP rescan), switch to `tokio::sync::RwLock`.
**Warning signs:** Deadlocks appearing only under async test load.

### Pitfall 5: Canonical Path Canonicalization for UUID Stability

**What goes wrong:** Two different paths that resolve to the same file (e.g., `/media/./Movies/foo.mp4` vs `/media/Movies/foo.mp4`) generate different UUIDs because the UUID is hashed from the raw path string.
**Why it happens:** String comparison of paths ignores `.` and `..` components and symlink resolution.
**How to avoid:** Always call `std::fs::canonicalize(path)` on the entry path before hashing for UUID generation. Handle canonicalize failures (returns Err for broken symlinks) by logging warn and skipping. Store the canonical path as the `MediaItem.path`.
**Warning signs:** The same file appearing twice in the library with different IDs after scanning through a symlinked directory.

### Pitfall 6: Zero-File Scan Must Exit, Not Silently Serve Empty Library

**What goes wrong:** All scanned paths are empty or contain no media files. The server starts and clients discover it but browse returns zero items. The LOCKED decision says this must be an error exit.
**Why it happens:** Easy to forget the exit condition when returning from scan.
**How to avoid:** After `scan()` returns, check `library.items.is_empty()`. If true, `tracing::error!("No media files found — exiting.")` and `std::process::exit(1)`. This must happen before the `Arc<RwLock<>>` is constructed.
**Warning signs:** Server starts but `Browse` returns zero results; no error reported to user.

---

## Code Examples

Verified patterns from official sources:

### Full walkdir traversal with error recovery

```rust
// Source: docs.rs/walkdir/latest/walkdir/ (v2.5.0, verified)
use walkdir::WalkDir;

for entry in WalkDir::new("/media").follow_links(true) {
    match entry {
        Ok(e) if e.file_type().is_file() => {
            // process file at e.path()
        }
        Ok(_) => {} // directory entry — skip
        Err(e) => tracing::warn!("Skipping inaccessible path: {}", e),
    }
}
```

### Symphonia duration extraction from track

```rust
// Source: github.com/pdeljanov/Symphonia issue #223 (community verified)
// time_base.calc_time(n_frames) → symphonia_core::units::Time { seconds: u64, frac: f64 }
let duration_opt = track.codec_params.time_base
    .and_then(|tb| track.codec_params.n_frames.map(|n| tb.calc_time(n)));
if let Some(t) = duration_opt {
    let upnp = format!("{:02}:{:02}:{:02}.{:03}",
        t.seconds / 3600,
        (t.seconds % 3600) / 60,
        t.seconds % 60,
        (t.frac * 1000.0).round() as u32,
    );
    // store upnp in MediaMeta.duration
}
```

### MP4 video track metadata

```rust
// Source: github.com/alfg/mp4-rust examples/mp4info.rs (verified API shape)
use mp4::{Mp4Reader, TrackType};
use std::io::BufReader;

let f = std::fs::File::open(&path)?;
let size = f.metadata()?.len();
let mp4 = Mp4Reader::read_header(BufReader::new(f), size)?;
let duration_ms = mp4.duration().as_millis() as u64;
for track in mp4.tracks().values() {
    if matches!(track.track_type(), Ok(TrackType::Video)) {
        let w = track.width();   // u16
        let h = track.height();  // u16
        let bps = track.bitrate(); // u32
    }
}
```

### imagesize dimension extraction

```rust
// Source: docs.rs/imagesize/latest/imagesize/ (v0.14.0)
match imagesize::size(&path) {
    Ok(dim) => {
        // dim.width: usize, dim.height: usize
        let resolution = format!("{}x{}", dim.width, dim.height);
    }
    Err(e) => tracing::warn!("Cannot read image size for {}: {}", path.display(), e),
}
```

### UUIDv5 generation

```rust
// Source: docs.rs/uuid/latest (v1.21.0, new_v5 verified)
// Source: docs.rs/machine-uid/latest (v0.5.4, machine_uid::get() verified)
use uuid::Uuid;

let machine_id = machine_uid::get().unwrap_or_else(|_| hostname_fallback());
let machine_ns = Uuid::new_v5(&Uuid::NAMESPACE_DNS, machine_id.as_bytes());
let item_id = Uuid::new_v5(&machine_ns, canonical_path.as_os_str().as_encoded_bytes());
```

---

## DLNA Profile Assignment

DLNA profiles are static string constants assigned based on container and (where detectable) codec. The profile is stored as `Option<&'static str>` — `None` means omit the `DLNA.ORG_PN=` component from `protocolInfo`.

### Profile Table (based on readymedia reference implementation)

| Container/Format | MIME | DLNA Profile | Notes |
|-----------------|------|--------------|-------|
| MP3 | audio/mpeg | `"MP3"` | Always assigned |
| AAC in MP4 (M4A) | audio/mp4 | `"AAC_ISO_320"` | When detectable as AAC |
| OGG/Vorbis | audio/ogg | None | No DLNA profile |
| FLAC | audio/flac | None | No DLNA profile |
| WAV | audio/wav | None | No DLNA profile |
| WMA | audio/x-ms-wma | None | No standard profile |
| MP4 H.264 video | video/mp4 | `"AVC_MP4_MP_HD_720p_AAC"` or similar | Profile depends on resolution tier; safe fallback = None |
| MPEG-TS | video/MP2T | `"MPEG_TS_SD_NA"` or `"MPEG_TS_HD_NA"` | Depends on resolution |
| AVI | video/x-msvideo | None | No DLNA profile |
| MKV | video/x-matroska | None | No DLNA profile |
| JPEG (any size) | image/jpeg | `"JPEG_LRG"` | Use LRG for unknown size; SM/MED/LRG by pixel area |
| PNG (any size) | image/png | `"PNG_LRG"` | Same tiered approach |
| GIF | image/gif | None | No profile |

**Simplification for Phase 2:** Profile assignment by extension is safe for audio (MP3 = "MP3", always). For video, the MP4 + H.264 profile detection requires reading the codec type from the track — this is doable with the `mp4` crate but adds complexity. **Recommended approach:** Assign MP3 profile definitively; assign image profiles based on size tier; for video containers, omit profile (None) in Phase 2 and enhance in Phase 5 when DLNA protocolInfo strings are built.

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Using ffmpeg subprocess for metadata | Pure Rust crates (symphonia, mp4) | Symphonia reached 0.5.x ~2022 | Eliminates system dependency, works in container/minimal env |
| Integer IDs (monotonic counter) | UUIDv5 from machine+path | Design decision for this project | IDs stable across restarts without persistence |
| Fully loading images to check size | imagesize header-only reads | imagesize 0.14.0 | Startup scan 10-100x faster for large image collections |
| std::fs::read_dir manual recursion | walkdir 2.x | walkdir stable for years | Proper symlink loop detection, correct error semantics |

**Deprecated/outdated:**
- `mp4parse` (Mozilla): Lower-level API designed for Firefox; ergonomics not suited for this use case vs `mp4` crate.
- `symphonium`: Unofficial wrapper around symphonia designed for loading audio into RAM; higher memory cost than metadata-only approach.

---

## Open Questions

1. **MKV video resolution extraction**
   - What we know: Symphonia's `CodecParameters` has no width/height fields; `symphonia-format-mkv` has 50% documentation coverage and it's unclear if it exposes video codec params
   - What's unclear: Is there a pure-Rust way to read MKV video width/height without writing a custom EBML parser?
   - Recommendation: During implementation, inspect `symphonia-format-mkv` source for CodecParameters extensions. If unavailable, skip resolution for MKV and log a debug note. MKV resolution is nice-to-have; the UPnP `resolution` attribute is optional in practice for most Samsung/Xbox clients.

2. **MP3 without Xing header — duration fallback**
   - What we know: `n_frames = None` for many VBR MP3 files without Xing tag; Symphonia estimates via bitrate+size but exposes this differently
   - What's unclear: Exact Symphonia API for the bitrate-estimated duration when n_frames is absent
   - Recommendation: Check `FormatReader::metadata()` and packet accumulation approach documented in issue #223. If neither works cleanly, omit duration for affected MP3 files (LOCKED: skip rather than null).

3. **DLNA profile for MP4/H.264 — resolution tier detection**
   - What we know: Profile names like `AVC_MP4_MP_HD_720p_AAC` are resolution-dependent; the mp4 crate provides width/height
   - What's unclear: Whether Phase 2 should implement full resolution-tiered profile lookup or defer to Phase 5
   - Recommendation: Implement simple extension-only table in Phase 2 (omit video profiles), then enhance in Phase 5 when the full protocolInfo string is built. Avoids premature complexity.

---

## Sources

### Primary (HIGH confidence)
- docs.rs/walkdir/latest/walkdir — WalkDir builder API, follow_links, DirEntry, error handling (v2.5.0 confirmed)
- docs.rs/uuid/latest uuid::Uuid::new_v5 — method signature, NAMESPACE_DNS constant (v1.21.0 confirmed)
- docs.rs/symphonia-core/latest/symphonia_core/units/struct.Time — Time struct fields (seconds: u64, frac: f64)
- docs.rs/symphonia-core/latest/symphonia_core/codecs/struct.CodecParameters — n_frames, time_base, audio-only (no video fields confirmed)
- docs.rs/imagesize/latest/imagesize — size() API (v0.14.0 confirmed)
- docs.rs/machine-uid/latest — machine_uid::get() API, platform support, v0.5.4 confirmed
- docs.rs/mp4/0.10.0/mp4/struct.Mp4Track — width(), height(), duration(), bitrate(), track_type() confirmed
- raw.githubusercontent.com/pdeljanov/Symphonia/master/symphonia/Cargo.toml — feature flag names (mp3, aac, isomp4, mkv, flac, ogg, wav, all) confirmed

### Secondary (MEDIUM confidence)
- github.com/pdeljanov/Symphonia issue #223 — duration via n_frames + time_base, packet accumulation fallback for VBR MP3 (community-verified, maintainer responded)
- github.com/alfg/mp4-rust examples/mp4info.rs — Mp4Reader::read_header(), tracks().values(), video track API
- github.com/necropotame/readymedia dlnameta.c — DLNA profile assignment table (reference implementation, cross-verified with allegrosoft cert doc)

### Tertiary (LOW confidence)
- Symphonia README claim that MKV has "Good" status — does not clarify whether video codec parameters (width/height) are exposed
- imagesize format coverage claim — "reads ≤16 bytes for most formats" not independently verified against all formats in Phase 1 MIME table

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all crate versions verified via docs.rs, crates.io, GitHub
- Architecture: HIGH — struct designs follow from locked decisions; patterns verified against official docs
- Pitfalls: HIGH — most pitfalls discovered from official docs and verified GitHub issues; MKV resolution pitfall is MEDIUM (incomplete upstream docs)
- DLNA profiles: MEDIUM — profiles confirmed from readymedia reference implementation, not from DLNA spec (paywalled)

**Research date:** 2026-02-22
**Valid until:** 2026-03-24 (30 days — symphonia and mp4 crate are stable; walkdir and uuid are very stable)
