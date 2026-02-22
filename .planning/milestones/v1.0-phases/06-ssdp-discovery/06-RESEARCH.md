# Phase 6: SSDP Discovery - Research

**Researched:** 2026-02-22
**Domain:** SSDP/UPnP Discovery over UDP multicast, Tokio signal handling, graceful shutdown
**Confidence:** HIGH (protocol formats from UPnP spec + minidlna source; Rust APIs from official docs)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Network interface scope:**
- Advertise on the same interfaces the HTTP server binds to (match HTTP listener interfaces), not all interfaces
- Both IPv4 (239.255.255.250:1900) and IPv6 (ff02::c:1900) SSDP — dual-stack, consistent with HTTP
- LOCATION URL in SSDP responses must use the IP of the interface the M-SEARCH arrived on (derive from incoming packet's interface — most accurate for multi-NIC machines)
- If no non-loopback interfaces exist: start anyway, log a warning — HTTP still works, SSDP gracefully degrades

**Shutdown behavior:**
- On Ctrl+C: wait for byebye messages to send, with a timeout (~1s) — ensures clients receive the notification and clean up
- Second Ctrl+C during shutdown wait: force-exit immediately
- Send byebye for all advertised USN types (root device + MediaServer:1 + CDS:1 + CMS:1) — spec-correct, clients clean up fully
- SIGKILL with no byebye: acceptable — clients will time out via cache-control TTL. No watchdog or pid file needed.

**Advertisement & timing:**
- Cache-control max-age: **900 seconds** (matches re-advertisement interval)
- Re-advertisement interval: 900 seconds
- Startup NOTIFY burst: 2-3 times with small delays between each (100-200ms)
- USN types advertised: full set — root device + `urn:schemas-upnp-org:device:MediaServer:1` + `urn:upnp-org:serviceId:ContentDirectory` + `urn:upnp-org:serviceId:ConnectionManager`
- M-SEARCH responses: unicast back to the requesting client only (spec-correct)

**Startup sequencing:**
- Startup order: media scan → HTTP server ready → SSDP NOTIFY burst
- HTTP must be accepting connections before SSDP advertises (no race window)
- SSDP advertises last — clients that immediately fetch /device.xml after discovery will get a valid response
- Log SSDP status to stdout on startup: print the interface address(es) being advertised on
- If SSDP socket binding fails (port 1900 already in use): abort with a clear error message

### Claude's Discretion

None specified — all key decisions are locked.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| DISC-01 | Server responds to SSDP M-SEARCH multicast with a unicast reply containing the correct LOCATION URL | M-SEARCH parsing and unicast response format documented in Standard Stack + Code Examples |
| DISC-02 | Server sends SSDP NOTIFY alive on startup (sent 2-3 times for UDP reliability) | NOTIFY alive format and burst pattern documented in Architecture Patterns |
| DISC-03 | Server sends SSDP NOTIFY alive periodically (every 900 seconds) so clients that start after the server still discover it | Re-advertisement timer via `tokio::time::interval` documented in Architecture Patterns |
| DISC-04 | Server sends SSDP NOTIFY byebye on Ctrl+C shutdown so clients remove it from their device lists | Graceful shutdown pattern with `tokio::signal::ctrl_c` + byebye-before-exit documented in Code Examples |
| CLI-06 | Server runs until Ctrl+C; shutdown is graceful (SSDP byebye before exit) | Shutdown sequence using `with_graceful_shutdown` + cancellation token documented in Architecture Patterns |
</phase_requirements>

---

## Summary

SSDP (Simple Service Discovery Protocol) is a UDP-based text protocol following the HTTPU convention. It uses multicast address 239.255.255.250:1900 for IPv4. Devices announce themselves via NOTIFY messages and respond to M-SEARCH queries from clients. The protocol is defined in UPnP Device Architecture v1.1 and is straightforward to implement directly — no third-party SSDP crate is needed.

The key implementation challenge is determining the correct LOCATION URL IP when multiple network interfaces exist. Tokio's `recv_from` does not expose the receiving interface (IP_PKTINFO), so the recommended approach (used by minidlna) is subnet-mask matching: compare the sender's IP against each known interface's address/mask to find which interface likely received the packet. This is accurate for typical home networks.

The graceful shutdown (CLI-06 + DISC-04) requires restructuring `main.rs` to use a shutdown signal channel, route the signal to both the axum server (via `with_graceful_shutdown`) and the new SSDP task (via `CancellationToken` or broadcast channel), and ensure byebye messages are sent before the process exits. Tokio must gain the `signal` feature in `Cargo.toml`.

**Primary recommendation:** Implement SSDP as a new `src/ssdp/` module, bound to a per-phase boot sequence in `main.rs` using a `tokio::sync::broadcast` shutdown channel. Use `socket2` (already in Cargo.toml) for socket creation with `SO_REUSEADDR` + `SO_REUSEPORT` before handing off to tokio's `UdpSocket::from_std`.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio::net::UdpSocket` | tokio 1.x (already in Cargo.toml) | Async UDP send/receive for SSDP | Built into tokio; has `join_multicast_v4`, `join_multicast_v6`, `recv_from`, `send_to` |
| `socket2` | 0.5 (already in Cargo.toml) | Create raw socket, set SO_REUSEADDR/SO_REUSEPORT, then convert to tokio | Needed because tokio::net::UdpSocket doesn't expose socket options before bind |
| `tokio::signal` | tokio 1.x (needs `signal` feature added) | `ctrl_c()` for graceful shutdown trigger | Official tokio signal API; cross-platform |
| `tokio::time` | tokio 1.x (needs `time` feature added) | `interval()` for 900s re-advertisement timer | Official tokio timer API |
| `tokio::sync::broadcast` | tokio 1.x (needs `sync` feature added) | Shutdown channel from main → SSDP task + HTTP server | Multi-receiver channel; clean shutdown coordination |
| `getifaddrs` | 0.4 | Enumerate non-loopback IPv4 interface addresses for LOCATION URL selection | Cross-platform (Linux, macOS, Windows); simple API |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio::time::timeout` | tokio 1.x | Bound byebye wait to ~1 second on shutdown | Prevents hang if socket is slow |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Custom SSDP implementation | `upnp-rs` or `cotton-ssdp` crates | Third-party crates add dependency; SSDP is ~200 lines of message formatting over UDP — not worth a dependency |
| `getifaddrs` crate | `nix::ifaddrs::getifaddrs` | `nix` is Unix-only; `getifaddrs` is cross-platform and simpler API |
| `getifaddrs` crate | `network-interface` crate | Both work; `getifaddrs` is purpose-built and lighter |

### Installation

```bash
# Add to Cargo.toml:
# getifaddrs = "0.4"
# Update tokio features:
# tokio = { version = "1", features = ["macros", "rt-multi-thread", "fs", "net", "io-util", "signal", "time", "sync"] }
```

**CRITICAL — missing tokio features:** The current `Cargo.toml` has `tokio` with `["macros", "rt-multi-thread", "fs", "net", "io-util"]`. Phase 6 requires adding `"signal"` (for `tokio::signal::ctrl_c`), `"time"` (for `tokio::time::interval`), and `"sync"` (for `tokio::sync::broadcast`).

---

## Architecture Patterns

### Recommended Module Structure

```
src/
├── ssdp/
│   ├── mod.rs          # pub mod; re-exports SsdpHandle and ssdp_start()
│   ├── socket.rs       # Socket creation helpers: build_recv_socket(), build_send_socket()
│   ├── messages.rs     # NOTIFY alive/byebye and M-SEARCH response builders (pure string formatting)
│   └── service.rs      # Main SSDP async task: recv loop, re-advertisement timer, shutdown
├── main.rs             # Updated: shutdown channel, startup sequencing, wait for byebye
└── (existing modules unchanged)
```

### Pattern 1: Socket Setup with socket2

**What:** Create a UDP socket via `socket2`, set `SO_REUSEADDR` (+ `SO_REUSEPORT` on macOS), join the multicast group, then convert to tokio.

**When to use:** Every time — tokio's `UdpSocket::bind` doesn't expose pre-bind socket options.

**Binding address:** On Linux and macOS (Unix), bind to the multicast address `239.255.255.250:1900` to filter at the kernel level. On Windows, bind to `0.0.0.0:1900`. The project targets macOS/Linux so bind to the multicast address.

```rust
// Source: bluejekyll.github.io/blog/posts/multicasting-in-rust/ + socket2 docs
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;

fn build_ssdp_recv_socket(iface_addr: Ipv4Addr) -> anyhow::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;  // Required on macOS; harmless on Linux
    let multicast_addr: std::net::SocketAddr =
        SocketAddrV4::new(Ipv4Addr::new(239, 255, 255, 250), 1900).into();
    socket.bind(&multicast_addr.into())?;
    socket.set_nonblocking(true)?;
    let std_socket: std::net::UdpSocket = socket.into();
    let tokio_socket = UdpSocket::from_std(std_socket)?;
    // Join multicast group on specific interface
    tokio_socket.join_multicast_v4(Ipv4Addr::new(239, 255, 255, 250), iface_addr)?;
    Ok(tokio_socket)
}
```

### Pattern 2: SSDP Message Formats

**NOTIFY alive** (sent on startup burst + every 900s for each USN type):

```
NOTIFY * HTTP/1.1\r\n
HOST: 239.255.255.250:1900\r\n
CACHE-CONTROL: max-age=900\r\n
LOCATION: http://{iface_ip}:{port}/device.xml\r\n
NT: {nt}\r\n
NTS: ssdp:alive\r\n
SERVER: Linux/1.0 UPnP/1.0 udlna/0.1\r\n
USN: {usn}\r\n
\r\n
```

**NOTIFY byebye** (sent on Ctrl+C for each USN type):

```
NOTIFY * HTTP/1.1\r\n
HOST: 239.255.255.250:1900\r\n
NT: {nt}\r\n
NTS: ssdp:byebye\r\n
USN: {usn}\r\n
\r\n
```

Note: byebye omits CACHE-CONTROL, LOCATION, and SERVER headers — only NT, NTS, and USN are required.

**M-SEARCH 200 OK response** (unicast to the requesting client):

```
HTTP/1.1 200 OK\r\n
CACHE-CONTROL: max-age=900\r\n
DATE: {rfc1123_date}\r\n
EXT:\r\n
LOCATION: http://{iface_ip}:{port}/device.xml\r\n
SERVER: Linux/1.0 UPnP/1.0 udlna/0.1\r\n
ST: {st}\r\n
USN: {usn}\r\n
Content-Length: 0\r\n
\r\n
```

The `Date:` header is optional but recommended. `EXT:` is required (empty value, signals HTTP extension support).

### Pattern 3: USN Set for a MediaServer

Each NOTIFY and M-SEARCH response must be sent once per USN type. The full set for a MediaServer with ContentDirectory + ConnectionManager:

```
# Advertisement 1 — UUID only (root device)
NT:  uuid:{device-uuid}
USN: uuid:{device-uuid}

