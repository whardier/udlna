# Architecture Research

**Domain:** Minimal DLNA/UPnP Media Server
**Researched:** 2026-02-22
**Confidence:** HIGH

Note on confidence: UPnP Device Architecture 1.0/1.1 and DLNA guidelines are frozen specifications. The DLNA organization dissolved in 2017; the UPnP Forum merged into OCF but the AV specs have not changed materially since ~2015. Protocol details from training data are highly reliable because nothing has changed. All protocol constants (multicast addresses, port numbers, XML namespaces, SOAP action formats) are stable facts verifiable against the published UPnP AV specifications.

## Standard Architecture

### System Overview

```
                          LAN (192.168.x.x)
                               |
          ┌────────────────────┼────────────────────┐
          |                    |                     |
   ┌──────┴──────┐    ┌───────┴───────┐    ┌───────┴───────┐
   |  DLNA       |    |  udlna        |    |  Other LAN    |
   |  Client     |    |  Server       |    |  Devices      |
   |  (TV/Xbox)  |    |               |    |               |
   └──────┬──────┘    └───────┬───────┘    └───────────────┘
          |                    |
          |   1. SSDP M-SEARCH (multicast UDP 239.255.255.250:1900)
          |------------------->|
          |   2. SSDP Response (unicast UDP)
          |<-------------------|
          |   3. GET /device.xml (HTTP)
          |------------------->|
          |   4. Device Description XML
          |<-------------------|
          |   5. SOAP POST /ContentDirectory (HTTP)
          |------------------->|
          |   6. DIDL-Lite XML response
          |<-------------------|
          |   7. GET /media/{id} (HTTP with Range headers)
          |------------------->|
          |   8. Media bytes (206 Partial Content)
          |<-------------------|
          └────────────────────┘
```

