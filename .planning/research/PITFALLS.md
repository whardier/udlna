# Pitfalls Research

**Domain:** DLNA/UPnP Media Server
**Researched:** 2026-02-22
**Confidence:** MEDIUM (based on well-established protocol specs and extensive community documentation in training data; web verification unavailable but DLNA specs are frozen since 2017)

## Critical Pitfalls

### Pitfall 1: Malformed ContentDirectory DIDL-Lite XML Breaks Samsung TVs

**What goes wrong:**
Samsung Smart TVs are the strictest DLNA clients in common use. They parse ContentDirectory Browse response XML with very little tolerance for deviation. The most common failure mode: the server returns DIDL-Lite XML that a lenient parser would accept but Samsung's parser rejects silently. The TV either shows an empty folder, fails to display the server at all, or shows items but refuses to play them.

Specific XML issues that break Samsung TVs:
1. Missing or incorrect XML namespace declarations. The `DIDL-Lite` root element MUST declare all four namespaces: `urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/`, `urn:schemas-upnp-org:metadata-1-0/upnp/`, `http://purl.org/dc/elements/1.1/`, and `urn:schemas-dlna-org:metadata-1-0/`.
2. Wrong element ordering within `<item>`. Samsung expects `<dc:title>` before `<res>`. Other clients don't care.
3. Missing `protocolInfo` attribute on `<res>` elements. Samsung requires the full four-field DLNA protocol info string: `http-get:*:video/mp4:DLNA.ORG_PN=AVC_MP4_MP_SD_AAC_MULT5;DLNA.ORG_OP=01;DLNA.ORG_FLAGS=...`.
4. Self-closing tags where Samsung expects explicit close tags (e.g., `<res .../>` vs `<res ...></res>`).

**Why it happens:**
Developers test with VLC or a web browser (both lenient XML parsers) and assume correctness. The UPnP ContentDirectory spec is ambiguous about element ordering. Samsung follows DLNA guidelines strictly while most other clients ignore them.

**How to avoid:**
- Construct DIDL-Lite XML from a template, not by string concatenation. Use a proper XML builder (Rust: `quick-xml` or `xmlwriter`).
- Always declare all four namespaces on the `DIDL-Lite` root element.
- Always include `protocolInfo` on every `<res>` element with all four fields populated.
- Maintain a reference DIDL-Lite response captured from MiniDLNA (the gold-standard DLNA server) and diff against it.
- Test with an actual Samsung TV early. Samsung's DLNA browser provides zero error messages -- it just shows nothing.

**Warning signs:**
- Server works with VLC/IINA but not Samsung TV
- Samsung sees the server but shows "No items" in folder
- Samsung sees items but they show as unsupported format despite being standard MP4/MKV

**Phase to address:**
ContentDirectory service implementation (early-mid phase). This must be correct from the start because every subsequent feature depends on browse responses working.

---

### Pitfall 2: SSDP Discovery Timing and Multicast Group Membership Failures

**What goes wrong:**
The server joins the SSDP multicast group (239.255.255.250:1900) but devices never discover it, or discovery works intermittently. Three distinct failure modes:

1. **Not binding to the correct network interface.** On machines with multiple NICs (common on macOS: en0, en1, lo0, utun*, bridge*), the SSDP socket binds to the wrong interface or only to `0.0.0.0` without setting `IP_ADD_MEMBERSHIP` on the correct interface. Result: multicast packets go out on the wrong interface or never arrive.

2. **Missing SSDP:alive re-advertisements.** UPnP requires periodic `ssdp:alive` NOTIFY messages (every `CACHE-CONTROL: max-age / 2` seconds, typically every 900 seconds). Many implementations send the initial advertisement but never re-advertise. Devices that power on after the server starts never discover it.

3. **Incorrect or incomplete M-SEARCH response.** When a device sends `M-SEARCH * HTTP/1.1` with `ST: urn:schemas-upnp-org:service:ContentDirectory:1`, the server must respond with a unicast UDP reply to the source address within `MX` seconds. Common mistakes: responding with wrong `ST` value, missing `USN` header, missing `LOCATION` header, or responding too slowly.

**Why it happens:**
SSDP uses UDP multicast which is inherently unreliable and hard to debug. Unlike TCP, there's no connection-level error. Packets silently disappear. macOS has additional complexity with its network interface management. Most developers test on localhost or a simple single-NIC setup.