# Advertisement 2 — root device type
NT:  upnp:rootdevice
USN: uuid:{device-uuid}::upnp:rootdevice

# Advertisement 3 — device type
NT:  urn:schemas-upnp-org:device:MediaServer:1
USN: uuid:{device-uuid}::urn:schemas-upnp-org:device:MediaServer:1

# Advertisement 4 — ContentDirectory service
NT:  urn:schemas-upnp-org:service:ContentDirectory:1
USN: uuid:{device-uuid}::urn:schemas-upnp-org:service:ContentDirectory:1

# Advertisement 5 — ConnectionManager service
NT:  urn:schemas-upnp-org:service:ConnectionManager:1
USN: uuid:{device-uuid}::urn:schemas-upnp-org:service:ConnectionManager:1
```

Note: The CONTEXT.md specifies advertising `urn:upnp-org:serviceId:ContentDirectory` in the USN type list. The spec-correct NT/ST for M-SEARCH is `urn:schemas-upnp-org:service:ContentDirectory:1` (serviceType, not serviceId). Research confirms minidlna uses serviceType (schemas-upnp-org) in SSDP, serviceId (upnp-org) only in the device.xml. **Use `urn:schemas-upnp-org:service:ContentDirectory:1` and `urn:schemas-upnp-org:service:ConnectionManager:1` in SSDP messages.**

### Pattern 4: M-SEARCH Request Handling

Parse incoming UDP packets. Respond only if:
- Start line is `M-SEARCH * HTTP/1.1`
- `MAN: "ssdp:discover"` present
- `ST` is one of: `ssdp:all`, `upnp:rootdevice`, `uuid:{our-uuid}`, `urn:schemas-upnp-org:device:MediaServer:1`, `urn:schemas-upnp-org:service:ContentDirectory:1`, `urn:schemas-upnp-org:service:ConnectionManager:1`

For `ssdp:all`: respond with all 5 USN types (one response per type).
For a specific ST: respond only with the matching USN.

**LOCATION URL determination:** Compare the sender's IP (from `recv_from` result) against each known interface address/mask pair. Select the interface whose subnet contains the sender's IP. Fall back to the first interface if none match.

```rust
// Subnet match approach (from minidlna source)
fn find_iface_for_sender(sender_ip: Ipv4Addr, ifaces: &[(Ipv4Addr, Ipv4Addr)]) -> Option<Ipv4Addr> {
    // ifaces: Vec of (addr, netmask)
    for &(addr, mask) in ifaces {
        let sender_net = u32::from(sender_ip) & u32::from(mask);
        let iface_net = u32::from(addr) & u32::from(mask);
        if sender_net == iface_net {
            return Some(addr);
        }
    }
    ifaces.first().map(|&(addr, _)| addr)  // fallback: first interface
}
```

### Pattern 5: Shutdown Sequence

**Main.rs restructure** required for CLI-06 (graceful shutdown):

```rust
// Source: axum graceful-shutdown example + tokio signal docs
use tokio::sync::broadcast;
use tokio::signal;

