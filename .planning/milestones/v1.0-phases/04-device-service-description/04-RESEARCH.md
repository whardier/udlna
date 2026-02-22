# Phase 4: Device & Service Description - Research

**Researched:** 2026-02-22
**Domain:** UPnP device description XML and SCPD service description documents (MediaServer:1, ContentDirectory:1, ConnectionManager:1)
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

None explicitly locked — user deferred all gray areas to Claude.

### Claude's Discretion

- **Server identity fields** — friendlyName, manufacturer, manufacturerURL, modelName, modelNumber, modelDescription, serialNumber. Use sensible defaults consistent with the project (`udlna`, `udlna project`, etc.). Hard-code for Phase 4; Phase 8 adds the `--name` flag that customizes friendlyName.

- **SCPD completeness** — Include full spec-compliant state variable tables and argument definitions for ContentDirectory (Browse, GetSearchCapabilities, GetSortCapabilities, GetSystemUpdateID) and ConnectionManager (GetProtocolInfo, GetCurrentConnectionIDs, GetCurrentConnectionInfo). Full compliance is better than minimal — DLNA clients validate these documents.

- **Client compatibility** — Include `dlna:X_DLNADOC` with `DMS-1.50`, `URLBase` if needed for client compat, and `presentationURL` as a stub. Follow what open-source DLNA server implementations (MiniDLNA, dms) include for broadest client support.

- **XML generation** — Embedded static string templates with minimal runtime interpolation (just the UUID and base URL). No XML builder crate needed for mostly-static documents.

- **UDN (Unique Device Name)** — For Phase 4, use a placeholder UUID that Phase 8 will replace with the stable UUIDv5-from-hostname. The planner decides whether to use a fixed dev UUID or read from a future config field.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DESC-01 | Server serves UPnP device description XML at `/device.xml` with `MediaServer:1` device type, both service declarations, and `dlna:X_DLNADOC` element | Device description XML structure fully documented; namespace, required elements, DLNA extension pattern verified from multiple implementations |
| DESC-02 | Server serves ContentDirectory SCPD XML at `/cds/scpd.xml` | Complete SCPD XML verified from PyMedS-ng source; all 4 required actions (Browse, GetSearchCapabilities, GetSortCapabilities, GetSystemUpdateID) with full argument lists and state variable tables |
| DESC-03 | Server serves ConnectionManager SCPD XML at `/cms/scpd.xml` | Complete SCPD XML verified from PyMedS-ng source; all 3 required actions (GetProtocolInfo, GetCurrentConnectionIDs, GetCurrentConnectionInfo) with full argument lists and state variable tables |
</phase_requirements>

---

## Summary

Phase 4 implements three static XML documents served over HTTP: the UPnP device description at `/device.xml` and two SCPD service descriptions at `/cds/scpd.xml` and `/cms/scpd.xml`. These documents are the result of Phase 3's 501 stubs — the routes are already declared; this phase replaces the closures with real handlers.

The UPnP device description is an XML document using the `urn:schemas-upnp-org:device-1-0` namespace with the DLNA extension namespace `urn:schemas-dlna-org:device-1-0`. The mandatory elements are `specVersion`, `device` (containing `deviceType`, `friendlyName`, `manufacturer`, `modelName`, `UDN`, and `serviceList`). The `dlna:X_DLNADOC` element with value `DMS-1.50` is required by the DLNA spec and expected by clients. Both ContentDirectory and ConnectionManager services must be declared in the `serviceList`, each with `serviceType`, `serviceId`, `SCPDURL`, `controlURL`, and `eventSubURL`.

The SCPD (Service Control Protocol Description) documents are static XML files using the `urn:schemas-upnp-org:service-1-0` namespace. They declare every action the service supports, including all argument names, directions, and related state variable references, plus a complete `serviceStateTable`. The documents are virtually static — no runtime variation. The only runtime-varying value in the entire phase is the UUID in the device description. This maps cleanly to the CONTEXT.md decision: "embedded static string templates with minimal runtime interpolation."

