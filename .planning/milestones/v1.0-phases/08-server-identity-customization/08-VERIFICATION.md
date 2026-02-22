---
phase: 08-server-identity-customization
verified: 2026-02-23T06:00:00Z
status: human_needed
score: 5/5 must-haves verified
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "Server generates a UUID v5 from hostname + server name using the DNS namespace"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Real DLNA client discovery of friendly name"
    expected: "Samsung TV or Xbox device list shows the value of --name (or default 'udlna@{hostname}') as the server display name"
    why_human: "Cannot programmatically test what a physical DLNA client renders on its device list UI"
---

# Phase 8: Server Identity & Customization Verification Report

**Phase Goal:** Server maintains a consistent identity across restarts and users can customize how it appears on TV device lists
**Verified:** 2026-02-23
**Status:** human_needed (all automated checks pass; 1 item requires a real DLNA client)
**Re-verification:** Yes — after gap closure (commit `50aa653`)

---

## Goal Achievement

### Observable Truths

Truths sourced from ROADMAP.md Phase 8 Success Criteria (3 items) plus PLAN.md must_haves.truths (5 items), consolidated into 5 distinct verifiable truths.

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Server generates a UUID v5 from hostname + server name using the DNS namespace; same UUID in device.xml and SSDP across restarts without a state file | VERIFIED | `build_server_uuid(hostname: &str, server_name: &str)` at `src/main.rs:38-41` concatenates `"{hostname}\x00{server_name}"` and passes to `Uuid::new_v5(&Uuid::NAMESPACE_DNS, seed.as_bytes())`. Called at line 92: `build_server_uuid(&raw_hostname, &config.name)`. Commit `50aa653` ("fix(08-01): derive UUID v5 from hostname + server_name per CLI-08") closes the previous gap. |
| 2 | Running `udlna --name "Living Room Server" /media` causes "Living Room Server" to appear as the friendly name in device.xml and startup banner | VERIFIED | `serve_device_xml` at `description.rs:104` does `let friendly_name = crate::http::soap::xml_escape(&state.server_name)` and inserts via `{name}` format arg at line 144. `AppState.server_name` is set from `config.name.clone()` (main.rs:122). Banner at lines 95-100 logs `config.name`. SUMMARY curl output confirms `<friendlyName>udlna@MacBookPro</friendlyName>` and `Living Room` test. |
| 3 | When no --name flag is provided, default name is "udlna@{hostname}" (e.g., "udlna@macbook") | VERIFIED | `src/config.rs` lines 6-17: `fn default_name()` returns `format!("udlna@{}", host)` when hostname available, falls back to `"udlna"`. `Config::resolve` line 39 calls `unwrap_or_else(default_name)`. Test `test_defaults_when_nothing_set` asserts `config.name == "udlna" || config.name.starts_with("udlna@")`. 81 tests pass. |
| 4 | Startup banner shows both name and UUID on a single line: `udlna "<name>" (uuid: <uuid>) on port <port>` | VERIFIED | `src/main.rs` lines 95-100: `tracing::info!("udlna \"{}\" (uuid: {}) on port {}", config.name, server_uuid, config.port)`. SSDP log at service.rs line 67: `tracing::info!("SSDP advertising \"{}\" on {}:1900", config.server_name, iface.addr)`. |
| 5 | `cargo build --release` produces zero compiler warnings | VERIFIED | Release build produces 0 lines matching `^warning\[`. `cargo test` shows 81 passed, 0 failed. `xml_escape` returns `Cow<'_, str>`. `IfaceV4.index` has `#[allow(dead_code)]`. |

**Score:** 5/5 truths verified

---

## Gap Closure Verification

**Previous gap (from initial VERIFICATION.md, status: partial):**

> "UUID derivation uses hostname alone, but CLI-08 and ROADMAP SC-1 specify 'hostname + server name'"

**Fix confirmed via commit `50aa653` ("fix(08-01): derive UUID v5 from hostname + server_name per CLI-08"):**

```rust
// src/main.rs lines 34-41 (current state)
fn build_server_uuid(hostname: &str, server_name: &str) -> String {
    let seed = format!("{}\x00{}", hostname, server_name);
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_DNS, seed.as_bytes()).to_string()
}

// Called at line 92:
let server_uuid = build_server_uuid(&raw_hostname, &config.name);
```

The implementation now:
- Accepts both `hostname` and `server_name` parameters (matching the requirement)
- Concatenates them with a null byte separator (preventing collisions like `"ab" + "c"` == `"a" + "bc"`)
- Uses `Uuid::NAMESPACE_DNS` (correct namespace per CLI-08)
- Is deterministic — same inputs always produce same UUID

---

## Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | `hostname` crate 0.4.2 dependency | VERIFIED | `hostname = "0.4.2"` confirmed present |
| `src/config.rs` | Dynamic `default_name()` computing `udlna@{hostname}` | VERIFIED | Lines 6-17: `fn default_name()` uses `hostname::get()`, returns `format!("udlna@{}", host)` or `"udlna"` fallback |
| `src/http/state.rs` | `AppState` with `server_name: String` field | VERIFIED | Line 11: `pub server_name: String` with Phase 8 comment; both `server_uuid` and `server_name` fields present |
| `src/main.rs` | UUID v5 derivation from hostname + server_name wired into AppState + startup banner | VERIFIED | `build_server_uuid(hostname, server_name)` at lines 38-41. Call at line 92 passes both `raw_hostname` and `config.name`. AppState construction at line 119-123 includes `server_name: config.name.clone()`. |
| `src/http/description.rs` | `friendlyName` from `state.server_name` (xml_escaped) | VERIFIED | Line 104: `let friendly_name = crate::http::soap::xml_escape(&state.server_name)`. Line 144: `{name}` format arg. |
| `src/ssdp/service.rs` | `SsdpConfig` with `server_name`, logged at startup | VERIFIED | Lines 7-14: `pub struct SsdpConfig { ... pub server_name: String }`. Line 67: `tracing::info!("SSDP advertising \"{}\" on {}:1900", config.server_name, iface.addr)`. Both SsdpConfig construction sites in main.rs (lines 144-148 and 240-244) include `server_name: config.name.clone()`. |

---

## Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/main.rs` | `src/http/state.rs` | `AppState { server_name: config.name.clone() }` | VERIFIED | main.rs line 122: `server_name: config.name.clone()` in AppState construction. Pattern `server_name.*config\.name` confirmed. |
| `src/http/description.rs` | `src/http/soap.rs` | `xml_escape(&state.server_name)` | VERIFIED | description.rs line 104: `crate::http::soap::xml_escape(&state.server_name)`. Pattern `xml_escape.*server_name` confirmed. |
| `src/main.rs` | `src/config.rs` | `Config::resolve` uses `default_name()` for fallback | VERIFIED | config.rs line 39: `args.name.clone().or(file.name).unwrap_or_else(default_name)`. `default_name` used as function value (not called immediately). |
| `src/main.rs` | `build_server_uuid` | Both hostname and server_name passed | VERIFIED | Line 92: `build_server_uuid(&raw_hostname, &config.name)`. Both arguments present, seed concatenated with null separator at lines 39-40. |

---

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| CLI-08 | 08-01-PLAN.md | "Server uses a stable UUID v5 derived from hostname + server name using the DNS namespace UUID (RFC 4122); consistent across restarts without requiring persistent state" | VERIFIED | `build_server_uuid(hostname, server_name)` concatenates both inputs with null separator, uses `Uuid::NAMESPACE_DNS`. UUID is deterministic (stable across restarts). Commit `50aa653` closed the previous partial-compliance gap. |
| DISC-05 | 08-01-PLAN.md | "User can set a custom friendly server name via `--name` flag; name is shown on Samsung/Xbox device lists" | VERIFIED (automated) / HUMAN for device list display | `--name` wired end-to-end: CLI -> config -> `AppState.server_name` -> `device.xml <friendlyName>` (xml_escaped) + SSDP startup log. Automated confirmation: SUMMARY shows `<friendlyName>udlna@MacBookPro</friendlyName>` from curl. Human verification needed to confirm actual Samsung/Xbox device list display. |

**Orphaned requirements check:** REQUIREMENTS.md traceability table maps only CLI-08 and DISC-05 to Phase 8. Both are claimed in the plan. No orphaned requirements.

---

## Anti-Patterns Found

No anti-patterns found in the modified files. No TODOs, FIXMEs, placeholder comments, empty implementations, or stub handlers detected in any of the 8 files touched by this phase.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| — | — | — | — | None found |

---

## Human Verification Required

### 1. DLNA Client Friendly Name Display

**Test:** Start server without --name (default), then with `--name "Living Room"`. Check both cases on a physical Samsung TV or Xbox Series X DLNA media browser.
**Expected:** TV device list shows `udlna@{hostname}` in the first case and `Living Room` in the second case.
**Why human:** Cannot programmatically test what a DLNA client renders in its device discovery UI. The `device.xml` `<friendlyName>` is correctly set in code and confirmed via curl, but client rendering behavior requires a real device.

---

## Regression Check (Previously Passing Items)

All 4 previously-passing truths remain intact after the gap-fix commit:

| Truth | Regression Check |
|-------|-----------------|
| Truth 2 (--name wired to friendlyName) | No regression — `description.rs:104` and `state.server_name` path unchanged |
| Truth 3 (default name = udlna@hostname) | No regression — `config.rs:6-17` and `Config::resolve:39` unchanged |
| Truth 4 (startup banner format) | No regression — banner format at `main.rs:95-100` unchanged |
| Truth 5 (zero compiler warnings) | No regression — `cargo build --release` produces 0 warnings; 81 tests pass |

---

## Summary

**Gap closure confirmed:** The sole gap from the initial verification (UUID derivation excluding server name) is fixed. Commit `50aa653` updated `build_server_uuid` from a single-parameter function taking only `hostname` to a two-parameter function taking `hostname` and `server_name`, concatenating them with a null byte separator before hashing. This fully satisfies CLI-08's requirement text ("hostname + server name") and ROADMAP SC-1.

**Phase goal achieved (automated):** The server maintains a consistent identity across restarts (UUID v5, deterministic from hostname + name) and users can customize the friendly name via `--name` (wired end-to-end through device.xml and SSDP). One human-verification item remains for confirming actual device-list rendering on a physical DLNA client.

---

_Verified: 2026-02-23_
_Verifier: Claude (gsd-verifier)_
_Re-verification: Yes — gap closure after initial gaps_found verdict_
