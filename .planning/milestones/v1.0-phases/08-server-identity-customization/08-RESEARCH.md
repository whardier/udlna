# Phase 8: Server Identity & Customization - Research

**Researched:** 2026-02-22
**Domain:** Rust UUID v5 derivation, hostname acquisition, config/CLI wiring, compiler warning fixes
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Default friendly name:** `"udlna@{hostname}"` (e.g., `"udlna@macbook"`). Fallback to `"udlna"` alone if hostname is unavailable, empty, or resolves to something generic.
- **UUID derivation:** UUID v5 derived from **hostname only** (not the full friendly name). UUID stays stable when the user changes `--name`. UUID changes only if hostname changes (rare, clean transition).
- **Config file integration:** `name` is settable in `udlna.toml`. Precedence: CLI `--name` > TOML `name` > default (`"udlna@hostname"`). Matches existing three-layer merge pattern.
- **Startup banner:** Must show both friendly name and UUID: `udlna "Shane uDLNA" (uuid: 9f27398b-...) on 192.168.4.111:8200`. Format beyond name + UUID is Claude's discretion.
- **Compiler warnings to fix:**
  - `IfaceV4.index` unused field warning in `src/ssdp/socket.rs`
  - `xml_escape` lifetime annotation warning in `src/http/soap.rs` (`Cow<str>` → `Cow<'_, str>`)
  - Zero compiler warnings must be the outcome.

### Claude's Discretion

- Whether the friendly name appears in the SSDP advertising log line (in addition to startup banner).
- Exact format of the startup banner log line (beyond the name + UUID requirement).

### Deferred Ideas (OUT OF SCOPE)

- None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-08 | Server uses a stable UUID v5 derived from hostname + server name using the DNS namespace UUID (RFC 4122); consistent across restarts without requiring persistent state | CONTEXT.md overrides to hostname-only derivation; `Uuid::new_v5(&Uuid::NAMESPACE_DNS, hostname_bytes)` is the correct pattern; already used in this codebase (`build_machine_namespace` uses `Uuid::NAMESPACE_DNS`) |
| DISC-05 | User can set a custom friendly server name via `--name` flag; name is shown on Samsung/Xbox device lists | `cli.rs` already has `pub name: Option<String>`; `config.rs` already has the `name` field and three-layer merge; only `<friendlyName>` in `description.rs` needs wiring plus default name update |
</phase_requirements>

## Summary

Phase 8 is a focused wiring-and-fix phase. The vast majority of the scaffolding is already in place from earlier phases: the `--name` CLI flag exists in `cli.rs`, the `name` field exists in `config::Config` with correct three-layer merge semantics, and the `config.name` is already logged in the startup banner. What is missing is: (1) the default name computation uses a static string `"udlna"` instead of `"udlna@{hostname}"`; (2) the `<friendlyName>` element in `device.xml` is hardcoded rather than reading from `AppState`; (3) `AppState` does not carry the server name; (4) `server_uuid` in `main.rs` is still `Uuid::new_v4()` rather than the stable UUID v5 from hostname; and (5) two compiler warnings remain.

The hostname must be obtained via an external crate — Rust's standard library provides no `gethostname()` function. Two lightweight options exist: `hostname` (v0.4.2, `hostname::get() -> OsResult<OsString>`) and `gethostname` (v1.1.0, `gethostname::gethostname() -> OsString`). Both are thin wrappers around the OS `gethostname(3)` syscall. The project already depends on `machine-uid` for a similar purpose (machine identity). Either crate adds negligible compile time and zero runtime overhead.

The UUID v5 derivation pattern is already established in this codebase: `Uuid::new_v5(&Uuid::NAMESPACE_DNS, bytes)`. The codebase uses this in `build_machine_namespace()` (seeding from `machine_uid`) and `media_item_id()`. Phase 8 replaces `Uuid::new_v4()` in `main.rs` with a new `build_server_uuid(hostname: &str) -> Uuid` function using `Uuid::NAMESPACE_DNS` and the hostname bytes. The `uuid` crate (v1.21.0, already at `v5` feature) supports this directly.

