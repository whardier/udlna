---
phase: 06-ssdp-discovery
plan: 01
subsystem: networking
tags: [ssdp, upnp, multicast, udp, socket2, getifaddrs, rust]

# Dependency graph
requires:
  - phase: 05-contentdirectory-service
    provides: Working HTTP DLNA server with ContentDirectory SOAP — SSDP advertises on top of this

provides:
  - src/ssdp/socket.rs with build_recv_socket_v4, build_recv_socket_v6, build_send_socket, list_non_loopback_v4, find_iface_for_sender
  - src/ssdp/messages.rs with notify_alive, notify_byebye, msearch_response, usn_set (5 USN types)
  - src/ssdp/mod.rs module declarations
  - getifaddrs 0.6 and tokio signal/time/sync features in Cargo.toml

affects:
  - 06-02 (SSDP service task that wires these building blocks)
  - All future SSDP/discovery work

# Tech tracking
tech-stack:
  added:
    - getifaddrs 0.6 (cross-platform network interface enumeration)
    - tokio signal feature (for Ctrl+C shutdown handling in Plan 02)
    - tokio time feature (for re-advertisement timer in Plan 02)
    - tokio sync feature (for channel-based shutdown coordination in Plan 02)
  patterns:
    - socket2 for low-level socket creation (SO_REUSEADDR + SO_REUSEPORT, set_only_v6, multicast join)
    - CRLF (\r\n) line endings in all SSDP messages (required by UPnP spec)
    - 5-USN advertisement set for MediaServer:1 (uuid, rootdevice, MediaServer:1, ContentDirectory:1, ConnectionManager:1)
    - Subnet-mask matching for LOCATION URL interface selection (minidlna approach)

key-files:
  created:
    - src/ssdp/socket.rs
    - src/ssdp/messages.rs
    - src/ssdp/mod.rs
    - src/ssdp/service.rs (stub — Plan 02 will implement)
  modified:
    - Cargo.toml (getifaddrs dep + tokio features)
    - src/main.rs (mod ssdp declaration)

key-decisions:
  - "getifaddrs 0.6 used (not 0.4/0.3 from plan spec) — latest available version; Address enum has V4/V6/Mac variants with NetworkAddress { address, netmask, associated_address } fields"
  - "socket2 set_only_v6() used (not set_only_ipv6) — matches existing main.rs convention for IPv6 socket configuration"
  - "service.rs stub created so mod.rs pub mod service declaration compiles — Plan 02 fills it in"
  - "list_non_loopback_v4 uses getifaddrs::Address::V4 match arm — avoids Option<> confusion; index is Option<u32>, unwrap_or(0)"

patterns-established:
  - "SSDP message builders use raw CRLF format strings without leading whitespace on continuation lines (spec-correct)"
  - "Socket creation follows: Socket::new -> set options -> bind -> set_nonblocking -> convert to tokio UdpSocket"
  - "Interface enumeration: filter loopback by flag, match Address::V4 variant for IPv4 only"

requirements-completed: [DISC-01, DISC-02, DISC-03, DISC-04]

# Metrics
duration: 2min
completed: 2026-02-22
---

# Phase 6 Plan 01: SSDP Foundational Module Summary

**SSDP socket helpers (IPv4/IPv6 multicast UDP) and message builders (notify_alive, notify_byebye, msearch_response, 5-USN set) using socket2 and getifaddrs 0.6**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-23T03:18:39Z
- **Completed:** 2026-02-23T03:20:39Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Socket helpers for IPv4 and IPv6 SSDP multicast receive sockets (with SO_REUSEADDR + SO_REUSEPORT)
- General-purpose send socket for NOTIFY and M-SEARCH responses
- Interface enumeration using getifaddrs 0.6 API (filters loopback, extracts IPv4 + netmask + index)
- Subnet-mask-based LOCATION URL selection (find_iface_for_sender)
- SSDP message builders with spec-correct CRLF endings (5 USN types, notify_alive/byebye, msearch_response)
- Cargo.toml updated with getifaddrs 0.6 and tokio signal/time/sync features for Plan 02

## Task Commits

Each task was committed atomically:

1. **Task 1: Cargo.toml -- add getifaddrs dep and tokio signal/time/sync features** - `c433919` (chore)
2. **Task 2: Create src/ssdp/ module -- socket.rs, messages.rs, mod.rs** - `ed7d90d` (feat)

**Plan metadata:** TBD (docs: complete plan)

## Files Created/Modified
- `src/ssdp/socket.rs` - build_recv_socket_v4/v6, build_send_socket, IfaceV4 struct, list_non_loopback_v4, find_iface_for_sender
- `src/ssdp/messages.rs` - usn_set (5 USN types), notify_alive, notify_byebye, msearch_response
- `src/ssdp/mod.rs` - Module declarations (messages, service, socket)
- `src/ssdp/service.rs` - Stub file (Plan 02 implements)
- `Cargo.toml` - Added getifaddrs 0.6, tokio signal/time/sync features
- `src/main.rs` - Added mod ssdp declaration

## Decisions Made
- getifaddrs 0.6 used (latest available; plan spec said 0.4 which doesn't exist on crates.io)
- socket2 uses set_only_v6() not set_only_ipv6() — consistent with main.rs IPv6 TCP socket setup
- service.rs stub created so pub mod service in mod.rs compiles (Plan 02 fills it in)
- list_non_loopback_v4 matches Address::V4 variant directly — getifaddrs 0.6 address field is Address enum, not Option

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed set_only_ipv6 -> set_only_v6 method name**
- **Found during:** Task 2 (build_recv_socket_v6 implementation)
- **Issue:** Plan code used socket.set_only_ipv6(true) but socket2 0.5 exposes set_only_v6()
- **Fix:** Changed to socket.set_only_v6(true) — matches existing usage in main.rs
- **Files modified:** src/ssdp/socket.rs
- **Verification:** cargo check exits 0 after fix
- **Committed in:** ed7d90d (Task 2 commit)

**2. [Rule 1 - Bug] Adapted getifaddrs 0.6 API (version mismatch from plan)**
- **Found during:** Task 2 (list_non_loopback_v4 implementation)
- **Issue:** Plan specified getifaddrs 0.4 API (with .address.and_then(|a| a.as_socket_ipv4())) but crates.io only has 0.6. The 0.6 API exposes address as Address enum (V4/V6/Mac variants) not Option<SocketAddr>. Also: no netmask field on Interface — netmask is inside NetworkAddress. Index is Option<u32> not u32.
- **Fix:** Rewrote list_non_loopback_v4 to match 0.6 API: match &i.address { Address::V4(net_addr) => ... }
- **Files modified:** src/ssdp/socket.rs
- **Verification:** cargo check exits 0; cargo run in test project confirmed V4/V6 output format
- **Committed in:** ed7d90d (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 bugs from API version differences)
**Impact on plan:** Both fixes essential for compilation. No scope creep. Module behavior is correct per spec.

## Issues Encountered
- getifaddrs crate has no 0.3 or 0.4 versions on crates.io; 0.6.0 is the only release. Discovered via cargo search before writing code.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All socket helpers and message builders ready for Plan 02 SSDP service task
- tokio signal/time/sync features available for shutdown handling and re-advertisement timer
- No blockers

---
*Phase: 06-ssdp-discovery*
*Completed: 2026-02-22*
