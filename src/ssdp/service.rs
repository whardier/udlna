use std::time::Duration;
use tokio::sync::broadcast;

use crate::ssdp::{messages, socket};

/// Configuration passed from main.rs to the SSDP service task.
pub struct SsdpConfig {
    /// Must match AppState.server_uuid (used in all USN/NT headers).
    pub device_uuid: String,
    /// HTTP server port for constructing LOCATION URLs.
    pub http_port: u16,
    /// Friendly name for startup log (Phase 8).
    pub server_name: String,
}

/// SSDP service async task.
///
/// Lifecycle:
/// 1. Discover non-loopback IPv4 interfaces; if none, log warning and return.
/// 2. Create receive socket (multicast) and send socket.
/// 3. Send startup NOTIFY alive burst (3x with 150ms delay).
/// 4. Main loop: re-advertise every 900s, respond to M-SEARCH, handle shutdown.
/// 5. On shutdown: send NOTIFY byebye for all USN types, then return.
pub async fn run(config: SsdpConfig, mut shutdown_rx: broadcast::Receiver<()>) {
    // --- 1. Interface discovery ---
    let ifaces = socket::list_non_loopback_v4();
    if ifaces.is_empty() {
        tracing::warn!(
            "SSDP: no non-loopback IPv4 interfaces found — SSDP disabled, HTTP still works"
        );
        return;
    }

    // --- 2. Socket creation ---
    // One recv socket that joins multicast on every interface.
    let recv_socket = match socket::build_recv_socket_v4(ifaces[0].addr) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!(
                "error: SSDP port 1900 is already in use — another UPnP daemon may be running. Stop it and retry."
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: failed to create SSDP receive socket: {e}");
            std::process::exit(1);
        }
    };

    // Join multicast group on additional interfaces (beyond the first, already joined).
    for iface in ifaces.iter().skip(1) {
        if let Err(e) = recv_socket.join_multicast_v4(socket::SSDP_MCAST_V4, iface.addr) {
            tracing::warn!("SSDP: could not join multicast on {}: {}", iface.addr, e);
        }
    }

    let send_socket = match socket::build_send_socket() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to create SSDP send socket: {e}");
            std::process::exit(1);
        }
    };

    // --- 3. Startup log ---
    for iface in &ifaces {
        tracing::info!("SSDP advertising \"{}\" on {}:1900", config.server_name, iface.addr);
    }

    // --- 4. USN set (built once, reused everywhere) ---
    let usn_set = messages::usn_set(&config.device_uuid);

    // --- 5. Startup NOTIFY burst (3x, 150ms delay between bursts) ---
    send_notify_alive_burst(&ifaces, &usn_set, config.http_port, &send_socket).await;

    // --- 6. IPv6 SSDP (best-effort, non-fatal) ---
    // Try to listen on ff02::c:1900 for IPv6 M-SEARCH. Failure is non-fatal.
    let recv_v6 = socket::build_recv_socket_v6(0).ok();
    if recv_v6.is_none() {
        tracing::debug!("SSDP: IPv6 multicast socket unavailable — IPv4 only");
    }

    // --- 7. Re-advertisement timer: skip immediate first tick (startup burst already sent) ---
    let mut re_advert = tokio::time::interval(Duration::from_secs(900));
    re_advert.tick().await; // consume immediate tick

    // Receive buffers: M-SEARCH packets are always well under 1KB.
    // Two separate buffers required — one per socket in select! (borrow checker).
    let mut buf_v4 = [0u8; 2048];
    let mut buf_v6 = [0u8; 2048];

    // --- 8. Main event loop ---
    loop {
        tokio::select! {
            // Re-advertisement: every 900s, re-send the NOTIFY alive burst.
            _ = re_advert.tick() => {
                tracing::debug!("SSDP: re-advertising (900s interval)");
                send_notify_alive_burst(&ifaces, &usn_set, config.http_port, &send_socket).await;
            }

            // M-SEARCH from IPv4 multicast socket.
            result = recv_socket.recv_from(&mut buf_v4) => {
                match result {
                    Ok((len, sender_addr)) => {
                        let packet = String::from_utf8_lossy(&buf_v4[..len]);
                        handle_msearch(
                            &packet,
                            sender_addr,
                            &ifaces,
                            &usn_set,
                            config.http_port,
                            &send_socket,
                        ).await;
                    }
                    Err(e) => {
                        tracing::debug!("SSDP: recv_from error (IPv4): {}", e);
                    }
                }
            }

            // M-SEARCH from IPv6 multicast socket (best-effort).
            result = recv_v6_from(&recv_v6, &mut buf_v6) => {
                match result {
                    Ok((len, sender_addr)) => {
                        let packet = String::from_utf8_lossy(&buf_v6[..len]);
                        handle_msearch(
                            &packet,
                            sender_addr,
                            &ifaces,
                            &usn_set,
                            config.http_port,
                            &send_socket,
                        ).await;
                    }
                    Err(e) => {
                        tracing::debug!("SSDP: recv_from error (IPv6): {}", e);
                    }
                }
            }

            // Shutdown signal: send byebye and exit.
            _ = shutdown_rx.recv() => {
                tracing::debug!("SSDP: shutdown signal received — sending byebye");
                send_byebye(&ifaces, &usn_set, &send_socket).await;
                tracing::info!("SSDP: byebye sent");
                return;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Send NOTIFY alive burst: 3 iterations with 150ms delay between each.
async fn send_notify_alive_burst(
    ifaces: &[socket::IfaceV4],
    usn_set: &[(String, String)],
    http_port: u16,
    send_socket: &tokio::net::UdpSocket,
) {
    for i in 0..3u8 {
        if i > 0 {
            tokio::time::sleep(Duration::from_millis(150)).await;
        }
        for iface in ifaces {
            let location = format!("http://{}:{}/device.xml", iface.addr, http_port);
            for (nt, usn) in usn_set {
                let msg = messages::notify_alive(&location, nt, usn);
                let _ = send_socket
                    .send_to(msg.as_bytes(), "239.255.255.250:1900")
                    .await;
            }
        }
    }
}

/// Send NOTIFY byebye for all USN types on all interfaces.
async fn send_byebye(
    ifaces: &[socket::IfaceV4],
    usn_set: &[(String, String)],
    send_socket: &tokio::net::UdpSocket,
) {
    for _iface in ifaces {
        for (nt, usn) in usn_set {
            let msg = messages::notify_byebye(nt, usn);
            let _ = send_socket
                .send_to(msg.as_bytes(), "239.255.255.250:1900")
                .await;
        }
    }
}

/// Parse an M-SEARCH packet and respond appropriately.
async fn handle_msearch(
    packet: &str,
    sender_addr: std::net::SocketAddr,
    ifaces: &[socket::IfaceV4],
    usn_set: &[(String, String)],
    http_port: u16,
    send_socket: &tokio::net::UdpSocket,
) {
    // Must start with M-SEARCH request line.
    if !packet.starts_with("M-SEARCH * HTTP/1.1") {
        return;
    }

    // Must contain MAN: "ssdp:discover" (case-insensitive header scan).
    let has_discover = packet.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.starts_with("man:") && lower.contains("ssdp:discover")
    });
    if !has_discover {
        return;
    }

    // Extract ST: value.
    let st = packet
        .lines()
        .find_map(|line| {
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("st:") {
                Some(line[3..].trim().to_string())
            } else {
                None
            }
        });
    let Some(st) = st else { return };

    // Determine which interface to use for LOCATION URL based on sender IP.
    let sender_ip = match sender_addr {
        std::net::SocketAddr::V4(v4) => *v4.ip(),
        std::net::SocketAddr::V6(_) => {
            // For IPv6 M-SEARCH, fall back to first interface.
            ifaces.first().map(|i| i.addr).unwrap_or(std::net::Ipv4Addr::LOCALHOST)
        }
    };
    let location_ip = socket::find_iface_for_sender(sender_ip, ifaces)
        .unwrap_or_else(|| ifaces[0].addr);
    let location = format!("http://{}:{}/device.xml", location_ip, http_port);

    // Respond based on ST value.
    match st.as_str() {
        "ssdp:all" => {
            // Respond with all 5 USN types.
            for (nt, usn) in usn_set {
                let msg = messages::msearch_response(&location, nt, usn);
                let _ = send_socket.send_to(msg.as_bytes(), sender_addr).await;
            }
        }
        st_val => {
            // Find matching (NT, USN) pair.
            let matched = usn_set.iter().find(|(nt, _usn)| nt == st_val);
            if let Some((nt, usn)) = matched {
                let msg = messages::msearch_response(&location, nt, usn);
                let _ = send_socket.send_to(msg.as_bytes(), sender_addr).await;
            }
        }
    }
}

/// Wrapper that calls recv_from on an optional IPv6 socket.
/// When socket is None, returns a future that never resolves (std::future::pending),
/// so the select! branch for IPv6 is effectively disabled.
async fn recv_v6_from(
    sock: &Option<tokio::net::UdpSocket>,
    buf: &mut [u8],
) -> std::io::Result<(usize, std::net::SocketAddr)> {
    match sock {
        Some(s) => s.recv_from(buf).await,
        None => {
            // Never resolves — this select! branch is permanently disabled.
            std::future::pending::<std::io::Result<(usize, std::net::SocketAddr)>>().await
        }
    }
}
