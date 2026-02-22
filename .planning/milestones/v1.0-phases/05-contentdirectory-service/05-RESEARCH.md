# Phase 5: ContentDirectory Service - Research

**Researched:** 2026-02-22
**Domain:** UPnP ContentDirectory SOAP service — Browse action, DIDL-Lite XML, SOAP fault handling
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Content hierarchy**: Root "0" returns four containers — Videos, Music, Photos, All Media — with UUIDv5 ObjectIDs
- **Container UUIDs**: `uuid5(machine_namespace, container_name_string)` — same `build_machine_namespace()` from Phase 2
- **Media item ObjectIDs**: Their Phase 2 UUIDv5 IDs (already in `MediaItem.id`)
- **No database**: Everything computed from in-memory `MediaLibrary` at request time
- **DIDL-Lite metadata richness**: Full metadata — dc:title (filename without extension), upnp:class (type-specific), res (full URL), res@size, res@duration (when available), res@resolution (when available), res@bitrate (when available), dc:date (file mtime ISO 8601)
- **protocolInfo**: Full DLNA string when profile known (`http-get:*:{mime}:DLNA.ORG_PN={profile};DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000`); omit DLNA.ORG_PN entirely when profile is None
- **DLNA.ORG_FLAGS constant**: Same value as Phase 3 (`01700000000000000000000000000000`)
- **Container parentID**: Root containers get parentID="0"; root itself gets parentID="-1"
- **BrowseMetadata on container**: Returns container itself as DIDL-Lite `<container>` with correct childCount
- **BrowseMetadata on "0"**: Returns root container with childCount=4
- **GetSearchCapabilities**: Returns empty string
- **GetSortCapabilities**: Returns empty string
- **GetSystemUpdateID**: Returns fixed value 1 (non-zero, deterministic)
- **Unknown ObjectID**: 701 NoSuchObject SOAP fault
- **Malformed SOAP**: 402 InvalidArgs SOAP fault
- **DIDL-Lite XML escaping**: All text content must be XML-escaped (titles with &, <, > must not break XML)

### Claude's Discretion

- **Album art / thumbnails**: Skip for Phase 5 — keep scope focused
- **Host URL for res elements**: Use Host header from incoming request (most portable for dual-bind)
- **Error handling specifics**: 701 for unknown ObjectID, 402 for malformed SOAP (confirmed in research as spec-correct)

### Deferred Ideas (OUT OF SCOPE)

- Album art / thumbnails — future enhancement (not Phase 5)
- Search support (GetSearchCapabilities returning actual criteria) — out of scope for Phase 5
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CONT-01 | Server handles Browse (BrowseDirectChildren on root ObjectID=0) SOAP request and returns DIDL-Lite XML with all four required namespaces and correct element ordering | DIDL-Lite XML structure fully documented; four required namespaces verified from multiple DLNA implementations |
| CONT-02 | Server handles Browse (BrowseMetadata) SOAP request for individual item IDs | BrowseMetadata pattern documented; container and item element formats verified |
| CONT-03 | DIDL-Lite `<res>` elements include correct protocolInfo fourth field (DLNA.ORG_OP, DLNA.ORG_FLAGS) for each MIME type | protocolInfo string format verified from CONTEXT.md locked decisions and Phase 3 constants |
| CONT-04 | Browse handles StartingIndex and RequestedCount pagination; RequestedCount=0 returns all items (per UPnP spec) | Pagination semantics verified from UPnP ContentDirectory:1 spec; RequestedCount=0 means "all" confirmed |
| CONT-05 | DIDL-Lite XML is correctly XML-escaped inside the SOAP `<Result>` element | quick-xml escape module provides `escape()` function; Result element must contain XML-escaped DIDL-Lite string |
| CONT-06 | Server handles GetSearchCapabilities SOAP request (stub returning empty search caps) | Response envelope format documented; SearchCaps="" verified |
| CONT-07 | Server handles GetSortCapabilities SOAP request (stub returning empty sort caps) | Response envelope format documented; SortCaps="" verified |
| CONT-08 | Server handles GetSystemUpdateID SOAP request | GetSystemUpdateID response returns Id element with integer value; fixed value 1 per CONTEXT.md |
</phase_requirements>