### Internal Component Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        udlna Process                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │ SSDP Module  │  │ HTTP Server  │  │ Media Scanner        │  │
│  │              │  │              │  │                      │  │
│  │ - Listener   │  │ - Device XML │  │ - Walk directories   │  │
│  │ - Responder  │  │ - SOAP/CDS   │  │ - Detect MIME types  │  │
│  │ - Advertiser │  │ - Streaming  │  │ - Assign stable IDs  │  │
│  └──────┬───────┘  └──────┬───────┘  └──────────┬───────────┘  │
│         │                 │                      │              │
│         │    ┌────────────┴──────────────┐       │              │
│         │    │     Request Router        │       │              │
│         │    │                           │       │              │
│         │    │  /device.xml → DeviceDesc │       │              │
│         │    │  /cds/* → ContentDir SOAP │       │              │
│         │    │  /media/* → FileStreamer  │       │              │
│         │    └───────────────────────────┘       │              │
│         │                                        │              │
│  ┌──────┴────────────────────────────────────────┴───────────┐  │
│  │                   Shared State                            │  │
│  │                                                           │  │
│  │  - MediaLibrary: Vec<MediaItem> (file list + metadata)    │  │
│  │  - ServerConfig: name, UUID, port, paths                  │  │
│  │  - SystemUpdateID: u32 (increments on rescan)             │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│                    Network Layer                                │
│  ┌─────────────────────┐  ┌──────────────────────────────────┐  │
│  │ UDP Socket          │  │ TCP Listener                     │  │
│  │ 239.255.255.250:1900│  │ 0.0.0.0:{configured_port}       │  │
│  │ (multicast join)    │  │ (HTTP/1.1)                       │  │
│  └─────────────────────┘  └──────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| **SSDP Module** | UPnP device discovery over UDP multicast. Listens for M-SEARCH queries, sends responses, and periodically advertises (NOTIFY ssdp:alive). Sends ssdp:byebye on shutdown. | Async UDP socket bound to 239.255.255.250:1900 with multicast group join. |
| **HTTP Server** | Serves all HTTP endpoints: device description XML, SOAP control requests for ContentDirectory, and raw media file streaming. | Single async TCP listener with path-based routing. |
| **Device Description** | Generates the UPnP device description XML served at a well-known URL. Declares the device type, friendly name, UUID, and lists available services (ContentDirectory). | Static XML template with runtime substitution of UUID, server name, and local IP. |
| **ContentDirectory Service** | Handles SOAP requests for Browse and GetSystemUpdateID. Returns DIDL-Lite XML listing media items. This is the core of DLNA browsing. | SOAP XML parser (extract action + arguments), DIDL-Lite XML builder for responses. |
| **Media Scanner** | Recursively walks configured directories, detects file types by extension, assigns stable IDs (index or hash-based), determines MIME types, and builds the in-memory media library. | Run once at startup. Produces a `Vec<MediaItem>` stored in shared state. |
| **File Streamer** | Serves actual media file bytes over HTTP. Must support Range requests (RFC 7233) for seeking. Sends correct Content-Type, Content-Length, Accept-Ranges headers. | Async file I/O with byte-range parsing. Returns 200 (full) or 206 (partial). |
| **Shared State** | Holds the media library, server configuration, and system update ID. Read-only after initial scan (no mutation concerns). | `Arc<ServerState>` passed to all handlers. No mutex needed if library is immutable after startup. |

## Full DLNA Client-Server Interaction Flow

### Phase 1: Discovery (SSDP)

The client discovers the server using SSDP (Simple Service Discovery Protocol) over UDP multicast.

**Step 1a: Client sends M-SEARCH (multicast)**

```http
M-SEARCH * HTTP/1.1
HOST: 239.255.255.250:1900
MAN: "ssdp:discover"
MX: 3
ST: urn:schemas-upnp-org:device:MediaServer:1
```

The client sends this to the multicast group 239.255.255.250 on port 1900. `MX` is the maximum wait time in seconds; the server should respond within a random delay of 0 to MX seconds to avoid network storms.

**Step 1b: Server responds (unicast)**

```http
HTTP/1.1 200 OK
CACHE-CONTROL: max-age=1800
LOCATION: http://192.168.1.100:8200/device.xml
ST: urn:schemas-upnp-org:device:MediaServer:1
USN: uuid:abcd-1234::urn:schemas-upnp-org:device:MediaServer:1
SERVER: Linux/1.0 UPnP/1.0 udlna/0.1
```

The response is sent as unicast UDP back to the client's source address/port. The critical field is `LOCATION` -- this tells the client where to fetch the device description.

**Step 1c: Server also sends periodic NOTIFY (multicast)**

```http
NOTIFY * HTTP/1.1
HOST: 239.255.255.250:1900
CACHE-CONTROL: max-age=1800
LOCATION: http://192.168.1.100:8200/device.xml
NT: urn:schemas-upnp-org:device:MediaServer:1
NTS: ssdp:alive
USN: uuid:abcd-1234::urn:schemas-upnp-org:device:MediaServer:1
SERVER: Linux/1.0 UPnP/1.0 udlna/0.1
```

Samsung TVs and Xbox in particular rely on NOTIFY advertisements because they may not actively M-SEARCH frequently. The server must send these periodically (every ~900 seconds, or half the max-age).

### Phase 2: Device Description (HTTP GET)

**Step 2a: Client fetches device description**

```
GET /device.xml HTTP/1.1
Host: 192.168.1.100:8200
```

**Step 2b: Server returns device description XML**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType>
    <friendlyName>udlna</friendlyName>
    <manufacturer>udlna</manufacturer>
    <modelName>udlna</modelName>
    <modelNumber>0.1</modelNumber>
    <UDN>uuid:abcd1234-abcd-1234-abcd-abcd1234abcd</UDN>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ContentDirectory</serviceId>
        <controlURL>/cds/control</controlURL>
        <eventSubURL>/cds/events</eventSubURL>
        <SCPDURL>/cds/scpd.xml</SCPDURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ConnectionManager</serviceId>
        <controlURL>/cms/control</controlURL>
        <eventSubURL>/cms/events</eventSubURL>
        <SCPDURL>/cms/scpd.xml</SCPDURL>
      </service>
    </serviceList>
  </device>
</root>
```

**Critical notes:**
- `UDN` must be a persistent UUID. If it changes between runs, clients treat it as a new device (duplicates in the TV's source list). Generate once, persist in config.
- `ConnectionManager` is technically required by the DLNA spec even if minimal. Samsung TVs may refuse to interact without it. It can return hardcoded "not implemented" for PrepareForConnection/ConnectionComplete, but `GetProtocolInfo` must return supported MIME types.
- The `SCPDURL` points to the service description XML (SCPD) which declares available actions and state variables. Clients fetch this to learn what the service supports.

### Phase 3: Service Description (SCPD)

**Step 3a: Client fetches ContentDirectory SCPD**

```
GET /cds/scpd.xml HTTP/1.1
```

**Step 3b: Server returns service description**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <actionList>
    <action>
      <name>Browse</name>
      <argumentList>
        <argument>
          <name>ObjectID</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_ObjectID</relatedStateVariable>
        </argument>
        <argument>
          <name>BrowseFlag</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_BrowseFlag</relatedStateVariable>
        </argument>
        <argument>
          <name>Filter</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_Filter</relatedStateVariable>
        </argument>
        <argument>
          <name>StartingIndex</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_Index</relatedStateVariable>
        </argument>
        <argument>
          <name>RequestedCount</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable>
        </argument>
        <argument>
          <name>SortCriteria</name>
          <direction>in</direction>
          <relatedStateVariable>A_ARG_TYPE_SortCriteria</relatedStateVariable>
        </argument>
        <argument>
          <name>Result</name>
          <direction>out</direction>
          <relatedStateVariable>A_ARG_TYPE_Result</relatedStateVariable>
        </argument>
        <argument>
          <name>NumberReturned</name>
          <direction>out</direction>
          <relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable>
        </argument>
        <argument>
          <name>TotalMatches</name>
          <direction>out</direction>
          <relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable>
        </argument>
        <argument>
          <name>UpdateID</name>
          <direction>out</direction>
          <relatedStateVariable>A_ARG_TYPE_UpdateID</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
    <action>
      <name>GetSystemUpdateID</name>
      <argumentList>
        <argument>
          <name>Id</name>
          <direction>out</direction>
          <relatedStateVariable>SystemUpdateID</relatedStateVariable>
        </argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="yes">
      <name>SystemUpdateID</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_ObjectID</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_Result</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_BrowseFlag</name>
      <dataType>string</dataType>
      <allowedValueList>
        <allowedValue>BrowseMetadata</allowedValue>
        <allowedValue>BrowseDirectChildren</allowedValue>
      </allowedValueList>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_Filter</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_SortCriteria</name>
      <dataType>string</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_Index</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_Count</name>
      <dataType>ui4</dataType>
    </stateVariable>
    <stateVariable sendEvents="no">
      <name>A_ARG_TYPE_UpdateID</name>
      <dataType>ui4</dataType>
    </stateVariable>
  </serviceStateTable>
</scpd>
```

### Phase 4: Content Browsing (SOAP)

**Step 4a: Client sends Browse request**

```http
POST /cds/control HTTP/1.1
Host: 192.168.1.100:8200
Content-Type: text/xml; charset="utf-8"
SOAPAction: "urn:schemas-upnp-org:service:ContentDirectory:1#Browse"

<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
      <ObjectID>0</ObjectID>
      <BrowseFlag>BrowseDirectChildren</BrowseFlag>
      <Filter>*</Filter>
      <StartingIndex>0</StartingIndex>
      <RequestedCount>100</RequestedCount>
      <SortCriteria></SortCriteria>
    </u:Browse>
  </s:Body>
</s:Envelope>
```

**Browse parameters explained:**
- `ObjectID`: "0" is the root container. For a flat list, all items live under "0".
- `BrowseFlag`: "BrowseDirectChildren" lists children; "BrowseMetadata" returns info about the object itself.
- `Filter`: "*" means return all properties. Clients may request specific properties.
- `StartingIndex` / `RequestedCount`: Pagination. Samsung TVs typically request 100 at a time.
- `SortCriteria`: Empty or like "+dc:title" for sorting.

**Step 4b: Server returns DIDL-Lite in SOAP envelope**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:BrowseResponse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
      <Result>&lt;DIDL-Lite
        xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/"
        xmlns:dc="http://purl.org/dc/elements/1.1/"
        xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/"&gt;
        &lt;item id="1" parentID="0" restricted="true"&gt;
          &lt;dc:title&gt;Movie.mkv&lt;/dc:title&gt;
          &lt;upnp:class&gt;object.item.videoItem&lt;/upnp:class&gt;
          &lt;res protocolInfo="http-get:*:video/x-matroska:*"
               size="1073741824"&gt;
            http://192.168.1.100:8200/media/1
          &lt;/res&gt;
        &lt;/item&gt;
        &lt;item id="2" parentID="0" restricted="true"&gt;
          &lt;dc:title&gt;Song.mp3&lt;/dc:title&gt;
          &lt;upnp:class&gt;object.item.audioItem.musicTrack&lt;/upnp:class&gt;
          &lt;res protocolInfo="http-get:*:audio/mpeg:*"
               size="5242880"&gt;
            http://192.168.1.100:8200/media/2
          &lt;/res&gt;
        &lt;/item&gt;
      &lt;/DIDL-Lite&gt;</Result>
      <NumberReturned>2</NumberReturned>
      <TotalMatches>2</TotalMatches>
      <UpdateID>1</UpdateID>
    </u:BrowseResponse>
  </s:Body>
</s:Envelope>
```

**Critical: The DIDL-Lite XML is XML-escaped inside the `<Result>` element.** This is the single most common implementation mistake -- the inner XML must be entity-encoded (`&lt;`, `&gt;`, `&amp;`) within the SOAP envelope. Failure to do this correctly will break every client.

**DIDL-Lite structure details:**
- `<item>` for playable content, `<container>` for folders (not needed for flat list)
- `id` must be unique and stable per item
- `parentID` is "0" for items in root (flat list model)
- `restricted="true"` means read-only (always true for a media server)
- `<dc:title>` is the display name
- `<upnp:class>` determines how the client categorizes/displays the item
- `<res>` contains the stream URL and protocolInfo (4-field format: protocol:network:mime:additionalInfo)
- `size` attribute on `<res>` is critical for Samsung TVs -- they use it to calculate seek positions

**upnp:class values for media types:**
- Video: `object.item.videoItem`
- Audio: `object.item.audioItem.musicTrack`
- Image: `object.item.imageItem.photo`

### Phase 5: Media Streaming (HTTP)

**Step 5a: Client requests media with Range**

```http
GET /media/1 HTTP/1.1
Host: 192.168.1.100:8200
Range: bytes=0-
```

**Step 5b: Server responds with partial content**

```http
HTTP/1.1 206 Partial Content
Content-Type: video/x-matroska
Content-Length: 1073741824
Content-Range: bytes 0-1073741823/1073741824
Accept-Ranges: bytes
transferMode.dlna.org: Streaming
contentFeatures.dlna.org: DLNA.ORG_OP=01;DLNA.ORG_FLAGS=01700000000000000000000000000000

[binary data]
```

**Critical HTTP headers for DLNA compliance:**
- `Accept-Ranges: bytes` -- MUST be present; clients use this to know seeking is supported
- `Content-Range` -- MUST be correct; Samsung TVs validate this strictly
- `transferMode.dlna.org: Streaming` -- DLNA-specific header; Samsung TVs require it
- `contentFeatures.dlna.org` -- DLNA profile flags; `DLNA.ORG_OP=01` means byte-range seeking supported; the FLAGS value enables streaming mode

### Phase 6: Graceful Shutdown

On Ctrl+C (SIGINT/SIGTERM), the server must send ssdp:byebye notifications:

```http
NOTIFY * HTTP/1.1
HOST: 239.255.255.250:1900
NT: urn:schemas-upnp-org:device:MediaServer:1
NTS: ssdp:byebye
USN: uuid:abcd-1234::urn:schemas-upnp-org:device:MediaServer:1
```

Without this, clients keep the stale server in their device list until the CACHE-CONTROL max-age expires (typically 30 minutes).

## Recommended Project Structure

```
src/
├── main.rs                # CLI parsing, config loading, async runtime bootstrap
├── config.rs              # TOML config + CLI arg merging
├── server.rs              # Shared ServerState, startup/shutdown orchestration
├── ssdp/
│   ├── mod.rs             # Re-exports
│   ├── listener.rs        # Multicast UDP listener for M-SEARCH
│   ├── responder.rs       # Unicast response sender
│   └── advertiser.rs      # Periodic NOTIFY ssdp:alive + byebye on shutdown
├── http/
│   ├── mod.rs             # HTTP server setup + router
│   ├── device.rs          # GET /device.xml handler
│   ├── scpd.rs            # GET /cds/scpd.xml and /cms/scpd.xml handlers
│   ├── soap.rs            # SOAP envelope parsing + response building
│   ├── content_directory.rs  # Browse + GetSystemUpdateID action logic
│   ├── connection_manager.rs # GetProtocolInfo (minimal stub)
│   └── streaming.rs       # Media file serving with Range support
├── media/
│   ├── mod.rs             # Re-exports
│   ├── scanner.rs         # Directory walker, MIME detection
│   └── library.rs         # MediaItem struct, MediaLibrary (flat list)
└── xml/
    ├── mod.rs             # Re-exports
    ├── device_desc.rs     # Device description XML builder
    ├── didl.rs            # DIDL-Lite XML builder for Browse responses
    └── soap.rs            # SOAP envelope builder/parser utilities
```

### Structure Rationale

- **`ssdp/`:** Isolated because SSDP is UDP-based and runs independently of HTTP. It has its own socket, its own async task, and its own protocol format (HTTP-like but not HTTP).
- **`http/`:** All HTTP handlers grouped together. The router dispatches based on path: `/device.xml`, `/cds/*`, `/cms/*`, `/media/*`.
- **`media/`:** Pure data logic with no network concerns. The scanner walks the filesystem; the library holds the result. Easy to test in isolation.
- **`xml/`:** XML generation is complex enough to warrant separation. Keeps DIDL-Lite building, SOAP wrapping, and device description generation out of handler logic.

## Architectural Patterns

### Pattern 1: Shared Immutable State

**What:** Build the media library at startup, wrap in `Arc<ServerState>`, and share a read-only reference with all async tasks. No mutex needed because the library never changes after initialization.

**When to use:** Always, for this project. The media library is scanned once and then served.

**Trade-offs:** Cannot add files without restart. This is acceptable for a CLI tool meant to run on-demand. If live rescan were needed later, swap to `Arc<RwLock<MediaLibrary>>`.

**Example:**
```rust
struct ServerState {
    config: ServerConfig,
    library: MediaLibrary,
    uuid: String,
    system_update_id: u32,
}

// At startup:
let state = Arc::new(ServerState { /* ... */ });
// Clone Arc for each async task:
let ssdp_state = state.clone();
let http_state = state.clone();
```

### Pattern 2: Two Independent Async Tasks

**What:** Run SSDP (UDP) and HTTP (TCP) as two independent async tasks joined at the top level. Both share the same `Arc<ServerState>` but operate on different sockets and protocols.

**When to use:** Always. SSDP and HTTP are fundamentally different protocols on different transports.

**Trade-offs:** Simple and correct. The SSDP task handles multicast UDP; the HTTP task handles TCP connections. They communicate only through shared state (which is read-only).

**Example:**
```rust
tokio::select! {
    result = ssdp::run(state.clone(), shutdown.clone()) => { /* ... */ },
    result = http::serve(state.clone(), shutdown.clone()) => { /* ... */ },
    _ = shutdown_signal() => {
        ssdp::send_byebye(state.clone()).await;
    }
}
```

### Pattern 3: Static XML Templates with Runtime Substitution

**What:** Device description and SCPD XMLs are essentially static documents with a few runtime values (UUID, server name, IP address, port). Use string templates rather than building XML nodes programmatically.

**When to use:** For device.xml, scpd.xml. These are fixed-structure documents.

**Trade-offs:** Less flexible than a full XML builder, but dramatically simpler and faster. The structure of these documents never varies at runtime.

**Example:**
```rust
fn device_xml(state: &ServerState, local_ip: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <device>
    <deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType>
    <friendlyName>{name}</friendlyName>
    <UDN>uuid:{uuid}</UDN>
    ...
  </device>
</root>"#,
        name = state.config.server_name,
        uuid = state.uuid,
    )
}
```

### Pattern 4: DIDL-Lite Builder (Programmatic XML)

**What:** Unlike device.xml, DIDL-Lite content varies per request (different items, pagination). Build this XML programmatically using string concatenation or a lightweight XML writer -- then XML-escape the entire output before embedding in the SOAP response.

**When to use:** For Browse response generation.

**Trade-offs:** A full XML library (like `quick-xml`) adds dependency but guarantees correct escaping. Manual string building is faster but risks escaping bugs. Recommend using `quick-xml` for DIDL-Lite because the content (file names) may contain XML-special characters (`&`, `<`, `>`).

**Example:**
```rust
fn build_didl_lite(items: &[MediaItem], base_url: &str) -> String {
    let mut result = String::from(
        r#"<DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/"
                    xmlns:dc="http://purl.org/dc/elements/1.1/"
                    xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/">"#
    );
    for item in items {
        // Use xml_escape() on title and other user-supplied values
        write!(result, r#"<item id="{id}" parentID="0" restricted="true">
          <dc:title>{title}</dc:title>
          <upnp:class>{class}</upnp:class>
          <res protocolInfo="http-get:*:{mime}:*" size="{size}">{url}/media/{id}</res>
        </item>"#,
            id = item.id,
            title = xml_escape(&item.title),
            class = item.upnp_class(),
            mime = item.mime_type,
            size = item.file_size,
            url = base_url,
        ).unwrap();
    }
    result.push_str("</DIDL-Lite>");
    result
}
```

### Pattern 5: Graceful Shutdown via Cancellation Token

**What:** Use a shared cancellation token (e.g., `tokio_util::sync::CancellationToken` or a simple `tokio::sync::broadcast` channel) to coordinate shutdown across all async tasks.

**When to use:** Always. SSDP must send byebye before the UDP socket closes; HTTP should stop accepting new connections but finish in-flight streams.

**Trade-offs:** Slightly more wiring at startup, but essential for spec compliance and good behavior.

## Data Flow

### Discovery Flow

```
DLNA Client                          udlna
    |                                  |
    |  M-SEARCH (UDP multicast)        |
    |--------------------------------->|
    |                                  |-- Parse M-SEARCH
    |                                  |-- Check ST matches our device type
    |                                  |-- Random delay (0..MX seconds)
    |  200 OK (UDP unicast)            |
    |<---------------------------------|
    |                                  |
    |  GET /device.xml (TCP/HTTP)      |
    |--------------------------------->|
    |  <root> XML                      |
    |<---------------------------------|
    |                                  |
    |  GET /cds/scpd.xml (TCP/HTTP)    |
    |--------------------------------->|
    |  <scpd> XML                      |
    |<---------------------------------|
    |                                  |
