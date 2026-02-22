---
phase: 05-contentdirectory-service
verified: 2026-02-22T00:00:00Z
status: passed
score: 16/16 must-haves verified
---

# Phase 5: ContentDirectory SOAP Service Verification Report

**Phase Goal:** DLNA clients can browse the server's media library via SOAP requests and receive correctly formatted DIDL-Lite XML responses with full metadata and DLNA protocol information
**Verified:** 2026-02-22
**Status:** passed
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

All truths are drawn from the must_haves declared across plans 01-04.

| #  | Truth | Status | Evidence |
|----|-------|--------|---------|
| 1  | quick-xml and chrono are available as crate dependencies (cargo build succeeds) | VERIFIED | Cargo.toml lines 27-28: quick-xml 0.39, chrono 0.4; Cargo.lock confirms 0.39.2 and 0.4.43; `cargo build` exits 0 |
| 2  | SOAP envelope builder produces correctly-structured SOAP response XML | VERIFIED | `soap_response()` in soap.rs lines 19-34 produces well-formed envelope; 3 test assertions pass (xml-declaration, closing tag, action tag) |
| 3  | SOAP fault builder produces HTTP 500 + UPnP error envelope for error codes 701 and 402 | VERIFIED | `soap_fault()` lines 42-71 returns `StatusCode::INTERNAL_SERVER_ERROR` + UPnPError envelope; used for 402 (unknown action) and 701 (no such object) |
| 4  | extract_soap_param correctly extracts Browse parameters from a SOAP body string | VERIFIED | Lines 81-87; 6 tests pass including ObjectID, BrowseFlag, StartingIndex, RequestedCount=0, missing param, empty body |
| 5  | apply_pagination returns all items when RequestedCount=0 (spec requirement) | VERIFIED | Lines 105-111; explicit test `apply_pagination_zero_count_returns_all` passes; also `apply_pagination_zero_count_with_offset_returns_remaining` |
| 6  | build_protocol_info produces correct DLNA string with and without profile name | VERIFIED | Lines 122-133; 7 tests: profile=Some includes DLNA.ORG_PN, profile=None omits it entirely (no wildcard) |
| 7  | container_uuid produces stable deterministic UUIDs for Videos/Music/Photos/All Media | VERIFIED | Lines 141-144 uses build_machine_namespace + UUIDv5; 5 tests: non-nil, deterministic, all 4 are distinct |
| 8  | format_dc_date returns ISO 8601 date string from a filesystem path | VERIFIED | Lines 152-160; chrono::DateTime<Utc>::from(mtime) then format("%Y-%m-%d"); fallback "1970-01-01" on error |
| 9  | POST /cds/control with SOAPAction GetSearchCapabilities returns 200 with empty SearchCaps | VERIFIED | content_directory.rs lines 74-79: `handle_get_search_capabilities()` returns `ok_xml(soap_response("GetSearchCapabilities", "<SearchCaps></SearchCaps>"))` |
| 10 | POST /cds/control with SOAPAction GetSortCapabilities returns 200 with empty SortCaps | VERIFIED | Lines 82-86: `handle_get_sort_capabilities()` returns `<SortCaps></SortCaps>` wrapped in SOAP envelope with HTTP 200 |
| 11 | POST /cds/control with SOAPAction GetSystemUpdateID returns 200 with Id element containing 1 | VERIFIED | Lines 89-93: `handle_get_system_update_id()` returns `<Id>1</Id>`; element name is `Id` per UPnP CDS spec |
| 12 | POST /cds/control with unknown SOAPAction returns 500 SOAP fault with error code 402 | VERIFIED | Lines 64-67: catch-all arm calls `soap_fault(402, "InvalidArgs").into_response()` |
| 13 | Browse(BrowseDirectChildren, ObjectID=0) returns DIDL-Lite with four containers (Videos, Music, Photos, All Media) | VERIFIED | content_directory.rs lines 231-247: root arm builds 4-item containers Vec, paginates, wraps in DIDL-Lite, xml_escapes, returns in SOAP envelope |
| 14 | Browse(BrowseMetadata, ObjectID=0) returns root container element with childCount=4 | VERIFIED | Lines 325-334: BrowseMetadata "0" arm creates container_element("0", "-1", "Root", 4) |
| 15 | Browse with unknown ObjectID returns HTTP 500 SOAP fault with errorCode 701 | VERIFIED | Lines 317-320 (BrowseDirectChildren) and 388-391 (BrowseMetadata) both call `soap_fault(701, "No such object")` |
| 16 | DIDL-Lite Result element contains XML-escaped DIDL-Lite; dc:title uses file_stem; dc:date is present; all four DIDL-Lite namespaces present | VERIFIED | `didl_lite_wrap()` (line 102) declares all 4 namespaces including `xmlns:dlna`; `soap::xml_escape(&didl_xml)` called at every Result assembly site; `file_stem()` at line 127; `format_dc_date` at line 136; dc:date in item template at line 158 |