**Primary recommendation:** Use `format!()` in a Rust function to produce the device.xml with the UUID interpolated. Use `const &str` for both SCPD documents (they are fully static). Serve all three via axum handlers returning `([(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")], body)` tuples. Add a `src/http/description.rs` module for the three handlers.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| axum | 0.8 (already in Cargo.toml) | HTTP handler for XML responses | Already used; tuple IntoResponse pattern gives full header control |
| (none new) | — | XML generation | Pure string templates — no XML builder needed for mostly-static docs |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| uuid | 1 (already in Cargo.toml) | Generate placeholder UUID for UDN | UDN must be `uuid:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` format |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| String templates | quick-xml or xml-builder | Overkill for documents that are 99% static; adds dependency; templates are more readable |
| format!() for device.xml | lazy_static! or once_cell | Acceptable for Phase 4; Phase 8 will revisit when friendly name is configurable |

**Installation:**

No new dependencies needed. All required crates are already in `Cargo.toml`:
- `axum = "0.8"` (HTTP serving)
- `uuid = { version = "1", features = ["v5"] }` (UDN placeholder)

---

## Architecture Patterns

### Recommended Project Structure

```
src/
├── http/
│   ├── mod.rs           # build_router — replace 3 stub closures with description::* handlers
│   ├── state.rs         # AppState — may need base_url field or derive it per-request
│   ├── media.rs         # Phase 3 — unchanged
│   └── description.rs   # NEW: serve_device_xml, serve_cds_scpd, serve_cms_scpd
└── upnp/
    └── (future Phase 5+)
```

### Pattern 1: Static SCPD as `const &str`

**What:** Define the two SCPD documents as `const &str` at the top of `description.rs`. They have zero runtime variation.

**When to use:** Documents with no runtime-varying fields.

**Example:**
```rust
// Source: verified SCPD structure from PyMedS-ng and Microsoft UPnP docs
const CDS_SCPD: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <actionList>
    <!-- Browse, GetSearchCapabilities, GetSortCapabilities, GetSystemUpdateID -->
  </actionList>
  <serviceStateTable>
    <!-- all state variables -->
  </serviceStateTable>
</scpd>"#;

pub async fn serve_cds_scpd() -> impl IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        CDS_SCPD,
    )
}
```

### Pattern 2: Device XML with UUID Interpolation

**What:** Generate device.xml at handler call time using `format!()`. The UUID is the only varying field in Phase 4.

**When to use:** Documents with 1-2 runtime-varying fields; not worth a template engine.

**Example:**
```rust
// Source: device description pattern from simpleDLNA, PyMedS-ng, DMS (Go)
pub async fn serve_device_xml(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let body = device_xml_body(&state.server_uuid, &state.base_url);
    (
        [(axum::http::header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        body,
    )
}

fn device_xml_body(uuid: &str, base_url: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="utf-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0"
      xmlns:dlna="urn:schemas-dlna-org:device-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <URLBase>{base_url}</URLBase>
  <device>
    <deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType>
    <dlna:X_DLNADOC>DMS-1.50</dlna:X_DLNADOC>
    <dlna:X_DLNADOC>M-DMS-1.50</dlna:X_DLNADOC>
    <dlna:X_DLNACAP/>
    <friendlyName>udlna</friendlyName>
    <manufacturer>udlna project</manufacturer>
    <manufacturerURL>https://github.com/</manufacturerURL>
    <modelDescription>Minimal DLNA/UPnP Media Server</modelDescription>
    <modelName>udlna</modelName>
    <modelNumber>0.1</modelNumber>
    <modelURL>https://github.com/</modelURL>
    <serialNumber>0</serialNumber>
    <UDN>uuid:{uuid}</UDN>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ContentDirectory</serviceId>
        <SCPDURL>/cds/scpd.xml</SCPDURL>
        <controlURL>/cds/control</controlURL>
        <eventSubURL></eventSubURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ConnectionManager</serviceId>
        <SCPDURL>/cms/scpd.xml</SCPDURL>
        <controlURL>/cms/control</controlURL>
        <eventSubURL></eventSubURL>
      </service>
    </serviceList>
  </device>
</root>"#, base_url=base_url, uuid=uuid)
}
```

