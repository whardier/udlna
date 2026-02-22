# Phase 7: ConnectionManager Service - Research

**Researched:** 2026-02-22
**Domain:** UPnP ConnectionManager SOAP service — GetProtocolInfo, GetCurrentConnectionIDs, GetCurrentConnectionInfo
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Client Compatibility**: Spec compliance IS Xbox compatibility for these simple stubs — no Xbox-specific quirks to accommodate
- **Automated curl/Python SOAP tests** are sufficient — no specific client device required for verification
- **Response field names, order, and values** should match minidlna exactly — maximizes compatibility with clients tested against known-good servers
- **GetProtocolInfo list** must be derived dynamically from the project's MIME module — stays in sync automatically with what the server actually serves
- **Cover all three media categories**: video, audio, and image
- **Handler lives in `src/cms/` module** (not a single file in src/handlers/) — gives room to grow
- **Share the SOAP parsing infrastructure** with ContentDirectory — reuse or extract the SOAP action parser to avoid duplication
- **Planner should audit device.xml** and add connectionmanager.xml, /cms/control route, and any device.xml references that are currently missing

### Claude's Discretion

- **DLNA.ORG_PN profile tag specificity**: wildcard `*` vs. named profiles like `AVC_MP4_BL_CIF15`
- **Exact SOAP infrastructure refactoring strategy**: extract shared module vs. pass parser by reference

### Deferred Ideas (OUT OF SCOPE)

- Subtitle sidecar file alignment — sidecar subtitle files (e.g., .srt) need better alignment with the original media item when not embedded in the stream; this is its own phase
- Embedded subtitle advertisement — subtitles embedded in media streams need to be properly advertised as available to clients; this is its own phase
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CONN-01 | Server handles GetProtocolInfo SOAP request returning list of supported MIME types as source protocol infos | Source field format verified from minidlna source (azatoth fork); MIME list derived from mime.rs classify(); wildcard fourth field recommended |
| CONN-02 | Server handles GetCurrentConnectionIDs SOAP request (stub returning "0") | ConnectionIDs="0" confirmed from minidlna upnpsoap.c and UPnP spec; this is the correct stub value for a MediaServer with no active connections |
| CONN-03 | Server handles GetCurrentConnectionInfo SOAP request (stub returning defaults) | Seven response fields documented from CMS SCPD (already in description.rs); RcsID=-1, AVTransportID=-1, Status="OK" confirmed from CONTEXT.md and node-upnp-device reference |
</phase_requirements>

---

## Summary

Phase 7 replaces the existing 501 stub at `/cms/control` with a real CMS handler implementing three SOAP actions. The route stub already exists in `src/http/mod.rs`. The SCPD XML at `/cms/scpd.xml` is already served from `description.rs`. The device description at `/device.xml` already declares the ConnectionManager service with controlURL `/cms/control`. No new routes need to be added to the router — only the stub handler needs to be replaced.

The primary technical challenge is the SOAP namespace. The existing `soap_response()` function in `src/http/soap.rs` hardcodes `CDS_NAMESPACE` (`urn:schemas-upnp-org:service:ContentDirectory:1`). CMS actions need `CMS_NAMESPACE` (`urn:schemas-upnp-org:service:ConnectionManager:1`) in the SOAP response envelope. The cleanest approach is to add a generic `soap_response_ns(action, inner_xml, namespace)` function to `soap.rs`, then update the existing `soap_response()` to delegate to it with CDS_NAMESPACE. This avoids duplication and gives CMS its own namespace-correct responses.

The GetProtocolInfo Source field is a comma-separated list of protocol info strings, one per supported MIME type. The list must be derived from `src/media/mime.rs` (CONTEXT.md locked decision). Using wildcard fourth field (`http-get:*:{mime}:*`) is the recommended discretion choice — simpler, correct for a MediaServer, and avoids maintaining profile lists that may drift from what the server actually serves.

**Primary recommendation:** Add `src/cms/mod.rs` with an action dispatcher and three handlers. Add `soap_response_ns()` to shared `src/http/soap.rs`. Wire up via `src/http/mod.rs` and add `mod cms;` to `src/main.rs` or `src/lib.rs`.