```

### Browse Flow

```
DLNA Client                          udlna
    |                                  |
    |  POST /cds/control (SOAP)        |
    |  SOAPAction: ...#Browse          |
    |  ObjectID=0, BrowseDirectChildren|
    |--------------------------------->|
    |                                  |-- Parse SOAP envelope
    |                                  |-- Extract Browse parameters
    |                                  |-- Query MediaLibrary (slice by StartingIndex/RequestedCount)
    |                                  |-- Build DIDL-Lite XML
    |                                  |-- XML-escape DIDL-Lite
    |                                  |-- Wrap in SOAP BrowseResponse
    |  SOAP BrowseResponse             |
    |<---------------------------------|
    |                                  |
```

### Streaming Flow

```
DLNA Client                          udlna
    |                                  |
    |  GET /media/42 (HTTP)            |
    |  Range: bytes=1048576-           |
    |--------------------------------->|
    |                                  |-- Look up item 42 in MediaLibrary
    |                                  |-- Open file, seek to offset
    |                                  |-- Calculate Content-Range
    |  206 Partial Content             |
    |  Content-Range: bytes 1048576-...|
    |  transferMode.dlna.org: Streaming|
    |<---------------------------------|
    |  [streaming bytes...]            |
    |<---------------------------------|
    |                                  |
