# Feature Research: DLNA/UPnP Media Server

**Domain:** Minimal DLNA/UPnP media server (CLI, no-config, Rust)
**Researched:** 2026-02-22
**Confidence:** MEDIUM (based on UPnP AV spec knowledge and community reports; web verification unavailable during research)

## Feature Landscape

### Table Stakes (Samsung TV + Xbox Series X Will Not Work Without These)

These are non-negotiable. Omitting any of these means one or both target clients will fail to discover, browse, or play media.

#### 1. SSDP Discovery (UPnP Device Discovery)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| SSDP M-SEARCH response | Clients discover servers by sending M-SEARCH to 239.255.255.250:1900; server MUST respond with HTTP 200 containing device location URL | MEDIUM | Must respond to `ssdp:all`, `upnp:rootdevice`, and device/service-specific search targets. Samsung TVs send multiple search target types. |
| SSDP NOTIFY (alive) | Server MUST announce itself on startup via multicast NOTIFY with `NTS: ssdp:alive` | LOW | Send for root device, device UUID, and each service type. Typically 3+ NOTIFY messages. |
| SSDP NOTIFY (byebye) | Server MUST send `NTS: ssdp:byebye` on shutdown for clean departure | LOW | Ctrl+C handler must send these before exit. Without this, Samsung TV shows stale server entries for minutes. |
| SSDP cache-control / max-age | NOTIFY must include `CACHE-CONTROL: max-age=1800` (or similar). Server must re-advertise before expiry. | LOW | Xbox uses this to decide when to drop the server from its list. Re-advertise at half the max-age interval. |
| USN (Unique Service Name) | Every SSDP message must carry a proper `USN` header with `uuid:{device-uuid}::` prefix | LOW | Samsung TV will ignore responses with malformed USN. Must be consistent across all messages. |

#### 2. UPnP Device Description (XML)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Root device description XML | HTTP GET on the location URL from SSDP must return valid UPnP device description XML | MEDIUM | Must declare `urn:schemas-upnp-org:device:MediaServer:1` device type. Samsung and Xbox both fetch this immediately after discovery. |
| Service list with ContentDirectory | Device XML must list `urn:schemas-upnp-org:service:ContentDirectory:1` with SCPDURL, controlURL, and eventSubURL | LOW | Xbox specifically validates that ContentDirectory:1 is listed. |
| Service list with ConnectionManager | Device XML must list `urn:schemas-upnp-org:service:ConnectionManager:1` | LOW | Xbox requires ConnectionManager to be present even if minimally implemented. Samsung also checks for it. |
| Friendly name | `<friendlyName>` element in device XML | LOW | This is what shows up in the Samsung TV media browser and Xbox media player UI. |
| UDN (Unique Device Name) | `<UDN>uuid:{consistent-uuid}</UDN>` | LOW | Must match the UUID in SSDP USN headers. Samsung TV correlates these. |
| Manufacturer / model fields | `<manufacturer>`, `<modelName>`, `<modelNumber>` | LOW | Xbox shows these in device info. Can be anything but must be present. |
| DLNA device capability (X_DLNADOC) | `<dlna:X_DLNADOC xmlns:dlna="urn:schemas-dlna-org:device-1-0">DMS-1.50</dlna:X_DLNADOC>` | LOW | Samsung TVs look for this to confirm DLNA compliance. Without it, some Samsung models will not show the server. HIGH confidence this is required for Samsung. |

#### 3. UPnP ContentDirectory Service (SOAP Actions)