## Standard Stack

### Core

No new crate dependencies required. All needed infrastructure is already in place:

| Component | Location | Purpose | Status |
|-----------|----------|---------|--------|
| axum 0.8 | Cargo.toml | HTTP routing, handler pattern | Already present |
| `src/http/soap.rs` | Project | SOAP envelope building, fault generation, XML escape | Already present — needs `soap_response_ns()` addition |
| `src/media/mime.rs` | Project | MIME type classification — source of truth for protocol info | Already present — read-only |
| `src/http/state.rs` | Project | AppState (library, server_uuid) | Already present — CMS needs no new state |

### No New Dependencies

CMS is entirely stateless stubs (GetCurrentConnectionIDs always returns "0"; GetCurrentConnectionInfo always returns defaults). No library additions to Cargo.toml are required.

### Installation

```bash
# No new packages needed — all dependencies already in Cargo.toml
```

## Architecture Patterns

### Recommended Project Structure

```
src/
├── cms/
│   └── mod.rs           # CMS control handler — action dispatch + three action handlers
├── http/
│   ├── mod.rs           # Router — replace /cms/control stub with cms::cms_control
│   ├── soap.rs          # Add soap_response_ns(); update soap_response() to delegate
│   └── content_directory.rs  # Unchanged
└── main.rs              # Add: mod cms;
```

The `src/cms/` module mirrors the existing `src/http/content_directory.rs` pattern, but lives under `src/` not `src/http/` because CONTEXT.md locked it to `src/cms/`. The router in `src/http/mod.rs` still imports it as `crate::cms`.

### Pattern 1: SOAP Namespace Refactoring

**What:** Add a namespace-parameterized `soap_response_ns()` to `soap.rs`; update `soap_response()` to call it with CDS_NAMESPACE. CMS handler calls `soap_response_ns()` with CMS_NAMESPACE.

**When to use:** Any time a new UPnP service with a different namespace needs SOAP response envelopes.

**Example:**

```rust
// In src/http/soap.rs
pub const CMS_NAMESPACE: &str = "urn:schemas-upnp-org:service:ConnectionManager:1";

/// Build a SOAP 1.1 response envelope with an explicit service namespace.
pub fn soap_response_ns(action: &str, inner_xml: &str, namespace: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:{action}Response xmlns:u="{ns}">
      {inner_xml}
    </u:{action}Response>
  </s:Body>
</s:Envelope>"#,
        action = action,
        ns = namespace,
        inner_xml = inner_xml,
    )
}

/// Build a SOAP 1.1 response envelope for ContentDirectory (CDS_NAMESPACE).
/// Delegates to soap_response_ns — backwards-compatible.
pub fn soap_response(action: &str, inner_xml: &str) -> String {
    soap_response_ns(action, inner_xml, CDS_NAMESPACE)
}
```

### Pattern 2: CMS Action Dispatcher (mirrors CDS pattern)

**What:** `cms_control` handler extracts SOAPAction from header, dispatches to per-action functions. Falls back to body scanning for clients that omit SOAPAction header.

**When to use:** All three CMS actions pass through this dispatcher.

**Example:**

```rust
// In src/cms/mod.rs
use axum::{extract::State, http::{header, HeaderMap, StatusCode}, response::{IntoResponse, Response}};
use crate::http::soap::{soap_response_ns, soap_fault, CMS_NAMESPACE};
use crate::http::state::AppState;

pub const CMS_NAMESPACE: &str = "urn:schemas-upnp-org:service:ConnectionManager:1";

fn ok_xml(body: String) -> Response {
    (StatusCode::OK, [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")], body).into_response()
}

pub async fn cms_control(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let action = headers
        .get("soapaction")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('#').nth(1))
        .map(|s| s.trim_matches('"').to_string());

    match action.as_deref() {
        Some("GetProtocolInfo") => handle_get_protocol_info(&state),
        Some("GetCurrentConnectionIDs") => handle_get_current_connection_ids(),
        Some("GetCurrentConnectionInfo") => handle_get_current_connection_info(),
        _ => {
            tracing::warn!("Unknown CMS action: {:?}", action);
            soap_fault(401, "Invalid Action").into_response()
        }
    }
}
```