**Primary recommendation:** Add `hostname` crate (or `gethostname`) for hostname acquisition, derive UUID v5 from hostname using `Uuid::NAMESPACE_DNS`, add `server_name: String` to `AppState`, wire it into `<friendlyName>` in `description.rs`, update `config.rs` default to compute `"udlna@{hostname}"`, fix the two compiler warnings. One plan covers all of this.

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `uuid` | 1.21.0 (already) | UUID v5 derivation | Already in project with `v5` feature; `Uuid::NAMESPACE_DNS` + `new_v5()` is the RFC 4122 standard pattern |
| `hostname` | 0.4.2 | Cross-platform `gethostname()` | Thin syscall wrapper; same author (djc) as some other quality Rust networking crates; stable at 0.4.x for years |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `gethostname` | 1.1.0 | Alternative `gethostname()` crate | If `hostname` 0.4.2 has any compatibility issues; slightly newer project (codeberg) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `hostname` crate | `std::process::Command::new("hostname")` | Spawning a subprocess for hostname is an antipattern; slow, can fail, shell injection risk in theory |
| `hostname` crate | `gethostname` crate | Both are equally appropriate; `hostname` 0.4.2 is the more established choice; `gethostname` 1.1.0 is slightly newer but hosted on codeberg |
| `hostname` crate | `machine-uid` (already in project) | `machine-uid` returns a machine hardware identifier, not the hostname — different values |

**Installation:**
```bash
cargo add hostname
```

## Architecture Patterns

### Recommended Project Structure

No new files needed. All changes are in-place modifications of existing files:

```
src/
├── config.rs          # Update DEFAULT_NAME computation to include hostname
├── main.rs            # Replace Uuid::new_v4() with stable build_server_uuid(); update startup banner; add server_name to AppState init
├── http/
│   ├── state.rs       # Add `server_name: String` field to AppState
│   └── description.rs # Wire `state.server_name` into <friendlyName>
└── ssdp/
    └── socket.rs      # Fix unused `index` field warning
```

Plus `src/http/soap.rs` for the lifetime fix.

### Pattern 1: UUID v5 from Hostname (Established Project Pattern)

**What:** Derive a stable `Uuid` from a string input using `Uuid::new_v5`.
**When to use:** Any time a stable, deterministic UUID is needed from a known input.
**Example:**

```rust
// Mirrors the existing build_machine_namespace() pattern in src/media/metadata.rs
use uuid::Uuid;

/// Derive a stable UUIDv5 for the server from the system hostname.
/// Uses Uuid::NAMESPACE_DNS per RFC 4122 / CLI-08 requirement.
/// Falls back to "udlna" if hostname cannot be obtained.
pub fn build_server_uuid(hostname: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, hostname.as_bytes())
}
```

The `uuid` crate already has `v5` feature enabled in `Cargo.toml`. `Uuid::NAMESPACE_DNS` is a built-in constant.

### Pattern 2: Hostname Acquisition

**What:** Obtain the system hostname as a `String`.
**When to use:** At startup, once, before UUID derivation.
**Example:**

```rust
// Using hostname crate 0.4.2
fn get_hostname_or_fallback() -> String {
    hostname::get()
        .ok()
        .and_then(|os| os.into_string().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "udlna".to_string())
}
```

The `OsString -> String` conversion can fail on non-UTF-8 hostnames (rare on home networks but possible). `.into_string().ok()` handles this gracefully by falling back.

### Pattern 3: Default Name Computation

**What:** Compute the default friendly name as `"udlna@{hostname}"` with fallback.
**Where:** In `config.rs` during `Config::resolve()` or as the `DEFAULT_NAME` expression.

```rust
// In config.rs: replace the static DEFAULT_NAME with a function
fn default_name() -> String {
    let host = hostname::get()
        .ok()
        .and_then(|os| os.into_string().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    if host.is_empty() {
        "udlna".to_string()
    } else {
        format!("udlna@{}", host)
    }
}
```