**How to avoid:**
- Enumerate network interfaces at startup and explicitly join the multicast group on each non-loopback IPv4 interface using `IP_ADD_MEMBERSHIP` with the interface address (not INADDR_ANY for the interface field).
- Implement the full SSDP lifecycle: initial NOTIFY (send 2-3 times with short delays for UDP reliability), periodic re-advertisements, M-SEARCH response handling, and `ssdp:byebye` on shutdown.
- Send each SSDP message 2-3 times with 100-300ms delays between sends (UDP has no delivery guarantee).
- Set `CACHE-CONTROL: max-age=1800` and re-advertise every 900 seconds.
- Log every M-SEARCH received and every response sent for debugging.

**Warning signs:**
- Server works on Linux but not macOS (interface binding differences)
- Discovery works sometimes but not others (UDP packet loss, timing)
- Device discovers server initially but loses it after 30 minutes (re-advertisement missing)
- Server appears on one device but not another on the same network (interface binding)

**Phase to address:**
SSDP/Discovery phase (must be the first networking phase). Everything depends on discovery working.

---

### Pitfall 3: HTTP Range Request Handling Breaks Seeking and Large Files

**What goes wrong:**
DLNA clients (especially Samsung and Xbox) send HTTP range requests (`Range: bytes=0-`) to probe file accessibility before playing, and then use range requests for seeking. Three common failures:

1. **Not supporting `Range: bytes=0-` (open-ended range).** Samsung TVs probe with this exact request. If the server returns 200 instead of 206 Partial Content with correct `Content-Range` header, Samsung either refuses to play or plays without seek support.

2. **Wrong `Content-Range` header format.** Must be exactly `Content-Range: bytes 0-{last_byte}/{total_size}` (note: `{last_byte}` = `{total_size} - 1`). Off-by-one errors here cause Samsung to report file corruption.

3. **Not returning `Accept-Ranges: bytes` header.** Even on non-range responses (HTTP 200), the server must include `Accept-Ranges: bytes` to signal range support. Without it, some clients never attempt range requests and fall back to buffering the entire file (fails for large files).

4. **Not supporting `Range: bytes={start}-` (start to end) requests.** This is how seeking works. The server must return 206 with the range from `{start}` to end of file, with correct `Content-Length` for the partial response (not the full file length).

**Why it happens:**
RFC 7233 (Range Requests) is straightforward, but DLNA clients are stricter than web browsers about the response format. Web browsers gracefully handle servers that return 200 instead of 206. Samsung TVs do not.

**How to avoid:**
- Implement full RFC 7233 range request handling from the start. Parse the `Range` header, return 206 with `Content-Range` and correct partial `Content-Length`.
- Always include `Accept-Ranges: bytes` on every HTTP response from the media endpoint.
- Handle open-ended ranges (`bytes=0-`, `bytes=12345-`) and closed ranges (`bytes=100-200`).
- Handle multi-range requests by rejecting them (return 200 with full file) -- multi-range is rarely used by DLNA clients and implementing it is complex.
- Test with `curl -H "Range: bytes=0-0" -v http://server/media/file.mp4` and verify the response is `206` with `Content-Range: bytes 0-0/{filesize}`.

**Warning signs:**
- Video plays but seeking causes playback to restart from beginning
- Large files (>2GB) fail to play or play only the first few seconds
- Samsung TV shows "format not supported" for files that play fine locally
- `Content-Length` in 206 response equals total file size (should be range size)

**Phase to address:**
HTTP streaming phase. Must be implemented before any device testing, as broken range requests make every file appear broken.

---

### Pitfall 4: Missing or Incorrect DLNA-Specific HTTP Headers

**What goes wrong:**
DLNA extends HTTP with several custom headers that strict clients (Samsung, Xbox) require. Missing these headers causes clients to either refuse to play files or treat the server as a non-DLNA generic HTTP server (losing features like seeking, duration display, and format detection).

Required DLNA HTTP headers on media streaming responses:
1. **`contentFeatures.dlna.org`** -- MUST be present. Contains the DLNA profile name, operation flags, and capability flags. Example: `DLNA.ORG_PN=AVC_MP4_MP_SD_AAC_MULT5;DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000`. Samsung and Xbox both check this header.
2. **`transferMode.dlna.org`** -- MUST be `Streaming` for video/audio, `Interactive` for images/subtitles. If missing, Samsung may refuse to play video files.
3. **`realTimeInfo.dlna.org`** -- Should be `DLNA.ORG_TLAG=*` (not required by all clients but Samsung has been observed to check for it in some firmware versions).