```

## Network Architecture

### Ports and Protocols

| Protocol | Transport | Address | Port | Purpose |
|----------|-----------|---------|------|---------|
| SSDP | UDP multicast | 239.255.255.250 | 1900 | Discovery (M-SEARCH listen + NOTIFY send) |
| SSDP | UDP unicast | Client's IP | Client's port | M-SEARCH response (sent back to requester) |
| HTTP | TCP | 0.0.0.0 (all interfaces) | Configurable (default 8200) | Device XML, SOAP, media streaming |

### Multicast Considerations

- The server must join the multicast group 239.255.255.250 on the correct network interface
- On multi-homed systems, the server should bind to a specific interface or auto-detect the LAN interface
- The LOCATION URL in SSDP responses must use the server's actual LAN IP (not 0.0.0.0 or 127.0.0.1)
- Auto-detecting the LAN IP: enumerate interfaces, pick the first non-loopback IPv4 address, or let the user override via config

### Required SSDP Notifications

The server must respond to M-SEARCH for these search targets (ST) and also send periodic NOTIFY for each:

| ST / NT value | Description |
|---------------|-------------|
| `upnp:rootdevice` | Generic root device |
| `uuid:{device-uuid}` | This specific device |
| `urn:schemas-upnp-org:device:MediaServer:1` | Media server device type |
| `urn:schemas-upnp-org:service:ContentDirectory:1` | ContentDirectory service |
| `urn:schemas-upnp-org:service:ConnectionManager:1` | ConnectionManager service |

Each NOTIFY cycle must advertise all of these (3 notifications per cycle minimum, typically). Samsung TVs are known to look for the specific MediaServer:1 device type.

## Build Order and Dependencies

The components have clear dependencies that dictate build order:

```
Phase 1: Foundation
  ├── config.rs (CLI + TOML parsing)
  ├── media/scanner.rs + media/library.rs (directory walking, MediaItem)
  └── server.rs (ServerState struct)