---

## Summary

Phase 5 implements the SOAP ContentDirectory control endpoint at `/cds/control`. The route already exists as a 501 stub in `src/http/mod.rs`. This phase replaces that stub with a real handler that dispatches to action-specific implementations: Browse, GetSearchCapabilities, GetSortCapabilities, and GetSystemUpdateID.

The core work is twofold: (1) parsing the incoming SOAP request to extract the action name and parameters, and (2) generating DIDL-Lite XML responses. Both can be done with string manipulation — SOAP parsing via quick-xml serde deserialization or simple string search, and DIDL-Lite generation via Rust string formatting. The SOAP envelope wrappers for both requests and responses are static XML templates with a handful of interpolated values.

The data model is entirely in-memory. The `MediaLibrary` (held in `AppState`) contains all `MediaItem` instances. Container UUIDs are derived deterministically via `uuid5(machine_namespace, container_name)` using the same `build_machine_namespace()` from `src/media/metadata.rs`. No persistence, no database — everything is recomputed per request from the locked in-memory state.

**Primary recommendation:** Add `src/http/content_directory.rs` as the main handler module. Parse action name from the `SOAPAction` header (extract after `#`). Parse SOAP body with quick-xml serde deserialization for Browse parameters. Generate DIDL-Lite XML via `format!()` / `String::push_str()` with proper XML escaping via `quick_xml::escape::escape()`. Return responses as `(StatusCode, [(CONTENT_TYPE, "text/xml; charset=\"utf-8\"")], body_string)`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| quick-xml | 0.39.2 | XML parsing and XML character escaping | Already used in ecosystem; provides serde integration and `escape()` function needed for DIDL-Lite Result element |
| axum | 0.8 | HTTP handler, String body extractor, HeaderMap extractor | Already in Cargo.toml; String extractor consumes SOAP body directly |
| uuid | 1.x (v5 feature) | Container UUIDs via `Uuid::new_v5()` | Already in Cargo.toml with v5 feature; same pattern as Phase 2 |
| std (Rust stdlib) | — | `std::fs::metadata` for file mtime (dc:date) | No extra crate needed for SystemTime → ISO 8601 |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| serde | 1.x | Derive Deserialize for SOAP Browse request structs | Only if using quick-xml serde deserialization for SOAP body parsing |
| tracing | 0.1 | Debug/warn logging for unknown ObjectIDs and parse errors | Already in Cargo.toml |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| quick-xml for SOAP parsing | Manual string search (`contains()`, `find()`) | String search is simpler for SOAP since the XML is short and well-known; quick-xml serde is safer against malformed inputs |
| quick-xml escape | Manual HTML-entity replacement | quick-xml `escape()` is a single well-tested function call vs. hand-rolling replacements with ordering bugs |
| format!() for DIDL-Lite | XML builder crate (xml-rs, minidom) | format!() is fast and readable for a fixed-schema document; XML builders add no value when schema is known and static |

**Installation (quick-xml not yet in Cargo.toml):**
```bash
# Add to Cargo.toml [dependencies]
quick-xml = { version = "0.39", features = ["serialize"] }
serde = { version = "1", features = ["derive"] }  # if using serde deserialization
```

---

## Architecture Patterns

### Recommended Module Structure

```
src/http/
├── mod.rs             # build_router — replace /cds/control stub
├── content_directory.rs   # NEW: cds_control handler + sub-dispatchers
├── soap.rs (optional)     # NEW: shared SOAP envelope builder + fault builder
├── description.rs     # existing Phase 4
├── media.rs           # existing Phase 3
└── state.rs           # existing AppState
```

### Pattern 1: Action Dispatch from SOAPAction Header

**What:** Extract action name from `SOAPAction: "urn:schemas-upnp-org:service:ContentDirectory:1#Browse"` header, then dispatch to action-specific handlers.