// Create shutdown broadcast channel
let (shutdown_tx, _) = broadcast::channel::<()>(1);

// Spawn SSDP task
let ssdp_shutdown_rx = shutdown_tx.subscribe();
let ssdp_handle = tokio::spawn(ssdp::service::run(ssdp_config, ssdp_shutdown_rx));

// Shutdown signal handler for axum:
let shutdown_rx_for_http = shutdown_tx.subscribe();
axum::serve(listener, app)
    .with_graceful_shutdown(async move {
        let _ = shutdown_rx_for_http.recv().await;
    })
    .await?;
```

**Shutdown sequence:**
1. `tokio::signal::ctrl_c().await` received
2. Broadcast shutdown signal on `shutdown_tx`
3. SSDP task receives signal → sends byebye for all USN types → exits
4. HTTP server receives signal → drains in-flight requests → exits
5. `tokio::time::timeout(Duration::from_secs(1), ssdp_handle).await` — wait up to 1s for byebye
6. Process exits

**Second Ctrl+C for force exit:**

```rust
// After first ctrl_c: re-install a signal handler that calls std::process::exit(1)
// Pattern: use tokio::signal with a flag
use std::sync::atomic::{AtomicBool, Ordering};
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

// On first ctrl_c:
if SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
    std::process::exit(1); // second ctrl_c → force exit
}
// Otherwise: proceed with graceful byebye
```

### Pattern 6: Re-advertisement Timer

```rust
// Source: tokio::time docs
use tokio::time::{interval, Duration};