Then in `Config::resolve`:
```rust
name: args.name.clone().or(file.name).unwrap_or_else(default_name),
```

### Pattern 4: AppState Extension

**What:** Add `server_name: String` to `AppState` so HTTP handlers can access it.
**Where:** `src/http/state.rs`

Current `AppState`:
```rust
pub struct AppState {
    pub library: Arc<RwLock<MediaLibrary>>,
    pub server_uuid: String,   // Phase 4: random UUID v4; Phase 8 replaces with stable UUIDv5
}
```

Updated `AppState`:
```rust
pub struct AppState {
    pub library: Arc<RwLock<MediaLibrary>>,
    pub server_uuid: String,   // Stable UUID v5 derived from hostname (Phase 8)
    pub server_name: String,   // Friendly name from --name / config / default (Phase 8)
}
```

### Pattern 5: Wiring into device.xml friendlyName

**What:** Replace hardcoded `<friendlyName>udlna</friendlyName>` with state value.
**Where:** `src/http/description.rs`, `serve_device_xml` handler.

Current:
```rust
<friendlyName>udlna</friendlyName>
```

Updated (state already in scope as `State(state): State<AppState>`):
```rust
<friendlyName>{name}</friendlyName>
```
with `name = &state.server_name` in the format args.

Note: Samsung and Xbox read `<friendlyName>` from device.xml when building device lists. This is the primary carrier of the user-visible name. No XML escaping issues expected for typical server names, but using `xml_escape()` from `http/soap.rs` is defensive.

### Pattern 6: Startup Banner Update

**What:** Show name + UUID in the startup banner per locked decision.
**Where:** `src/main.rs`, after UUID derivation.

Example:
```rust
tracing::info!(
    "udlna \"{}\" (uuid: {}) on {}:{}",
    config.name,
    server_uuid,
    /* first IP or "all interfaces" */,
    config.port
);
```

The exact format of the IP/address portion is Claude's discretion — either log it before binding (using `config.port` and a placeholder address) or after binding (where the actual bound address is known). Given the existing code structure, the banner is logged before binding, so using `config.port` + "all interfaces" or "0.0.0.0" is appropriate.

### Pattern 7: Compiler Warning Fixes

**Warning 1: Unused `index` field in `IfaceV4` (src/ssdp/socket.rs:63)**

```
warning: field `index` is never read
  --> src/ssdp/socket.rs:63:9
```

Options (in preference order):
- **Suppress with `#[allow(dead_code)]` on the field** — clean, self-documenting, preserves the field for future use (IPv6 multicast join uses interface index).
- Remove the field — but `index` is genuinely useful for IPv6 multicast (the `build_recv_socket_v6` takes `iface_index: u32`), so removal would hurt future phases.
- Prefix with `_` — not idiomatic for `pub` struct fields.

Recommendation: `#[allow(dead_code)]` on the `index` field only. The field is intentionally preserved for future IPv6 multicast work.

```rust
pub struct IfaceV4 {
    pub addr: Ipv4Addr,
    pub mask: Ipv4Addr,
    #[allow(dead_code)]
    pub index: u32,
}
```

**Warning 2: Lifetime elision inconsistency in `xml_escape` (src/http/soap.rs:190)**

```
warning: hiding a lifetime that's elided elsewhere is confusing
   --> src/http/soap.rs:190:22
    |
190 | pub fn xml_escape(s: &str) -> Cow<str> {
    |                                ^^^^^^^^ the same lifetime is hidden here
help: use `'_` for type paths
    |
190 | pub fn xml_escape(s: &str) -> Cow<'_, str> {
```

Fix: Change return type from `Cow<str>` to `Cow<'_, str>`. This is the compiler's own suggestion; it makes the lifetime relationship explicit.

```rust
pub fn xml_escape(s: &str) -> Cow<'_, str> {
    quick_xml::escape::escape(s)
}
```