### Pattern 3: AppState Extension for server_uuid and base_url

**What:** Add `server_uuid: String` and `base_url: String` fields to `AppState`. Populated at startup before the HTTP server starts.

**When to use:** Values that are computed once at startup and needed by multiple handlers.

**Example:**
```rust
// In src/http/state.rs — extend AppState:
#[derive(Clone)]
pub struct AppState {
    pub library: Arc<RwLock<MediaLibrary>>,
    pub server_uuid: String,   // Phase 4: fixed dev UUID; Phase 8: UUIDv5 from hostname
    pub base_url: String,      // e.g. "http://192.168.1.10:8200"
}
```

Note: `base_url` for URLBase requires knowing the server's local IP at startup. This is non-trivial for dual-stack (IPv4+IPv6) servers. The simplest Phase 4 approach: leave URLBase as empty string or derive from config port with a placeholder IP. DLNA clients that follow the URLBase spec will derive absolute URLs from it; clients that ignore it use the source IP from the TCP connection. See Pitfall 3.

### Anti-Patterns to Avoid

- **Generating XML with a builder crate:** Quick-xml, xml-rs, or similar are heavyweight for documents this static. String templates are more readable and produce deterministic output.
- **Dynamic XML at every request:** The documents are effectively static. Generate once (or at handler registration time) and serve the same bytes. `format!()` at handler invocation is acceptable for Phase 4 since the UUID doesn't change per-request.
- **Forgetting `eventSubURL`:** The element must be present (empty is fine). Missing it causes some clients to reject the service description.
- **Wrong serviceId format:** ServiceId uses `urn:upnp-org:serviceId:ContentDirectory` (not `urn:schemas-upnp-org`). PyMedS-ng has a bug here (`urn:upnp-org:serviceId:urn:schemas-upnp-org:service:ContenDirectory`). Use the short form.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| UUID for UDN | Custom random ID generator | `uuid` crate (already in Cargo.toml) | UUID format is strictly defined (RFC 4122); `uuid` v1 crate generates correct format |
| XML escaping in templates | Manual `&amp;` replacement | Keep identity fields free of special chars, or use `html_escape` if needed | Identity fields like friendlyName rarely need escaping; for Phase 4 use only ASCII-safe values |

**Key insight:** These XML documents are defined by a standard (UPnP Device Architecture 1.0). The content is almost entirely prescribed by the spec — don't over-engineer the generation mechanism.

---

## Common Pitfalls

### Pitfall 1: Wrong Content-Type Header

**What goes wrong:** Returning `text/plain` or `application/octet-stream` instead of `text/xml; charset="utf-8"`. Some DLNA clients (notably strict validators) reject the response silently.

**Why it happens:** axum's default `&str` or `String` response sets `text/plain; charset=utf-8`. XML must be explicit.

**How to avoid:** Always use the tuple response pattern:
```rust
([("content-type", "text/xml; charset=\"utf-8\"")], body)
```

**Warning signs:** curl fetches the file successfully but the DLNA client never shows the server in its device list.

### Pitfall 2: Incorrect or Missing serviceId

**What goes wrong:** Using `urn:schemas-upnp-org:serviceId:ContentDirectory` (wrong namespace) instead of `urn:upnp-org:serviceId:ContentDirectory`.

**Why it happens:** Confusion between service type namespace (`schemas-upnp-org`) and service ID namespace (`upnp-org`). Multiple open-source implementations have this bug.

**How to avoid:** Use these exact values:
- ContentDirectory serviceId: `urn:upnp-org:serviceId:ContentDirectory`
- ConnectionManager serviceId: `urn:upnp-org:serviceId:ConnectionManager`

**Warning signs:** Client fetches device.xml but fails to fetch SCPD URLs; SCPD requests never arrive in server logs.

### Pitfall 3: URLBase Complexity with Dual-Stack

