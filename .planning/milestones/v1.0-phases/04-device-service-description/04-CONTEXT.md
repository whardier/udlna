# Phase 4: Device & Service Description - Context

**Gathered:** 2026-02-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Serve the UPnP device description XML (`/device.xml`) and SCPD service description documents (`/cds/scpd.xml`, `/cms/scpd.xml`) that DLNA clients fetch to discover what this server offers. Phase 3 already has 501 stubs at these routes — this phase replaces them with real XML responses. Network discovery (SSDP), SOAP action handling, and server identity customization are later phases.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

The user reviewed all gray areas and deferred all decisions to Claude. The planner has full discretion on:

- **Server identity fields** — friendlyName, manufacturer, manufacturerURL, modelName, modelNumber, modelDescription, serialNumber. Use sensible defaults consistent with the project (`udlna`, `udlna project`, etc.). Hard-code for Phase 4; Phase 8 adds the `--name` flag that customizes friendlyName.

- **SCPD completeness** — Include full spec-compliant state variable tables and argument definitions for ContentDirectory (Browse, GetSearchCapabilities, GetSortCapabilities, GetSystemUpdateID) and ConnectionManager (GetProtocolInfo, GetCurrentConnectionIDs, GetCurrentConnectionInfo). Full compliance is better than minimal — DLNA clients validate these documents.

- **Client compatibility** — Include `dlna:X_DLNADOC` with `DMS-1.50`, `URLBase` if needed for client compat, and `presentationURL` as a stub. Follow what open-source DLNA server implementations (MiniDLNA, dms) include for broadest client support.

- **XML generation** — Embedded static string templates with minimal runtime interpolation (just the UUID and base URL). No XML builder crate needed for mostly-static documents.

- **UDN (Unique Device Name)** — For Phase 4, use a placeholder UUID that Phase 8 will replace with the stable UUIDv5-from-hostname. The planner decides whether to use a fixed dev UUID or read from a future config field.

</decisions>

<specifics>
## Specific Ideas

- Phase 3 stubs are at exactly `/device.xml`, `/cds/scpd.xml`, `/cms/scpd.xml`, `/cms/control`, `/cds/control` — Phase 4 replaces the description routes; control routes remain 501 for Phase 5/7.
- Phase 8 is "Server Identity & Customization" — don't over-engineer the name/UUID plumbing now, just make it easy to replace in Phase 8.

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

*Phase: 04-device-service-description*
*Context gathered: 2026-02-23*