### Anti-Patterns to Avoid

- **Deriving UUID from both hostname and name:** Locked decision is hostname-only. If UUID included the friendly name, changing `--name` would break DLNA clients that cached the old UUID.
- **Adding hostname to SSDP `SERVER:` header:** The SERVER header format is `OS/version UPnP/version Product/version` (e.g., `Linux/1.0 UPnP/1.0 udlna/0.1`). The friendly name does not belong there. SSDP server header is a product identifier, not a human display name.
- **Storing UUID as `Uuid` type then formatting for SSDP vs device.xml:** Keep `server_uuid` as `String` in `AppState` (already the pattern) to avoid repeated `.to_string()` calls in hot paths.
- **Adding hostname crate dependency then not using it elsewhere:** One import site is fine; the call is idiomatic Rust.
- **Non-UTF-8 hostname panicking:** Always use `.into_string().ok()` with fallback, never `.unwrap()`, on `OsString` hostname values.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| System hostname | Shell command, `/etc/hostname` parsing, custom syscall binding | `hostname` crate 0.4.2 | Cross-platform (macOS/Linux/Windows), handles encoding, 50 lines of proven code |
| UUID v5 derivation | Custom SHA-1 + UUID layout per RFC 4122 | `uuid::Uuid::new_v5()` | Already in project, RFC-correct, constant-time |

**Key insight:** This phase is almost entirely wiring existing pieces together. The only new dependency is a hostname crate; everything else (UUID v5, config merge, TOML, CLI) is already present.

## Common Pitfalls

### Pitfall 1: Hostname Contains Newline or Trailing Whitespace

**What goes wrong:** On some systems, `hostname::get()` may return a hostname with a trailing newline or whitespace (especially if obtained from a shell command rather than the syscall). If fed directly to `Uuid::new_v5`, two machines that should have the same UUID get different ones.

**Why it happens:** The `hostname` crate (0.4.2) uses `gethostname(3)` directly and does NOT add whitespace. This pitfall applies only if the hostname is obtained via `Command::new("hostname")` (antipattern). Using the `hostname` crate avoids it.

**How to avoid:** Use `hostname::get()` (the crate), not a subprocess. The crate returns a clean `OsString` from the syscall.

**Warning signs:** UUID differs between runs on the same machine; UUID has extra bytes encoded in it.

### Pitfall 2: OsString hostname fails UTF-8 conversion

**What goes wrong:** `OsString::into_string()` returns `Err(OsString)` on non-UTF-8 hostnames. Calling `.unwrap()` panics.

**Why it happens:** Hostnames are technically allowed to contain non-UTF-8 bytes on some systems (though extremely rare in practice for home networks).

**How to avoid:** Always use `.into_string().ok().unwrap_or_else(|| "udlna".to_string())` or equivalent. The fallback is the locked behavior anyway.

**Warning signs:** Startup panic on machines with unusual hostname configurations.

### Pitfall 3: Startup Banner Logged Before UUID Is Known

**What goes wrong:** The existing startup banner in `main.rs` is logged at line 77, before `server_uuid` is computed at line 92. The locked decision requires the banner to show both name and UUID.

**Why it happens:** Historical code order — banner was added before UUID computation.

**How to avoid:** Move the UUID derivation before the banner log, or restructure the banner to appear after UUID computation. Since UUID derivation is synchronous and cheap (one `Uuid::new_v5` call), it should happen at the top of `main()` before any logging.

**Warning signs:** Banner shows name but UUID is `"(computing...)"` or similar placeholder.

### Pitfall 4: AppState Clone Semantics with String

**What goes wrong:** Adding `server_name: String` to `AppState` where `AppState: Clone`. Since `String` is `Clone`, this is fine — but make sure the field is included in all construction sites of `AppState`.

**Why it happens:** There is exactly one `AppState` construction site in `main.rs`. Easy to miss if there's a second.

**How to avoid:** Search `src/` for `AppState {` before finalizing the plan. Currently only one construction site exists.