Phase 2: HTTP Core
  ├── http/mod.rs (TCP listener + router)
  ├── http/device.rs (device description XML)
  ├── http/scpd.rs (service description XML)
  └── http/streaming.rs (file serving with Range support)
      [Testable: can fetch device.xml and stream files via curl]

Phase 3: ContentDirectory
  ├── xml/soap.rs (SOAP parsing/building)
  ├── xml/didl.rs (DIDL-Lite builder)
  ├── http/soap.rs (SOAP request routing)
  └── http/content_directory.rs (Browse + GetSystemUpdateID)
      [Testable: can POST SOAP Browse and get DIDL-Lite back via curl]

Phase 4: SSDP Discovery
  ├── ssdp/listener.rs (multicast join + M-SEARCH parsing)
  ├── ssdp/responder.rs (unicast response)
  └── ssdp/advertiser.rs (periodic NOTIFY + byebye)
      [Testable: real DLNA clients can now discover and browse the server]

Phase 5: ConnectionManager + DLNA Headers
  ├── http/connection_manager.rs (GetProtocolInfo)
  └── DLNA-specific HTTP headers on streaming responses
      [Testable: Samsung TV and Xbox can discover, browse, and play media]

Phase 6: Polish
  ├── Graceful shutdown (signal handling + byebye)
  ├── Persistent UUID (generate once, save to config)
  └── Error handling, logging, edge cases