This is the heart of the server. Clients browse your media library through SOAP calls to the ContentDirectory control URL.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **Browse** action | THE core action. Clients call Browse with `BrowseFlag` of `BrowseDirectChildren` or `BrowseMetadata`. Returns DIDL-Lite XML with items/containers. | HIGH | This is the most complex single feature. Must handle ObjectID "0" as root, return proper DIDL-Lite XML, support RequestedCount and StartingIndex for paging. Samsung and Xbox both use this extensively. |
| **GetSearchCapabilities** action | Returns supported search capabilities. Can return empty string. | LOW | Mandatory in ContentDirectory:1 spec. Xbox calls this. Return empty `<SearchCaps></SearchCaps>` if search is not supported. |
| **GetSortCapabilities** action | Returns supported sort fields. Can return empty string. | LOW | Mandatory in ContentDirectory:1 spec. Xbox calls this. Return empty `<SortCaps></SortCaps>` if sorting is not supported. |
| **GetSystemUpdateID** action | Returns a system-wide update counter. Clients poll this to know if content changed. | LOW | Mandatory in ContentDirectory:1 spec. Return a counter that increments when content changes. Can start at 1 and stay at 1 for a static file server. |
| DIDL-Lite XML response format | Browse results must be wrapped in proper DIDL-Lite XML with correct namespaces | HIGH | This is where most DIY servers fail. Samsung is extremely picky about namespace declarations, `dc:title`, `upnp:class`, `res` elements with `protocolInfo`. |
| `protocolInfo` in `<res>` elements | Each media resource must have `protocolInfo="http-get:*:video/mp4:DLNA.ORG_PN=AVC_MP4_MP_SD_AAC;DLNA.ORG_OP=01;DLNA.ORG_FLAGS=..."` | HIGH | Samsung TVs will not play files without proper protocolInfo. The DLNA.ORG_OP=01 flag indicates byte-range support. DLNA.ORG_FLAGS indicate streaming capabilities. This is the single most finicky area. |
| `upnp:class` element | Each item must have correct UPnP class: `object.item.videoItem`, `object.item.audioItem.musicTrack`, `object.item.imageItem.photo` | LOW | Samsung uses this to decide which player to launch. Wrong class = file won't open. |
| `dc:title` element | Dublin Core title for each item | LOW | Display name in the browser. Use filename without extension. |
| Browse paging (StartingIndex + RequestedCount) | Browse must support subset returns via StartingIndex and RequestedCount params, with TotalMatches in response | MEDIUM | Xbox and Samsung both page through large directories. Without paging, only the first batch appears. |
| NumberReturned and TotalMatches | Browse response must include `<NumberReturned>` and `<TotalMatches>` SOAP elements | LOW | Xbox specifically validates these counts match the actual DIDL-Lite content. |
| ObjectID "0" as root container | Browse with ObjectID "0" must return the root container contents | LOW | Universal UPnP convention. Both clients start browsing from "0". |

#### 4. ConnectionManager Service (Minimal)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **GetProtocolInfo** action | Returns supported protocols. Response lists source protocols the server can serve. | LOW | Mandatory. Return `http-get:*:video/mp4:*,http-get:*:video/x-matroska:*,http-get:*:audio/mpeg:*,...` for each MIME type you serve. Xbox calls this to determine what the server can provide. |
| **GetCurrentConnectionIDs** action | Returns active connection IDs. Can return "0" or empty. | LOW | Mandatory in ConnectionManager:1. Minimal implementation returns `<ConnectionIDs>0</ConnectionIDs>`. |
| **GetCurrentConnectionInfo** action | Returns info about a connection. Minimal implementation returns defaults. | LOW | Mandatory in ConnectionManager:1. Return default values (status OK, direction Output, etc.) |
| Service description XML (SCPD) | ConnectionManager SCPD XML must be served at the declared SCPDURL | LOW | Static XML file describing the service actions and state variables. |