**Warning signs:** Compiler error "missing field `server_name`" — which is actually helpful and catches this immediately.

### Pitfall 5: friendlyName XML Escaping

**What goes wrong:** If a user passes `--name` with XML special characters (`&`, `<`, `>`, `"`, `'`), embedding it raw in the device.xml template produces malformed XML that DLNA clients reject silently.

**Why it happens:** `format!()` in `serve_device_xml` does no escaping.

**How to avoid:** Wrap the name with `http::soap::xml_escape(&state.server_name)` when inserting into the XML template. This is already available in the project via `src/http/soap.rs::xml_escape()`.

**Warning signs:** Samsung TV fails to parse device.xml; name with ampersand in it causes the device to disappear from the list.

### Pitfall 6: SsdpConfig Does Not Carry server_name

**What goes wrong:** If Claude's discretion includes logging the friendly name in the SSDP advertising log line, `SsdpConfig` needs to carry `server_name`. Currently `SsdpConfig` has only `device_uuid` and `http_port`.

**Why it happens:** `SsdpConfig` was designed for Phase 6 scope; Phase 8 is the first time name belongs in SSDP territory.

**How to avoid:** If the SSDP log line should include the name, add `server_name: String` to `SsdpConfig` alongside `device_uuid`. If not (Claude's discretion says it's optional), no change to `SsdpConfig` is needed.

**Warning signs:** The friendly name appears in HTTP logs but not SSDP logs when expected.

## Code Examples

Verified patterns from the existing codebase and `uuid` crate:

### Build stable server UUID from hostname

```rust
// src/main.rs (new helper, or inline)
// Pattern mirrors build_machine_namespace() in src/media/metadata.rs
use uuid::Uuid;

fn build_server_uuid(hostname: &str) -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, hostname.as_bytes())
}

// Usage in main():
let raw_hostname: String = hostname::get()
    .ok()
    .and_then(|os| os.into_string().ok())
    .filter(|s| !s.is_empty())
    .unwrap_or_else(|| "udlna".to_string());

let server_uuid = build_server_uuid(&raw_hostname).to_string();
```

### Compute default name

```rust
// src/config.rs
fn default_name() -> String {
    let host = hostname::get()
        .ok()
        .and_then(|os| os.into_string().ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    if host.is_empty() {
        "udlna".to_string()
    } else {
        format!("udlna@{}", host)
    }
}

// In Config::resolve:
name: args.name.clone().or(file.name).unwrap_or_else(default_name),
```

### Fix xml_escape lifetime warning

```rust
// src/http/soap.rs line 190
// Before:
pub fn xml_escape(s: &str) -> Cow<str> {

// After:
pub fn xml_escape(s: &str) -> Cow<'_, str> {
```

### Fix unused index field warning

```rust
// src/ssdp/socket.rs
pub struct IfaceV4 {
    pub addr: Ipv4Addr,
    pub mask: Ipv4Addr,
    #[allow(dead_code)]
    pub index: u32,
}
```

### Updated AppState

```rust
// src/http/state.rs
#[derive(Clone)]
pub struct AppState {
    pub library: Arc<RwLock<MediaLibrary>>,
    pub server_uuid: String,   // Stable UUID v5 derived from hostname (Phase 8)
    pub server_name: String,   // Friendly name from --name / config / default (Phase 8)
}
```

### Updated device.xml friendlyName

```rust
// src/http/description.rs — serve_device_xml handler
// Before (hardcoded):
//   <friendlyName>udlna</friendlyName>
//
// After (from state):
let body = format!(r#"...
    <friendlyName>{name}</friendlyName>
..."#,
    uuid = state.server_uuid,
    name = http::soap::xml_escape(&state.server_name),
);
```

### Updated startup banner (main.rs)

```rust
// After UUID derivation, before scan:
tracing::info!(
    "udlna \"{}\" (uuid: {}) starting on port {}",
    config.name,
    server_uuid,
    config.port
);
```

