---
phase: 05-contentdirectory-service
plan: "01"
subsystem: http/soap
tags: [soap, didl-lite, xml, upnp, dlna, utilities]
dependency_graph:
  requires:
    - src/media/metadata.rs (build_machine_namespace for container_uuid)
    - uuid crate (v5 feature, already in Cargo.toml)
  provides:
    - src/http/soap.rs (all SOAP/DIDL-Lite utility functions for Plans 03-04)
  affects:
    - src/http/mod.rs (pub mod soap added)
tech_stack:
  added:
    - quick-xml 0.39.2 (serialize feature) — XML escaping via escape() module
    - chrono 0.4.43 (std feature) — ISO 8601 date formatting from SystemTime
  patterns:
    - UPnP SOAP 1.1 envelope with CDS_NAMESPACE (urn:schemas-upnp-org:service:ContentDirectory:1)
    - UPnP SOAP fault with HTTP 500 (spec-correct, not 200)
    - UPnP pagination: RequestedCount=0 returns all (spec anti-intuitive requirement)
    - DLNA protocolInfo: full string when profile known, omit PN entirely when None
    - Container UUIDs: UUIDv5 via build_machine_namespace() (same pattern as Phase 2)
key_files:
  created:
    - src/http/soap.rs (185 lines — all SOAP/DIDL-Lite utility functions)
  modified:
    - Cargo.toml (added quick-xml and chrono dependencies)
    - src/http/mod.rs (added pub mod soap)
decisions:
  - extract_soap_param uses simple string-find (Approach A from RESEARCH.md) — avoids quick-xml serde namespace complexity for short, well-known SOAP bodies
  - format_dc_date uses chrono for clean SystemTime->YYYY-MM-DD conversion rather than hand-rolling calendar math
  - soap_fault returns tuple (StatusCode, [(CONTENT_TYPE, ...), String]) for direct IntoResponse use by callers
  - xml_escape is a thin wrapper around quick_xml::escape::escape — single well-tested call for all 5 XML special chars
metrics:
  duration: "2 min 23 sec"
  completed: "2026-02-23"
  tasks_completed: 2
  files_modified: 3
---

# Phase 5 Plan 01: SOAP Utility Functions Summary

**One-liner:** SOAP envelope/fault builders, DIDL-Lite helpers, and XML escaping utilities using quick-xml and chrono.

## What Was Built

Added `quick-xml 0.39.2` and `chrono 0.4.43` as Cargo dependencies, then created `src/http/soap.rs` — the pure-function foundation for the ContentDirectory SOAP handler in Plans 03-04.

### Public Items in src/http/soap.rs

**Constants (6):**
- `CDS_NAMESPACE` — `urn:schemas-upnp-org:service:ContentDirectory:1`
- `DLNA_FLAGS` — `01700000000000000000000000000000`
- `CONTAINER_VIDEOS`, `CONTAINER_MUSIC`, `CONTAINER_PHOTOS`, `CONTAINER_ALL_MEDIA` — stable container name strings for UUIDv5 derivation

**Functions (7):**
- `soap_response(action, inner_xml) -> String` — complete SOAP 1.1 envelope
- `soap_fault(error_code, error_description) -> (StatusCode, [...], String)` — HTTP 500 + UPnP fault body
- `extract_soap_param(body, param) -> Option<String>` — string-find SOAP parameter extraction
- `apply_pagination<T>(items, starting_index, requested_count) -> &[T]` — UPnP pagination (0=all)
- `build_protocol_info(mime, dlna_profile) -> String` — DLNA protocolInfo with optional PN
- `container_uuid(name) -> uuid::Uuid` — UUIDv5 from machine namespace
- `format_dc_date(path) -> String` — ISO 8601 date from file mtime with chrono
- `build_res_url(headers, item_id) -> String` — streaming URL from Host header
- `xml_escape(s) -> Cow<str>` — wrapper around quick_xml::escape::escape

## Verification Results

- `cargo build` exits 0 (no errors, 16 dead-code warnings expected — items consumed in Plans 03-04)
- `cargo test` passes 48 tests (no regressions from prior phases)
- `src/http/soap.rs` is 185 lines (well above the 80-line minimum)
- `pub mod soap` declared in `src/http/mod.rs`

## Deviations from Plan

None — plan executed exactly as written.

The plan specified "9 public items (2 constants + 7 functions)" but the actual count is 6 constants + 7 functions = 13 public items. The 4 container name constants (CONTAINER_VIDEOS, CONTAINER_MUSIC, CONTAINER_PHOTOS, CONTAINER_ALL_MEDIA) were listed in the Constants block of the task spec and were all implemented as specified. The "2 constants" figure in the done criteria likely referred to just CDS_NAMESPACE and DLNA_FLAGS; all 4 container constants were explicitly required by the task action and implemented.

## Commits

| Task | Name | Commit | Key Files |
|------|------|--------|-----------|
| 1 | Add quick-xml and chrono dependencies | 3ea6852 | Cargo.toml, Cargo.lock |
| 2 | Implement src/http/soap.rs | c230ea4 | src/http/soap.rs, src/http/mod.rs |

## Self-Check: PASSED

- [x] `src/http/soap.rs` exists (185 lines)
- [x] `pub mod soap` in `src/http/mod.rs`
- [x] Commit 3ea6852 found in git log
- [x] Commit c230ea4 found in git log
- [x] `cargo build` exits 0
- [x] `cargo test` passes 48 tests
- [x] Cargo.toml contains quick-xml 0.39.x and chrono 0.4.x