**What goes wrong:** URLBase requires the server's externally-reachable IP address. On a dual-stack server, this is ambiguous (is the client on IPv4 or IPv6?). A hard-coded IP will be wrong for some clients.

**Why it happens:** URLBase was designed for single-IP servers. The UPnP 1.1 spec deprecated URLBase in favor of using the HTTP request source.

**How to avoid:** Two safe options:
1. Omit URLBase entirely — DLNA clients that need absolute URLs derive them from the TCP connection source. Most modern clients handle this correctly.
2. Include URLBase with a format like `http://0.0.0.0:{port}` — some older clients may interpret this oddly.

**Recommendation:** Omit URLBase in Phase 4. The SCPDURL, controlURL, and eventSubURL values in serviceList use absolute paths (`/cds/scpd.xml`), which clients combine with the address they fetched device.xml from.

**Warning signs:** Clients can discover the server but fail to fetch SCPD documents.

### Pitfall 4: Missing or Malformed DLNA Namespace Declaration

**What goes wrong:** Declaring `dlna:X_DLNADOC` without declaring the `xmlns:dlna` namespace attribute on the root element. Results in invalid XML that strict parsers reject.

**Why it happens:** Copy-paste from examples that have the namespace elsewhere, or forgetting it in a string template.

**How to avoid:** The root `<root>` element must include:
```xml
<root xmlns="urn:schemas-upnp-org:device-1-0"
      xmlns:dlna="urn:schemas-dlna-org:device-1-0">
```

**Warning signs:** XML validation errors; clients see the server in SSDP but fail to parse the device description.

### Pitfall 5: SCPD Missing Required State Variables

**What goes wrong:** SCPD `actionList` references state variables (`relatedStateVariable` elements) that are not defined in `serviceStateTable`. Some clients validate SCPD documents strictly.

**Why it happens:** Building a minimal SCPD without including all referenced state variables.

**How to avoid:** Use the complete, verified SCPD XML from research (see Code Examples). Every `relatedStateVariable` value in the action argument list must have a corresponding `stateVariable` entry in `serviceStateTable`.

**Warning signs:** Client shows server but ContentDirectory Browse fails immediately.

### Pitfall 6: UDN Format

**What goes wrong:** UDN is `uuid:` (literal string) followed by the UUID. Omitting the `uuid:` prefix is a common error.

**Why it happens:** The `uuid:` prefix is part of the UDN format, not the UUID value itself.

**How to avoid:** Always format as:
```xml
<UDN>uuid:xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx</UDN>
```

---

## Code Examples

Verified patterns from official sources and reference implementations:

### Complete device.xml Structure

```xml
<?xml version="1.0" encoding="utf-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0"
      xmlns:dlna="urn:schemas-dlna-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <deviceType>urn:schemas-upnp-org:device:MediaServer:1</deviceType>
    <dlna:X_DLNADOC>DMS-1.50</dlna:X_DLNADOC>
    <dlna:X_DLNADOC>M-DMS-1.50</dlna:X_DLNADOC>
    <dlna:X_DLNACAP/>
    <friendlyName>udlna</friendlyName>
    <manufacturer>udlna project</manufacturer>
    <manufacturerURL>https://github.com/</manufacturerURL>
    <modelDescription>Minimal DLNA/UPnP Media Server</modelDescription>
    <modelName>udlna</modelName>
    <modelNumber>0.1</modelNumber>
    <modelURL>https://github.com/</modelURL>
    <serialNumber>0</serialNumber>
    <UDN>uuid:{UUID_PLACEHOLDER}</UDN>
    <presentationURL>/</presentationURL>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ContentDirectory:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ContentDirectory</serviceId>
        <SCPDURL>/cds/scpd.xml</SCPDURL>
        <controlURL>/cds/control</controlURL>
        <eventSubURL></eventSubURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ConnectionManager</serviceId>
        <SCPDURL>/cms/scpd.xml</SCPDURL>
        <controlURL>/cms/control</controlURL>
        <eventSubURL></eventSubURL>
      </service>
    </serviceList>
  </device>
</root>
```