```

**Why this order:**

1. **Foundation first** because every other component depends on config and the media library.
2. **HTTP before SSDP** because you can test HTTP endpoints manually with curl/wget before adding discovery. SSDP is harder to test and debug.
3. **Streaming before ContentDirectory** because streaming is simpler (no SOAP/XML) and gives you immediate feedback -- you can play a file in a browser.
4. **ContentDirectory before SSDP** because you need the full Browse flow working before a real DLNA client tries to use it. If SSDP works but Browse is broken, clients fail silently and debugging is painful.
5. **SSDP last among core components** because it ties everything together. Once SSDP works, real clients can discover the server, so everything it points to must be ready.
6. **ConnectionManager and DLNA headers** come after basic functionality works, because they are only needed for strict clients (Samsung, Xbox). Lenient clients (VLC, some Android apps) work without them.

## Anti-Patterns

### Anti-Pattern 1: Building the DIDL-Lite Without Escaping

**What people do:** Concatenate raw file names into XML strings without escaping.
**Why it's wrong:** A file named `Tom & Jerry.mkv` produces invalid XML (`&` must be `&amp;`). Samsung TVs will silently fail to parse the response and show an empty list.
**Do this instead:** Always XML-escape user-derived content (file names, paths) in DIDL-Lite. Use a library like `quick-xml` or write a simple escape function for `&`, `<`, `>`, `"`, `'`.