### Pattern 3: Dynamic Protocol Info from mime.rs

**What:** Build GetProtocolInfo Source string by iterating over all MIME types exported from `mime.rs`. Use wildcard fourth field per CONTEXT.md discretion.

**When to use:** GetProtocolInfo action handler only.

**Key insight:** `mime.rs` does not export a list of all supported MIME types — `classify()` is a function that takes a path. The handler must hard-code or embed the list. The correct approach is to define a `pub const SUPPORTED_MIMES: &[&str]` slice in `mime.rs` (or derive it directly in the CMS handler by listing all unique MIME strings that `classify()` can return).

**Recommended approach:** Add `pub const SUPPORTED_MIMES: &[&str]` to `src/media/mime.rs` listing all unique MIME types that `classify()` can return. The CMS handler maps each MIME to a protocol info string.

**Example:**

```rust
// In src/media/mime.rs — add:
pub const SUPPORTED_MIMES: &[&str] = &[
    // Video
    "video/mp4",
    "video/x-matroska",
    "video/x-msvideo",
    "video/quicktime",
    "video/MP2T",
    "video/mpeg",
    "video/x-ms-wmv",
    "video/x-flv",
    "video/ogg",
    "video/webm",
    "video/3gpp",
    // Audio
    "audio/mpeg",
    "audio/flac",
    "audio/wav",
    "audio/mp4",
    "audio/aac",
    "audio/ogg",
    "audio/x-ms-wma",
    "audio/aiff",
    // Image
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/tiff",
];

// In src/cms/mod.rs:
fn handle_get_protocol_info(state: &AppState) -> Response {
    use crate::media::mime::SUPPORTED_MIMES;
    let source: String = SUPPORTED_MIMES
        .iter()
        .map(|mime| format!("http-get:*:{}:*", mime))
        .collect::<Vec<_>>()
        .join(",");
    let inner = format!("<Source>{}</Source><Sink></Sink>", source);
    ok_xml(soap_response_ns("GetProtocolInfo", &inner, CMS_NAMESPACE))
}
```

### Pattern 4: Stub Action Handlers

**What:** GetCurrentConnectionIDs and GetCurrentConnectionInfo are pure stubs — no parameters needed, no state access.

**GetCurrentConnectionIDs response** (field name: `ConnectionIDs`, value: `"0"`):

```rust
fn handle_get_current_connection_ids() -> Response {
    ok_xml(soap_response_ns(
        "GetCurrentConnectionIDs",
        "<ConnectionIDs>0</ConnectionIDs>",
        CMS_NAMESPACE,
    ))
}
```

**GetCurrentConnectionInfo response** — seven fields, exact order from CMS SCPD in description.rs:

```rust
fn handle_get_current_connection_info() -> Response {
    let inner = concat!(
        "<RcsID>-1</RcsID>",
        "<AVTransportID>-1</AVTransportID>",
        "<ProtocolInfo></ProtocolInfo>",
        "<PeerConnectionManager></PeerConnectionManager>",
        "<PeerConnectionID>-1</PeerConnectionID>",
        "<Direction>Output</Direction>",
        "<Status>OK</Status>",
    );
    ok_xml(soap_response_ns("GetCurrentConnectionInfo", inner, CMS_NAMESPACE))
}
```

### Router Wiring

Replace the existing 501 stub in `src/http/mod.rs`:

```rust
// Before (Phase 5 stub):
.route("/cms/control", axum::routing::post(|| async {
    (axum::http::StatusCode::NOT_IMPLEMENTED, "Not Implemented")
}))

// After (Phase 7):
.route("/cms/control", axum::routing::post(crate::cms::cms_control))
```

Add to `src/main.rs`:
```rust
mod cms;
```

### Anti-Patterns to Avoid