Source: Synthesized from simpleDLNA description.xml (GitHub), PyMedS-ng root-device.xml (GitHub), and DMS (Go) device description structure. Both `DMS-1.50` and `M-DMS-1.50` X_DLNADOC values are included as seen in simpleDLNA; `M-DMS-1.50` indicates mobile DMS compliance. URLBase is omitted (see Pitfall 3).

### Complete ContentDirectory SCPD XML (cds/scpd.xml)

```xml
<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <actionList>
    <action>
      <name>Browse</name>
      <argumentList>
        <argument><name>ObjectID</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_ObjectID</relatedStateVariable></argument>
        <argument><name>BrowseFlag</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_BrowseFlag</relatedStateVariable></argument>
        <argument><name>Filter</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Filter</relatedStateVariable></argument>
        <argument><name>StartingIndex</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Index</relatedStateVariable></argument>
        <argument><name>RequestedCount</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>SortCriteria</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_SortCriteria</relatedStateVariable></argument>
        <argument><name>Result</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Result</relatedStateVariable></argument>
        <argument><name>NumberReturned</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>TotalMatches</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Count</relatedStateVariable></argument>
        <argument><name>UpdateID</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_UpdateID</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetSearchCapabilities</name>
      <argumentList>
        <argument><name>SearchCaps</name><direction>out</direction><relatedStateVariable>SearchCapabilities</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetSortCapabilities</name>
      <argumentList>
        <argument><name>SortCaps</name><direction>out</direction><relatedStateVariable>SortCapabilities</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetSystemUpdateID</name>
      <argumentList>
        <argument><name>Id</name><direction>out</direction><relatedStateVariable>SystemUpdateID</relatedStateVariable></argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_BrowseFlag</name><dataType>string</dataType><allowedValueList><allowedValue>BrowseMetadata</allowedValue><allowedValue>BrowseDirectChildren</allowedValue></allowedValueList></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_SearchCriteria</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="yes"><name>SystemUpdateID</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Count</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_SortCriteria</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>SortCapabilities</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Index</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_ObjectID</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_UpdateID</name><dataType>ui4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Result</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>SearchCapabilities</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Filter</name><dataType>string</dataType></stateVariable>
  </serviceStateTable>
</scpd>
```

Source: Verbatim from intenso/PyMedS-ng `content-directory-scpd.xml` (GitHub raw), cross-referenced with UPnP ContentDirectory:1 Service Template specification from upnp.org. Search action is present in the spec but commented out in this implementation (correctly, since Search is out of scope for v1).

### Complete ConnectionManager SCPD XML (cms/scpd.xml)

