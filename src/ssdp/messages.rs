/// The 5 USN advertisement types for a MediaServer:1 with CDS + CMS.
/// Returns Vec of (NT, USN) pairs.
pub fn usn_set(device_uuid: &str) -> Vec<(String, String)> {
    let uuid = format!("uuid:{device_uuid}");
    vec![
        // Advertisement 1: UUID only
        (uuid.clone(), uuid.clone()),
        // Advertisement 2: root device
        ("upnp:rootdevice".into(), format!("{uuid}::upnp:rootdevice")),
        // Advertisement 3: device type
        (
            "urn:schemas-upnp-org:device:MediaServer:1".into(),
            format!("{uuid}::urn:schemas-upnp-org:device:MediaServer:1"),
        ),
        // Advertisement 4: ContentDirectory service
        (
            "urn:schemas-upnp-org:service:ContentDirectory:1".into(),
            format!("{uuid}::urn:schemas-upnp-org:service:ContentDirectory:1"),
        ),
        // Advertisement 5: ConnectionManager service
        (
            "urn:schemas-upnp-org:service:ConnectionManager:1".into(),
            format!("{uuid}::urn:schemas-upnp-org:service:ConnectionManager:1"),
        ),
    ]
}

/// Build a NOTIFY alive message.
/// LOCATION is the full HTTP URL to device.xml (e.g. "http://192.168.1.5:8200/device.xml").
/// Uses CRLF (\r\n) line endings -- required by SSDP/HTTP protocol. Bare \n causes silent
/// parse failures on strict clients (Samsung TVs).
pub fn notify_alive(location: &str, nt: &str, usn: &str) -> String {
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

/// Build a NOTIFY byebye message.
/// byebye MUST NOT include CACHE-CONTROL, LOCATION, or SERVER headers -- only NT/NTS/USN.
pub fn notify_byebye(nt: &str, usn: &str) -> String {
    format!(
        "NOTIFY * HTTP/1.1\r\n\
HOST: 239.255.255.250:1900\r\n\
NT: {nt}\r\n\
NTS: ssdp:byebye\r\n\
USN: {usn}\r\n\
\r\n"
    )
}

/// Build an M-SEARCH 200 OK unicast response.
/// `st` matches the ST header from the M-SEARCH request.
pub fn msearch_response(location: &str, st: &str, usn: &str) -> String {
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
