# Phase 1: Project Setup & CLI - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Cargo project scaffold, CLI argument parsing, TOML config loading, and MIME type detection. Deliverable: a working `udlna` binary that accepts media directory paths, loads optional TOML config with correct flag/config/default precedence, and detects media file types by extension. No networking, no scanning, no streaming — just the foundation everything else builds on.

</domain>

<decisions>
## Implementation Decisions

### MIME type coverage
- Detect `.srt` files as a recognized type alongside video/audio/image extensions
- Silent skip for unrecognized extensions (no error output for non-media files)

### Claude's Discretion
- Config file discovery strategy (CWD-only vs XDG paths vs --config flag)
- Invalid path handling (error-and-exit vs warn-and-skip)
- Logging verbosity flags and default log level
- Startup output format
- Full list of supported media extensions

</decisions>

<specifics>
## Specific Ideas

- SRT subtitle files should be recognized/tracked — they'll need to be delivered alongside video files in a later phase

</specifics>

<deferred>
## Deferred Ideas

- Serving .srt subtitle files alongside video files — belongs in Phase 3 (HTTP streaming) or Phase 5 (ContentDirectory)
- Inline subtitle extraction from MKV containers — separate concern, future phase

</deferred>

---

*Phase: 01-project-setup-cli*
*Context gathered: 2026-02-22*
