# Phase 8: Server Identity & Customization - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Give the server a stable, derived UUID (no state file) and a user-customizable friendly name that appears on DLNA client device lists (device.xml, SSDP NOTIFY/M-SEARCH responses). Adds `--name` CLI flag, `name` TOML config key, and wires the name through all relevant layers. Also fixes the two existing Rust compiler warnings from previous phases.

</domain>

<decisions>
## Implementation Decisions

### Default Name
- Default friendly name when `--name` is not provided and no config: `"udlna@{hostname}"` (e.g., `"udlna@macbook"`)
- If hostname is unavailable, empty, or resolves to something generic: fall back to `"udlna"` alone
- The default is meaningful on a home network where multiple instances might run

### UUID Derivation
- UUID v5 derived from **hostname only** (not the full friendly name)
- This means UUID stays stable even when the user changes `--name`
- UUID changes only if the hostname changes — which is rare and always a clean transition (byebye is sent on shutdown)
- No need to warn users about UUID changes

### Config File Integration
- `name` is settable in `udlna.toml` (consistent with `port` and other flags)
- Precedence: **CLI `--name` > TOML `name` > default (`"udlna@hostname"`)**
- Matches the existing three-layer merge pattern already in the project

### Startup & Log Visibility
- Startup banner must show both the friendly name and UUID:
  - e.g., `udlna "Shane uDLNA" (uuid: 9f27398b-...) on 192.168.4.111:8200`
- Log placement beyond the startup banner: Claude's discretion

### Compiler Warnings (fix in this phase)
- Fix `IfaceV4.index` unused field warning in `src/ssdp/socket.rs`
- Fix `xml_escape` lifetime annotation warning in `src/http/soap.rs` (`Cow<str>` → `Cow<'_, str>`)
- Zero compiler warnings must be the outcome of this phase

### Claude's Discretion
- Whether the friendly name appears in the SSDP advertising log line (in addition to startup banner)
- Exact format of the startup banner log line (beyond the name + UUID requirement)

</decisions>

<specifics>
## Specific Ideas

- User tested with `--name "Shane uDLNA"` before the flag was implemented — the name should flow cleanly from CLI through device.xml and SSDP with no extra configuration
- UUID stability within a session is a hard requirement — same UUID must appear in device.xml and all SSDP messages during a single run

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 08-server-identity-customization*
*Context gathered: 2026-02-22*
