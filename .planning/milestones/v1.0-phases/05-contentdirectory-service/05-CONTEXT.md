# Phase 5: ContentDirectory Service - Context

**Gathered:** 2026-02-23
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement the SOAP ContentDirectory service at `/cds/control`: parse Browse requests, generate DIDL-Lite XML responses, handle pagination, and return spec-correct SOAP faults for error cases. GetSearchCapabilities, GetSortCapabilities, and GetSystemUpdateID are also in scope. No database — all state is derived at runtime from the in-memory `MediaLibrary` built in Phase 2. Connection Manager SOAP stubs and SSDP are later phases.

</domain>

<decisions>
## Implementation Decisions

### Content organization
- **Hierarchical by type** — not flat. Root ObjectID "0" returns four containers:
  - Videos (UUIDv5)
  - Music (UUIDv5)
  - Photos (UUIDv5)
  - All Media (UUIDv5)
- Each container lists only its type's items (Videos → MediaKind::Video, etc.); "All Media" lists everything
- Container ObjectIDs use **UUIDv5** — same pattern as media items: `uuid5(machine_namespace, container_name_string)`. No integer IDs, no database, fully deterministic across restarts
- BrowseMetadata on a container returns the container itself as a DIDL-Lite `<container>` element with correct `childCount`
- BrowseMetadata on "0" (root) returns the root container with childCount=4

### DIDL-Lite metadata richness
- **Full metadata** — every available field from Phase 2's MediaMeta:
  - `dc:title` — filename **without** extension (strip `.mkv`, `.mp3`, etc.)
  - `upnp:class` — type-specific: `object.item.videoItem`, `object.item.audioItem.musicTrack`, `object.item.imageItem.photo`
  - `res` — full URL to Phase 3 streaming endpoint (`http://{host}:{port}/media/{uuid}`)
  - `res@size` — always included (file_size from Phase 2)
  - `res@duration` — included when available from MediaMeta (HH:MM:SS.mmm format from Phase 2)
  - `res@resolution` — included when available (WxH)
  - `res@bitrate` — included when available
  - `dc:date` — file modification time from filesystem (ISO 8601)
- **Album art / thumbnails**: Claude's discretion — skip for Phase 5, keep scope focused

### protocolInfo construction
- **Full DLNA string when profile known**: `http-get:*:{mime}:DLNA.ORG_PN={profile};DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000`
- **Minimal when profile is None**: `http-get:*:{mime}:DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000` (omit DLNA.ORG_PN entirely)
- Use the same `DLNA.ORG_FLAGS` constant as Phase 3 (already in codebase)
- `res@protocolInfo` uses this constructed string

### SOAP error handling
- All edge cases deferred to Claude's discretion:
  - Unknown ObjectID → Claude picks (701 NoSuchObject is spec-correct; recommend this)
  - Malformed SOAP → Claude picks (strict 402 InvalidArgs recommended to match roadmap success criteria)
  - GetSearchCapabilities → empty string (roadmap mandates this)
  - GetSortCapabilities → empty string (roadmap mandates this)
  - GetSystemUpdateID → fixed non-zero integer (1) — deterministic, no state needed

### UUID / identity (confirmed as existing pattern)
- Media item ObjectIDs = their Phase 2 UUIDv5 IDs (already stored in `MediaItem.id`)
- Container ObjectIDs = UUIDv5(machine_namespace, container_name) — same `build_machine_namespace()` from Phase 2
- No database, no persistence — everything computed from in-memory library on every request
- The DLNA `parentID` for root containers = "0"; for items = their container's UUID

### Host URL for res elements
- The `<res>` URL must include the server's actual IP and port
- The host/port comes from `AppState` or the incoming request's `Host` header
- Claude decides the best approach (Host header is most portable for dual-bind)

</decisions>

<specifics>
## Specific Ideas

- User was explicit: **no database, no ingest step** — everything must be derivable from the in-memory `MediaLibrary` at request time
- Container UUIDs must be stable — same container always gets the same ObjectID across restarts (UUIDv5 guarantees this)
- Phase 2's `build_machine_namespace()` and `media_item_id()` are already implemented — the same functions should be used or extended for container IDs
- DIDL-Lite XML must be properly escaped — titles with `&`, `<`, `>` in filenames must not break the response XML

</specifics>

<deferred>
## Deferred Ideas

- Album art / thumbnails — future enhancement (not Phase 5)
- Search support (GetSearchCapabilities returning actual criteria) — out of scope for Phase 5

</deferred>

---

*Phase: 05-contentdirectory-service*
*Context gathered: 2026-02-23*
