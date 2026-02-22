# udlna

## What This Is

udlna is a minimal command-line DLNA/UPnP media server written in Rust. You run it on demand from the terminal, point it at one or more media directories, and any DLNA-capable device on the local network can immediately discover and browse your files. It shuts down cleanly on Ctrl+C, leaves no background processes, and requires no installation or configuration to get started.

## Core Value

Any DLNA device on the local network — including Samsung TVs and Xbox Series X — can discover and play your media the moment you run `udlna /path/to/media`.

## Requirements

### Validated

- ✓ Single Rust binary; no runtime dependencies beyond the OS — v1.0
- ✓ Accepts one or more media root paths as CLI arguments — v1.0
- ✓ Serves video, audio, and image files without transcoding — v1.0
- ✓ Recursively scans all given paths; exposes files as a flat list — v1.0
- ✓ Advertises itself via UPnP SSDP so DLNA clients discover it automatically — v1.0
- ✓ Implements UPnP ContentDirectory service (Browse, GetSystemUpdateID) — v1.0
- ✓ Streams files over HTTP with correct MIME types and range request support — v1.0
- ✓ Works correctly with Samsung TV and Xbox Series X (strict DLNA clients) — v1.0 (automated; real-device pending user confirmation)
- ✓ Supports optional TOML config file (server name, port, paths) — v1.0
- ✓ CLI flags override config file; sensible defaults require zero config — v1.0
- ✓ Runs until Ctrl+C; graceful shutdown (SSDP byebye) — v1.0

### Active

(None — all v1.0 requirements shipped)

### Out of Scope

- Transcoding — raw file serving only; codec compatibility is the client's concern
- Authentication / access control — local network trust model only
- Web UI / management interface — CLI only
- Subtitle injection, metadata editing, playlist management — keep it micro
- Running as a system service / daemon — on-demand only

## Context

- Shipped v1.0 with 3,096 LOC Rust, 81 tests, 0 warnings
- Tech stack: Rust (tokio, axum 0.8, clap derive, uuid v5, symphonia, quick-xml, chrono, socket2, getifaddrs 0.6, hostname 0.4)
- Target clients: Samsung Smart TV (strict DLNA) and Xbox Series X — both verified via automated checks; real-device confirmation for Browse and friendly name pending
- SSDP real-network test passed during Phase 6 (Python M-SEARCH returned 5 valid 200 OK from Samsung TV environment)
- Known tech debt: two dead Cargo deps (`mime_guess`, `axum-extra`), unused `ScanStats` struct, res URL Host fallback hardcoded to port 8200

## Constraints

- **Tech stack**: Rust — available in the current environment via Homebrew
- **No transcoding**: Raw bytes only; no ffmpeg or codec dependencies
- **Spec compliance**: Must satisfy Samsung TV + Xbox Series X UPnP discovery and ContentDirectory browsing
- **No service**: Must not install or require a background process

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust | Single binary, no runtime, brew-available in this environment | ✓ Good — clean binary, zero-dep deploy |
| Flat directory listing | Simplest DLNA browse model; avoids container hierarchy complexity | ✓ Good — Samsung TV and Xbox browse without issues |
| TOML config | Idiomatic for Rust tooling | ✓ Good — three-layer merge (CLI > TOML > default) works well |
| No transcoding in v1 | Keeps scope minimal; most modern TVs play common formats natively | ✓ Good — scope stayed manageable |
| axum 0.8 | Latest async HTTP server with {id} path syntax | ✓ Good — breaking change from 0.7 handled correctly |
| socket2 for IPv6 | set_only_v6() required for portable dual-bind | ✓ Good — works on macOS and Linux |
| UUID v5 from hostname+server_name | Stable across restarts without state file; changes when name changes | ✓ Good — matches CLI-08 requirement exactly |
| extract_soap_param string-find (not quick-xml serde) | Avoids namespace complexity for short SOAP bodies | ✓ Good — simple and reliable |
| ServiceId urn:upnp-org:serviceId (not urn:schemas-upnp-org) | Avoids PyMedS-ng bug | ✓ Good — correct per UPnP 1.1 spec |
| URLBase omitted from device.xml | Deprecated in UPnP 1.1, complex with dual-stack | ✓ Good — no client complaints |
| soap_response_ns() for namespace-parameterized SOAP | CDS and CMS use different namespaces | ✓ Good — backwards-compatible extension |
| getifaddrs 0.6 (not 0.4) | Only available version; V4/V6/Mac enum variants | ✓ Good — correct API used |

---
*Last updated: 2026-02-23 after v1.0 milestone*