#### 5. HTTP Media Streaming

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| HTTP GET for media files | Serve actual media file bytes at the URLs declared in DIDL-Lite `<res>` elements | MEDIUM | Standard HTTP file serving. The URL structure is up to you (e.g., `/media/{id}`) |
| HTTP Range requests (RFC 7233) | Must support `Range: bytes=0-`, `Range: bytes=1000-2000`, etc. Respond with 206 Partial Content. | MEDIUM | CRITICAL. Samsung TV and Xbox both seek into media files constantly. Without range support, playback will either fail or videos cannot be fast-forwarded/rewound. |
| Content-Length header | Must include accurate Content-Length on all responses | LOW | Both clients use this to calculate duration estimates and seek positions. |
| Content-Type header | Must return correct MIME type matching what was declared in protocolInfo | LOW | Mismatch between DIDL-Lite protocolInfo and actual HTTP Content-Type will cause Samsung to reject the stream. |
| Accept-Ranges: bytes | Must include `Accept-Ranges: bytes` header | LOW | Signals to client that range requests are supported. Xbox checks for this. |
| `transferMode.dlna.org` header | Response must include `transferMode.dlna.org: Streaming` header for media content | LOW | DLNA-specific HTTP header. Samsung TVs expect this. Without it, some models will refuse to play. Value is `Streaming` for video/audio, `Interactive` for images. |
| `contentFeatures.dlna.org` header | Response should include DLNA content features matching what was in protocolInfo | MEDIUM | Samsung expects this header on HTTP responses. Format: `DLNA.ORG_PN=AVC_MP4_MP_SD_AAC;DLNA.ORG_OP=01;DLNA.ORG_FLAGS=...`. Must match what Browse returned. |
| Connection: keep-alive or close | Properly handle connection lifecycle | LOW | Some Samsung models send rapid sequential requests. Keep-alive avoids connection storm. |
| HEAD request support | Must respond to HTTP HEAD requests with same headers as GET but no body | LOW | Samsung TV sends HEAD before GET to check content metadata. Xbox also does this for some formats. |

#### 6. SOAP/XML Infrastructure

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| SOAP envelope parsing | Parse incoming SOAP requests with proper XML namespace handling | MEDIUM | All UPnP control actions are SOAP. Must handle the `s:Envelope` / `s:Body` / `u:ActionName` structure. |
| SOAP response formatting | Generate valid SOAP responses with correct namespaces | MEDIUM | Samsung is picky about namespace prefixes. Use `xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"` consistently. |
| SOAP fault responses | Return proper SOAP faults for unsupported actions | LOW | Prevents client crashes on unexpected actions. Return error code 401 (Invalid Action) for unsupported actions. |
| Service SCPD XML | Serve ContentDirectory and ConnectionManager SCPD XML at declared URLs | LOW | Static XML files declaring actions and state variables. Both clients fetch these. |

#### 7. MIME Type Detection

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| File extension to MIME mapping | Map common media extensions to correct MIME types | LOW | Essential for protocolInfo and Content-Type. At minimum: .mp4, .mkv, .avi, .mp3, .flac, .jpg, .png. Use a static lookup table. |
| Correct video MIME types | video/mp4, video/x-matroska, video/avi, video/x-msvideo, video/mpeg | LOW | Samsung is strict about MIME types. Use `video/x-matroska` for .mkv (not `video/mkv`). |
| Correct audio MIME types | audio/mpeg, audio/flac, audio/x-flac, audio/mp4, audio/x-wav | LOW | Standard mappings. |
| Correct image MIME types | image/jpeg, image/png, image/gif | LOW | Standard mappings. |

### Summary of UPnP Mandatory vs Optional Actions

#### ContentDirectory:1 -- Mandatory Actions

| Action | Spec Status | Samsung TV | Xbox Series X | Notes |
|--------|-------------|------------|---------------|-------|
| Browse | REQUIRED | Uses extensively | Uses extensively | Core browsing action. Must support both BrowseDirectChildren and BrowseMetadata BrowseFlags. |
| GetSearchCapabilities | REQUIRED | Calls on connect | Calls on connect | Return empty string if no search support. |
| GetSortCapabilities | REQUIRED | Calls on connect | Calls on connect | Return empty string if no sort support. |
| GetSystemUpdateID | REQUIRED | Polls periodically | Polls periodically | Return static counter for non-changing content. |

#### ContentDirectory:1 -- Optional Actions

| Action | Spec Status | Samsung TV | Xbox Series X | Notes |
|--------|-------------|------------|---------------|-------|
| Search | OPTIONAL | Rarely used | Rarely used | Only needed if search is advertised in GetSearchCapabilities. Do NOT implement for MVP. |
| CreateObject | OPTIONAL | Never | Never | For upload/record-to-server. Out of scope entirely. |
| DestroyObject | OPTIONAL | Never | Never | Delete content. Out of scope. |
| UpdateObject | OPTIONAL | Never | Never | Modify metadata. Out of scope. |
| ImportResource | OPTIONAL | Never | Never | Upload resources. Out of scope. |
| ExportResource | OPTIONAL | Never | Never | Export resources. Out of scope. |
| StopTransferResource | OPTIONAL | Never | Never | Paired with import/export. Out of scope. |
| GetTransferProgress | OPTIONAL | Never | Never | Paired with import/export. Out of scope. |