**When to use:** Single entry point `/cds/control` receives all CDS actions.

**Example:**
```rust
// Source: verified from Samsung TV behavior and MiniDLNA source
pub async fn cds_control(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    // Extract action name from SOAPAction header
    // Format: "urn:schemas-upnp-org:service:ContentDirectory:1#Browse"
    let action = headers
        .get("soapaction")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('#').nth(1))
        .map(|s| s.trim_matches('"'));

    match action {
        Some("Browse") => handle_browse(&state, &body).await,
        Some("GetSearchCapabilities") => handle_get_search_capabilities(),
        Some("GetSortCapabilities") => handle_get_sort_capabilities(),
        Some("GetSystemUpdateID") => handle_get_system_update_id(),
        _ => soap_fault(402, "InvalidArgs"),
    }
}
```

### Pattern 2: SOAP Browse Parameter Extraction

**What:** Parse ObjectID, BrowseFlag, StartingIndex, RequestedCount from the SOAP body.

**Approach A — Simple string search (recommended for this use case):**
The SOAP body is a short, well-formed XML document with known field names. Simple text extraction is robust for the element values that cannot contain nested XML.

```rust
// Simple extraction using find + slice (works reliably for SOAP parameter values)
fn extract_soap_param(body: &str, param: &str) -> Option<String> {
    let open = format!("<{}>", param);
    let close = format!("</{}>", param);
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    Some(body[start..end].to_string())
}

// Usage:
let object_id = extract_soap_param(&body, "ObjectID").unwrap_or_default();
let browse_flag = extract_soap_param(&body, "BrowseFlag").unwrap_or_default();
let starting_index: u32 = extract_soap_param(&body, "StartingIndex")
    .and_then(|s| s.parse().ok()).unwrap_or(0);
let requested_count: u32 = extract_soap_param(&body, "RequestedCount")
    .and_then(|s| s.parse().ok()).unwrap_or(0);
```

**Approach B — quick-xml serde (more robust):**
```rust
// Source: quick-xml docs.rs/quick-xml serde deserialization
// Note: quick-xml serde does NOT handle namespace prefix stripping (issue #218 unresolved)
// Use #[serde(rename = "...")] with the local name only; quick-xml strips prefixes
#[derive(Deserialize)]
struct BrowseArgs {
    #[serde(rename = "ObjectID")]
    object_id: String,
    #[serde(rename = "BrowseFlag")]
    browse_flag: String,
    #[serde(rename = "StartingIndex", default)]
    starting_index: u32,
    #[serde(rename = "RequestedCount", default)]
    requested_count: u32,
}
```