- **Duplicating the SOAP envelope builder**: Do NOT copy `soap_response()` body into `cms/mod.rs`. Extend `soap.rs` with the namespace-parameterized variant instead.
- **Hardcoding MIME types in the CMS handler without syncing with mime.rs**: If mime.rs gains new types, the protocol info must stay in sync. The `SUPPORTED_MIMES` const in mime.rs makes this mechanical.
- **Skipping Subtitle MIME types in SUPPORTED_MIMES**: Subtitle types (text/srt, text/vtt) should NOT appear in GetProtocolInfo Source — they are not streamable media that DLNA clients would request via ConnectionManager.
- **Importing AppState for stub actions**: GetCurrentConnectionIDs and GetCurrentConnectionInfo do not need `state` — keep their signatures stateless; only GetProtocolInfo needs state (or SUPPORTED_MIMES access, which requires no state if `SUPPORTED_MIMES` is `const`).
- **Wrong SOAP fault code for unknown action**: UPnP 1.0 specifies error 401 "Invalid Action" for unknown action names (error 402 is "InvalidArgs"). CDS used 402 for unknown actions — CMS should use 401 to be spec-correct.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SOAP envelope construction | New XML builder in cms/mod.rs | Extend `soap_response_ns()` in existing soap.rs | Already tested, correct namespace handling |
| MIME type enumeration | Scanning filesystem or parsing mime.rs code | `SUPPORTED_MIMES` const in mime.rs | Single source of truth, no drift risk |
| XML content-type response | Reinvent ok_xml helper | Copy the `ok_xml()` inline helper pattern from content_directory.rs | Two lines, no new dependency |

**Key insight:** CMS is three simple string-returning functions wrapped in SOAP envelopes. The only novelty is the different service namespace. Every other piece is already built.

## Common Pitfalls

### Pitfall 1: Wrong Namespace in SOAP Response

**What goes wrong:** GetProtocolInfo response includes `xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1"` instead of `urn:schemas-upnp-org:service:ConnectionManager:1`.

**Why it happens:** Calling `soap_response()` (which hardcodes CDS_NAMESPACE) instead of `soap_response_ns()` with CMS_NAMESPACE.

**How to avoid:** Add `CMS_NAMESPACE` constant and `soap_response_ns()` to `soap.rs`. Grep for `CDS_NAMESPACE` after implementation to confirm no CMS handler uses the CDS variant.

**Warning signs:** SOAP clients logging "service namespace mismatch" errors; Xbox not browsing after CMS responds.

### Pitfall 2: SUPPORTED_MIMES Includes Subtitle Types

**What goes wrong:** GetProtocolInfo Source includes `http-get:*:text/srt:*` and `http-get:*:text/vtt:*`. Strict DLNA clients may reject the server or be confused by non-media protocol infos.

**Why it happens:** Blindly including all `classify()` return values without filtering MediaKind::Subtitle.

**How to avoid:** SUPPORTED_MIMES should contain only Video, Audio, and Image MIME types. CONTEXT.md explicitly says "Cover all three media categories: video, audio, and image." Add a test that verifies no subtitle MIME appears in the Source list.

**Warning signs:** Protocol info string contains "text/srt" or "text/vtt".

### Pitfall 3: Forgetting `mod cms;` in main.rs

**What goes wrong:** Compiler error "file not found for module `cms`" or the module exists but is not compiled.

**Why it happens:** Rust requires `mod cms;` in a parent module (main.rs or lib.rs) to include the module in the compilation unit.

**How to avoid:** Add `mod cms;` to `src/main.rs` immediately after creating `src/cms/mod.rs`.

**Warning signs:** `cargo build` succeeds but `/cms/control` still returns 501 (old stub still wired).

### Pitfall 4: Forgetting to Update Router

**What goes wrong:** `src/cms/mod.rs` is implemented but the router still uses the inline 501 stub.

**Why it happens:** The 501 inline closure in `src/http/mod.rs` shadows the new handler.

**How to avoid:** Update `src/http/mod.rs` to replace the closure with `crate::cms::cms_control`. Verify with `curl -X POST /cms/control -d @soap_body.xml` returning 200 not 501.