#### ConnectionManager:1 -- Mandatory Actions

| Action | Spec Status | Samsung TV | Xbox Series X | Notes |
|--------|-------------|------------|---------------|-------|
| GetProtocolInfo | REQUIRED | Calls on connect | Calls on connect | Return list of supported MIME types with `http-get:*:mime/type:*` format. |
| GetCurrentConnectionIDs | REQUIRED | Calls occasionally | Calls on connect | Return "0" for minimal implementation. |
| GetCurrentConnectionInfo | REQUIRED | Rarely | Calls on connect | Return default connection info. |

#### ConnectionManager:1 -- Optional Actions

| Action | Spec Status | Notes |
|--------|-------------|-------|
| PrepareForConnection | OPTIONAL | For connection management. Not needed for simple streaming. |
| ConnectionComplete | OPTIONAL | For connection teardown. Not needed for simple streaming. |

### DLNA-Specific HTTP Headers (Required for Samsung/Xbox)

| Header | Direction | Value | Required By | Notes |
|--------|-----------|-------|-------------|-------|
| `transferMode.dlna.org` | Response | `Streaming` (video/audio) or `Interactive` (images/xml) | Samsung, Xbox | DLNA clients send this in request too. Echo it back or set appropriately. |
| `contentFeatures.dlna.org` | Response | DLNA.ORG_PN, DLNA.ORG_OP, DLNA.ORG_FLAGS | Samsung strongly | Full DLNA profile string. Samsung uses this to decide codec compatibility. |
| `Accept-Ranges` | Response | `bytes` | Both | Standard HTTP but explicitly checked by DLNA clients. |
| `Content-Range` | Response (206) | `bytes start-end/total` | Both | Standard HTTP range response header. |
| `transferMode.dlna.org` | Request | `Streaming` | Samsung | Samsung sends this header in its GET request. Server should acknowledge. |

### DLNA protocolInfo Fourth Field (Critical for Samsung)

The `protocolInfo` attribute in DIDL-Lite `<res>` elements has four fields separated by colons:
`http-get:*:mime/type:additional_info`

The fourth field contains DLNA profile information that Samsung TVs parse strictly:

| Component | Format | Purpose |
|-----------|--------|---------|
| `DLNA.ORG_PN` | Profile name, e.g., `AVC_MP4_MP_SD_AAC` | Declares the media profile. Can be omitted with `*` for unknown profiles. |
| `DLNA.ORG_OP` | Two binary digits, e.g., `01` | Operations: first digit = time-seek, second digit = byte-seek. `01` = byte-range seek supported. |
| `DLNA.ORG_FLAGS` | 32 hex digits, e.g., `01700000000000000000000000000000` | Bitfield of capabilities. Key flags: streaming mode (bit 21), background mode (bit 22), connection stalling (bit 20). |
| `DLNA.ORG_CI` | `0` or `1` | Conversion indicator. `0` = not transcoded. Always `0` for us. |

**Recommended fourth field for most video files:**
`DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000`

**Recommended fourth field when DLNA profile is known:**
`DLNA.ORG_PN=AVC_MP4_MP_SD_AAC;DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000`

**Confidence:** MEDIUM -- these values are well-documented in the DLNA spec and confirmed by multiple open-source DLNA server implementations (MiniDLNA/ReadyMedia, Serviio, Gerbera), but could not verify against latest Samsung firmware during this research session.

### DLNA.ORG_FLAGS Breakdown

The flags field is a 256-bit hex string (64 hex chars in spec, commonly truncated to 32 with trailing zeros). Key bits:

| Bit | Name | Value When Set | Purpose |
|-----|------|----------------|---------|
| 20 | DLNA_ORG_FLAG_CONNECTION_STALL | `0x00100000` | Server supports connection stalling |
| 21 | DLNA_ORG_FLAG_STREAMING_TRANSFER | `0x01000000` | Streaming transfer mode supported |
| 22 | DLNA_ORG_FLAG_BACKGROUND_TRANSFER | `0x00200000` | Background transfer mode supported |
| 24 | DLNA_ORG_FLAG_DLNA_V15 | `0x00000000` in practice | DLNA 1.5 compliant |