### Anti-Pattern 2: Changing UUID Between Runs

**What people do:** Generate a new random UUID every time the server starts.
**Why it's wrong:** Clients see a "new" device each time. Samsung TVs accumulate stale entries in their source list. Xbox may cache state from the old UUID and get confused.
**Do this instead:** Generate the UUID once on first run, persist it (in the TOML config or a separate state file), and reuse it on subsequent runs.

### Anti-Pattern 3: Returning 200 OK Instead of 206 for Range Requests

**What people do:** Ignore the Range header and return the full file with status 200.
**Why it's wrong:** Media players send Range requests to seek. If the server returns 200, the player either downloads from the beginning (destroying seek) or errors out. Samsung TVs in particular will refuse to play content that doesn't properly support Range.
**Do this instead:** Parse the Range header, seek to the correct offset, return 206 Partial Content with correct Content-Range header. If no Range header, return 200 with the full file.

### Anti-Pattern 4: Using 0.0.0.0 or 127.0.0.1 in SSDP LOCATION

**What people do:** Use the bind address (0.0.0.0) or localhost (127.0.0.1) in the LOCATION header of SSDP responses.
**Why it's wrong:** The client uses this URL to fetch device.xml over TCP. If the URL contains 0.0.0.0 or 127.0.0.1, the client tries to connect to itself. Discovery appears to work but device.xml fetch fails.
**Do this instead:** Detect the server's actual LAN IP address and use it in all LOCATION URLs. Also use the same IP as the base URL in DIDL-Lite `<res>` elements.