let mut re_advert_interval = interval(Duration::from_secs(900));
re_advert_interval.tick().await; // skip the immediate first tick (startup burst already sent)

loop {
    tokio::select! {
        _ = re_advert_interval.tick() => {
            send_notify_alive_burst(&socket, &usn_set, &location_url).await;
        }
        _ = shutdown_rx.recv() => {
            send_notify_byebye(&socket, &usn_set).await;
            break;
        }
    }
}
```

### Anti-Patterns to Avoid

- **Binding the send socket to 0.0.0.0:** This makes LOCATION URL determination impossible. Bind the send socket to the specific interface IP for outbound NOTIFY messages.
- **Using IP_PKTINFO for interface detection in tokio:** Tokio's `recv_from` doesn't expose ancillary data; use subnet-mask matching (the minidlna approach) instead.
- **Using one receive socket without joining multicast per-interface:** On multi-homed hosts, must join the multicast group on each non-loopback interface separately.
- **Sending byebye with CACHE-CONTROL and LOCATION headers:** Byebye must not include these headers — clients that receive a byebye with LOCATION may behave unexpectedly.
- **Forgetting CRLF line endings:** SSDP uses `\r\n` terminators (like HTTP), not `\n`. Wrong line endings cause silent parse failures on strict clients.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Network interface enumeration | Custom `/proc/net/if_inet6` or libc FFI | `getifaddrs` crate | Cross-platform; handles netmask retrieval needed for subnet matching |
| Signal handling | `libc::signal()` or ctrlc crate | `tokio::signal::ctrl_c()` | Already in tokio; integrates with async runtime cleanly |
| Re-advertisement timer | `std::thread::sleep` in a thread | `tokio::time::interval` | Async-native; integrates with `tokio::select!` |
| Socket option configuration | tokio UdpSocket directly | `socket2::Socket` → `UdpSocket::from_std` | tokio has no pre-bind socket option API |

**Key insight:** SSDP itself is simple enough to hand-roll (the UPnP spec is clear), but the peripheral concerns (interface enumeration, socket options, signal handling, timers) all have solid standard solutions in the existing dependency tree.

---

## Common Pitfalls

### Pitfall 1: SO_REUSEPORT Required on macOS

**What goes wrong:** On macOS, binding a UDP socket to port 1900 when another process also listens (e.g., Bonjour/mDNSResponder uses 5353 but some other DLNA tool may bind 1900) fails with "Address already in use" even with `SO_REUSEADDR` set.

**Why it happens:** macOS requires both `SO_REUSEADDR` and `SO_REUSEPORT` for multiple processes to share a UDP port. Linux only requires `SO_REUSEADDR`.

**How to avoid:** Set both via `socket2`:
```rust
socket.set_reuse_address(true)?;
#[cfg(unix)]
socket.set_reuse_port(true)?;
```

**Warning signs:** Error on startup: `Os { code: 48, kind: AddrInUse, message: "Address already in use" }` on macOS.

### Pitfall 2: Wrong Binding Address for Multicast Receive

**What goes wrong:** Binding the receive socket to `0.0.0.0:1900` on Linux works but on macOS it may receive packets not intended for the multicast group (e.g., unicast to port 1900).

**Why it happens:** The kernel's multicast packet delivery depends on the bound address. On Unix, binding to the multicast address `239.255.255.250:1900` filters more cleanly.

**How to avoid:** On Unix, bind to `239.255.255.250:1900`; on Windows, bind to `0.0.0.0:1900` (Windows doesn't allow binding to multicast address). Use `#[cfg(windows)]` / `#[cfg(unix)]` if needed, but for macOS + Linux dev targets, binding to the multicast address is correct.

