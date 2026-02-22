# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-23)

**Core value:** Any DLNA device on the local network -- including Samsung TVs and Xbox Series X -- can discover and play your media the moment you run `udlna /path/to/media`.
**Current focus:** Planning next milestone — v1.0 shipped 2026-02-23

## Current Position

Phase: 8 of 8 (Server Identity Customization) — Complete
Plan: 1 of 1 in current phase (08-01 complete — UUID v5 stable identity + --name flag wired end-to-end)
Status: Phase 8 Plan 01 Complete — All phases complete, v1 milestone achieved
Last activity: 2026-02-23 -- Completed 08-01 (stable UUID v5 + server name in device.xml and SSDP; zero warnings)

Progress: [████████████████████] 100%

## Performance Metrics

**Velocity:**
- Total plans completed: 9
- Average duration: 2 min
- Total execution time: 0.55 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01-project-setup-cli | 2/2 | 5 min | 2.5 min |
| 02-media-scanner-and-metadata | 4/4 | 10 min | 2.5 min |

**Recent Trend:**
- Last 5 plans: 2 min
- Trend: stable

*Updated after each plan completion*
| Phase 01-project-setup-cli P01 | 3 | 2 tasks | 7 files |
| Phase 01-project-setup-cli P02 | 2 | 2 tasks | 2 files |
| Phase 02-media-scanner-and-metadata P01 | 2 | 2 tasks | 3 files |
| Phase 02-media-scanner-and-metadata P02 | 3 | 1 tasks | 2 files |
| Phase 02-media-scanner-and-metadata P03 | 3 | 2 tasks | 3 files |
| Phase 02-media-scanner-and-metadata P04 | 2 | 2 tasks | 4 files |
| Phase 03-http-server-file-streaming P01 | 3 | 2 tasks | 7 files |
| Phase 03-http-server-file-streaming P02 | 2 | 2 tasks | 1 files |
| Phase 03-http-server-file-streaming P03 | 15 | 2 tasks | 1 files |
| Phase 04-device-service-description P01 | 3 | 3 tasks | 5 files |
| Phase 04-device-service-description P02 | 1 | 2 tasks | 0 files |
| Phase 05-contentdirectory-service P01 | 2 | 2 tasks | 3 files |
| Phase 05-contentdirectory-service P02 | 1 | 1 tasks | 1 files |
| Phase 05-contentdirectory-service P03 | 5 | 2 tasks | 2 files |
| Phase 05-contentdirectory-service P04 | 4 | 2 tasks | 1 files |
| Phase 06-ssdp-discovery P01 | 2 | 2 tasks | 6 files |
| Phase 06-ssdp-discovery P02 | 3 | 2 tasks | 2 files |
| Phase 06-ssdp-discovery P03 | 5 | 2 tasks | 0 files |
| Phase 07-connectionmanager-service P01 | 3 | 3 tasks | 5 files |
| Phase 08-server-identity-customization P01 | 8 | 2 tasks | 8 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Roadmap]: Build order follows testability -- foundation first, HTTP before SSDP, ContentDirectory before discovery
- [Roadmap]: Media metadata extraction (INDX-01 through INDX-04) placed in Phase 2 so MediaItem structs carry duration/resolution/bitrate from the start
- [Roadmap]: ConnectionManager separated into its own phase (7) since Xbox testing happens after SSDP is working
- [01-01]: Option<u16> for --port (not u16) so absence is detectable during three-layer config merge in Plan 02
- [01-01]: No --log-level flag; RUST_LOG env var exclusively per research recommendation
- [01-01]: classify() uses explicit static match strings, not mime_guess return values (API not stable)
- [01-01]: .srt classified as Subtitle (LOCKED DECISION -- must not be skipped)
- [Phase 01-project-setup-cli]: Option<u16> for --port in Args so absence is detectable during config merge in Plan 02
- [Phase 01-project-setup-cli]: .srt classified as Subtitle (LOCKED) -- never skipped, to be delivered alongside video in Phase 3/5
- [Phase 01-02]: FileConfig uses Option<T> fields only — absent TOML keys never fail deserialization
- [Phase 01-02]: No #[serde(deny_unknown_fields)] on FileConfig — unknown future TOML keys silently ignored for forward compatibility
- [Phase 01-02]: Malformed config file logs warn and continues — startup not blocked when user passed valid CLI paths
- [Phase 02-01]: dlna_profile is Option<&'static str> — None means omit DLNA.ORG_PN entirely, never use wildcard "*"
- [Phase 02-01]: MediaItem.path stores canonicalize() result (resolved symlinks), not raw walkdir path
- [Phase 02-01]: MediaLibrary.items must never contain Subtitle kind items (filtered at scan time in Plan 02)
- [Phase 02-media-scanner-and-metadata]: video/mp4 returns None from dlna_profile_for — DLNA profile assignment deferred to Phase 5
- [Phase 02-media-scanner-and-metadata]: image/jpeg and image/png use unconditional JPEG_LRG/PNG_LRG — no size-tier logic in Phase 2
- [Phase 02-media-scanner-and-metadata]: Non-MP4 video resolution is None — symphonia does not expose video frame dimensions (Pitfall 2); resolution omitted per plan spec
- [Phase 02-media-scanner-and-metadata]: Audio bitrate derived from bits_per_coded_sample when available; None otherwise
- [Phase 02-media-scanner-and-metadata]: _library underscore prefix suppresses unused-variable warning until Phase 3 passes Arc::clone(&library) to HTTP server
- [Phase 02-media-scanner-and-metadata]: Targeted #[allow(dead_code)] on MediaMeta, MediaItem, ScanStats (fields consumed by Phase 3) rather than re-adding file-level suppressor
- [Phase 03-01]: AppState uses std::sync::RwLock (write-once at startup); switch to tokio::sync::RwLock if Phase 6 SIGHUP rescan needed
- [Phase 03-01]: localhost flag is a plain bool in Args — merged with TOML Option<bool> via args.localhost || file.localhost.unwrap_or(false)
- [Phase 03-01]: All 6 DLNA routes declared in build_router upfront; Phase 4/5 stubs return 501 inline closures
- [Phase 03-01]: axum 0.8 {id} path syntax used — breaking change from 0.7 :id syntax confirmed
- [Phase 03-02]: FileStream::try_range_response not used — manual seek+take pattern gives full control over DLNA header injection on 206 responses
- [Phase 03-02]: http-range-header validates overlapping ranges at validate() — all validation errors uniformly return 416
- [Phase 03-02]: MediaItem.mime is &'static str — HeaderValue::from_static(item.mime) used directly (no allocation)
- [Phase 03-02]: Content-Length overridden to partial length (end - start + 1) on 206 responses for correct transfer
- [Phase 03-03]: IPV6_V6ONLY explicitly set to true via TcpSocket::set_only_v6(true) for portable dual-bind across Linux and macOS
- [Phase 03-03]: IPv4 and IPv6 listeners run as separate tokio::spawn tasks joined with tokio::join! — no shared shutdown signal needed for Phase 3
- [Phase 03-03]: Synchronous scan() before server startup is safe because server cannot receive requests until after tokio::spawn
- [Phase 04-device-service-description]: uuid crate v4 feature added explicitly — uuid 1.x requires opt-in feature for new_v4()
- [Phase 04-device-service-description]: serviceId uses urn:upnp-org:serviceId (not urn:schemas-upnp-org) to avoid known PyMedS-ng bug
- [Phase 04-device-service-description]: URLBase omitted from device.xml — deprecated in UPnP 1.1, complex with dual-stack (RESEARCH.md Pitfall 3)
- [Phase 04-device-service-description]: Static const &str for SCPD XML — no XML builder crate needed for fully-static documents
- [Phase 05-01]: extract_soap_param uses simple string-find (Approach A) — avoids quick-xml serde namespace complexity for short SOAP bodies
- [Phase 05-01]: format_dc_date uses chrono for SystemTime->YYYY-MM-DD conversion rather than hand-rolling calendar math
- [Phase 05-01]: soap_fault returns tuple (StatusCode, header array, String) for direct IntoResponse use by callers
- [Phase 05-contentdirectory-service]: Post-hoc TDD verification: all 33 tests passed immediately — Plan 01 implementation was correct, no bug fixes needed
- [Phase 05-03]: ok_xml() inline helper wraps soap_response into HTTP 200 + text/xml tuple — avoids repeating in every action handler
- [Phase 05-03]: handle_browse is async fn so Plan 04 can add await calls without changing the dispatch signature
- [Phase 05-03]: SOAPAction body fallback scans for <u: namespace prefix to extract action name per RESEARCH.md Pitfall 3
- [Phase 05-contentdirectory-service]: didl_lite_wrap() outputs inline single-line DIDL-Lite with all four namespaces — avoids whitespace issues in some DLNA parsers
- [Phase 05-contentdirectory-service]: BrowseMetadata on item UUID: parent container determined from item.kind (Video→videos_id, Audio→music_id, Image→photos_id)
- [Phase 06-ssdp-discovery]: getifaddrs 0.6 used (not 0.4) — only available version; Address enum V4/V6/Mac variants, netmask inside NetworkAddress
- [Phase 06-ssdp-discovery]: socket2 set_only_v6() for IPv6-only socket mode — consistent with main.rs TCP IPv6 socket setup
- [Phase 06-ssdp-discovery]: service.rs stub created for Plan 02 — allows mod.rs pub mod service to compile before Plan 02 implementation
- [06-02]: Separate buf_v4/buf_v6 buffers in select! — borrow checker rejects two simultaneous &mut borrows of same buffer across select! arms
- [06-02]: recv_v6_from() returns std::future::pending when IPv6 socket is None — cleanly disables IPv6 select! branch without conditional compilation
- [06-02]: tokio::spawn(async move { axum::serve().with_graceful_shutdown().await }) — WithGracefulShutdown is not a Future; must be awaited inside async block
- [06-02]: SSDP task spawned after TcpListener::bind (not after await) — HTTP listener already accepting before SSDP startup burst fires
- [06-03]: Python raw UDP socket used for M-SEARCH verification — zero dependencies, portable macOS/Linux, identical network behavior to real DLNA clients
- [06-03]: No code changes required — Phase 06-02 SSDP implementation was correct on first real-network test with Samsung TVs present
- [07-01]: UPnP error 401 for unknown CMS action (not 402 InvalidArgs — CDS copy-paste pitfall)
- [07-01]: Status=OK in GetCurrentConnectionInfo — CONTEXT.md locked decision overrides minidlna's "Unknown"
- [07-01]: soap_response() delegates to soap_response_ns(CDS_NAMESPACE) for backwards compatibility with all CDS callers
- [08-01]: UUID v5 uses Uuid::NAMESPACE_DNS with raw hostname bytes — mirrors build_machine_namespace() in media/metadata.rs
- [08-01]: default_name() is a fn not a const — hostname is only available at runtime, not compile time
- [08-01]: xml_escape applied to server_name in description.rs — protects against XML injection from user --name values
- [08-01]: Two SsdpConfig construction sites in main.rs (localhost + dual-bind paths) both updated with server_name

### Pending Todos

None yet.

### Blockers/Concerns

- Samsung/Xbox behavioral quirks are MEDIUM confidence; real-device testing needed starting Phase 5

## Session Continuity

Last session: 2026-02-23
Stopped at: Completed 08-01-PLAN.md (UUID v5 stable identity + --name end-to-end; zero warnings; 81 tests pass; v1 complete)
Resume file: None
