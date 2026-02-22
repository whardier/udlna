---
phase: "07-connectionmanager-service"
plan: "01"
subsystem: "cms"
tags: ["soap", "upnp", "dlna", "connection-manager", "mime"]
dependency_graph:
  requires:
    - "05-contentdirectory-service/05-03"
  provides:
    - "CMS SOAP handler at /cms/control"
    - "SUPPORTED_MIMES const in media/mime.rs"
    - "soap_response_ns() for namespace-parameterized SOAP envelopes"
  affects:
    - "src/http/soap.rs"
    - "src/media/mime.rs"
    - "src/http/mod.rs"
    - "src/main.rs"
tech_stack:
  added: []
  patterns:
    - "action-dispatch SOAP handler pattern (mirrors CDS content_directory.rs)"
    - "soap_response_ns() delegation pattern for multi-namespace SOAP support"
key_files:
  created:
    - "src/cms/mod.rs"
  modified:
    - "src/media/mime.rs"
    - "src/http/soap.rs"
    - "src/http/mod.rs"
    - "src/main.rs"
decisions:
  - "UPnP error 401 for unknown CMS action (not 402 InvalidArgs — CDS copy-paste pitfall)"
  - "Status=OK in GetCurrentConnectionInfo (per CONTEXT.md locked decision over minidlna's Unknown)"
  - "_body prefix on unused SOAP body parameter (CMS actions don't parse request params)"
  - "soap_response() delegates to soap_response_ns(CDS_NAMESPACE) for backwards compatibility"
metrics:
  duration: "~2.5 min"
  completed: "2026-02-22"
  tasks_completed: 3
  files_changed: 5
requirements_satisfied:
  - CONN-01
  - CONN-02
  - CONN-03
---

# Phase 7 Plan 01: ConnectionManager Service Summary

**One-liner:** CMS SOAP handler with GetProtocolInfo/GetCurrentConnectionIDs/GetCurrentConnectionInfo using ConnectionManager:1 namespace, backed by SUPPORTED_MIMES const.

## What Was Built

Replaced the existing 501 stub at `/cms/control` with a real ConnectionManager Service (CMS) handler implementing all three mandatory DLNA SOAP actions.

### src/cms/mod.rs (new)

CMS control handler with action dispatch:
- `cms_control()` — reads `SOAPAction` header, dispatches to action handlers or returns SOAP fault 401
- `handle_get_protocol_info()` — builds Source field from SUPPORTED_MIMES as `http-get:*:{mime}:*` entries, empty Sink
- `handle_get_current_connection_ids()` — returns `<ConnectionIDs>0</ConnectionIDs>`
- `handle_get_current_connection_info()` — returns all seven fields (RcsID, AVTransportID, ProtocolInfo, PeerConnectionManager, PeerConnectionID, Direction, Status=OK)

### src/media/mime.rs (modified)

Added `SUPPORTED_MIMES: &[&str]` const listing all 25 video/audio/image MIME types served by the server. Subtitle MIME types (text/srt, text/vtt) intentionally excluded per DLNA ConnectionManager spec.

### src/http/soap.rs (modified)

- Added `CMS_NAMESPACE` constant: `urn:schemas-upnp-org:service:ConnectionManager:1`
- Added `soap_response_ns(action, inner_xml, namespace)` for namespace-parameterized SOAP envelopes
- Refactored `soap_response()` to delegate to `soap_response_ns(CDS_NAMESPACE)` — backwards-compatible

### src/http/mod.rs (modified)

Replaced 501 inline stub with `crate::cms::cms_control` handler.

### src/main.rs (modified)

Added `mod cms;` module declaration.

## Verification Results

All four curl tests passed against live server:

1. GetProtocolInfo — HTTP 200, ConnectionManager:1 namespace, 25 MIME entries as `http-get:*:{mime}:*`, no subtitle types
2. GetCurrentConnectionIDs — HTTP 200, `<ConnectionIDs>0</ConnectionIDs>`
3. GetCurrentConnectionInfo — HTTP 200, all 7 fields, `<Status>OK</Status>`
4. Unknown action — SOAP fault `<errorCode>401</errorCode>`

All 81 cargo tests pass (no regressions).

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| UPnP error 401 for unknown action | CONTEXT.md spec: 401 = Invalid Action. 402 = InvalidArgs (common CDS copy-paste pitfall) |
| Status=OK in GetCurrentConnectionInfo | CONTEXT.md locked decision overrides minidlna's "Unknown" behavior |
| `_body` prefix on CMS body parameter | CMS actions don't parse request body params; prefix suppresses unused-variable warning |
| `soap_response()` delegates to `soap_response_ns()` | Backwards compatibility for all CDS callers; no CDS regression |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing] Prefix unused body parameter with underscore**
- **Found during:** Task 2 (cargo build warning)
- **Issue:** `body: String` parameter in `cms_control()` triggers unused variable warning; plan noted SUPPORTED_MIMES is a const but didn't specify underscore prefix
- **Fix:** Renamed to `_body` to suppress warning cleanly
- **Files modified:** `src/cms/mod.rs`
- **Commit:** f1aa715

## Self-Check

Files exist:
- src/cms/mod.rs: FOUND
- src/media/mime.rs (SUPPORTED_MIMES): FOUND
- src/http/soap.rs (CMS_NAMESPACE, soap_response_ns): FOUND

Commits:
- d1ea901: feat(07-01): add SUPPORTED_MIMES to mime.rs and soap_response_ns() to soap.rs
- f1aa715: feat(07-01): create CMS handler and wire into router and main.rs
- 45ce88b: chore(07-01): verify CMS SOAP actions with curl