Common combined value: `01700000000000000000000000000000` = streaming + background + connection stalling.

### Differentiators (Competitive Advantage)

Features that set the product apart. Not required, but valuable for a micro-server.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Zero-config instant startup | `udlna /path/to/media` and it works. No config file, no setup wizard, no database initialization. | LOW | This IS the product's core value. Other DLNA servers (Plex, MiniDLNA, Jellyfin) all require configuration. |
| Single static binary | No runtime dependencies, no database, no ffmpeg. Copy one file and run. | LOW | Rust gives us this naturally. MiniDLNA needs libsqlite, libjpeg, etc. |
| Ephemeral operation | Runs only while you need it. No background service, no state files, no database. | LOW | Unique among DLNA servers which all assume persistent operation. |
| Fast directory scanning | Scan thousands of files in seconds at startup. No database indexing step. | MEDIUM | Rust + walkdir makes this straightforward. MiniDLNA can take minutes to index large libraries. |
| Multiple root paths | `udlna /movies /music /photos` merges multiple directories | LOW | Minor convenience. Most servers support this but require config file editing. |
| Clean shutdown with SSDP byebye | Server disappears from client device lists promptly | LOW | Most servers do this, but it's particularly important for ephemeral usage pattern. |
| Recursive + flat listing | Show all files from all subdirectories as a single flat list | LOW | Simplest possible browse model. User sees all files immediately without navigating folder trees. |
| TOML config file (optional) | Save preferences for repeated use without re-specifying CLI args | LOW | Nice for users who use it regularly. Not needed for first-time use. |
| Custom server name | `--name "Movie Night"` sets the friendly name shown on TVs | LOW | Small touch that makes it feel polished when multiple servers exist on network. |
| Subtitle file association | Detect .srt/.sub files next to video files and include in DIDL-Lite | MEDIUM | Samsung TVs support external subtitles via DLNA if declared in DIDL-Lite with `res` elements of type `text/srt`. Nice but not MVP. |
| Thumbnail serving for images | Serve downscaled JPEG thumbnails for image items | HIGH | Samsung TV shows album art / image previews if a thumbnail `res` is declared. Requires image processing library -- skip for MVP. |

### Anti-Features (Explicitly NOT Building for a Micro-Server)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Transcoding** | "My TV can't play .mkv" | Requires ffmpeg dependency (~100MB), massively increases complexity, CPU usage, and binary size. Fundamentally conflicts with "micro" and "no dependencies." | Document supported formats. Users who need transcoding should use Plex/Jellyfin. |
| **Media metadata database** | "Index once, browse fast" | Requires SQLite or similar, persistent state, re-indexing logic, database migrations. Fundamentally conflicts with ephemeral operation. | Scan filesystem on startup. For a few thousand files this takes <1 second in Rust. |
| **Web UI / management interface** | "I want to browse on my phone" | Scope creep. Requires HTTP server for UI, frontend assets, CORS handling. Completely different product. | This is a CLI tool. Use the TV's built-in DLNA browser. |
| **User authentication / access control** | "Restrict who can access" | DLNA protocol has no authentication mechanism. Adding it would break every client. Local network trust model is the standard. | Use network segmentation / firewall rules if access control is needed. |
| **UPnP eventing (SUBSCRIBE/NOTIFY)** | "Real-time content updates" | UPnP eventing requires HTTP callback server, subscription management, event key tracking. Adds significant complexity. | Clients poll GetSystemUpdateID. For ephemeral server with static content, eventing adds no value. Samsung and Xbox work fine without it for media servers. |
| **Search action** | "Find files by name" | Requires metadata indexing, query parsing (UPnP search syntax), and search capability declaration. Significant complexity for a micro-server. | Users browse the flat list. For large libraries, this isn't the right tool anyway. |
| **Playlist / queue management** | "Create and manage playlists" | Requires state management, persistence, and complex UPnP object hierarchy. Antithetical to ephemeral server. | Client-side playlist management (most TVs and Xbox support this natively). |
| **Album art extraction** | "Show cover art from MP3 ID3 tags" | Requires audio file parsing library, memory for embedded images, HTTP serving for generated thumbnails. | Show generic icon. Users who want rich metadata should use Plex/Jellyfin. |
| **Content change watching (inotify/FSEvents)** | "Auto-detect new files" | Adds OS-specific code, file watcher dependencies. For an ephemeral server you restart, this is unnecessary. | Restart the server to pick up new files. Could add a simple rescan signal (SIGHUP) later. |
| **DLNA MediaRenderer control** | "Push content to my TV" | Completely different UPnP service (AVTransport). Different product category entirely. | This is a MediaServer, not a control point. |
| **Multi-network-interface binding** | "Serve on multiple NICs" | Complicates SSDP (must join multicast on each interface), URL generation (which IP to advertise), and adds configuration burden. | Bind to one interface. Auto-detect or let user specify with `--interface`. |