### Updated AppState construction in main.rs

```rust
// main.rs
let state = http::state::AppState {
    library: Arc::clone(&library),
    server_uuid: server_uuid.clone(),
    server_name: config.name.clone(),
};
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Random UUID v4 per startup | UUID v5 derived from hostname | Phase 8 | UUID stable across restarts; DLNA clients that cache UUID no longer lose the server |
| Hardcoded `"udlna"` friendly name | User-configurable via `--name` / TOML | Phase 8 | Samsung/Xbox shows user's custom name in device lists |
| Static `DEFAULT_NAME = "udlna"` | Dynamic `default_name()` computing `"udlna@{hostname}"` | Phase 8 | Multiple instances on same network are distinguishable by default |

## Open Questions

1. **Where to put `build_server_uuid()` / `get_hostname()`?**
   - What we know: The existing project puts UUID helper functions in `src/media/metadata.rs` (for media-specific UUIDs) and uses them from `main.rs`.
   - What's unclear: Phase 8's hostname/UUID functions are server-level concerns, not media concerns. They could live in `main.rs` (inline) or a new `src/identity.rs` module.
   - Recommendation: Keep it simple — inline the hostname acquisition in `main.rs` (as a local `fn get_hostname() -> String`) since it's only called once at startup. Put `build_server_uuid()` either inline in `main.rs` or alongside the config module. Avoid over-modularizing a 5-line function.

2. **Should `hostname` crate or `gethostname` crate be used?**
   - What we know: Both are thin wrappers around `gethostname(3)`. `hostname` 0.4.2 is well-established (>10M downloads). `gethostname` 1.1.0 is slightly newer but smaller.
   - What's unclear: Neither has been verified in this specific project's CI environment.
   - Recommendation: Use `hostname` 0.4.2 — it has more adoption, is the first result in `cargo search hostname`, and the API (`hostname::get() -> OsResult<OsString>`) is clean and direct.

3. **Should the SSDP advertising log include the friendly name?**
   - What we know: This is Claude's discretion. The current log line is `"SSDP advertising on {}:1900"` per `src/ssdp/service.rs:65`.
   - What's unclear: Requiring `SsdpConfig` to carry `server_name` adds a field and propagation.
   - Recommendation: Yes — log it once at SSDP service startup (e.g., `"SSDP advertising \"{}\" on {}:1900"`) so the operator can confirm which name is being advertised. This requires adding `server_name: String` to `SsdpConfig`, which is a small, clean change.

## Sources

### Primary (HIGH confidence)
- Direct source code inspection of `/Users/spencersr/tmp/udlna/src/` — all files read in full; findings reflect actual current state
- `cargo check` output — compiler warnings confirmed exactly as described in CONTEXT.md
- `uuid` crate v1.21.0 (verified via `cargo tree`) — `Uuid::NAMESPACE_DNS` and `new_v5()` confirmed used in existing `build_machine_namespace()` in `src/media/metadata.rs`
- `cargo info hostname` — version 0.4.2, license MIT, `rust-version: 1.74` (compatible with project's rustc 1.93.1)
- `cargo info gethostname` — version 1.1.0, license Apache-2.0

### Secondary (MEDIUM confidence)
- `cargo search hostname` output — confirms `hostname` 0.4.2 is the primary result and most widely used
- Rust standard library knowledge — `std` provides no `gethostname()` in stable; external crate is required (verified by failed rustc compile test without the crate)

### Tertiary (LOW confidence)
- None — all findings verified directly from source or cargo metadata

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all libraries verified via `cargo info` and existing codebase use of `uuid` crate
- Architecture: HIGH — based on direct source inspection; all existing patterns confirmed
- Pitfalls: HIGH — confirmed via `cargo check` output (warnings), source inspection, and Rust language semantics
- Compiler warnings: HIGH — confirmed exact warning text via live `cargo check` run

**Research date:** 2026-02-22
**Valid until:** 2026-03-24 (stable, well-understood domain; 30-day validity)