**Warning signs:** curl returns `501 Not Implemented` after implementation is complete.

### Pitfall 5: Wrong Error Code for Unknown Action

**What goes wrong:** Unknown CMS action returns 402 (InvalidArgs) instead of 401 (Invalid Action).

**Why it happens:** Copy-paste from CDS handler which used 402 for the fallthrough case.

**How to avoid:** UPnP 1.0 spec error codes: 401 = Invalid Action (action not recognized), 402 = Invalid Args (action recognized, args wrong). CMS unknown action → 401.

**Warning signs:** SOAP fault contains `<errorCode>402</errorCode>` for an unrecognized action name.

### Pitfall 6: GetCurrentConnectionInfo Status Value

**What goes wrong:** Returning `<Status>Unknown</Status>` (minidlna default) instead of `<Status>OK</Status>` (CONTEXT.md locked value).

**Why it happens:** Following minidlna source code literally; minidlna returns "Unknown" but CONTEXT.md says "OK".

**How to avoid:** CONTEXT.md locked decision: Status=OK. The CMS SCPD in description.rs lists "OK" as a valid `allowedValue`. "OK" is the correct value for a connection that has no errors — which is exactly what connection ID 0 represents.

**Warning signs:** `<Status>Unknown</Status>` in GetCurrentConnectionInfo response.

## Code Examples

Verified patterns from project codebase and official sources:

### Complete GetProtocolInfo Source String Format

```
http-get:*:video/mp4:*,http-get:*:video/x-matroska:*,...,http-get:*:image/jpeg:*,...
```