### Pitfall 3: CRLF vs LF in SSDP Messages

**What goes wrong:** Samsung TVs and some UPnP clients silently reject SSDP messages that use bare `\n` instead of `\r\n` as line separators.

**Why it happens:** SSDP is HTTP-over-UDP; HTTP mandates CRLF. The spec is unambiguous.

**How to avoid:** All message builders must use `\r\n` terminators. Include a test that verifies each message string contains `\r\n` and not bare `\n`.

### Pitfall 4: Sending Responses to the Multicast Group Instead of Unicast

**What goes wrong:** Sending M-SEARCH responses to 239.255.255.250:1900 instead of the requesting client's address floods the network and violates the spec.

**Why it happens:** Confusion between NOTIFY (multicast) and M-SEARCH response (unicast). M-SEARCH responses MUST be unicast to the sender's `(ip, port)` from `recv_from`.

**How to avoid:** Use `socket.send_to(msg, sender_addr)` where `sender_addr` comes from `recv_from`'s return value.

### Pitfall 5: Port 1900 Already in Use — Abort, Don't Silently Degrade

**What goes wrong:** Another process holds port 1900; SSDP socket bind fails. Silent fallback means users wonder why their TV can't find the server.

**Why it happens:** minidlna, VLC, or another UPnP daemon is running.