**Pitfall:** quick-xml serde namespace prefix handling is a known limitation (issue #218 open). The struct rename must use the local element name. For parsing the full SOAP envelope, you'd need nested structs for Envelope > Body > u:Browse. The string search approach avoids all namespace complexity.

### Pattern 3: SOAP Response Envelope

**What:** Wrap action-specific content in the required SOAP envelope.

**Template (verified from multiple DLNA implementations):**
```rust
fn soap_response(action: &str, service_ns: &str, inner_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:{action}Response xmlns:u="{service_ns}">
      {inner_xml}
    </u:{action}Response>
  </s:Body>
</s:Envelope>"#,
        action = action,
        service_ns = service_ns,
        inner_xml = inner_xml
    )
}

const CDS_NAMESPACE: &str = "urn:schemas-upnp-org:service:ContentDirectory:1";
```

### Pattern 4: SOAP Fault Response

**What:** Return UPnP-compliant SOAP fault for errors.

**Template (verified from MiniDLNA source and UPnP spec):**
```rust
fn soap_fault(error_code: u32, error_description: &str) -> Response {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <s:Fault>
      <faultcode>s:Client</faultcode>
      <faultstring>UPnPError</faultstring>
      <detail>
        <UPnPError xmlns="urn:schemas-upnp-org:control-1-0">
          <errorCode>{}</errorCode>
          <errorDescription>{}</errorDescription>
        </UPnPError>
      </detail>
    </s:Fault>
  </s:Body>
</s:Envelope>"#,
        error_code, error_description
    );
    (
        StatusCode::INTERNAL_SERVER_ERROR,  // UPnP uses 500 for SOAP faults
        [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        body,
    ).into_response()
}
```

### Pattern 5: Browse Response with DIDL-Lite

**What:** Generate Browse action SOAP response containing XML-escaped DIDL-Lite.

```rust
fn browse_response(didl_lite: &str, number_returned: u32, total_matches: u32) -> String {
    // DIDL-Lite must be XML-escaped before embedding in the Result element
    // Source: UPnP ContentDirectory spec — Result is a string containing escaped XML
    let escaped = quick_xml::escape::escape(didl_lite);
    format!(
        r#"<Result>{}</Result>
<NumberReturned>{}</NumberReturned>
<TotalMatches>{}</TotalMatches>
<UpdateID>1</UpdateID>"#,
        escaped, number_returned, total_matches
    )
}
```

### Pattern 6: DIDL-Lite XML Generation

**What:** Build the inner DIDL-Lite XML document for Browse results.

**Required namespaces (all four must be present on root element):**
```rust
const DIDL_LITE_OPEN: &str = r#"<DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/"
  xmlns:dc="http://purl.org/dc/elements/1.1/"
  xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/"
  xmlns:dlna="urn:schemas-dlna-org:metadata-1-0/">"#;
const DIDL_LITE_CLOSE: &str = "</DIDL-Lite>";
```

**Container element format:**
```rust
// Source: verified from multiple DLNA implementations and UPnP AV spec
fn container_element(id: &str, parent_id: &str, title: &str, child_count: u32, upnp_class: &str) -> String {
    format!(
        r#"<container id="{}" parentID="{}" restricted="1" childCount="{}">
  <dc:title>{}</dc:title>
  <upnp:class>{}</upnp:class>
</container>"#,
        id,
        parent_id,
        child_count,
        quick_xml::escape::escape(title),
        upnp_class
    )
}
// upnp:class for type containers: "object.container.storageFolder"
// Note: use "object.container.storageFolder" — most widely compatible with Samsung/Xbox
```

**Item element format:**
```rust
fn item_element(item: &MediaItem, parent_id: &str, res_url: &str, protocol_info: &str) -> String {
    let title = item.path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    let upnp_class = match item.kind {
        MediaKind::Video => "object.item.videoItem",
        MediaKind::Audio => "object.item.audioItem.musicTrack",
        MediaKind::Image => "object.item.imageItem.photo",
        _ => "object.item",
    };

    // Build res element attributes
    let mut res_attrs = format!(r#"protocolInfo="{}" size="{}""#, protocol_info, item.file_size);
    if let Some(dur) = &item.meta.duration {
        res_attrs.push_str(&format!(r#" duration="{}""#, dur));
    }
    if let Some(res) = &item.meta.resolution {
        res_attrs.push_str(&format!(r#" resolution="{}""#, res));
    }
    if let Some(br) = item.meta.bitrate {
        res_attrs.push_str(&format!(r#" bitrate="{}""#, br));
    }

    // dc:date from file modification time
    let dc_date = /* file mtime as ISO 8601 string */;

    format!(
        r#"<item id="{id}" parentID="{parent_id}" restricted="1">
  <dc:title>{title}</dc:title>
  <upnp:class>{upnp_class}</upnp:class>
  <dc:date>{dc_date}</dc:date>
  <res {res_attrs}>{res_url}</res>
</item>"#,
        id = item.id,
        parent_id = parent_id,
        title = quick_xml::escape::escape(&title),
        upnp_class = upnp_class,
        dc_date = dc_date,
        res_attrs = res_attrs,
        res_url = quick_xml::escape::escape(res_url)
    )
}
```

### Pattern 7: Container UUIDs

**What:** Derive stable container UUIDs using the same pattern as media item IDs.

```rust
// Source: Phase 2 metadata.rs — same build_machine_namespace() function
use crate::media::metadata::build_machine_namespace;
use uuid::Uuid;

fn container_uuid(name: &str) -> Uuid {
    let ns = build_machine_namespace();
    Uuid::new_v5(&ns, name.as_bytes())
}

// Fixed container names (stable across restarts)
const CONTAINER_VIDEOS: &str = "Videos";
const CONTAINER_MUSIC: &str = "Music";
const CONTAINER_PHOTOS: &str = "Photos";
const CONTAINER_ALL_MEDIA: &str = "All Media";
```

### Pattern 8: protocolInfo Construction

**What:** Build the fourth-field DLNA protocolInfo string for each media item.

```rust
// Source: CONTEXT.md locked decision; verified against Phase 3 DLNA constants
const DLNA_FLAGS: &str = "01700000000000000000000000000000";

fn build_protocol_info(mime: &'static str, dlna_profile: Option<&'static str>) -> String {
    match dlna_profile {
        Some(profile) => format!(
            "http-get:*:{}:DLNA.ORG_PN={};DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS={}",
            mime, profile, DLNA_FLAGS
        ),
        None => format!(
            "http-get:*:{}:DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS={}",
            mime, DLNA_FLAGS
        ),
    }
}
```

### Pattern 9: Pagination

**What:** Apply StartingIndex and RequestedCount to slice the full item list.

```rust
// Source: UPnP ContentDirectory:1 spec — RequestedCount=0 means "return all"
fn apply_pagination<T>(items: &[T], starting_index: u32, requested_count: u32) -> &[T] {
    let start = (starting_index as usize).min(items.len());
    let slice = &items[start..];
    if requested_count == 0 {
        slice  // return all remaining
    } else {
        let count = (requested_count as usize).min(slice.len());
        &slice[..count]
    }
}
// TotalMatches = total items in container (before pagination)
// NumberReturned = slice.len() (what was actually returned)
```

### Pattern 10: Host URL Resolution

**What:** Build the res element URL using the Host header.

```rust
// Use Host header — portable across IPv4/IPv6 dual-bind (CONTEXT.md discretion)
fn build_res_url(headers: &HeaderMap, item_id: &Uuid) -> String {
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost:8200");
    format!("http://{}/media/{}", host, item_id)
}
```

### Anti-Patterns to Avoid

- **Missing DIDL-Lite escaping in Result**: The `<Result>` SOAP element must contain the DIDL-Lite as an XML-escaped string (not raw XML). Embedding raw DIDL-Lite directly into the SOAP Result tag will produce invalid SOAP that clients reject.
- **Using integer ObjectIDs**: The CONTEXT.md mandates UUIDv5 for all container and item IDs. Clients cache ObjectIDs; changing ID scheme breaks bookmarks.
- **Returning HTTP 200 for SOAP faults**: SOAP faults must be HTTP 500 per SOAP 1.1 spec. Some clients check the HTTP status code to detect errors.
- **Omitting all four DIDL-Lite namespaces**: Samsung TVs validate that all required namespace declarations are present on the `<DIDL-Lite>` root element. Missing `xmlns:dlna` causes silent browse failures.
- **Using `object.container` instead of `object.container.storageFolder`**: Most DLNA clients, including Samsung TVs, expect `object.container.storageFolder` for typed containers. The plain `object.container` class is the abstract base — use the derived class.
- **Not stripping file extension for dc:title**: The CONTEXT.md locks this: title is filename WITHOUT extension. `file_stem()` not `file_name()`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| XML character escaping | Custom `&` → `&amp;` replace chain | `quick_xml::escape::escape()` | Ordering matters (&amp; must go first); `escape()` handles all five special chars correctly |
| UUID v5 for containers | Custom hash-based ID | `Uuid::new_v5()` from uuid crate | Already in Cargo.toml; UUID spec guarantees collision resistance |
| File mtime → ISO 8601 | Custom date formatting | `std::time::SystemTime` + manual epoch math | `std::fs::metadata().modified()` returns `SystemTime`; format with `chrono` or stdlib arithmetic |

**Key insight:** SOAP bodies for this service are short (under 1KB) with known element names. String templating for responses and simple string-find for parsing is pragmatic and avoids XML library complexity for what is essentially a template-fill problem.

---

## Common Pitfalls

### Pitfall 1: DIDL-Lite Not Escaped in Result Element

**What goes wrong:** Browse response returns SOAP with raw DIDL-Lite XML embedded directly in `<Result>`, which produces invalid XML (nested XML tags within an XML text node).
**Why it happens:** Developer builds SOAP envelope by string concatenation and forgets that `<Result>` content must be escaped.
**How to avoid:** Always call `quick_xml::escape::escape(didl_xml_string)` before inserting into `<Result>`. The DIDL-Lite ampersands become `&amp;`, angle brackets become `&lt;`/`&gt;`.
**Warning signs:** Samsung TV sees empty or broken content; `xmllint --noout` on the SOAP response XML fails.

### Pitfall 2: Unknown ObjectID Returns 200 Instead of SOAP Fault

**What goes wrong:** Handler returns HTTP 200 with empty DIDL-Lite for unknown ObjectIDs instead of a 701 SOAP fault. VLC and Samsung expect 701 to detect bad navigation.
**Why it happens:** Default case in match returns empty browse response.
**How to avoid:** Explicitly check for unrecognized ObjectID (not "0", not a known container UUID, not a known media item UUID) and return `soap_fault(701, "No such object")` with HTTP 500.
**Warning signs:** Client navigation hangs or shows empty container instead of error; curl returns 200 for arbitrary fake UUIDs.

### Pitfall 3: SOAPAction Header Case Sensitivity

**What goes wrong:** `headers.get("SOAPAction")` returns None because HTTP/1.1 header names are case-insensitive but axum's `HeaderMap::get()` performs case-insensitive lookup — this is actually fine. The pitfall is that some clients omit the header entirely or put the action name in the body only.
**Why it happens:** Some older clients put `SOAPAction: ""` (empty) and embed the action in `xmlns:u` attribute of the body element.
**How to avoid:** Fall back to parsing the action from the SOAP body (`<u:Browse ...>` → extract tag local name) if SOAPAction header is absent/empty.
**Warning signs:** All SOAP calls return 402 InvalidArgs even with valid requests from specific clients.

### Pitfall 4: Pagination with RequestedCount=0

**What goes wrong:** Implementation treats RequestedCount=0 as "return 0 items" (natural integer interpretation) rather than "return all items" (UPnP spec behavior).
**Why it happens:** Spec behavior is counter-intuitive (0 = all).
**How to avoid:** Explicit check: `if requested_count == 0 { return_all }`.
**Warning signs:** Clients send RequestedCount=0 on first Browse and receive empty list; Samsung TV shows "No content found".

### Pitfall 5: Missing dc:date Causes Samsung Rejection

**What goes wrong:** Samsung TVs on some firmware versions require dc:date on items. Missing dc:date causes items to not appear or appear without metadata.
**Why it happens:** dc:date is optional in the spec but Samsung validates it.
**How to avoid:** Always include `<dc:date>` using file mtime from `std::fs::metadata(&item.path).modified()`. Format as ISO 8601 date string (`YYYY-MM-DD` is sufficient; full datetime is better).
**Warning signs:** Items visible in VLC but not on Samsung TV browse list.

### Pitfall 6: Host Header Absent on Some Requests

**What goes wrong:** `headers.get(HOST)` returns None for some test requests (curl without -H Host).
**Why it happens:** HTTP/1.1 requires Host header from clients, but HTTP/1.0 requests and some test tools omit it.
**How to avoid:** Provide a fallback: `unwrap_or("localhost:8200")` or use `AppState`'s port config as fallback.
**Warning signs:** res URL becomes empty string or panics in test environments.

### Pitfall 7: BrowseMetadata for Items vs Containers

**What goes wrong:** BrowseMetadata on a media item ID returns `<container>` element instead of `<item>` element (or vice versa).
**Why it happens:** Developer uses same code path for both container and item metadata.
**How to avoid:** BrowseMetadata dispatch: if ObjectID == "0" or a known container UUID → return `<container>` element. If ObjectID matches a MediaItem.id → return `<item>` element. Unknown → 701 fault.
**Warning signs:** Xbox rejects metadata response; seeking fails on video items.

### Pitfall 8: Filename Extension Not Stripped from dc:title

**What goes wrong:** `dc:title` shows "movie.mkv" instead of "movie". DLNA clients display the raw filename with extension.
**Why it happens:** Using `file_name()` instead of `file_stem()` on the path.
**How to avoid:** `item.path.file_stem().and_then(|s| s.to_str()).unwrap_or("")` — always use `file_stem()`.
**Warning signs:** TV shows "Movie.mkv" in content list instead of "Movie".

---

## Code Examples

Verified patterns from official sources and existing codebase:

### GetSearchCapabilities Response

```rust
// Source: UPnP ContentDirectory:1 spec — empty string means no search support
fn handle_get_search_capabilities() -> Response {
    let inner = "<SearchCaps></SearchCaps>";
    let body = soap_response("GetSearchCapabilities", CDS_NAMESPACE, inner);
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        body,
    ).into_response()
}
```

### GetSortCapabilities Response

```rust
// Source: UPnP ContentDirectory:1 spec — empty string means no sort support
fn handle_get_sort_capabilities() -> Response {
    let inner = "<SortCaps></SortCaps>";
    let body = soap_response("GetSortCapabilities", CDS_NAMESPACE, inner);
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")], body).into_response()
}
```

### GetSystemUpdateID Response

```rust
// Source: UPnP ContentDirectory:1 spec — Id element name (capital I lowercase d)
// CONTEXT.md: fixed value 1
fn handle_get_system_update_id() -> Response {
    let inner = "<Id>1</Id>";
    let body = soap_response("GetSystemUpdateID", CDS_NAMESPACE, inner);
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")], body).into_response()
}
```

### dc:date from File Metadata

```rust
// Source: std::fs::Metadata — no external crate needed for YYYY-MM-DD
use std::time::{SystemTime, UNIX_EPOCH};

fn format_dc_date(path: &std::path::Path) -> String {
    if let Ok(meta) = std::fs::metadata(path) {
        if let Ok(mtime) = meta.modified() {
            let secs = mtime.duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            // Convert epoch seconds to YYYY-MM-DD
            let days = secs / 86400;
            // Simple Gregorian calendar arithmetic (or use chrono if available)
            // Alternatively: just use the epoch-seconds as a fixed date string
            return format_date_from_epoch(secs);
        }
    }
    "1970-01-01".to_string()  // fallback
}
```

**Note:** ISO 8601 date from epoch without chrono requires calendar arithmetic. The planner should decide: (a) add `chrono` to Cargo.toml for clean date formatting, or (b) implement simple epoch-to-date math. Chrono is the standard approach.

### Router Update in mod.rs

```rust
// Replace Phase 5 stub in build_router()
.route("/cds/control", axum::routing::post(content_directory::cds_control))
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Integer ObjectIDs (MiniDLNA) | UUIDv5 ObjectIDs | 2010s | Stable, stateless, no DB needed |
| URLBase in device.xml | Omit URLBase (Phase 4 decision) | UPnP 1.1 | No impact; Host header used instead |
| XML builder crates for DIDL-Lite | format!() / string templates | N/A | Simpler, no extra crate needed |
| Separate browse handler per container type | Single handler with ObjectID dispatch | N/A | Less code, same result |

**Deprecated/outdated:**
- `s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/"` on `<s:Envelope>`: The encoding style attribute is technically part of SOAP 1.1 RPC encoding. For DLNA "document style" SOAP, it is technically unnecessary but harmless and expected by many DLNA clients — include it for compatibility.

---

## Open Questions

1. **chrono for dc:date formatting**
   - What we know: `std::fs::Metadata::modified()` returns `SystemTime`; formatting as ISO 8601 requires epoch conversion
   - What's unclear: Whether to add chrono crate or use stdlib-only arithmetic
   - Recommendation: Add `chrono = { version = "0.4", default-features = false, features = ["std"] }` to Cargo.toml — it is the standard Rust date library and avoids hand-rolling calendar math. Alternatively, output just a fixed "1970-01-01" placeholder if dc:date is not strictly required by Samsung/Xbox (LOW confidence on whether Samsung requires it vs. just preferring it).

2. **SOAP body action parsing fallback**
   - What we know: SOAPAction header always set by Samsung TV and Xbox; may be absent in test/edge cases
   - What's unclear: Whether any real DLNA client omits SOAPAction entirely and embeds action only in body
   - Recommendation: Primary dispatch from SOAPAction header; add body-based fallback for robustness (LOW confidence on which clients omit SOAPAction).

3. **childCount for "All Media" container on BrowseMetadata**
   - What we know: BrowseMetadata on a container must return childCount; "All Media" contains all items across all types
   - What's unclear: Whether childCount should count subtitle items or only the filtered-out non-Subtitle items
   - Recommendation: childCount = total MediaLibrary items (all non-Subtitle, which is what MediaLibrary.items already contains — Subtitle was filtered at scan time).

---

## Sources

### Primary (HIGH confidence)
- Official UPnP ContentDirectory:1 Service specification (https://upnp.org/specs/av/UPnP-av-ContentDirectory-v1-Service.pdf) — action definitions, Browse parameters, SOAP fault format
- quick-xml docs.rs (https://docs.rs/quick-xml/latest/quick_xml/escape/index.html) — escape() function; version 0.39.2 verified
- axum docs.rs (https://docs.rs/axum/latest/axum/extract/index.html) — String extractor, HeaderMap extractor; axum 0.8 confirmed
- Existing codebase: `src/media/metadata.rs` — `build_machine_namespace()`, `media_item_id()` patterns
- Existing codebase: `src/http/media.rs` — `DLNA_CONTENT_FEATURES` constant, DLNA_FLAGS value
- Existing codebase: `src/media/library.rs` — `MediaItem`, `MediaMeta` struct fields

### Secondary (MEDIUM confidence)
- MiniDLNA upnpsoap.c analysis (https://github.com/NathanaelA/minidlna/blob/master/upnpsoap.c) — SOAP Browse parameter names, error codes 701/402, DIDL-Lite structure
- Samsung TV DLNA bug reports (Jellyfin, rclone issues) — Samsung quirk: sends Browse with all required fields; standard protocolInfo accepted
- UPnP AV Tutorial July 2014 (https://upnp.org/resources/documents/UPnP_AV_tutorial_July2014.pdf) — DIDL-Lite structure, container class hierarchy
- quick-xml GitHub issue #218 — confirmed namespace prefix handling limitation; serde rename uses local name only

### Tertiary (LOW confidence)
- Samsung TV behavior with `object.container.storageFolder` — multiple bug reports suggest this class works; `object.container` is abstract base class; recommend `object.container.storageFolder` for typed containers
- dc:date requirement on Samsung TV — anecdotally required by some Samsung firmware; include it as it costs nothing

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — quick-xml, uuid, axum all verified; already in or adjacent to Cargo.toml
- Architecture: HIGH — SOAP envelope format cross-verified from spec and MiniDLNA; DIDL-Lite namespaces verified from multiple implementations
- Pitfalls: HIGH for escaping, pagination, ObjectID handling (spec-verified); MEDIUM for Samsung-specific dc:date requirement (anecdotal)
- Container UUIDs: HIGH — exact same pattern as Phase 2 UUIDv5

**Research date:** 2026-02-22
**Valid until:** 2026-06-22 (DLNA spec is stable; axum 0.8 API is stable)