## Feature Dependencies

```
SSDP Discovery
    |
    +--requires--> Device Description XML
    |                  |
    |                  +--requires--> ContentDirectory SCPD XML
    |                  |
    |                  +--requires--> ConnectionManager SCPD XML
    |
    +--requires--> UDN (UUID generation)

ContentDirectory Browse Action
    |
    +--requires--> DIDL-Lite XML generation
    |                  |
    |                  +--requires--> MIME type detection
    |                  |
    |                  +--requires--> protocolInfo generation (DLNA fourth field)
    |                  |
    |                  +--requires--> File system scanning
    |
    +--requires--> SOAP envelope parsing/generation

HTTP Media Streaming
    |
    +--requires--> Range request handling (206 Partial Content)
    |
    +--requires--> DLNA HTTP headers (transferMode, contentFeatures)
    |
    +--requires--> MIME type detection
    |
    +--requires--> HEAD request support

ConnectionManager Service
    |
    +--requires--> SOAP envelope parsing/generation
    |
    +--requires--> MIME type list (for GetProtocolInfo)
```

### Dependency Notes

- **Browse requires DIDL-Lite:** The Browse response body is DIDL-Lite XML embedded inside SOAP. Cannot implement Browse without DIDL-Lite generation.
- **DIDL-Lite requires protocolInfo:** Every `<res>` element needs a protocolInfo attribute. This is where DLNA profile strings go. Cannot serve browseable content without it.
- **HTTP streaming requires DLNA headers:** Standard HTTP file serving is not enough. Samsung/Xbox expect DLNA-specific response headers.
- **Everything requires SSDP:** Without discovery, no client will ever find the server. SSDP is the entry point.
- **SSDP requires device XML:** The SSDP response points to the device description URL. Without valid device XML, clients drop the server.

## MVP Definition

### Launch With (v1)

Minimum viable product -- what's needed for Samsung TV and Xbox Series X to discover, browse, and play media.

- [ ] **SSDP discovery** (M-SEARCH response + NOTIFY alive/byebye) -- without this, no client finds you
- [ ] **Device description XML** with MediaServer:1 device type, ContentDirectory + ConnectionManager services, and DLNA X_DLNADOC -- without this, clients reject you after discovery
- [ ] **ContentDirectory Browse action** with BrowseDirectChildren and BrowseMetadata, proper DIDL-Lite with protocolInfo -- without this, clients cannot see your files
- [ ] **ContentDirectory GetSearchCapabilities** (return empty) -- mandatory action, Xbox calls it
- [ ] **ContentDirectory GetSortCapabilities** (return empty) -- mandatory action, Xbox calls it
- [ ] **ContentDirectory GetSystemUpdateID** (return static counter) -- mandatory action, clients poll it
- [ ] **ConnectionManager GetProtocolInfo** (return supported MIME list) -- mandatory action, Xbox calls it
- [ ] **ConnectionManager GetCurrentConnectionIDs** (return "0") -- mandatory action
- [ ] **ConnectionManager GetCurrentConnectionInfo** (return defaults) -- mandatory action
- [ ] **Service SCPD XML** for ContentDirectory and ConnectionManager -- clients fetch these
- [ ] **HTTP file serving with Range support** (206 Partial Content) -- without this, media will not play
- [ ] **DLNA HTTP headers** (transferMode.dlna.org, contentFeatures.dlna.org) -- without these, Samsung rejects streams
- [ ] **HEAD request support** -- Samsung sends HEAD before GET
- [ ] **MIME type detection** from file extension -- needed for protocolInfo and Content-Type
- [ ] **DLNA protocolInfo fourth field** with DLNA.ORG_OP=01, DLNA.ORG_FLAGS -- Samsung requires this
- [ ] **File system recursive scan** of given paths -- core input
- [ ] **Flat file listing** (all files as direct children of root "0") -- simplest browse model
- [ ] **CLI argument parsing** for media paths -- core UX
- [ ] **Graceful shutdown** with SSDP byebye on Ctrl+C -- clean client experience