The `DLNA.ORG_OP` field in `contentFeatures.dlna.org` is critical:
- `DLNA.ORG_OP=01` means the server supports byte-based seeking (Range header). This is what you want for file serving.
- `DLNA.ORG_OP=10` means time-based seeking (not applicable for raw file serving).
- `DLNA.ORG_OP=00` means no seeking -- Samsung will not show a progress bar.

The `DLNA.ORG_FLAGS` field must have at minimum bits 24 (DLNA 1.5) and 20 (streaming transfer) set. The standard "background transfer" flags value is `01700000000000000000000000000000` (hex, 32 chars, zero-padded).

**Why it happens:**
These headers are documented in the DLNA Guidelines (a paid specification), not in the free UPnP specs. Most developers only read the UPnP specification and miss the DLNA overlay. The headers look optional because generic UPnP control points (like VLC) work without them.

**How to avoid:**
- Add all three DLNA headers to every media streaming response from day one.
- Build a `DlnaHeaders` utility module that generates correct header values based on MIME type.
- Map MIME types to DLNA profile names (see Pitfall 6 for the mapping table).
- Hardcode `DLNA.ORG_OP=01` for all file-served content (byte seeking supported).
- Hardcode `DLNA.ORG_FLAGS=01700000000000000000000000000000`.
- The `contentFeatures.dlna.org` header value MUST also match the fourth field of the `protocolInfo` attribute in the DIDL-Lite `<res>` element. If these don't match, Samsung may reject the file.

**Warning signs:**
- Files play on VLC but not on Samsung/Xbox
- Samsung shows the file but displays "Unable to play this file" on selection
- No seek bar appears on the TV during playback
- Xbox plays the file but shows unknown duration (0:00 / 0:00)

**Phase to address:**
HTTP streaming phase, same phase as range requests. These headers are part of the HTTP media response and should be implemented together.

---

### Pitfall 5: UPnP Device Description XML Rejected by Strict Clients

**What goes wrong:**
The UPnP device description document (served at the URL in the SSDP `LOCATION` header) must conform to the UPnP Device Architecture schema exactly. Xbox and Samsung both fetch this document and parse it strictly. Common failures:

1. **Missing required elements.** The device description must include `<friendlyName>`, `<manufacturer>`, `<modelName>`, `<UDN>` (unique device name in format `uuid:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`), and the service list with `<serviceType>`, `<serviceId>`, `<controlURL>`, `<eventSubURL>`, and `<SCPDURL>` for each service.

2. **Wrong XML namespace.** Must be `urn:schemas-upnp-org:device-1-0`. Using `device-1-1` or omitting the namespace causes Xbox to ignore the device.

3. **`<UDN>` changes between restarts.** If the UDN is randomly generated every time the server starts, devices treat it as a new device each time. Samsung may show duplicate entries. Xbox may not show it at all if the old UDN is still cached.

4. **Missing `<serviceList>` or incomplete service declarations.** The `ContentDirectory` and `ConnectionManager` services must both be declared. Some clients require both even if you only implement `ContentDirectory`.

5. **`<URLBase>` element issues.** Older UPnP 1.0 devices use `<URLBase>` to prefix relative URLs in the device description. If present, it must be correct. If the server's IP changes or it's behind NAT, `<URLBase>` with the wrong address breaks everything. Best practice: omit `<URLBase>` and use absolute URLs in `<controlURL>`, `<eventSubURL>`, and `<SCPDURL>`.

**Why it happens:**
Developers copy device description XML from examples that may be for UPnP 1.1 or 2.0, or from non-DLNA devices. The Xbox DLNA stack is based on Microsoft's older Windows Media Player stack which expects UPnP 1.0 conventions.

