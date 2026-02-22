use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

pub const SSDP_MCAST_V4: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
pub const SSDP_MCAST_V6: Ipv6Addr = Ipv6Addr::new(0xff02, 0, 0, 0, 0, 0, 0, 0x000c);
pub const SSDP_PORT: u16 = 1900;

/// Create a UDP socket for receiving IPv4 SSDP multicast on port 1900.
/// Binds to 239.255.255.250:1900 (Unix convention -- kernel-level multicast filtering).
/// Sets SO_REUSEADDR + SO_REUSEPORT (macOS requires both).
/// Joins multicast group on `iface_addr`.
pub fn build_recv_socket_v4(iface_addr: Ipv4Addr) -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    let bind_addr: SocketAddr = SocketAddrV4::new(SSDP_MCAST_V4, SSDP_PORT).into();
    socket.bind(&bind_addr.into())?;
    socket.set_nonblocking(true)?;
    let std_udp: std::net::UdpSocket = socket.into();
    let tokio_udp = UdpSocket::from_std(std_udp)?;
    tokio_udp.join_multicast_v4(SSDP_MCAST_V4, iface_addr)?;
    Ok(tokio_udp)
}

/// Create a UDP socket for receiving IPv6 SSDP multicast (ff02::c) on port 1900.
/// Binds to [::]:1900 (Windows-compatible; IPv6 multicast binding differs per platform).
/// Joins multicast group ff02::c on `iface_index` (interface index, not address).
pub fn build_recv_socket_v6(iface_index: u32) -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_only_v6(true)?;
    let bind_addr: SocketAddr = "[::]:1900".parse().unwrap();
    socket.bind(&bind_addr.into())?;
    socket.set_nonblocking(true)?;
    let std_udp: std::net::UdpSocket = socket.into();
    let tokio_udp = UdpSocket::from_std(std_udp)?;
    tokio_udp.join_multicast_v6(&SSDP_MCAST_V6, iface_index)?;
    Ok(tokio_udp)
}

/// Create a general-purpose send socket (not bound to multicast address).
/// Used for sending NOTIFY and M-SEARCH response packets.
/// Bind to 0.0.0.0:0 -- OS picks source port; destination comes from send_to caller.
pub fn build_send_socket() -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    socket.set_nonblocking(true)?;
    let bind_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    socket.bind(&bind_addr.into())?;
    let std_udp: std::net::UdpSocket = socket.into();
    UdpSocket::from_std(std_udp)
}

/// An interface entry: IPv4 address + netmask + interface index.
#[derive(Debug, Clone)]
pub struct IfaceV4 {
    pub addr: Ipv4Addr,
    pub mask: Ipv4Addr,
    #[allow(dead_code)]
    pub index: u32,
}

/// Enumerate non-loopback IPv4 interfaces using the `getifaddrs` crate.
/// Returns empty Vec if enumeration fails (not a fatal error for SSDP startup).
pub fn list_non_loopback_v4() -> Vec<IfaceV4> {
    use getifaddrs::{Address, InterfaceFlags};

    let Ok(ifaces) = getifaddrs::getifaddrs() else {
        return vec![];
    };
    ifaces
        .filter(|i| !i.flags.contains(InterfaceFlags::LOOPBACK))
        .filter_map(|i| {
            let (addr, mask) = match &i.address {
                Address::V4(net_addr) => {
                    let mask = net_addr.netmask
                        .unwrap_or(Ipv4Addr::new(255, 255, 255, 0));
                    (net_addr.address, mask)
                }
                _ => return None,
            };
            let index = i.index.unwrap_or(0);
            Some(IfaceV4 { addr, mask, index })
        })
        .collect()
}

/// Subnet-mask matching: find the interface whose subnet contains `sender_ip`.
/// Falls back to the first interface if no match. Returns None if no interfaces.
/// This is the minidlna approach -- accurate for home networks.
pub fn find_iface_for_sender(sender_ip: Ipv4Addr, ifaces: &[IfaceV4]) -> Option<Ipv4Addr> {
    for iface in ifaces {
        let sender_net = u32::from(sender_ip) & u32::from(iface.mask);
        let iface_net = u32::from(iface.addr) & u32::from(iface.mask);
        if sender_net == iface_net {
            return Some(iface.addr);
        }
    }
    ifaces.first().map(|i| i.addr)
}
