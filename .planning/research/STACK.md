# Stack Research

**Domain:** DLNA/UPnP Media Server (Rust, CLI)
**Researched:** 2026-02-22
**Confidence:** MEDIUM (web search unavailable; versions based on training data through early 2025 -- verify all versions against crates.io before pinning)

## Critical Context

DLNA/UPnP is a niche in the Rust ecosystem. There is no mature, batteries-included "Rust DLNA framework" the way MiniDLNA (C) or Jellyfin (.NET) exist. The practical approach is to assemble purpose-built crates for each protocol layer (SSDP, HTTP, XML/SOAP) and implement the UPnP device description + ContentDirectory service logic by hand. This is actually fine for this project's scope -- the UPnP spec surface we need is small (device description XML, Browse action, GetSystemUpdateID) and hand-rolling it avoids fighting an abstraction that does not quite fit.

## Recommended Stack

### Async Runtime

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| tokio | ^1.36 | Async runtime | The Rust async ecosystem gravitational center. hyper, axum, and every serious networking crate builds on tokio. Using async-std would cut us off from the best HTTP and networking crates. tokio's multi-threaded runtime also handles concurrent SSDP multicast + HTTP serving naturally. |

**Confidence:** HIGH -- tokio dominance is well established and stable.

### HTTP Server (for media streaming + SOAP endpoints)

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| hyper | ^1.2 | Low-level HTTP server | We need precise control over HTTP headers (Content-Range, Content-Type, transferMode.dlna.org, etc.) and byte-range responses. hyper 1.x gives us that control without opinions. Samsung TVs and Xbox are strict about HTTP response formatting; a higher-level framework would make it harder to get headers exactly right. |
| hyper-util | ^0.1 | Utilities for hyper 1.x | hyper 1.x split utilities out; hyper-util provides the TokioIo adapter, server auto-connection handling, etc. Required companion to hyper 1.x. |
| http-body-util | ^0.1 | HTTP body types | Provides Full, StreamBody and other body implementations needed with hyper 1.x. |
| tower | ^0.4 | Service middleware | Optional. Useful if we want layered routing (SOAP endpoint vs. media file endpoint). Can start without it and add later. |

**Why not axum?** axum is excellent for REST APIs but adds abstraction over hyper that makes it harder to control exact HTTP response semantics. For a DLNA server where byte-range headers, DLNA-specific headers (transferMode.dlna.org, contentFeatures.dlna.org), and precise Content-Type values matter enormously, hyper gives us the direct control we need without fighting the framework. If the project grows to include a web UI later, axum could be layered on top.

**Why not warp/actix-web?** warp is in maintenance mode. actix-web uses its own runtime (actix-rt, built on tokio but with its own patterns) and is heavier than needed for this use case. Neither provides advantages over hyper for our precise-header-control requirements.

**Confidence:** HIGH -- hyper 1.x is stable and well-documented. The choice of hyper over a framework is an informed opinion based on DLNA's header requirements.

### SSDP / UPnP Discovery

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| socket2 | ^0.5 | Low-level socket control | For constructing the UDP multicast socket bound to 239.255.255.250:1900. We need SO_REUSEADDR, IP_ADD_MEMBERSHIP, and multicast TTL control. socket2 wraps these platform-specific socket options cleanly. |
| **Hand-rolled SSDP** | N/A | SSDP M-SEARCH response + NOTIFY | SSDP is a simple text-over-UDP protocol. The existing Rust SSDP crates (ssdp-client, rupnp) are either client-only (designed to discover devices, not advertise as one) or unmaintained. Implementing the server side of SSDP is ~200 lines: parse M-SEARCH, send responses, periodically send NOTIFY alive/byebye. Not worth a dependency. |

**Why not rupnp?** rupnp is a UPnP *client* library -- it discovers and controls other devices. We are building a UPnP *device/server*. rupnp's architecture is inverted from what we need.

**Why not ssdp-client?** Same issue: it sends M-SEARCH queries and parses responses. We need to *receive* M-SEARCH queries and *send* responses. The crate does not support the server role.

**Why not the `upnp` crate?** The `upnp` crate on crates.io (if it exists) has historically been either abandoned or extremely minimal. No mature Rust crate provides a UPnP device framework. This is the main gap in the ecosystem and the reason we hand-roll.

**Confidence:** MEDIUM -- I am confident that no good server-side UPnP crate exists in Rust based on the ecosystem as of early 2025, but this should be re-verified. The recommendation to hand-roll is robust either way, since the SSDP server protocol is genuinely simple.