**How to avoid:** Per CONTEXT.md decision, abort on bind failure with a clear error message: `"error: SSDP port 1900 is already in use — another UPnP daemon may be running. Stop it and retry."`. Do not silently fall back.

### Pitfall 6: M-SEARCH MX Header — Don't Respond Immediately

**What goes wrong:** Responding instantly to every M-SEARCH packet creates response storms on busy networks with many clients.

**Why it happens:** The `MX` header in M-SEARCH requests specifies a random delay (0 to MX seconds) before responding, to spread out responses.

**How to avoid:** For a single-server implementation, a small fixed delay (0-100ms random) before responding is acceptable. Alternatively, ignore MX and respond immediately — Samsung and Xbox are tolerant of immediate responses. The spec recommends delay but clients work without it. For simplicity: respond immediately.

### Pitfall 7: USN Prefix Must Match UDN in device.xml

**What goes wrong:** Clients that cache SSDP advertisements and compare them with device.xml discovery may reject the device if the UUID in `USN: uuid:{x}::...` doesn't match `<UDN>uuid:{x}</UDN>` in device.xml.

**Why it happens:** The same `server_uuid` from `AppState` must be used in SSDP messages. If SSDP uses a different UUID than the HTTP server, clients may treat them as different devices.

**How to avoid:** Pass `server_uuid` from `AppState` (or `Config`) into the SSDP module. Both must use the same value.

### Pitfall 8: tokio "signal" Feature Not Enabled

**What goes wrong:** `use tokio::signal` fails to compile with "use of undeclared crate or module `signal`".

**Why it happens:** `tokio::signal` is behind the `signal` feature gate. Current Cargo.toml has `["macros", "rt-multi-thread", "fs", "net", "io-util"]` — `signal` is absent.

**How to avoid:** Update Cargo.toml: add `"signal"`, `"time"`, and `"sync"` to tokio features.

---

## Code Examples

### Building a socket2 UDP Socket and Converting to Tokio

```rust
// Source: socket2 docs + tokio UdpSocket::from_std docs
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use tokio::net::UdpSocket;

fn make_ssdp_recv_socket(iface_addr: Ipv4Addr) -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    // macOS needs SO_REUSEPORT in addition to SO_REUSEADDR for port sharing
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    let bind_addr = SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(239, 255, 255, 250), 1900));
    socket.bind(&bind_addr.into())?;
    socket.set_nonblocking(true)?;
    let std_udp: std::net::UdpSocket = socket.into();
    let tokio_udp = UdpSocket::from_std(std_udp)?;
    // Join multicast group on this interface
    tokio_udp.join_multicast_v4(Ipv4Addr::new(239, 255, 255, 250), iface_addr)?;
    Ok(tokio_udp)
}
```

### Building a NOTIFY Alive Message

```rust
// Source: UPnP Device Architecture 1.1 spec + minidlna minissdp.c
fn notify_alive(location: &str, nt: &str, usn: &str) -> String {
    format!(
        "NOTIFY * HTTP/1.1\r\n\
         HOST: 239.255.255.250:1900\r\n\
         CACHE-CONTROL: max-age=900\r\n\
         LOCATION: {location}\r\n\
         NT: {nt}\r\n\
         NTS: ssdp:alive\r\n\
         SERVER: Linux/1.0 UPnP/1.0 udlna/0.1\r\n\
         USN: {usn}\r\n\
         \r\n"
    )
}
```

### Building a NOTIFY Byebye Message

```rust
// Source: UPnP Device Architecture 1.1 spec
fn notify_byebye(nt: &str, usn: &str) -> String {
    format!(
        "NOTIFY * HTTP/1.1\r\n\
         HOST: 239.255.255.250:1900\r\n\
         NT: {nt}\r\n\
         NTS: ssdp:byebye\r\n\
         USN: {usn}\r\n\
         \r\n"
    )
}
```

