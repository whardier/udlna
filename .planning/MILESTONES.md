# Milestones

## v1.0 MVP (Shipped: 2026-02-23)

**Phases completed:** 8 phases (1–8), 20 plans
**Lines of code:** 3,096 Rust (single binary, no runtime deps)
**Tests:** 81 passing, 0 failures
**Build:** 0 compiler warnings

**Key accomplishments:**
1. Single Rust binary: `udlna /path/to/media` — no install, no config required
2. Recursive media scanner with metadata extraction (duration, resolution, bitrate) via symphonia/mp4/imagesize
3. HTTP file server with RFC 7233 byte-range streaming, dual IPv4+IPv6 bind, DLNA transfer headers
4. UPnP device description (device.xml, CDS SCPD, CMS SCPD) — UPnP 1.0/1.1 compliant
5. ContentDirectory Browse/BrowseMetadata with DIDL-Lite XML — Samsung TV and Xbox Series X verified
6. SSDP NOTIFY/M-SEARCH multicast discovery — real Samsung TV discovery confirmed
7. ConnectionManager GetProtocolInfo advertising 25 supported MIME types — Xbox requirement
8. UUID v5 from hostname+server_name, `--name` flag wires to `<friendlyName>` on DLNA device lists

**Tech debt carried forward:**
- `mime_guess` and `axum-extra` are dead Cargo dependencies (no functional impact)
- `ScanStats` struct is unused (stats logged via tracing instead)
- `res` URL Host fallback hardcoded to port 8200 (edge case: non-8200 port + broken HTTP client)

**Archive:** `.planning/milestones/v1.0-ROADMAP.md`, `.planning/milestones/v1.0-REQUIREMENTS.md`

---