### XML / SOAP

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| quick-xml | ^0.36 | XML serialization/deserialization | Fastest Rust XML parser. Needed for: (1) generating device description XML, (2) generating ContentDirectory Browse SOAP responses (DIDL-Lite), (3) parsing incoming SOAP Browse requests. quick-xml's event-based API is ideal for generating DIDL-Lite XML without needing a full DOM. |

**Why not xmltree or roxmltree?** roxmltree is read-only (cannot generate XML). xmltree is unmaintained and slower. quick-xml handles both reading and writing efficiently.

**Why not serde-xml-rs?** serde-xml-rs builds on xml-rs (slow) and has trouble with XML namespaces, which are essential for SOAP envelopes and DIDL-Lite. UPnP SOAP responses require precise namespace control (urn:schemas-upnp-org:service:ContentDirectory:1, urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/, etc.).

**Why not hard-coded XML strings?** Tempting for the MVP, and actually fine for device description XML (which is static). But ContentDirectory Browse responses are dynamic (different items per request, pagination via StartingIndex/RequestedCount). Using quick-xml for the dynamic parts prevents XML injection bugs and makes the code maintainable.

**Pragmatic approach:** Use static XML string templates for device description and service description (they never change at runtime). Use quick-xml for DIDL-Lite Browse response generation (dynamic per-request content).

**Confidence:** HIGH -- quick-xml is the clear standard for XML in Rust.

### CLI Argument Parsing

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| clap | ^4.5 | CLI argument parsing | De facto standard. Derive macro support makes it ergonomic. Handles `udlna /path/to/media` positional args, `--port`, `--name`, `--config` flags naturally. |

**Confidence:** HIGH -- clap is the undisputed Rust CLI parsing standard.

### Configuration

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| toml | ^0.8 | TOML config file parsing | Project specifies TOML config. The `toml` crate is the standard. Used with serde for deserialization. |
| serde | ^1.0 | Serialization framework | Required by toml, clap (derive), and useful throughout. |
| serde_derive | (via serde features) | Derive macros | `serde = { version = "1.0", features = ["derive"] }` |
| dirs | ^5.0 | Platform-specific directories | For finding `~/.config/udlna/config.toml` cross-platform. |

**Confidence:** HIGH -- all standard choices.

### MIME Type Detection

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| mime_guess | ^2.0 | File extension to MIME type | Maps .mp4 -> video/mp4, .mkv -> video/x-matroska, etc. Essential for correct Content-Type headers. Extension-based detection is sufficient for a no-transcode server. |

**Why not tree_magic or infer?** Those do content-based (magic byte) detection, which requires reading file headers. Extension-based is faster, simpler, and correct for our use case since we serve files as-is.

**Note:** Samsung TVs are picky about MIME types. We may need a small override map for edge cases (e.g., .mkv might need video/x-matroska or video/x-mkv depending on the client). This is a ~10-line lookup table, not a crate.

**Confidence:** HIGH for mime_guess; MEDIUM for whether we need custom overrides (requires testing with actual devices).

### Logging

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| tracing | ^0.1 | Structured logging/diagnostics | Better than env_logger for async code. Integrates with tokio. Structured spans make it easy to trace per-request behavior, which is essential when debugging why a Samsung TV rejects a response. |
| tracing-subscriber | ^0.3 | Log output formatting | Provides the fmt subscriber for human-readable terminal output. |

**Why not log + env_logger?** The `log` facade works fine but `tracing` is the modern standard for async Rust, provides spans (not just events), and has better tokio integration. Since we are tokio-native, tracing is the natural fit.

**Confidence:** HIGH.

### Graceful Shutdown

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| tokio (signal feature) | ^1.36 | Ctrl+C signal handling | `tokio::signal::ctrl_c()` provides an async-friendly way to catch SIGINT/SIGTERM. On Ctrl+C, we send SSDP byebye notifications, then shut down the HTTP server. Built into tokio, no extra crate needed. |

**Confidence:** HIGH.

### File System Scanning

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| walkdir | ^2.5 | Recursive directory traversal | Standard crate for walking directory trees. Fast, handles symlinks, provides depth control. Used at startup to build the flat file list. |

**Why not std::fs::read_dir recursively?** walkdir handles edge cases (symlink loops, permission errors) gracefully and provides an iterator interface. Worth the dependency for robustness.

**Why not ignore or globwalk?** Those are designed for gitignore-style filtering. Overkill for our simple "list all media files" use case.