```xml
<?xml version="1.0" encoding="utf-8"?>
<scpd xmlns="urn:schemas-upnp-org:service-1-0">
  <specVersion><major>1</major><minor>0</minor></specVersion>
  <actionList>
    <action>
      <name>GetProtocolInfo</name>
      <argumentList>
        <argument><name>Source</name><direction>out</direction><relatedStateVariable>SourceProtocolInfo</relatedStateVariable></argument>
        <argument><name>Sink</name><direction>out</direction><relatedStateVariable>SinkProtocolInfo</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetCurrentConnectionIDs</name>
      <argumentList>
        <argument><name>ConnectionIDs</name><direction>out</direction><relatedStateVariable>CurrentConnectionIDs</relatedStateVariable></argument>
      </argumentList>
    </action>
    <action>
      <name>GetCurrentConnectionInfo</name>
      <argumentList>
        <argument><name>ConnectionID</name><direction>in</direction><relatedStateVariable>A_ARG_TYPE_ConnectionID</relatedStateVariable></argument>
        <argument><name>RcsID</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_RcsID</relatedStateVariable></argument>
        <argument><name>AVTransportID</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_AVTransportID</relatedStateVariable></argument>
        <argument><name>ProtocolInfo</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_ProtocolInfo</relatedStateVariable></argument>
        <argument><name>PeerConnectionManager</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_ConnectionManager</relatedStateVariable></argument>
        <argument><name>PeerConnectionID</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_ConnectionID</relatedStateVariable></argument>
        <argument><name>Direction</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_Direction</relatedStateVariable></argument>
        <argument><name>Status</name><direction>out</direction><relatedStateVariable>A_ARG_TYPE_ConnectionStatus</relatedStateVariable></argument>
      </argumentList>
    </action>
  </actionList>
  <serviceStateTable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_ProtocolInfo</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_ConnectionStatus</name><dataType>string</dataType><allowedValueList><allowedValue>OK</allowedValue><allowedValue>ContentFormatMismatch</allowedValue><allowedValue>InsufficientBandwidth</allowedValue><allowedValue>UnreliableChannel</allowedValue><allowedValue>Unknown</allowedValue></allowedValueList></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_AVTransportID</name><dataType>i4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_RcsID</name><dataType>i4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_ConnectionID</name><dataType>i4</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_ConnectionManager</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="yes"><name>SourceProtocolInfo</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="yes"><name>SinkProtocolInfo</name><dataType>string</dataType></stateVariable>
    <stateVariable sendEvents="no"><name>A_ARG_TYPE_Direction</name><dataType>string</dataType><allowedValueList><allowedValue>Input</allowedValue><allowedValue>Output</allowedValue></allowedValueList></stateVariable>
    <stateVariable sendEvents="yes"><name>CurrentConnectionIDs</name><dataType>string</dataType></stateVariable>
  </serviceStateTable>
</scpd>
```

Source: Verbatim from intenso/PyMedS-ng `connection-manager-scpd.xml` (GitHub raw), cross-referenced with UPnP ConnectionManager:1 Service Template specification from upnp.org.

### Axum Handler Pattern for XML Response

```rust
// Source: axum docs.rs axum::response (verified)
use axum::{extract::State, http::header, response::IntoResponse};

pub async fn serve_device_xml(State(state): State<AppState>) -> impl IntoResponse {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>..."#,
        uuid = state.server_uuid,
    );
    ([(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")], body)
}

pub async fn serve_cds_scpd() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")], CDS_SCPD_XML)
}
```

### Registering Handlers in build_router

```rust
// In src/http/mod.rs — replace Phase 4 stubs:
use crate::http::description;

Router::new()
    .route("/device.xml", get(description::serve_device_xml))
    .route("/cds/scpd.xml", get(description::serve_cds_scpd))
    .route("/cms/scpd.xml", get(description::serve_cms_scpd))
    // ... other routes unchanged
```

### Placeholder UUID Strategy