### Building an M-SEARCH Response

```rust
// Source: UPnP Device Architecture 1.1 spec + Microsoft MS-SSDP examples
fn msearch_response(location: &str, st: &str, usn: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\n\
         CACHE-CONTROL: max-age=900\r\n\
         EXT:\r\n\
         LOCATION: {location}\r\n\
         SERVER: Linux/1.0 UPnP/1.0 udlna/0.1\r\n\
         ST: {st}\r\n\
         USN: {usn}\r\n\
         Content-Length: 0\r\n\
         \r\n"
    )
}
```

### Enumerating Non-Loopback IPv4 Interfaces

```rust
// Source: getifaddrs 0.4 docs (github.com/mmastrac/getifaddrs)
use getifaddrs::{getifaddrs, InterfaceFlags};
use std::net::IpAddr;

fn list_non_loopback_ipv4() -> Vec<std::net::Ipv4Addr> {
    let Ok(ifaces) = getifaddrs() else { return vec![] };
    ifaces
        .filter(|iface| !iface.flags.contains(InterfaceFlags::LOOPBACK))
        .filter_map(|iface| {
            if let Some(IpAddr::V4(v4)) = iface.address.ip_addr() {
                Some(v4)
            } else {
                None
            }
        })
        .collect()
}
```

### Tokio Signal Handling with Second Ctrl+C Force Exit

```rust
// Source: tokio::signal docs + axum graceful-shutdown example
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal;

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

async fn wait_for_shutdown_signal() {
    loop {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
        if SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
            // Second Ctrl+C: force exit immediately
            eprintln!("\nudlna: forced exit");
            std::process::exit(1);
        }
        // First Ctrl+C: signal graceful shutdown and return
        // (caller proceeds with byebye + server drain)
        return;
    }
}
```

### Main.rs Graceful Shutdown Integration

