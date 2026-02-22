# Phase 6: SSDP Discovery - Context

**Gathered:** 2026-02-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement SSDP so DLNA clients on the local network automatically discover the server without configuration. Scope includes: UDP multicast listener on 239.255.255.250:1900 (IPv4) and ff02::c:1900 (IPv6), M-SEARCH response, NOTIFY alive on startup + periodic re-advertisement, NOTIFY byebye on Ctrl+C shutdown. HTTP server and media scanning are already implemented — this phase wires network discovery on top of them.

</domain>

<decisions>
## Implementation Decisions

### Network interface scope
- Advertise on the same interfaces the HTTP server binds to (match HTTP listener interfaces), not all interfaces
- Both IPv4 (239.255.255.250:1900) and IPv6 (ff02::c:1900) SSDP — dual-stack, consistent with HTTP
- LOCATION URL in SSDP responses must use the IP of the interface the M-SEARCH arrived on (derive from incoming packet's interface — most accurate for multi-NIC machines)
- If no non-loopback interfaces exist: start anyway, log a warning — HTTP still works, SSDP gracefully degrades

### Shutdown behavior
- On Ctrl+C: wait for byebye messages to send, with a timeout (~1s) — ensures clients receive the notification and clean up
- Second Ctrl+C during shutdown wait: force-exit immediately
- Send byebye for all advertised USN types (root device + MediaServer:1 + CDS:1 + CMS:1) — spec-correct, clients clean up fully
- SIGKILL with no byebye: acceptable — clients will time out via cache-control TTL. No watchdog or pid file needed.

### Advertisement & timing
- Cache-control max-age: **900 seconds** (matches re-advertisement interval — tight TTL, clients expire faster on ungraceful exit)
- Re-advertisement interval: 900 seconds (from roadmap)
- Startup NOTIFY burst: 2-3 times with small delays between each (100-200ms) — avoids flooding, ensures reliability
- USN types advertised: full set — root device + `urn:schemas-upnp-org:device:MediaServer:1` + `urn:upnp-org:serviceId:ContentDirectory` + `urn:upnp-org:serviceId:ConnectionManager`
- M-SEARCH responses: unicast back to the requesting client only (spec-correct)

### Startup sequencing
- Startup order: media scan → HTTP server ready → SSDP NOTIFY burst
  - Scan completes first (already the case from Phase 3)
  - HTTP must be accepting connections before SSDP advertises (no race window)
  - SSDP advertises last — clients that immediately fetch /device.xml after discovery will get a valid response
- Log SSDP status to stdout on startup: print the interface address(es) being advertised on (e.g. "SSDP advertising on 192.168.1.5:1900") — helps users confirm which interface is active
- If SSDP socket binding fails (port 1900 already in use): abort with a clear error message — do not silently fall back to HTTP-only mode

</decisions>

<specifics>
## Specific Ideas

- The LOCATION URL IP must match the interface the M-SEARCH came in on — important for multi-NIC machines where the wrong IP would make /device.xml unreachable to the client
- Startup log line showing the SSDP interface is explicitly desired for operational visibility

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 06-ssdp-discovery*
*Context gathered: 2026-02-22*
