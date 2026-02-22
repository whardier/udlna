# Phase 7: ConnectionManager Service - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement the UPnP ConnectionManager Service (CMS) SOAP control endpoint with three actions: GetProtocolInfo, GetCurrentConnectionIDs, and GetCurrentConnectionInfo. Strict DLNA clients (Xbox Series X and equivalents) require this service before they will browse or stream. This phase does not include subtitle handling or new media classification capabilities.

</domain>

<decisions>
## Implementation Decisions

### Client Compatibility
- Spec compliance IS Xbox compatibility for these simple stubs — no Xbox-specific quirks to accommodate
- Automated curl/Python SOAP tests are sufficient — no specific client device required for verification
- Response field names, order, and values should match minidlna exactly — maximizes compatibility with clients tested against known-good servers

### Protocol Info Content
- GetProtocolInfo list must be derived dynamically from the project's MIME module — stays in sync automatically with what the server actually serves
- Cover all three media categories: video, audio, and image
- DLNA.ORG_PN profile tags in the 4th field: Claude's discretion (wildcard vs. profile tags)

### Service Wiring
- Planner should audit device.xml and add connectionmanager.xml, /cms/control route, and any device.xml references that are currently missing
- Handler lives in `src/cms/` module (not a single file in src/handlers/) — gives room to grow
- Share the SOAP parsing infrastructure with ContentDirectory — reuse or extract the SOAP action parser to avoid duplication

### Claude's Discretion
- DLNA.ORG_PN profile tag specificity (wildcard `*` vs. named profiles like `AVC_MP4_BL_CIF15`)
- Exact SOAP infrastructure refactoring strategy (extract shared module vs. pass parser by reference)

</decisions>

<specifics>
## Specific Ideas

- Match minidlna's exact response field names and values for all three CMS actions — treat minidlna as the reference implementation
- The SOAP parser should be shared infrastructure, not duplicated in each handler

</specifics>

<deferred>
## Deferred Ideas

- Subtitle sidecar file alignment — sidecar subtitle files (e.g., .srt) need better alignment with the original media item when not embedded in the stream; this is its own phase
- Embedded subtitle advertisement — subtitles embedded in media streams need to be properly advertised as available to clients; this is its own phase

</deferred>

---

*Phase: 07-connectionmanager-service*
*Context gathered: 2026-02-22*
