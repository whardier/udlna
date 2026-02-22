# Phase 2: Media Scanner & Metadata - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Recursive directory walk at startup → container metadata extraction → in-memory media library with stable IDs. This phase delivers the shared `MediaLibrary` state that all downstream phases (HTTP streaming, ContentDirectory SOAP, etc.) depend on. File serving, DLNA XML, and network protocols are later phases.

</domain>

<decisions>
## Implementation Decisions

### Metadata extraction strategy
- **Pure Rust only** — no ffmpeg subprocess, no FFI bindings
- Use whatever formats pure-Rust crates cover well (symphonia for audio, mp4parse for MP4, matroska/nom for MKV, image crate for images)
- Don't chase obscure formats — support what the crate ecosystem handles cleanly
- If metadata extraction fails for a file: **skip the file entirely** (do not include with null fields)

### DLNA profile assignment
- When a DLNA profile cannot be determined (unknown codec/container): **omit the DLNA profile field** entirely
- Do not use wildcard (`*`) — don't assign a profile we can't determine

### Failure & error handling
- Missing CLI/config directory: **warn and continue** scanning the rest — do not hard-fail on missing directories
- Unreadable files (permission denied, broken symlinks): **log at warn level** so user sees what was skipped
- Zero media files found after scanning: **exit with error** — refuse to start with nothing to serve
- Symlinks: **follow symlinks** (traverse symlinked directories, include symlinked files)

### Scan progress & logging
- Scan happens **synchronously** — scan completes before server starts accepting connections
- Startup banner prints **before** scan begins; summary line prints **after** scan completes
- Summary format: `"Scanned N files (X video, Y audio, Z image) in T.Ts"`

### Media item IDs & state
- IDs use **UUIDv5**: namespace derived from **machine ID**, name is the canonical file path
- Same file on same machine always gets the same ID across restarts
- Shared server state structure: `Arc<RwLock<MediaLibrary>>` — built in Phase 2, ready for thread-safe access in Phase 3+
- Scan at startup only — no filesystem watching, no periodic re-scan; restart server to pick up new files

</decisions>

<specifics>
## Specific Ideas

- User explicitly wants UUIDv5 for all media item IDs: `uuid5(namespace=uuid5(MACHINE_ID_NAMESPACE, machine_id), name=file_path)`
- Machine ID is the namespace seed — IDs are stable per-machine but differ across machines (correct DLNA behavior)
- The `Arc<RwLock<MediaLibrary>>` pattern is explicitly confirmed — build this in Phase 2 so Phase 3 just clones the Arc

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-media-scanner-and-metadata*
*Context gathered: 2026-02-22*