**Confidence:** HIGH.

### Network Interface Discovery

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| local-ip-address | ^0.6 | Get local network IP | Needed to construct the UPnP device Location URL (http://{local_ip}:{port}/description.xml). The server must advertise its reachable IP in SSDP responses. |

**Alternative:** `if-addrs` crate (also works). `local-ip-address` is simpler for the common case of "give me the primary non-loopback IPv4 address."

**Why not hard-code or use 0.0.0.0?** UPnP requires the LOCATION header in SSDP responses to contain an actual reachable IP address. 0.0.0.0 will not work. We need to detect the actual LAN IP.

**Confidence:** MEDIUM -- verify the crate is still maintained. Fallback: use `if-addrs` or query network interfaces directly via `nix` or `libc`.

### UUID Generation

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| uuid | ^1.7 | UPnP device UUID | Every UPnP device needs a stable uuid:device-UUID. The `uuid` crate with `v4` feature generates random UUIDs, or `v5` for deterministic UUIDs based on server name. |

**Confidence:** HIGH.

## Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| bytes | ^1.5 | Efficient byte buffer management | Used by hyper/tokio internally; useful for constructing HTTP response bodies from file reads without extra copies. |
| tokio-util | ^0.7 | Tokio utilities | `ReaderStream` converts `tokio::fs::File` into a `Stream<Item=Result<Bytes>>` for streaming file responses. Essential for byte-range serving without loading entire files into memory. |
| percent-encoding | ^2.3 | URL encoding | For encoding file paths in HTTP URLs served in DIDL-Lite metadata. File names with spaces/special chars must be properly encoded. |
| httpdate | ^1.0 | HTTP date formatting | For Date and Last-Modified headers in HTTP responses. Some DLNA clients check these. |

## Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| cargo-watch | Live reloading during dev | `cargo watch -x run` for rapid iteration |
| wireshark | SSDP/SOAP packet inspection | Filter: `ssdp` or `http.request.method == "POST"` to debug UPnP traffic |
| gssdp-discover (gupnp-tools) | UPnP test client | Linux/macOS tool to verify SSDP advertisement works. `brew install gupnp-tools` on macOS. |
| DLNA test client | Device emulation | VLC (supports UPnP browsing) or BubbleUPnP (Android) for testing without a real TV |

## Cargo.toml Dependencies (Recommended)

```toml
[package]
name = "udlna"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP server
hyper = { version = "1", features = ["http1", "server"] }
hyper-util = { version = "0.1", features = ["tokio", "server-auto", "http1"] }
http-body-util = "0.1"

# Byte buffers and streaming
bytes = "1"
tokio-util = { version = "0.7", features = ["io"] }

# XML generation/parsing (for SOAP and DIDL-Lite)
quick-xml = { version = "0.36", features = ["serialize"] }

# CLI
clap = { version = "4", features = ["derive"] }

# Configuration
serde = { version = "1", features = ["derive"] }
toml = "0.8"
dirs = "5"

# MIME types
mime_guess = "2"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# File system
walkdir = "2"

# Networking
socket2 = { version = "0.5", features = ["all"] }
local-ip-address = "0.6"

# Utilities
uuid = { version = "1", features = ["v4", "v5"] }
percent-encoding = "2"
httpdate = "1"
```

**Note on versions:** These versions are based on the ecosystem as of early 2025. Before starting the project, run `cargo add` for each dependency to get the latest compatible versions, or check crates.io. The major version bounds (1.x, 0.36.x, etc.) should be correct, but patch versions may have advanced.

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| hyper (direct) | axum | If the project later adds a web management UI; axum layers cleanly on hyper |
| hyper (direct) | actix-web | Never for this project; different runtime model, more opinionated than needed |
| quick-xml | hard-coded XML strings | Acceptable for device/service description XML (static); not for dynamic Browse responses |
| hand-rolled SSDP | rupnp | Only if building a UPnP *client* (control point), not a server |
| socket2 | tokio::net::UdpSocket alone | tokio UdpSocket works but socket2 gives finer control over multicast options needed for SSDP |
| tracing | log + env_logger | If you strongly prefer simplicity and do not need async span tracing |
| walkdir | std::fs::read_dir | If you want zero deps for dir traversal and can handle edge cases manually |
| local-ip-address | if-addrs or nix | if-addrs gives all interfaces (more control); nix for full POSIX socket API |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| rupnp | Client-only (discovers devices, does not advertise as one). Wrong direction for a DLNA server. | Hand-roll SSDP server (~200 lines) |
| ssdp-client | Client-only (sends M-SEARCH, does not respond to them) | Hand-roll with socket2 + tokio::net::UdpSocket |
| xml-rs | Slow, older XML parser; serde-xml-rs builds on it and has namespace issues | quick-xml |
| serde-xml-rs | Poor namespace handling; SOAP/DIDL-Lite requires precise XML namespace control | quick-xml (with or without serde integration) |
| async-std | Splits the async ecosystem; hyper/tower/axum all require tokio. Using async-std means giving up the best HTTP stack. | tokio |
| warp | Maintenance mode; built on hyper 0.14 (old). hyper 1.x is the current generation. | hyper 1.x directly |
| minidom / xmpp-parsers | XMPP-focused XML crates; wrong domain, unnecessary XMPP assumptions | quick-xml |
| reqwest | HTTP *client* crate; we need an HTTP *server*. reqwest is for making requests, not serving them. | hyper (server) |

## Stack Patterns by Variant

**If targeting only macOS/Linux (this project):**
- Use `tokio::signal::ctrl_c()` for graceful shutdown
- Use `local-ip-address` or `if-addrs` for network interface detection
- SSDP multicast works out of the box on these platforms

**If we ever needed Windows support:**
- Same stack works; tokio and hyper are cross-platform
- Windows firewall may block SSDP multicast; document this for users
- `local-ip-address` works on Windows

**If the file list grows very large (>100K files):**
- Consider lazy scanning with `notify` crate for filesystem watching instead of full re-scan
- Consider a SQLite index (via `rusqlite`) for ContentDirectory pagination
- For MVP, in-memory Vec<FileEntry> is fine

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| hyper 1.x | tokio 1.x, http 1.x | hyper 1.x requires hyper-util for server setup; different API than hyper 0.14 |
| hyper-util 0.1 | hyper 1.x only | Not compatible with hyper 0.14 |
| quick-xml 0.36 | serde 1.x (optional) | Breaking changes between 0.31 and 0.36; use 0.36+ |
| clap 4.x | serde 1.x (optional derive) | clap 4 is the current major; clap 3 is old |
| tracing 0.1 | tracing-subscriber 0.3 | These version numbers look odd but are the current stable pair |

## Key Technical Notes for Implementation

### SSDP Protocol (hand-rolled)
The SSDP server needs to:
1. Join multicast group 239.255.255.250 on port 1900
2. Listen for M-SEARCH requests with ST: ssdp:all, upnp:rootdevice, or specific service types
3. Respond with unicast UDP containing LOCATION, USN, ST headers
4. Periodically send NOTIFY ssdp:alive (multicast) to keep devices aware
5. On shutdown, send NOTIFY ssdp:byebye (multicast)

This is genuinely ~200-300 lines of Rust including proper header formatting.

### HTTP Byte-Range Serving
The HTTP server needs to support:
1. Full GET (no Range header) -- serve entire file
2. Single range: `Range: bytes=0-999` -> 206 Partial Content
3. Open range: `Range: bytes=1000-` -> from offset to end
4. HEAD requests (many DLNA clients probe with HEAD first)
5. Correct Content-Range, Content-Length, Accept-Ranges headers

hyper gives us direct access to request headers and full control over response construction. This is the main reason to use hyper directly rather than a framework.

### SOAP Request/Response
ContentDirectory Browse requests arrive as HTTP POST with a SOAP XML body. We need to:
1. Parse the SOAP envelope to extract BrowseFlag, ObjectID, StartingIndex, RequestedCount
2. Build a DIDL-Lite XML response with `<item>` elements for each file
3. Wrap it in a SOAP response envelope
4. Return with Content-Type: text/xml; charset="utf-8"

quick-xml handles both the parsing and generation sides.

## Sources

- Training data (Rust ecosystem knowledge through early 2025) -- MEDIUM confidence on versions
- UPnP Device Architecture 2.0 specification (upnp.org) -- HIGH confidence on protocol requirements
- DLNA Guidelines (dlna.org) -- HIGH confidence on DLNA profile requirements
- hyper 1.0 release announcement and migration guide -- HIGH confidence on hyper 1.x API
- NOTE: Could not verify current crate versions against crates.io due to web access restrictions. All versions should be verified with `cargo add` or `cargo search` before use.

---
*Stack research for: udlna -- Minimal DLNA/UPnP Media Server in Rust*
*Researched: 2026-02-22*
*Version verification needed: Run `cargo add <crate>` for each dependency to confirm latest versions*