### Add After Validation (v1.x)

Features to add once core is working and tested with real Samsung/Xbox devices.

- [ ] **TOML config file** -- when users want to save preferences
- [ ] **Custom friendly name** (`--name` flag) -- when users have multiple servers
- [ ] **Multiple root paths as containers** -- show each path as a separate folder rather than merged flat list
- [ ] **Hierarchical directory browsing** -- expose directory structure as UPnP containers (instead of flat)
- [ ] **SIGHUP rescan** -- reload file list without restarting
- [ ] **Subtitle file association** (.srt/.sub alongside videos) -- Samsung supports external subtitles via DLNA
- [ ] **Size/duration metadata in DIDL-Lite** -- file size in `<res size="...">`, duration if detectable from filename
- [ ] **Network interface selection** (`--interface` flag) -- for multi-NIC systems

### Future Consideration (v2+)

Features to defer until product-market fit is established.

- [ ] **Thumbnail serving** -- requires image processing, significant complexity
- [ ] **Basic search** -- requires metadata indexing, UPnP search query parsing
- [ ] **Content sorting** -- requires declaring sort capabilities, implementing sort logic in Browse
- [ ] **DLNA profile detection** -- map file extensions + basic header sniffing to proper DLNA.ORG_PN values
- [ ] **UPnP eventing** -- SUBSCRIBE/NOTIFY for content change notifications
- [ ] **IPv6 support** -- some newer networks are IPv6-only

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| SSDP discovery (alive + M-SEARCH + byebye) | HIGH | MEDIUM | P1 |
| Device description XML | HIGH | LOW | P1 |
| ContentDirectory Browse (DIDL-Lite) | HIGH | HIGH | P1 |
| ContentDirectory mandatory actions (3 simple ones) | HIGH | LOW | P1 |
| ConnectionManager mandatory actions (3 simple ones) | HIGH | LOW | P1 |
| SCPD XML for both services | HIGH | LOW | P1 |
| HTTP streaming with Range support | HIGH | MEDIUM | P1 |
| DLNA HTTP headers | HIGH | LOW | P1 |
| HEAD request support | HIGH | LOW | P1 |
| MIME type detection | HIGH | LOW | P1 |
| protocolInfo with DLNA fourth field | HIGH | MEDIUM | P1 |
| File system scanning | HIGH | LOW | P1 |
| Graceful shutdown (byebye) | MEDIUM | LOW | P1 |
| CLI argument parsing | HIGH | LOW | P1 |
| TOML config file | MEDIUM | LOW | P2 |
| Custom friendly name | LOW | LOW | P2 |
| Hierarchical directory browsing | MEDIUM | MEDIUM | P2 |
| Subtitle association | MEDIUM | MEDIUM | P2 |
| SIGHUP rescan | LOW | LOW | P2 |
| Thumbnail serving | LOW | HIGH | P3 |
| Search action | LOW | HIGH | P3 |
| UPnP eventing | LOW | HIGH | P3 |

## Competitor Feature Analysis