**How to avoid:**
- Start from a known-working device description XML (MiniDLNA's is the gold standard).
- Use UPnP 1.0 namespace (`urn:schemas-upnp-org:device-1-0`).
- Generate a deterministic UDN from the machine's hostname or MAC address. Store it in the config file so it persists across restarts.
- Always declare both `ContentDirectory:1` and `ConnectionManager:1` services.
- Omit `<URLBase>` and use absolute URLs everywhere.
- Include `<dlna:X_DLNADOC xmlns:dlna="urn:schemas-dlna-org:device-1-0">DMS-1.50</dlna:X_DLNADOC>` in the device element to identify as a DLNA Digital Media Server.

**Warning signs:**
- SSDP discovery works (device appears) but browsing fails (can't load service description)
- Xbox sees the server name but shows "Can't connect to media server"
- Server works on first run but after restart, Samsung shows two entries (old cached + new)
- HTTP 404 when device tries to fetch control/SCPD URLs (relative URL resolution failed)

**Phase to address:**
UPnP device description phase (immediately after SSDP, before ContentDirectory). The device description is the entry point that clients fetch after SSDP discovery.

---

### Pitfall 6: MIME Type Mapping and DLNA Profile Name Mismatches

**What goes wrong:**
DLNA clients use the MIME type (from the HTTP `Content-Type` header and the `protocolInfo` in DIDL-Lite) to determine how to handle a file. Wrong MIME types cause two kinds of failure: (1) the client refuses to play the file because it doesn't recognize the format, or (2) the client tries to play with the wrong decoder.

Critical MIME type mistakes:
1. **Using `video/x-matroska` for MKV files.** This is the correct MIME type per IANA but Samsung TVs do not recognize it. Samsung requires `video/x-mkv` -- no, actually this varies by firmware. Some Samsung firmware versions accept `video/x-matroska` and others only accept `video/mkv`. The safest approach is `video/x-matroska` with DLNA profile name `MKV` but be prepared to adjust.
2. **Using `audio/mp3` instead of `audio/mpeg`.** The correct MIME type is `audio/mpeg`. `audio/mp3` is non-standard and some clients reject it.
3. **Not distinguishing MPEG-4 variants.** `video/mp4` is correct for .mp4 files. `video/mpeg` is for .mpg/.mpeg. Confusing them causes decoder errors.
4. **Missing DLNA profile name or wrong profile name.** The DLNA profile name (e.g., `AVC_MP4_MP_SD_AAC_MULT5`) is supposed to describe the exact codec configuration. Since we're not transcoding, we can't guarantee the file matches a specific profile. For file-based serving, use wildcard-like profiles: `AVC_MP4` for MP4 with H.264, `MPEG_TS` for MPEG transport streams, or omit the profile name (use `*` in protocolInfo) and let the client figure it out.

**Why it happens:**
MIME type standards, DLNA profile names, and client implementations all disagree on exact mappings. There is no single correct mapping table. Each client has firmware-specific behavior. Additionally, DLNA profile names are tied to specific codec configurations (resolution, bitrate, audio codec) that can't be determined from the file extension alone without probing the file.

**How to avoid:**
- Build a MIME type map based on file extension. Start with the most common:
  - `.mp4`, `.m4v` -> `video/mp4`
  - `.mkv` -> `video/x-matroska`
  - `.avi` -> `video/avi` (some clients prefer `video/x-msvideo`)
  - `.mov` -> `video/quicktime`
  - `.ts`, `.m2ts` -> `video/vnd.dlna.mpeg-tts` (Samsung-specific) or `video/mpeg`
  - `.mp3` -> `audio/mpeg`
  - `.flac` -> `audio/flac`
  - `.wav` -> `audio/wav`
  - `.jpg`, `.jpeg` -> `image/jpeg`
  - `.png` -> `image/png`
- For DLNA profile names, use a conservative approach: provide a general profile name or omit it (use `*`) rather than claiming a specific profile the file might not match.
- The `protocolInfo` fourth field can be just `*` if you don't want to claim DLNA compliance per-file. Samsung will still play the file based on MIME type alone.
- Consider using `contentFeatures.dlna.org` with `DLNA.ORG_PN=*` or omitting `DLNA.ORG_PN` entirely and only specifying `DLNA.ORG_OP` and `DLNA.ORG_FLAGS`.

**Warning signs:**
- Specific file formats fail while others work (MIME type mapping issue)
- Files that play on one TV model fail on another (firmware-specific MIME handling)
- Client shows "Format not supported" for common formats like MKV or AVI

**Phase to address:**
HTTP streaming phase and ContentDirectory phase (MIME types appear in both the HTTP response and the DIDL-Lite XML). Should be implemented as a shared utility module.

---

### Pitfall 7: Xbox Series X Specific ContentDirectory and SOAP Quirks

**What goes wrong:**
Xbox Series X uses the Microsoft DLNA/UPnP stack (descended from Windows Media Player / Windows Media Center). This stack has several idiosyncrasies:

1. **Xbox requires the `ConnectionManager` service to respond.** Even if you only implement `ContentDirectory`, the Xbox will call `ConnectionManager:GetProtocolInfo` during initial connection. If this returns an error or the service URL 404s, Xbox abandons the server. You must implement at minimum `GetProtocolInfo` returning a list of supported MIME types.

2. **Xbox sends `X-AV-Client-Info` and `User-Agent` headers** with specific patterns. While you don't need to parse these, some implementations accidentally reject requests with unusual headers via overly strict request validation.

3. **SOAP action header format.** Xbox sends the SOAPAction header with the full service type URI. It must match exactly: `"urn:schemas-upnp-org:service:ContentDirectory:1#Browse"` (note the quotes around the entire value are part of the HTTP header value). Some implementations strip or misparse the quotes.

4. **Browse request parameters.** Xbox always sends `BrowseFlag` as `BrowseDirectChildren` (not `BrowseMetadata` for individual items initially). The `Filter` parameter will be `*` (requesting all metadata fields). The `RequestedCount` will often be `0` meaning "return everything". If you interpret 0 as "return nothing", Xbox shows an empty library.

5. **`SystemUpdateID` must be consistent.** Xbox calls `GetSystemUpdateID` to check if the library has changed. If this returns different values on subsequent calls without the library actually changing, Xbox re-fetches everything repeatedly, causing UI lag and potential infinite loops.

**Why it happens:**
Microsoft's UPnP stack is old (Windows Media Player 11 era) and follows the spec literally, including edge cases that most other clients ignore. Xbox's DLNA support is a media app, not a dedicated media player, so it has less tolerance for server quirks.

**How to avoid:**
- Implement `ConnectionManager` service with at minimum `GetProtocolInfo` returning a `Source` value listing supported MIME types (e.g., `http-get:*:video/mp4:*,http-get:*:video/x-matroska:*,...`).
- Handle `RequestedCount=0` as "return all items" (per UPnP spec, 0 means "no limit").
- Return consistent `SystemUpdateID` values -- increment only when files are actually added/removed. Use a hash of the file list or a monotonic counter stored in memory.
- Parse SOAP requests with an XML parser, not regex. The SOAP envelope namespace, body structure, and action elements must be correctly handled.
- Don't reject requests with unknown headers -- ignore headers you don't recognize.

**Warning signs:**
- Xbox sees server but says "Can't connect" or "No media found"
- Xbox connects initially but library appears empty
- Xbox shows server but re-scans repeatedly, UI is sluggish
- Server works on Samsung but not Xbox (or vice versa)

**Phase to address:**
ContentDirectory service phase. ConnectionManager should be implemented in the same phase even though it's simpler, because Xbox requires both.

---

### Pitfall 8: SOAP Envelope and XML Namespace Precision in Action Responses

**What goes wrong:**
UPnP service actions (Browse, GetSystemUpdateID, GetProtocolInfo) use SOAP over HTTP POST. The SOAP response envelope must be precisely formatted. Common failures:

1. **Wrong SOAP namespace.** Must be `http://schemas.xmlsoap.org/soap/envelope/` (SOAP 1.1). Using SOAP 1.2 namespace (`http://www.w3.org/2003/05/soap-envelope`) causes all clients to fail.

2. **Wrong encoding style.** The `encodingStyle` attribute must be `http://schemas.xmlsoap.org/soap/encoding/`.

3. **DIDL-Lite content inside SOAP must be XML-escaped.** The Browse response includes DIDL-Lite XML as a string value inside the SOAP XML. This string must be XML-escaped (i.e., `<` becomes `&lt;`, `>` becomes `&gt;`, `&` becomes `&amp;`). If you embed raw DIDL-Lite XML without escaping, the SOAP envelope becomes malformed. This is the single most common implementation mistake.

4. **Missing `NumberReturned`, `TotalMatches`, and `UpdateID` in Browse response.** All three are required in the Browse action response. `NumberReturned` must match the actual count of items in the DIDL-Lite `Result` string. `TotalMatches` must be the total count of items matching the query (may differ from `NumberReturned` if pagination is used). `UpdateID` must match `SystemUpdateID`.

5. **HTTP Content-Type for SOAP responses.** Must be `text/xml; charset="utf-8"`. Using `application/xml` or omitting the charset causes some clients to reject the response.

**Why it happens:**
SOAP-in-HTTP is an awkward protocol. XML-in-XML (DIDL-Lite inside SOAP) requires careful escaping. Most modern developers have never worked with SOAP and may not realize the Content-Type requirements or the escaping rules.

**How to avoid:**
- Use an XML library to construct SOAP responses. Never build SOAP XML by string concatenation.
- Double-check: the DIDL-Lite string inside `<Result>` must be XML-escaped text, not raw XML elements.
- Validate that `NumberReturned` matches the actual item count in the DIDL-Lite.
- Always return Content-Type `text/xml; charset="utf-8"` on SOAP responses.
- Test SOAP responses by piping them through an XML validator to ensure well-formedness.

**Warning signs:**
- Client receives response but can't parse it (XML parser error on client side -- invisible to server)
- "No items" displayed despite server returning items (DIDL-Lite not escaped, parsed as SOAP structure)
- Some items appear but count is wrong (NumberReturned mismatch)

**Phase to address:**
ContentDirectory service phase. This is core protocol work.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| String concatenation for XML | Fast to write, no XML library dependency | Escaping bugs, malformed XML when filenames contain `<`, `&`, `"` characters | Never -- use an XML builder from day one |
| Hardcoding server IP address | Works on dev machine | Breaks when IP changes, doesn't work on multi-NIC machines | Never -- detect interface addresses dynamically |
| Random UDN on each startup | No persistence needed | Duplicate device entries on clients, confusing user experience | Only during initial development, fix before device testing |
| Ignoring ConnectionManager service | Fewer endpoints to implement | Xbox refuses to connect, any client calling GetProtocolInfo gets 404 | Never if Xbox is a target client |
| Single-threaded HTTP server | Simpler implementation | Seeking/browsing blocks while another file is streaming; second device can't browse while first is playing | Only for initial proof-of-concept; switch to async before device testing |
| Using synchronous file I/O for streaming | Simpler implementation | Thread pool exhaustion with multiple concurrent streams | Never in production -- use async I/O from the start (tokio::fs) |

## Integration Gotchas

Common mistakes when connecting to external services/protocols.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| SSDP Multicast (UDP) | Binding to `0.0.0.0` without `IP_ADD_MEMBERSHIP` per interface | Enumerate interfaces, join multicast group on each non-loopback IPv4 interface |
| SSDP M-SEARCH response | Sending multicast response instead of unicast to requesting device | Respond via unicast UDP to the source IP:port from the M-SEARCH request |
| SOAP over HTTP | Using HTTP/1.0 or not supporting persistent connections | Use HTTP/1.1 with `Connection: keep-alive` support (some clients send multiple SOAP requests on one connection) |
| File system scanning | Following symlinks without cycle detection | Use `canonicalize()` on paths and track visited inodes to prevent infinite loops |
| macOS network | Assuming `en0` is always the active WiFi interface | Use `getifaddrs()` to enumerate interfaces; filter by `IFF_UP`, `IFF_MULTICAST`, and non-loopback |
| Large file serving | Using `read_to_end()` or buffering entire file in memory | Stream directly from file handle using `tokio::io::copy()` or chunked reads |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Full directory scan on every Browse request | Slow UI response, high CPU during browsing | Scan once at startup, cache file list, optionally watch for changes | >1000 files in a directory |
| Reading file metadata (size) on every HTTP request | Slow response, especially on NFS/network drives | Cache file sizes from initial scan; stat() is cheap on local SSD but expensive on network storage | Network-mounted media directories |
| Allocating new strings for every XML response | GC pressure (not relevant for Rust, but allocation overhead) | Use a template with pre-allocated buffers; `String::with_capacity()` for DIDL-Lite responses | >500 items in a single Browse response |
| Logging every SSDP packet at INFO level | Log files grow rapidly, I/O overhead | Log SSDP at DEBUG/TRACE level; only log discovery events (new device found) at INFO | Always -- SSDP is chatty, ~1 packet/second from each device on network |
| Not implementing Browse pagination | Works fine with small libraries | Client timeout on Browse response for large libraries; some clients have response size limits (~64KB) | >200 items (Samsung has been reported to truncate at ~200-300 items without pagination) |

## Security Mistakes

Domain-specific security issues beyond general web security.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Serving files outside the declared media root via path traversal (`../../etc/passwd` in browse requests) | Arbitrary file read from any network device | Canonicalize all paths and verify they start with the media root prefix before serving |
| Binding HTTP server to `0.0.0.0` with no auth on a machine with a public IP | Media files accessible from the internet | Bind only to private network interfaces (10.x, 172.16-31.x, 192.168.x); alternatively bind to specific interface IPs only |
| SSDP amplification (responding to spoofed M-SEARCH requests) | Server becomes a UDP amplification reflector | Rate-limit SSDP responses; don't respond to M-SEARCH from non-local addresses |
| Including full filesystem paths in DIDL-Lite XML | Leaks directory structure to all network devices | Use opaque IDs in DIDL-Lite; map IDs to paths internally |
| No request size limit on SOAP POST bodies | Denial of service via large SOAP request | Limit SOAP request body to 64KB (Browse requests are typically <2KB) |

## UX Pitfalls

Common user experience mistakes in this domain (for the CLI user running the server).

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| No feedback after `udlna /path/to/media` | User doesn't know if server is running, what IP/port, or if discovery is working | Print: server name, bound IP:port, number of files found, "Waiting for DLNA clients..." |
| No indication when a device connects/browses | User can't tell if their TV found the server | Log device connections at INFO level: "Samsung TV (192.168.1.50) browsing library" |
| Cryptic error on invalid media path | User confused by panic or obscure error | Validate paths at startup, print clear error: "Error: /bad/path does not exist or is not a directory" |
| No progress indication during file scan | Appears hung on large libraries | Print file count as scanning progresses, or at minimum print "Scanning... found N files" on completion |
| Server name shows as UUID or "Unknown" on TV | User doesn't know which server is theirs | Default friendly name to hostname or "udlna on {hostname}"; make configurable |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **SSDP Discovery:** Server advertises on startup -- but does it re-advertise periodically? Does it send `ssdp:byebye` on shutdown? Does it respond to M-SEARCH? Does it work on WiFi (not just Ethernet)?
- [ ] **ContentDirectory Browse:** Returns items -- but does it handle `BrowseMetadata` for individual items (not just `BrowseDirectChildren`)? Does it handle `ObjectID=0` (root container)? Does `RequestedCount=0` return all items?
- [ ] **HTTP Streaming:** Files download correctly -- but does it return 206 for Range requests? Does it include `Accept-Ranges: bytes`? Does it include DLNA headers? Does it handle `HEAD` requests (some clients probe with HEAD before GET)?
- [ ] **DIDL-Lite XML:** Items display on client -- but does it include `protocolInfo` on `<res>`? Are filenames with special characters (`&`, `<`, `"`, non-ASCII) properly XML-escaped? Does the `<res>` URL use the server's actual LAN IP (not 127.0.0.1)?
- [ ] **Device Description:** Clients fetch it -- but does it declare both ContentDirectory and ConnectionManager? Is the UDN stable across restarts? Are control/SCPD URLs absolute or correctly relative?
- [ ] **SOAP Handling:** Browse works -- but does it return `SOAP Fault` for unknown actions (not HTTP 500 with no body)? Does it handle `GetSystemUpdateID`? Does it handle `GetSortCapabilities` and `GetSearchCapabilities` (some clients call these)?
- [ ] **ConnectionManager:** It's declared in device description -- but does `GetProtocolInfo` actually return data? Does `GetCurrentConnectionInfo` return a valid response?
- [ ] **File Serving:** Standard files play -- but do files with spaces in the name work? Files with Unicode names? Files over 2GB (ensure `Content-Length` uses 64-bit integer)? Files with no extension?

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Malformed DIDL-Lite XML | LOW | Capture a working response from MiniDLNA, diff against yours, fix discrepancies |
| SSDP not working | MEDIUM | Use Wireshark to capture multicast traffic on 239.255.255.250:1900; compare with working server |
| Range request bugs | LOW | Test with curl range requests; fix header formatting; verify with Samsung TV |
| Missing DLNA headers | LOW | Add the three DLNA headers; match protocolInfo between DIDL-Lite and HTTP response |
| Wrong device description | LOW | Compare with MiniDLNA device description XML; fix namespace and element structure |
| MIME type mismatches | LOW | Build mapping table; test each format on target devices; adjust per firmware |
| Xbox ConnectionManager missing | LOW | Add ConnectionManager service with GetProtocolInfo stub; return supported MIME list |
| SOAP escaping broken | MEDIUM | Switch from string concat to XML builder; re-test all SOAP responses through XML validator |
| Path traversal vulnerability | LOW | Add path canonicalization check; verify all served paths are under media root |
| UDN changes on restart | LOW | Generate deterministic UDN from hostname; clear client device caches after fix |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Malformed DIDL-Lite XML | ContentDirectory implementation | Test Browse response against Samsung TV; validate XML with `xmllint` |
| SSDP multicast failures | SSDP/Discovery implementation | Verify with `gssdp-discover` tool or Wireshark capture; test on WiFi and Ethernet |
| HTTP range request bugs | HTTP streaming implementation | Test with `curl -r 0-0`, `curl -r 100-200`, `curl -r 1000-` against server |
| Missing DLNA HTTP headers | HTTP streaming implementation | Inspect response headers with `curl -v`; verify on Samsung TV and Xbox |
| Device description rejected | UPnP device setup (post-SSDP) | Validate XML with schema; test fetch from Samsung TV debug tools |
| MIME type mismatches | HTTP + ContentDirectory (shared module) | Test each supported format on both Samsung TV and Xbox |
| Xbox ConnectionManager | ContentDirectory phase (implement together) | Test specifically with Xbox Series X after implementation |
| SOAP envelope/escaping | ContentDirectory implementation | XML-validate all SOAP responses; test with filenames containing `&`, `<`, `"` |
| Path traversal | HTTP streaming implementation | Automated test with `../` path sequences; verify 403/404 returned |
| Browse pagination | ContentDirectory implementation | Test with library >200 items on Samsung TV |

## Samsung TV Specific Gotchas

Samsung Smart TVs deserve a dedicated section because they are the most problematic DLNA client.

| Issue | Samsung Behavior | Required Server Behavior |
|-------|------------------|--------------------------|
| Empty library shown | Samsung parsed XML but found no valid items | Ensure every `<item>` has `<dc:title>`, `<upnp:class>`, and `<res>` with `protocolInfo` |
| "Format not supported" | Samsung determined format from protocolInfo/MIME, not file content | Use correct MIME type and DLNA profile; if unsure, use `*` for profile |
| No seek bar | Samsung didn't detect range support | Include `DLNA.ORG_OP=01` in both protocolInfo and contentFeatures.dlna.org header; return 206 for range requests |
| Server disappears after ~30 min | SSDP cache expired, no re-advertisement received | Re-advertise SSDP every `max-age/2` seconds |
| Duplicate server entries | UDN changed between restarts | Use deterministic UDN |
| Can't play files >4GB | Samsung older firmware has 32-bit file size limitation | Test with specific TV model; some firmware updates fix this; no server-side workaround |
| Fails to browse after first page | Pagination not implemented or broken | Return correct `TotalMatches` and handle `StartingIndex` parameter in Browse |
| Korean characters in filename break | XML encoding issue | Ensure DIDL-Lite XML uses UTF-8 encoding declaration; properly escape all Unicode in XML |

## Xbox Series X Specific Gotchas

| Issue | Xbox Behavior | Required Server Behavior |
|-------|---------------|--------------------------|
| "Can't connect to media server" | ConnectionManager service not implemented or returning errors | Implement GetProtocolInfo with supported MIME types |
| Empty library | Browse returned 0 items (RequestedCount=0 misinterpreted) | Treat RequestedCount=0 as "no limit" |
| Server not discovered | Xbox SSDP stack expects specific `NT` and `USN` values | Advertise with `NT: urn:schemas-upnp-org:device:MediaServer:1` and correct `USN` format |
| Re-scanning constantly | SystemUpdateID changing between calls | Return consistent value; only change when library actually changes |
| Only shows video (no audio/images) | `upnp:class` values not matching Xbox expectations | Use standard class values: `object.item.videoItem`, `object.item.audioItem.musicTrack`, `object.item.imageItem.photo` |
| "Codec not supported" for MKV with DTS | Xbox doesn't support DTS audio in MKV container | No server-side fix (no transcoding); document as limitation |

## Sources

- UPnP Device Architecture 1.0 specification (UPnP Forum) -- HIGH confidence (stable spec, not subject to change)
- DLNA Guidelines (DLNA.org / SpireSpark, now defunct) -- MEDIUM confidence (spec frozen since 2017; knowledge from training data, web verification unavailable)
- MiniDLNA/ReadyMedia source code and issue tracker patterns -- MEDIUM confidence (well-documented open source reference implementation)
- Samsung Smart TV DLNA compatibility reports from community forums -- MEDIUM confidence (consistent patterns across multiple training data sources)
- Xbox DLNA support documentation from Microsoft community -- MEDIUM confidence (consistent patterns across multiple training data sources)
- Note: Web search and web fetch were unavailable during this research session. All findings are based on training data covering well-established, frozen specifications (DLNA/UPnP). The specifications have not changed since 2017. Confidence levels reflect this limitation.

---
*Pitfalls research for: DLNA/UPnP Media Server (udlna)*
*Researched: 2026-02-22*