**Score:** 16/16 truths verified

---

## Required Artifacts

| Artifact | Min Lines | Actual Lines | Status | Details |
|----------|-----------|--------------|--------|---------|
| `Cargo.toml` | n/a | 34 | VERIFIED | Contains `quick-xml = { version = "0.39", features = ["serialize"] }` and `chrono = { version = "0.4", default-features = false, features = ["std"] }` |
| `src/http/soap.rs` | 120 (plan 02) | 491 | VERIFIED | 13 public items (6 constants + 7 functions); 33-test `#[cfg(test)]` module; all functions substantive |
| `src/http/content_directory.rs` | 200 (plan 04) | 400 | VERIFIED | Full Browse handler with BrowseDirectChildren + BrowseMetadata + pagination; no stubs or TODO markers |
| `src/http/mod.rs` | n/a | 26 | VERIFIED | `pub mod content_directory;`, `pub mod soap;` declared; `/cds/control` routed to `content_directory::cds_control` |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/http/soap.rs` | `quick_xml::escape::escape` | XML escaping in DIDL-Lite generation | WIRED | Line 184: `quick_xml::escape::escape(s)` called directly; `quick-xml` in Cargo.toml |
| `src/http/soap.rs` | `crate::media::metadata::build_machine_namespace` | container UUID derivation | WIRED | Lines 142-143: `let ns = crate::media::metadata::build_machine_namespace(); uuid::Uuid::new_v5(&ns, ...)` |
| `src/http/mod.rs` | `src/http/content_directory.rs` | axum route `/cds/control` | WIRED | Line 20: `.route("/cds/control", axum::routing::post(content_directory::cds_control))` |
| `src/http/content_directory.rs` | `src/http/soap.rs` | soap utility functions | WIRED | Line 7: `use crate::http::soap::{self, soap_response, soap_fault, extract_soap_param, apply_pagination};` |
| `handle_browse` | `apply_pagination` | pagination of item slices | WIRED | Called 5 times (lines 234, 251, 268, 285, 302) — root containers and all four media kind branches |
| `handle_browse` | `soap::xml_escape` | DIDL-Lite escaping before embedding | WIRED | Called at every Result assembly point (lines 243, 260, 277, 294, 311, 331, 339, 347, 355, 363, 385) |
| `handle_browse` | `AppState.library` | RwLock read guard | WIRED | Line 201: `let lib = state.library.read().expect("library lock poisoned");` |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| CONT-01 | 05-01, 05-04 | Browse (BrowseDirectChildren on root ObjectID=0) returns DIDL-Lite with all four namespaces | SATISFIED | content_directory.rs: root arm builds 4 containers; didl_lite_wrap includes all 4 namespaces including xmlns:dlna |
| CONT-02 | 05-04 | Browse (BrowseMetadata) for individual item IDs | SATISFIED | BrowseMetadata match in handle_browse searches lib.items for UUID match, returns item_element() |
| CONT-03 | 05-01, 05-04 | DIDL-Lite `<res>` elements include correct protocolInfo fourth field | SATISFIED | soap::build_protocol_info() called in item_element(); produces correct DLNA.ORG_OP=01, DLNA.ORG_FLAGS, optional DLNA.ORG_PN |
| CONT-04 | 05-01, 05-02, 05-04 | Pagination: StartingIndex + RequestedCount; RequestedCount=0 returns all | SATISFIED | apply_pagination() implements 0=all semantics; used for all Browse arms; 7 tests pass |
| CONT-05 | 05-01, 05-02, 05-04 | DIDL-Lite XML correctly XML-escaped inside SOAP Result element | SATISFIED | soap::xml_escape(&didl_xml) called at every Result assembly; 3 xml_escape tests pass |
| CONT-06 | 05-03 | GetSearchCapabilities returns empty search caps | SATISFIED | handle_get_search_capabilities() returns `<SearchCaps></SearchCaps>` |
| CONT-07 | 05-03 | GetSortCapabilities returns empty sort caps | SATISFIED | handle_get_sort_capabilities() returns `<SortCaps></SortCaps>` |
| CONT-08 | 05-03 | GetSystemUpdateID SOAP request | SATISFIED | handle_get_system_update_id() returns `<Id>1</Id>` |

All 8 requirement IDs declared in plan frontmatter (CONT-01 through CONT-08) are satisfied with code evidence. No orphaned requirements found — REQUIREMENTS.md traceability table maps exactly CONT-01 to CONT-08 to Phase 5, all marked complete.

---

## Anti-Patterns Found

No anti-patterns detected.

Scanned `src/http/soap.rs` and `src/http/content_directory.rs` for:
- TODO/FIXME/HACK/PLACEHOLDER markers — none found
- `return null`, `return {}`, empty closures — none found
- Stub responses ("Not Implemented", "501", "Action Failed") — none found in final content_directory.rs
- `console.log`-only implementations — not applicable (Rust)

The Plan 03 `handle_browse` placeholder (fault 401) was replaced in Plan 04 — confirmed by searching the final file, which contains no "Action Failed" or "Not Implemented" strings.

---

## Human Verification Required

### 1. Real DLNA Client Browse

**Test:** Connect a Samsung TV or Xbox Series X on the same LAN, run `udlna /path/to/media/dir`, observe the server appear in the TV's media source list, navigate into Videos/Music/Photos containers, and play a file.
**Expected:** Server appears by name; four containers are browseable; media items appear with titles (no file extension), play back correctly.
**Why human:** Requires physical DLNA hardware and network setup; cannot verify actual Samsung protocol acceptance programmatically.

### 2. DIDL-Lite XML Escaping in Wire Format

**Test:** `curl -s -X POST http://localhost:8200/cds/control -H 'SOAPAction: "urn:schemas-upnp-org:service:ContentDirectory:1#Browse"' -H 'Content-Type: text/xml' -d '...'` and inspect the raw `<Result>` element content.
**Expected:** `<Result>` contains `&lt;DIDL-Lite` (escaped), not raw `<DIDL-Lite`; `xmlns:dlna` appears in the escaped content.
**Why human:** Programmatic code inspection confirms `xml_escape()` is called, but wire-level verification confirms no double-escaping or encoding bugs.

### 3. BrowseMetadata on Individual Item UUID

**Test:** Scan a directory with a known video file, get its UUID from BrowseDirectChildren on the videos container, then call BrowseMetadata with that UUID.
**Expected:** Returns single `<item>` element with correct dc:title (no extension), dc:date, correct UPnP class, and res element with size and protocolInfo.
**Why human:** Requires a real media file and UUID extraction — integration test not representable with static grep.

---

## Gaps Summary

No gaps. All automated checks passed:

- `cargo build` exits 0 (no errors)
- `cargo test` exits 0 (81 passed, 0 failed)
- 33 soap.rs unit tests all pass
- All 4 artifacts exist and are substantive (above minimum line counts)
- All 7 key links are wired with evidence in the source
- All 8 requirement IDs (CONT-01 to CONT-08) have implementation evidence
- No anti-patterns (stubs, TODOs, empty returns) found

The phase goal — "DLNA clients can browse the server's media library via SOAP requests and receive correctly formatted DIDL-Lite XML responses with full metadata and DLNA protocol information" — is achieved by the verified implementation.

---

_Verified: 2026-02-22_
_Verifier: Claude (gsd-verifier)_