### Anti-Pattern 5: Omitting ConnectionManager Service

**What people do:** Implement only ContentDirectory since it is the "interesting" service.
**Why it's wrong:** The DLNA spec requires both ContentDirectory and ConnectionManager. Samsung TVs will check the device description for both services and refuse to interact if ConnectionManager is missing.
**Do this instead:** Implement a minimal ConnectionManager that supports only GetProtocolInfo (returns a list of supported MIME types). PrepareForConnection and ConnectionComplete can return SOAP faults or simply not be listed in the SCPD.

### Anti-Pattern 6: Forgetting BrowseMetadata for ObjectID "0"

**What people do:** Only implement BrowseDirectChildren and forget that clients also send BrowseMetadata for the root container.
**Why it's wrong:** Some clients (including Xbox) send BrowseMetadata for ObjectID "0" to get information about the root container before browsing its children. If this returns an error, the client gives up.
**Do this instead:** Handle BrowseMetadata for ObjectID "0" by returning a `<container>` element describing the root with `childCount` equal to the total number of items.

## Integration Points

### External Services

This server has no external service dependencies. It is entirely self-contained on the local network.

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| SSDP <-> Shared State | Read-only access to UUID, config, and base URL | SSDP needs server name, UUID, and the HTTP URL for LOCATION |
| HTTP Router <-> Handlers | Function dispatch based on path | Router matches path prefix, hands request + state to handler |
| ContentDirectory <-> MediaLibrary | Read-only query: list items, get item by ID | Pagination via slice; lookup by ID via HashMap or linear scan |
| Streaming <-> Filesystem | Async file I/O | Open file by path from MediaItem, seek to Range offset, stream bytes |
| DIDL Builder <-> MediaLibrary | Iterates over MediaItems to build XML | Pure function: items in, XML string out |

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 1-100 files | Flat `Vec<MediaItem>` is fine. Linear scan for ID lookup. Everything in memory. |
| 100-10,000 files | Add a `HashMap<String, usize>` for O(1) ID lookup. Still flat list. Pagination in Browse becomes important. |
| 10,000+ files | Consider adding container hierarchy (virtual folders by media type). Startup scan may take noticeable time -- add a progress indicator. |

### Scaling Priorities

1. **First bottleneck:** Concurrent streaming of large files. Async I/O handles this well, but ensure buffer sizes are reasonable (64KB-256KB chunks). Do not load entire files into memory.
2. **Second bottleneck:** SSDP response storms on large networks. The random delay (0..MX) in M-SEARCH responses handles this by spec, but ensure the implementation actually delays rather than responding immediately.

## Sources

- UPnP Device Architecture 1.0 specification (UPnP Forum / Open Connectivity Foundation)
- UPnP AV ContentDirectory:1 Service Template (UPnP Forum)
- UPnP AV ConnectionManager:1 Service Template (UPnP Forum)
- DLNA Guidelines (DLNA organization, dissolved 2017; guidelines frozen)
- SSDP (IETF draft-cai-ssdp-v1-03, implemented as described in UPnP Device Architecture)
- RFC 7233: HTTP Range Requests
- Implementation patterns observed across minidlna, ReadyMedia, gmrender-resurrect, and similar minimal DLNA implementations

**Confidence note:** WebSearch and WebFetch were unavailable during this research. All protocol details are from training data. However, UPnP/DLNA are frozen specifications that have not changed since before the training cutoff. The multicast address, port numbers, XML formats, SOAP structures, and DIDL-Lite schema are stable facts. Confidence: HIGH for protocol specifics, MEDIUM for Samsung TV / Xbox behavioral quirks (based on community reports in training data, not verified against current firmware).

---
*Architecture research for: Minimal DLNA/UPnP Media Server (udlna)*
*Researched: 2026-02-22*