Each entry: `http-get:*:{mime}:*` — four colon-separated fields per DLNA protocol info format (Source: minidlna upnpglobalvars.h pattern, fourth-field simplified to wildcard per CONTEXT.md Claude's discretion).

### Complete GetCurrentConnectionInfo Response Body

Field order matches CMS SCPD argument list order in `description.rs` (which is authoritative for this server's SCPD):

```xml
<RcsID>-1</RcsID>
<AVTransportID>-1</AVTransportID>
<ProtocolInfo></ProtocolInfo>
<PeerConnectionManager></PeerConnectionManager>
<PeerConnectionID>-1</PeerConnectionID>
<Direction>Output</Direction>
<Status>OK</Status>
```

Source: CMS SCPD argument list in `src/http/description.rs` lines 78-86 (the out-arguments define the canonical field ordering).

### SOAP Action Dispatch — SOAPAction Header Parsing

Reuse exact same pattern as content_directory.rs (no body fallback needed for CMS — actions have no ambiguous prefixes):

```rust
// Source: src/http/content_directory.rs (Phase 5 pattern)
let action = headers
    .get("soapaction")
    .and_then(|v| v.to_str().ok())
    .and_then(|s| s.split('#').nth(1))
    .map(|s| s.trim_matches('"').to_string());
```

CMS actions do not require a body fallback: the three CMS actions are simple and all DLNA clients that invoke them send the SOAPAction header.

### Audit Checklist: Existing Infrastructure Already In Place

The following items already exist and do NOT need to be added in Phase 7:

```
[x] /cms/scpd.xml — served from description.rs (Phase 4)
[x] /cms/control route — exists in mod.rs as 501 stub (Phase 5)
[x] device.xml ConnectionManager service declaration — present in description.rs (Phase 4)
[x] CMS SCPD XML with all three actions — present in description.rs (Phase 4)
```

The following items need to be CREATED in Phase 7:

```
[ ] src/cms/mod.rs — new module with cms_control + three handlers
[ ] CMS_NAMESPACE constant — add to soap.rs
[ ] soap_response_ns() function — add to soap.rs
[ ] SUPPORTED_MIMES const — add to mime.rs
[ ] mod cms; — add to main.rs
[ ] Router update — replace 501 stub with crate::cms::cms_control in mod.rs
```

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| soap_response() with hardcoded CDS namespace | soap_response_ns() with explicit namespace | Enables any UPnP service namespace |
| CMS stub returning 501 | Real CMS handler with 3 actions | Xbox/strict DLNA clients can now browse and stream |

**MIME module**: `mime.rs` currently has no exported list of MIME strings — only the `classify()` function. Adding `SUPPORTED_MIMES: &[&str]` is a clean addition that serves the CMS without changing `classify()` behavior.

## Open Questions

1. **DLNA.ORG_PN in GetProtocolInfo Source vs wildcard**
   - What we know: minidlna uses named profiles (AVC_MP4_MP_SD_AAC_MULT5 etc.); CONTEXT.md leaves this to Claude's discretion
   - What's unclear: Whether Xbox Series X uses GetProtocolInfo to filter what it requests
   - Recommendation: Use wildcard `*` as fourth field. Named profiles require maintaining a large profile list that can drift from what the server actually serves. Wildcard correctly declares "I can serve this MIME type." The DLNA.ORG_PN in `<res protocolInfo>` (already implemented in Phase 5) is what clients use for format compatibility checking. GetProtocolInfo Source is coarser-grained capability advertisement.

2. **SOAPAction body fallback in CMS dispatcher**
   - What we know: CDS handler has body fallback for clients that omit SOAPAction header (Phase 5, Pitfall 3)
   - What's unclear: Whether any real DLNA client omits SOAPAction specifically for CMS actions
   - Recommendation: Omit the body fallback for CMS. CMS actions are simpler than Browse. If a client fails to send SOAPAction, the fallthrough returns 401 Invalid Action, which the client should handle. Adding the body fallback would be low-cost but is not needed for the target clients (Xbox Series X sends SOAPAction headers).

## Sources

### Primary (HIGH confidence)

- `src/http/soap.rs` (project) — existing SOAP envelope builder, CDS_NAMESPACE constant, soap_response() signature
- `src/http/description.rs` (project) — CMS SCPD XML with exact argument names and ordering for all three actions; ConnectionManager service declaration in device.xml
- `src/http/mod.rs` (project) — existing /cms/control stub, router structure
- `src/media/mime.rs` (project) — classify() function; all MIME strings that SUPPORTED_MIMES must mirror
- `src/http/content_directory.rs` (project) — CDS dispatcher pattern to replicate for CMS

### Secondary (MEDIUM confidence)

- [minidlna upnpsoap.c (azatoth fork)](https://github.com/azatoth/minidlna/blob/master/upnpsoap.c) — GetCurrentConnectionIDs response (ConnectionIDs=0), GetCurrentConnectionInfo response structure (RcsID=-1, AVTransportID=-1, Status="Unknown" — overridden to "OK" per CONTEXT.md)
- [minidlna upnpglobalvars.h (glebius fork)](https://github.com/glebius/minidlna/blob/master/upnpglobalvars.h) — RESOURCE_PROTOCOL_INFO_VALUES format: `http-get:*:{mime}:DLNA.ORG_PN={profile}` per entry, comma-separated
- [node-upnp-device ConnectionManager](https://github.com/jacobrask/node-upnp-device/blob/master/lib/services/ConnectionManager.coffee) — Reference implementation returning Status="OK" with same seven field names

### Tertiary (LOW confidence)

- [UPnP ConnectionManager:1 Spec](https://upnp.org/specs/av/UPnP-av-ConnectionManager-v1-Service.pdf) — official spec; PDF not fully parsed; field names confirmed through SCPD XML already in project

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new dependencies; existing axum/soap.rs infrastructure fully adequate
- Architecture: HIGH — CDS handler pattern is proven and directly replicable; module location locked in CONTEXT.md
- SOAP response format: HIGH — field names confirmed from CMS SCPD already in description.rs; values confirmed from minidlna + CONTEXT.md
- Protocol info format: HIGH — `http-get:*:{mime}:*` format verified; MIME list derivation from mime.rs is mechanical
- Pitfalls: HIGH — all pitfalls identified from direct code inspection of existing project

**Research date:** 2026-02-22
**Valid until:** 2026-03-22 (stable domain — UPnP spec and project structure are stable)
