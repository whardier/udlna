# Project Research Summary

**Project:** udlna — Minimal DLNA/UPnP Media Server
**Domain:** DLNA/UPnP protocol implementation (Rust, CLI)
**Researched:** 2026-02-22
**Confidence:** MEDIUM (protocol specs are frozen and HIGH confidence; Rust ecosystem versions and Samsung/Xbox behavioral quirks are MEDIUM due to web verification being unavailable)

## Executive Summary

udlna is a no-config, single-binary, CLI-driven DLNA/UPnP media server targeting Samsung Smart TVs and Xbox Series X as primary clients. The core value proposition is radically lower friction than existing DLNA servers (MiniDLNA, Jellyfin, Plex): `udlna /path/to/media` runs immediately with no database, no daemon, no configuration file required. This product occupies a real gap — there is no mature Rust DLNA server framework, so every protocol layer (SSDP, SOAP/XML, HTTP streaming) must be hand-assembled from purpose-built crates or implemented from scratch. This is tractable because the required UPnP surface is small: device description XML, the ContentDirectory Browse action, and a minimal ConnectionManager stub.

The recommended approach is tokio for async runtime, hyper 1.x for HTTP (not a framework — DLNA's strict header requirements demand direct control), socket2 for SSDP multicast sockets, quick-xml for XML generation/parsing, and clap for CLI. SSDP server-side implementation should be hand-rolled (~200-300 lines) because no Rust crate provides a UPnP device/server role. The architecture is two independent async tasks (SSDP on UDP, HTTP on TCP) sharing an immutable `Arc<ServerState>` built at startup. Build order matters: media scanner and shared state first, then HTTP endpoints (testable with curl), then ContentDirectory SOAP/DIDL-Lite, then SSDP discovery, then ConnectionManager and DLNA-specific headers.

The two highest risks are DIDL-Lite XML correctness (Samsung TVs are the strictest DLNA clients in common use and fail silently) and SOAP envelope escaping (DIDL-Lite XML must be XML-escaped inside the SOAP Result element — this is the single most common implementation mistake). Both risks are mitigated by using proper XML builders from day one, never string concatenation, and testing against real Samsung hardware early. Secondary risks include SSDP multicast interface binding on macOS, HTTP Range request correctness, and DLNA-specific HTTP headers that VLC/lenient clients don't require but Samsung/Xbox enforce strictly.

## Key Findings

### Recommended Stack

The Rust DLNA/UPnP ecosystem has no batteries-included server framework. The practical approach is to assemble crates for each protocol layer and hand-implement SSDP server logic. This is actually appropriate for the project's small UPnP surface area. See `/Users/spencersr/tmp/udlna/.planning/research/STACK.md` for full rationale and Cargo.toml.

**Core technologies:**
- **tokio** ^1.36: Async runtime — gravitational center of the Rust networking ecosystem; hyper and every serious networking crate builds on it
- **hyper** 1.x + hyper-util + http-body-util: HTTP server — chosen over axum/actix specifically because DLNA requires precise control over Content-Range, transferMode.dlna.org, contentFeatures.dlna.org, and other headers that framework abstractions obscure
- **socket2** ^0.5: Low-level UDP socket control — needed for SO_REUSEADDR, IP_ADD_MEMBERSHIP, and multicast TTL required by SSDP
- **Hand-rolled SSDP**: ~200-300 lines — no usable Rust server-side SSDP crate exists; rupnp and ssdp-client are client-only (they discover devices, not advertise as one)
- **quick-xml** ^0.36: XML serialization/deserialization — fastest Rust XML parser, handles both reading (SOAP requests) and writing (DIDL-Lite responses); critical because DLNA requires precise namespace control that serde-xml-rs cannot provide
- **clap** ^4.5: CLI argument parsing — de facto standard
- **serde** + **toml** ^0.8: Configuration — for optional TOML config file
- **walkdir** ^2.5: Recursive directory traversal — handles symlink loops and permission errors gracefully
- **tracing** + **tracing-subscriber**: Logging — modern async-native structured logging; essential for debugging why Samsung/Xbox rejects a response
- **mime_guess** ^2.0: MIME type detection — extension-based is correct for no-transcode file serving
- **uuid** ^1.7: UPnP device UUID — stable UUID per run, persisted in config

**Important version note:** All versions are from training data (early 2025). Verify against crates.io before pinning.

### Expected Features

See `/Users/spencersr/tmp/udlna/.planning/research/FEATURES.md` for full feature tables, dependency graph, and competitor analysis.

**Must have (table stakes) — Samsung TV and Xbox will not work without all of these:**
- SSDP discovery: M-SEARCH response + NOTIFY alive/byebye + periodic re-advertisement
- UPnP device description XML with MediaServer:1 type, ContentDirectory + ConnectionManager services, and `dlna:X_DLNADOC` element
- ContentDirectory Browse action (BrowseDirectChildren + BrowseMetadata) with correct DIDL-Lite XML and DLNA protocolInfo fourth field
- ContentDirectory mandatory actions: GetSearchCapabilities, GetSortCapabilities, GetSystemUpdateID (all trivial stubs)
- ConnectionManager mandatory actions: GetProtocolInfo, GetCurrentConnectionIDs, GetCurrentConnectionInfo (minimal stubs; Xbox requires these)
- SCPD service description XML for both services
- HTTP file serving with RFC 7233 Range support (206 Partial Content) — without this, seeking fails entirely
- DLNA HTTP headers: `transferMode.dlna.org: Streaming` and `contentFeatures.dlna.org` with DLNA.ORG_OP=01 — Samsung refuses streams without these
- HEAD request support — Samsung sends HEAD before every GET
- MIME type detection from file extension + correct DLNA protocolInfo flags
- Recursive filesystem scan at startup, flat file listing
- Graceful shutdown with SSDP byebye on Ctrl+C
- CLI positional args for media paths

**Should have (competitive differentiators, v1.x):**
- TOML config file (optional, for repeated use)
- Custom `--name` flag for friendly server name shown on TVs
- Multiple root paths exposed as separate containers (hierarchical view)
- Hierarchical directory browsing (folder structure as UPnP containers)
- SIGHUP rescan without restart
- Subtitle file association (.srt/.sub alongside videos for Samsung external subtitle support)
- `<res size="...">` attribute in DIDL-Lite (strongly recommended for Samsung seek position calculation)

**Defer (v2+):**
- Thumbnail serving (requires image processing library)
- Search action (requires metadata indexing + UPnP search query parsing)
- Content sorting
- DLNA profile detection beyond extension mapping
- UPnP eventing (SUBSCRIBE/NOTIFY)
- IPv6 support

**Anti-features (explicitly out of scope):**
- Transcoding — conflicts with single-binary, no-dependency goal; refer users needing transcoding to Plex/Jellyfin
- Media metadata database — conflicts with ephemeral operation; scan filesystem at startup
- Web UI — different product category entirely

### Architecture Approach

The architecture is two independent async tasks (SSDP on UDP multicast, HTTP on TCP) sharing immutable state via `Arc<ServerState>`. All media metadata is loaded at startup into an in-memory `Vec<MediaItem>` (with a `HashMap<String, usize>` for O(1) ID lookup once scale warrants it). No mutex needed because the library is read-only after scan. Graceful shutdown uses a cancellation token to coordinate byebye SSDP messages before socket teardown. See `/Users/spencersr/tmp/udlna/.planning/research/ARCHITECTURE.md` for complete protocol flow diagrams, exact XML/SOAP examples, and module structure.

**Major components:**
1. **Media Scanner** (`media/scanner.rs`, `media/library.rs`) — walks directories, detects MIME types, assigns stable integer IDs, builds `Vec<MediaItem>` at startup; no network concerns, easily unit-tested
2. **SSDP Module** (`ssdp/listener.rs`, `ssdp/responder.rs`, `ssdp/advertiser.rs`) — UDP multicast listener for M-SEARCH, unicast responder, periodic NOTIFY alive sender, byebye on shutdown; independent async task
3. **HTTP Server** (`http/`) — single TCP listener with path-based routing: `/device.xml` for device description, `/cds/*` for ContentDirectory SOAP, `/cms/*` for ConnectionManager SOAP, `/media/*` for file streaming
4. **ContentDirectory Service** (`http/content_directory.rs`, `xml/didl.rs`, `xml/soap.rs`) — SOAP request parser, DIDL-Lite XML builder, Browse + GetSystemUpdateID logic; most complex component
5. **File Streamer** (`http/streaming.rs`) — async file I/O with Range header parsing, 206 Partial Content responses, DLNA-specific HTTP headers
6. **Shared State** (`server.rs`) — `Arc<ServerState>` holding config, media library, UUID, SystemUpdateID; read-only after startup

**Recommended module layout:**
```
src/
├── main.rs, config.rs, server.rs
├── ssdp/ (listener, responder, advertiser)
├── http/ (device, scpd, soap, content_directory, connection_manager, streaming)
├── media/ (scanner, library)
└── xml/ (device_desc, didl, soap)
```

### Critical Pitfalls

See `/Users/spencersr/tmp/udlna/.planning/research/PITFALLS.md` for full detail including warning signs, recovery strategies, and client-specific gotcha tables.

1. **Malformed DIDL-Lite XML breaks Samsung TVs** — Samsung is the strictest DLNA client; missing namespaces, wrong element ordering, missing protocolInfo, or self-closing `<res>` tags all cause silent failure (empty folder, no error). Prevention: use quick-xml from day one (never string concatenation for dynamic XML), declare all four namespaces on DIDL-Lite root, test against real Samsung hardware early; compare output against MiniDLNA as the gold standard

2. **SOAP envelope escaping** — DIDL-Lite XML inside the SOAP `<Result>` element MUST be XML-escaped (`<` → `&lt;`, `&` → `&amp;`). Raw XML embedded without escaping makes the SOAP envelope malformed. This is the single most common implementation mistake and produces invisible client failures. Prevention: always use an XML library for SOAP construction; validate all SOAP responses through xmllint

3. **SSDP multicast interface binding failures** — On macOS (multiple NICs: en0, en1, lo0, utun*, bridge*), binding to `0.0.0.0` without calling `IP_ADD_MEMBERSHIP` per interface causes multicast packets to go out on the wrong interface. Also: missing periodic re-advertisement means devices that power on after the server starts never discover it. Prevention: enumerate interfaces, join multicast group on each non-loopback IPv4 interface; implement full SSDP lifecycle (initial NOTIFY × 2-3, periodic every 900s, byebye on shutdown)

4. **HTTP Range request correctness** — Samsung TVs require 206 Partial Content with correct `Content-Range: bytes start-end/total` (off-by-one in last_byte = file size - 1 is a common bug); `Accept-Ranges: bytes` must be present on all media responses; `RequestedCount=0` in Browse must be interpreted as "no limit" (per UPnP spec), not "return zero items." Prevention: implement full RFC 7233 from the start; test with `curl -H "Range: bytes=0-0" -v`

5. **Missing DLNA-specific HTTP headers** — `contentFeatures.dlna.org` and `transferMode.dlna.org` are documented only in the paid DLNA Guidelines (not the free UPnP spec), so developers miss them. Samsung refuses streams without these. The `contentFeatures.dlna.org` value must also match the protocolInfo fourth field in DIDL-Lite. Prevention: build a `DlnaHeaders` utility module early; hardcode `DLNA.ORG_OP=01;DLNA.ORG_FLAGS=01700000000000000000000000000000` for all file content

6. **Stable UDN required** — Generating a new random UUID on each startup causes Samsung TVs to accumulate stale duplicate entries and Xbox to get confused with cached state. Prevention: generate UUID once, persist in TOML config or state file, reuse on subsequent runs

## Implications for Roadmap

The build order is dictated by testability: get the lowest-layer components working first so each layer can be verified before adding the next. The architecture research provides a specific 6-phase build order that the pitfalls research confirms.

### Phase 1: Foundation — Config, Media Scanner, Shared State

**Rationale:** Every other component depends on `ServerConfig`, `MediaLibrary`, and `Arc<ServerState>`. No networking, no protocols — pure data and I/O. Easy to unit test. Must come first.
**Delivers:** Working directory scanner that produces a `Vec<MediaItem>` with MIME types, stable IDs, and file sizes; CLI argument parsing; TOML config loading; `Arc<ServerState>` structure
**Addresses:** File system scanning (table stakes), CLI argument parsing (table stakes)
**Avoids:** Path traversal vulnerability (canonicalize all paths before storing; verify served paths are under media root)
**Research flag:** Standard patterns — no deeper research needed. walkdir, clap, serde/toml are well-documented.

### Phase 2: HTTP Core — File Streaming + Device/SCPD XML

**Rationale:** HTTP is testable with curl before any DLNA client is involved. Building and validating the HTTP layer in isolation prevents debugging confusion later. Streaming is simpler than SOAP/DIDL-Lite and provides immediate feedback (files play in a browser). Device and SCPD XML are static documents — implement them here so the HTTP router is complete.
**Delivers:** TCP listener with path router; GET /device.xml (static, with runtime UUID/name/IP substitution); GET /cds/scpd.xml and /cms/scpd.xml (static); GET/HEAD /media/{id} with RFC 7233 Range support (206 Partial Content); DLNA HTTP headers (contentFeatures.dlna.org, transferMode.dlna.org, Accept-Ranges)
**Implements:** HTTP Server component, File Streamer component, Device Description component
**Avoids:** HTTP Range request bugs (pitfall 3), Missing DLNA HTTP headers (pitfall 5), Large file serving problems (use tokio::fs streaming, never read_to_end)
**Research flag:** Range request handling and DLNA header format are well-documented in research. No additional research needed. Verify with `curl -r` tests.

### Phase 3: ContentDirectory Service — SOAP + DIDL-Lite

**Rationale:** This is the most complex phase and the highest-risk area. SOAP parsing, DIDL-Lite generation, Browse action logic, and pagination must all work correctly and produce Samsung-compatible XML before any discovery layer is added. Testing via direct `curl POST` to the SOAP endpoint avoids the compounding difficulty of debugging SOAP and SSDP simultaneously.
**Delivers:** SOAP envelope parser (extract action + arguments); DIDL-Lite XML builder with correct namespace declarations and XML-escaped content inside SOAP Result; Browse action (BrowseDirectChildren + BrowseMetadata, ObjectID "0" root, StartingIndex/RequestedCount pagination); GetSearchCapabilities, GetSortCapabilities, GetSystemUpdateID stubs; NumberReturned/TotalMatches accuracy
**Implements:** ContentDirectory Service, XML modules (didl, soap)
**Avoids:** Malformed DIDL-Lite XML (pitfall 1 — use quick-xml, declare all four namespaces, include protocolInfo on every res element), SOAP escaping bug (pitfall 2 — DIDL-Lite must be XML-escaped inside Result), Xbox Browse quirks (RequestedCount=0 = no limit, not zero items)
**Research flag:** SOAP/DIDL-Lite are the highest-risk area. Reference MiniDLNA's Browse response as the gold standard during implementation. Validate every SOAP response with `xmllint`. Test against Samsung TV as early as possible in this phase.

### Phase 4: SSDP Discovery

**Rationale:** SSDP ties everything together — once working, real DLNA clients can discover the server. It's implemented after HTTP+SOAP because clients that discover the server will immediately attempt to fetch device.xml, SCPD files, and call Browse; if those aren't ready, debugging SSDP becomes impossible. SSDP is also the hardest to debug (UDP multicast, no connection-level errors, macOS interface complexity).
**Delivers:** UDP multicast socket joined on all non-loopback IPv4 interfaces (socket2); M-SEARCH listener and unicast responder; NOTIFY alive on startup (sent 2-3 times for UDP reliability); periodic re-advertisement every 900 seconds; NOTIFY byebye on Ctrl+C (graceful shutdown via tokio signal); correct USN/LOCATION headers in all SSDP messages; two independent async tasks (SSDP + HTTP) joined with tokio::select!
**Implements:** SSDP Module, graceful shutdown coordination
**Avoids:** SSDP multicast interface binding failures (pitfall — enumerate interfaces, use IP_ADD_MEMBERSHIP per interface), missing re-advertisement (server disappears from TV after 30 min), wrong LOCATION IP (use local-ip-address, never 0.0.0.0 or 127.0.0.1)
**Research flag:** SSDP interface binding on macOS is the trickiest part. The hand-rolled implementation is well-specified in the architecture research. Use gssdp-discover and Wireshark to verify.

### Phase 5: ConnectionManager + DLNA Compliance Polish

**Rationale:** ConnectionManager is minimal (three stub actions) but Xbox requires it before connecting. This phase also catches any remaining DLNA compliance gaps from testing with real Samsung and Xbox hardware. MIME type mapping edge cases surface here.
**Delivers:** ConnectionManager SOAP handler (GetProtocolInfo returning supported MIME list, GetCurrentConnectionIDs returning "0", GetCurrentConnectionInfo returning defaults); MIME type edge case overrides (e.g., Samsung firmware differences for .mkv); persistent UUID (generate once, store in config); SOAP fault responses for unknown actions; UX improvements (startup summary output, file count, bound IP:port)
**Implements:** ConnectionManager Service
**Avoids:** Missing ConnectionManager (Xbox refuses to connect — pitfall 7), MIME type mismatches (pitfall 6), UUID changing between restarts (pitfall — deterministic UDN from hostname)
**Research flag:** Xbox-specific SOAP quirks (X_GetFeatureList, X-AV-Client-Info headers) are documented in research. Standard patterns apply. No deeper research needed.

### Phase 6: Polish + v1.x Features

**Rationale:** After Samsung TV and Xbox validation with v1 MVP, add quality-of-life features that have no compatibility risk.
**Delivers:** TOML config file for persistent preferences; `--name` flag for custom friendly name; `--port` flag; multiple root paths as separate UPnP containers; hierarchical directory browsing (expose folder tree as containers); `<res size="...">` attribute in DIDL-Lite; subtitle file association (.srt/.sub); SIGHUP rescan
**Avoids:** None of the critical pitfalls (they're addressed in phases 1-5)
**Research flag:** Hierarchical directory browsing requires designing a container ID scheme. This is well-understood but needs care — may benefit from a brief design step before implementation.

### Phase Ordering Rationale

- **Foundation before networking:** Config and media library have no dependencies and enable all components
- **HTTP before SSDP:** curl-testable endpoints expose bugs before adding the compounding complexity of SSDP; also, SSDP's LOCATION must point to working HTTP endpoints
- **Streaming before SOAP:** Simpler protocol, immediate browser-testable feedback; establishes the HTTP router structure
- **ContentDirectory before SSDP:** A real DLNA client that discovers the server will immediately call Browse; broken Browse is invisible through SSDP debugging
- **ConnectionManager with polish phase:** Functionally minimal, but separating it allows phases 1-4 to be validated with VLC before adding Xbox as a target client
- **This order directly avoids the compounding-failure trap** described in PITFALLS.md: debugging SSDP while Browse is broken means every client failure could be either discovery or content issues, with no way to distinguish them

### Research Flags

Phases likely needing deeper research or careful reference-checking during implementation:
- **Phase 3 (ContentDirectory/DIDL-Lite):** Highest risk phase. Cross-reference MiniDLNA/ReadyMedia source code as the reference implementation for Samsung-compatible DIDL-Lite. Validate XML output against xmllint before device testing. Samsung provides zero error messages on failure.
- **Phase 4 (SSDP on macOS):** Interface enumeration and multicast group joining differ between Linux and macOS. Use Wireshark + gssdp-discover to verify. The architecture research provides exact SSDP message formats but macOS network stack behavior may require iteration.

Phases with well-documented standard patterns (can proceed without additional research):
- **Phase 1 (Foundation):** walkdir, clap, serde/toml are thoroughly documented standard crates
- **Phase 2 (HTTP/Streaming):** RFC 7233 is unambiguous; hyper 1.x documentation is comprehensive; DLNA header values are specified in research
- **Phase 5 (ConnectionManager):** Three trivial stub actions; MIME type map is a lookup table

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | MEDIUM | Core choices (tokio, hyper, quick-xml, clap) are HIGH confidence and stable. Version numbers need crates.io verification. The "hand-roll SSDP" recommendation is robust regardless of ecosystem state. |
| Features | MEDIUM | UPnP spec requirements (ContentDirectory action list, SOAP format, DIDL-Lite structure) are HIGH confidence — frozen specs. Samsung/Xbox behavioral quirks are MEDIUM — based on community reports in training data, not verified against current firmware. |
| Architecture | HIGH | UPnP/DLNA are frozen specifications (DLNA org dissolved 2017, UPnP AV specs unchanged since ~2015). Protocol details, multicast addresses, XML namespaces, SOAP structures are stable facts. The two-task (SSDP+HTTP) + immutable shared state pattern is an established Rust async pattern. |
| Pitfalls | MEDIUM | Protocol-level pitfalls (SOAP escaping, Range request format, SSDP interface binding) are HIGH confidence. Client-specific quirks (Samsung firmware-specific behavior, Xbox DLNA stack edge cases) are MEDIUM — consistent across multiple training data sources but not verified against current firmware. |

**Overall confidence:** MEDIUM

### Gaps to Address

- **Samsung firmware version variation:** MIME type handling (especially for .mkv) varies by Samsung TV model and firmware version. Build a configurable MIME override map and expect to iterate based on real-device testing. The research-recommended starting point (video/x-matroska for .mkv) is the most broadly compatible but not universal.

- **Crate version verification:** All recommended crate versions (tokio ^1.36, hyper ^1.2, quick-xml ^0.36, etc.) are from training data. Run `cargo add` for each dependency before starting implementation to confirm current compatible versions.

- **local-ip-address crate maintenance status:** Flagged as MEDIUM confidence in stack research. If unmaintained, substitute `if-addrs` crate (provides same capability). Verify on crates.io before depending on it.

- **DLNA.ORG_PN profile name vs. wildcard:** Using specific DLNA profile names (e.g., `AVC_MP4_MP_SD_AAC`) requires knowing the exact codec configuration, which isn't determinable from file extension alone. The recommended approach (use `DLNA.ORG_OP=01;DLNA.ORG_FLAGS=01700000000000000000000000000000` without a specific PN, or `DLNA.ORG_PN=*`) should be validated against actual Samsung TVs — some firmware versions behave differently.

- **Xbox X_GetFeatureList SOAP call:** Xbox sends this Microsoft-specific extension action. It must return a SOAP fault (401 Invalid Action) rather than crashing. Verify this behavior during Xbox testing in Phase 5.

## Sources

### Primary (HIGH confidence)
- UPnP Device Architecture 1.0 specification (UPnP Forum / Open Connectivity Foundation) — device description schema, SSDP protocol, service model
- UPnP AV ContentDirectory:1 Service Template (UPnP Forum) — mandatory/optional action list, DIDL-Lite schema, Browse action parameters
- UPnP AV ConnectionManager:1 Service Template (UPnP Forum) — GetProtocolInfo, connection model
- RFC 7233: HTTP Range Requests — partial content response format, Content-Range header
- hyper 1.0 release announcement and migration guide — hyper 1.x API changes from 0.14

### Secondary (MEDIUM confidence)
- DLNA Guidelines (DLNA.org, frozen 2017) — DLNA-specific HTTP headers, protocolInfo fourth field format, X_DLNADOC element requirement
- MiniDLNA/ReadyMedia source code patterns — Samsung/Xbox compatibility techniques, DIDL-Lite structure reference
- Gerbera DLNA server documentation and issue tracker — client compatibility reports
- Rust ecosystem knowledge through early 2025 — crate selection, version compatibility

### Tertiary (LOW-MEDIUM confidence)
- Samsung Smart TV DLNA community reports — Samsung-specific firmware quirks (X_DLNADOC requirement, MIME type handling, protocolInfo strictness)
- Xbox Series X DLNA community reports — Xbox-specific SOAP edge cases, ConnectionManager requirement, RequestedCount=0 behavior

**Note:** Web search and WebFetch were unavailable during research. All findings are based on training data. Protocol specifications (DLNA/UPnP) are frozen and reliable. Samsung/Xbox behavioral specifics should be validated against real hardware during development.

---
*Research completed: 2026-02-22*
*Ready for roadmap: yes*