```rust
// Source: axum graceful-shutdown example (github.com/tokio-rs/axum)
use tokio::sync::broadcast;
use tokio::time::{timeout, Duration};

// In main():
let (shutdown_tx, _) = broadcast::channel::<()>(4);

// Axum HTTP server with graceful shutdown
let mut http_shutdown_rx = shutdown_tx.subscribe();
let http_task = tokio::spawn(
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = http_shutdown_rx.recv().await;
        })
);

// SSDP task
let ssdp_shutdown_rx = shutdown_tx.subscribe();
let ssdp_task = tokio::spawn(ssdp::service::run(ssdp_config, ssdp_shutdown_rx));

// Wait for Ctrl+C
wait_for_shutdown_signal().await;
tracing::info!("Shutting down — sending SSDP byebye...");
let _ = shutdown_tx.send(());

// Wait up to 1 second for SSDP byebye to be sent
let _ = timeout(Duration::from_secs(1), ssdp_task).await;
tracing::info!("Goodbye.");
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Bind multicast recv to 0.0.0.0 | Bind to multicast address on Unix | Documented since early 2000s | Cleaner kernel-level filtering |
| IP_PKTINFO for interface detection | Subnet-mask matching (or per-iface sockets) | N/A — IP_PKTINFO not in tokio API | Subnet matching works for home networks |
| Custom SSDP crates | Roll SSDP messages directly | N/A — crates exist but are over-engineered for a server-only use case | Fewer dependencies |
| axum 0.7 `.serve()` API | axum 0.8 `axum::serve()` top-level function with `.with_graceful_shutdown()` | axum 0.8 (already used in project) | Project already on correct API |

**Deprecated/outdated:**
- `URLBase` in device.xml: deprecated in UPnP 1.1, already omitted in this project (Phase 4 decision)
- `tokio-signal` crate (separate crate): superseded by `tokio::signal` module built into tokio 1.x

---

## Open Questions

1. **IPv6 SSDP (ff02::c:1900)**
   - What we know: CONTEXT.md specifies IPv6 SSDP is in scope; scope says "ff02::c:1900"; most home networks are IPv4 for DLNA; the REQUIREMENTS.md Out of Scope list says "IPv6 — Out of scope for v1"
   - What's unclear: The CONTEXT.md says "Both IPv4 and IPv6 SSDP — dual-stack" but REQUIREMENTS.md says IPv6 is out of scope for v1
   - **Recommendation:** Clarify with user. The REQUIREMENTS.md is the authoritative spec document. CONTEXT.md appears to capture an in-discussion aspiration. Implement IPv4 SSDP only for Phase 6; flag IPv6 as follow-up. If user confirms IPv6 is desired, the pattern is analogous: `join_multicast_v6(ff02::c, iface_index)`, bind `[::]:1900`.

2. **Netmask retrieval via getifaddrs**
   - What we know: `getifaddrs` 0.4 docs show `interface.address.ip_addr()` but netmask access may be in a different field
   - What's unclear: Whether the `Interface` struct exposes netmask directly or only IP address
   - **Recommendation:** Check `interface.netmask` field at implementation time; if absent, fall back to /24 assumption for home networks, or use `network-interface` crate which explicitly exposes netmask

3. **Send socket per-interface vs shared send socket**
   - What we know: NOTIFY messages should appear to originate from the interface's address; sending from a socket bound to 0.0.0.0 lets the OS choose
   - What's unclear: Whether Samsung/Xbox care about the source IP of NOTIFY packets (they likely only care about the LOCATION URL)
   - **Recommendation:** Use one shared send socket for simplicity. If Samsung fails to discover, revisit per-interface send sockets.

---

## Sources

### Primary (HIGH confidence)
- [UPnP Device Architecture v1.1 PDF](https://upnp.org/specs/arch/UPnP-arch-DeviceArchitecture-v1.1.pdf) — SSDP message format, USN structure, NOTIFY/M-SEARCH specification
- [minidlna minissdp.c source](https://github.com/ntfreak/minidlna/blob/master/minissdp.c) — Production DLNA server: interface matching via subnet mask, message construction, ST filtering
- [tokio::net::UdpSocket docs](https://docs.rs/tokio/latest/tokio/net/struct.UdpSocket.html) — `join_multicast_v4`, `recv_from`, `send_to`, `from_std` API
- [tokio::signal::ctrl_c docs](https://docs.rs/tokio/latest/tokio/signal/fn.ctrl_c.html) — Signal handling API
- [axum graceful-shutdown example](https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs) — `with_graceful_shutdown` pattern
- [getifaddrs crate docs](https://docs.rs/getifaddrs/latest/getifaddrs/) — Interface enumeration API, version 0.4
- [getifaddrs GitHub](https://github.com/mmastrac/getifaddrs) — Cross-platform support confirmation
- [socket2 + multicast in Rust](https://bluejekyll.github.io/blog/posts/multicasting-in-rust/) — `join_multicast_v4`, binding patterns, platform differences

### Secondary (MEDIUM confidence)
- [MS-SSDP Protocol Examples (Microsoft)](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-ssdp/d3bf5086-04cb-4692-a9ac-d7683b8a69c3) — Concrete NOTIFY message examples
- [Tokio Graceful Shutdown guide](https://tokio.rs/tokio/topics/shutdown) — CancellationToken and broadcast channel patterns
- [Multicasting quirks blog](https://rg3.name/201504241907.html) — SO_REUSEPORT requirement on macOS

### Tertiary (LOW confidence — flag for validation)
- Samsung/Xbox SSDP tolerance of immediate M-SEARCH responses (no MX delay): inferred from minidlna behavior and community reports; not officially documented

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — tokio/socket2/axum are already in use; getifaddrs verified via docs
- SSDP message formats: HIGH — verified against UPnP spec and minidlna production source
- Architecture: HIGH — minidlna subnet-matching approach is proven; tokio signal/graceful-shutdown is official
- Pitfalls: HIGH (SO_REUSEPORT, CRLF, unicast response) / MEDIUM (Samsung behavioral quirks)
- IPv6 SSDP scope: LOW — CONTEXT.md and REQUIREMENTS.md are contradictory; needs clarification

**Research date:** 2026-02-22
**Valid until:** 2026-05-22 (stable protocols; 90 days)