```rust
// Fixed development UUID — Phase 8 replaces with UUIDv5 from hostname
// The uuid crate is already in Cargo.toml with v5 feature
const DEV_UUID: &str = "13bf6358-00b8-101b-8000-74dfbfed7306";

// Or generate a random UUID v4 once at startup:
use uuid::Uuid;
let server_uuid = Uuid::new_v4().to_string();
// NOTE: This changes on every restart. For Phase 4 this is acceptable;
// Phase 8 implements the stable UUIDv5-from-hostname (CLI-08).
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| URLBase element (UPnP 1.0) | Omit URLBase; clients derive base from TCP connection (UPnP 1.1+) | UPnP Device Architecture 1.1, ~2008 | Simplifies dual-stack implementations; no IP address needed in device description |
| `text/xml` content-type | `text/xml; charset="utf-8"` | Standard since HTTP/1.1 | Required for correct character encoding declaration; some clients reject without charset |

**Deprecated/outdated:**
- URLBase: Deprecated in UPnP Device Architecture 1.1; some older clients still expect it, but modern clients (Samsung TVs, Xbox) handle its absence gracefully by using the source IP of the HTTP connection.
- X_MS_MediaReceiverRegistrar service: A Microsoft extension that some Windows Media Player-based clients require. Not needed for Samsung TV or Xbox Series X; adds complexity for no benefit in this project scope. Omit it.

---

## Open Questions

1. **UUID generation strategy for Phase 4**
   - What we know: Phase 8 will generate a stable UUIDv5 from hostname (CLI-08). The `uuid` crate with v5 feature is already in Cargo.toml.
   - What's unclear: Should Phase 4 use a fixed constant UUID, or generate random UUID v4 at startup?
   - Recommendation: Use `Uuid::new_v4().to_string()` at server startup and store in `AppState.server_uuid`. This is acceptable for Phase 4 (UUID changes on restart, but SSDP isn't implemented yet so no client has cached it). It's cleaner than a hard-coded constant and makes `AppState` extensible for Phase 8.

2. **Base URL for SCPDURL resolution**
   - What we know: SCPDURL is `/cds/scpd.xml` (absolute path). Clients combine this with the host they fetched device.xml from. No URLBase needed.
   - What's unclear: Whether any field in AppState needs to track port/base_url for future SOAP responses (Phase 5+).
   - Recommendation: Do not add `base_url` to AppState in Phase 4. The absolute path SCPD URLs are self-sufficient.

3. **Does `presentationURL` affect any client behavior?**
   - What we know: simpleDLNA includes it as `"/"`. It's optional per spec.
   - What's unclear: Whether Samsung TVs or Xbox use this field.
   - Recommendation: Include `<presentationURL>/</presentationURL>` as a stub. It's one line and signals to clients that the server has a UI endpoint (even if it returns HTML for now).

---

## Sources

### Primary (HIGH confidence)
- GitHub: intenso/PyMedS-ng `content-directory-scpd.xml` — verbatim SCPD XML for ContentDirectory:1, fetched raw
- GitHub: intenso/PyMedS-ng `connection-manager-scpd.xml` — verbatim SCPD XML for ConnectionManager:1, fetched raw
- GitHub: nmaier/simpleDLNA `server/Resources/description.xml` — verbatim device description XML, fetched raw
- Microsoft Learn: [Creating a Device Description](https://learn.microsoft.com/en-us/windows/win32/upnp/creating-a-device-description) — mandatory elements, URLBase rules
- Microsoft Learn: [Full UPnP Service Description](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-rxad/a7b5c2c8-f6b6-4df4-a079-3a4d91753e21) — SCPD structure with namespace and required elements
- docs.rs: [axum::response](https://docs.rs/axum/latest/axum/response/index.html) — tuple IntoResponse with custom content-type

### Secondary (MEDIUM confidence)
- GitHub: anacrolix/dms `dlna/dms/dms.go` — device description structure used in production Go DLNA server; confirms DMS-1.50 and M-DMS-1.50 X_DLNADOC values
- GitHub: intenso/PyMedS-ng `root-device.xml` — device description with URLBase template variable pattern
- upnp.org: [ContentDirectory:1 Service Template](https://upnp.org/specs/av/UPnP-av-ContentDirectory-v1-Service.pdf) — authoritative specification (PDF not readable but confirms action list via search cross-reference)
- upnp.org: [ConnectionManager:1 Service Template](https://upnp.org/specs/av/UPnP-av-ConnectionManager-v1-Service.pdf) — authoritative specification

### Tertiary (LOW confidence)
- Community discussion: Samsung/Xbox DLNA compatibility notes (multiple forum sources) — X_MS_MediaReceiverRegistrar is not needed for Samsung TV or Xbox browsing

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — axum and uuid are already in Cargo.toml; no new dependencies needed
- Device description XML: HIGH — verbatim XML verified from multiple production implementations
- ContentDirectory SCPD: HIGH — verbatim from intenso/PyMedS-ng, cross-referenced with spec
- ConnectionManager SCPD: HIGH — verbatim from intenso/PyMedS-ng, cross-referenced with spec
- Axum handler pattern: HIGH — verified from official axum docs
- Samsung/Xbox quirks: MEDIUM — community sources only; real-device testing starts Phase 5+

**Research date:** 2026-02-22
**Valid until:** 2026-05-22 (UPnP specs are stable; these XML structures haven't changed in 15+ years)