| Feature | MiniDLNA (ReadyMedia) | Jellyfin DLNA | Plex DLNA | udlna (Our Approach) |
|---------|----------------------|---------------|-----------|---------------------|
| Setup complexity | Config file required, daemon | Full server install, web setup | Account required, full server | `udlna /path` -- zero config |
| Binary size | ~2MB + deps | ~200MB+ | ~300MB+ | Target <5MB single binary |
| Database | SQLite required | SQLite/PostgreSQL | Proprietary | None -- filesystem scan |
| Transcoding | No | Yes (ffmpeg) | Yes (built-in) | No -- deliberate omission |
| Metadata | Full ID3/EXIF parsing | Full with internet lookup | Full with internet lookup | Filename only |
| Background service | Yes (daemon) | Yes (service) | Yes (service) | No -- on-demand only |
| Samsung TV compat | Excellent | Good | Good | Target: Excellent |
| Xbox compat | Excellent | Good | Good | Target: Excellent |
| Memory usage | ~10-30MB | ~500MB+ | ~200MB+ | Target: <20MB |
| Startup time | Minutes (indexing) | Minutes | Minutes | Seconds |
| Content change | inotify + rescan | Watch + rescan | Watch + rescan | Restart or SIGHUP |

## Samsung TV Specific Requirements (MEDIUM Confidence)

Based on community reports from Samsung TV DLNA compatibility discussions:

1. **X_DLNADOC element in device XML is required** -- Samsung firmware checks for DLNA compliance marker
2. **protocolInfo fourth field must not be empty** -- Samsung parses DLNA.ORG_OP and DLNA.ORG_FLAGS; empty fourth field = file hidden
3. **DIDL-Lite namespace declarations must be exact** -- Samsung's XML parser is strict about `xmlns:dc`, `xmlns:upnp`, `xmlns:dlna` prefixes
4. **transferMode.dlna.org header required on HTTP responses** -- Samsung refuses streams without this
5. **Content-Type must match protocolInfo MIME** -- any mismatch and Samsung drops the connection
6. **HEAD requests before GET** -- Samsung always sends HEAD first to check content metadata
7. **Samsung may send `getCaptionInfo.sec` SOAP header** -- Samsung-specific extension for subtitle discovery. Can be ignored but should not cause a server error.
8. **Samsung expects `<res>` size attribute** -- file size in bytes in the DIDL-Lite `<res size="123456">` element. Not mandatory but strongly recommended.

## Xbox Series X Specific Requirements (MEDIUM Confidence)

Based on community reports from Xbox DLNA/media streaming discussions:

1. **ConnectionManager service must be present** -- Xbox validates service list more strictly than Samsung
2. **GetCurrentConnectionInfo must return valid XML** -- Xbox calls this immediately after discovery
3. **GetProtocolInfo must list supported types** -- Xbox uses this to filter which files it shows to the user
4. **Browse paging must work correctly** -- Xbox uses StartingIndex/RequestedCount pagination
5. **NumberReturned and TotalMatches must be accurate** -- Xbox validates these counts
6. **Xbox uses Microsoft's DLNA stack** -- tends to follow spec strictly but is less picky about DLNA profile strings
7. **Xbox supports fewer codecs than Samsung** -- focuses on MP4 (H.264), MP3, WMA, JPEG. MKV support is limited.
8. **Xbox may request `X_GetFeatureList`** -- Microsoft-specific extension. Should return SOAP fault (401 Invalid Action) rather than crashing.

## Sources

- UPnP ContentDirectory:1 Service Template (UPnP Forum specification) -- HIGH confidence for action list
- UPnP ConnectionManager:1 Service Template (UPnP Forum specification) -- HIGH confidence for action list
- DLNA Guidelines (DLNA specification documents) -- MEDIUM confidence (based on training data, spec PDFs not fetchable during session)
- MiniDLNA/ReadyMedia source code (widely referenced open-source DLNA server) -- MEDIUM confidence for Samsung/Xbox compatibility patterns
- Gerbera DLNA server documentation and issue tracker -- MEDIUM confidence for client compatibility reports
- Samsung TV DLNA community reports (various forums, GitHub issues) -- LOW-MEDIUM confidence
- Xbox DLNA community reports (various forums, Reddit) -- LOW-MEDIUM confidence

**Note:** Web search and fetch tools were unavailable during this research session. All findings are based on training data knowledge of UPnP/DLNA specifications and community documentation. Confidence levels reflect this limitation. Critical findings (especially Samsung-specific requirements like X_DLNADOC and protocolInfo fourth field formatting) should be validated against real devices during development.

---
*Feature research for: DLNA/UPnP micro media server*
*Researched: 2026-02-22*
