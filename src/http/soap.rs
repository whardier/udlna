use std::borrow::Cow;
use axum::http::{StatusCode, header};

// ── Constants ─────────────────────────────────────────────────────────────────

pub const CDS_NAMESPACE: &str = "urn:schemas-upnp-org:service:ContentDirectory:1";
pub const CMS_NAMESPACE: &str = "urn:schemas-upnp-org:service:ConnectionManager:1";
pub const DLNA_FLAGS: &str = "01700000000000000000000000000000";

/// Stable container name strings used for UUIDv5 derivation (locked in CONTEXT.md).
pub const CONTAINER_VIDEOS: &str = "Videos";
pub const CONTAINER_MUSIC: &str = "Music";
pub const CONTAINER_PHOTOS: &str = "Photos";
pub const CONTAINER_ALL_MEDIA: &str = "All Media";

// ── SOAP envelope builder ─────────────────────────────────────────────────────

/// Build a SOAP 1.1 response envelope with an explicit service namespace.
/// Used by CMS and any future UPnP service with a different namespace.
pub fn soap_response_ns(action: &str, inner_xml: &str, namespace: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:{action}Response xmlns:u="{ns}">
      {inner_xml}
    </u:{action}Response>
  </s:Body>
</s:Envelope>"#,
        action = action,
        ns = namespace,
        inner_xml = inner_xml,
    )
}

/// Build a complete SOAP 1.1 response envelope wrapping the given inner XML.
/// The service namespace is always CDS_NAMESPACE.
pub fn soap_response(action: &str, inner_xml: &str) -> String {
    soap_response_ns(action, inner_xml, CDS_NAMESPACE)
}

// ── SOAP fault builder ────────────────────────────────────────────────────────

/// Build a UPnP SOAP fault response (HTTP 500 per SOAP 1.1 spec).
///
/// Returns a tuple `(StatusCode, [(CONTENT_TYPE, "text/xml; charset=\"utf-8\"")], String)`
/// that callers can return directly from axum handlers (implements IntoResponse).
pub fn soap_fault(
    error_code: u32,
    error_description: &str,
) -> (StatusCode, [(axum::http::HeaderName, &'static str); 1], String) {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <s:Fault>
      <faultcode>s:Client</faultcode>
      <faultstring>UPnPError</faultstring>
      <detail>
        <UPnPError xmlns="urn:schemas-upnp-org:control-1-0">
          <errorCode>{error_code}</errorCode>
          <errorDescription>{error_description}</errorDescription>
        </UPnPError>
      </detail>
    </s:Fault>
  </s:Body>
</s:Envelope>"#,
        error_code = error_code,
        error_description = error_description,
    );
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        body,
    )
}

// ── SOAP parameter extraction ─────────────────────────────────────────────────

/// Extract a single SOAP body parameter by element name using simple string search.
///
/// Finds `<{param}>...</{param}>` and returns the content between the tags.
/// Returns None if the element is absent (handled gracefully — no panic).
///
/// This is Approach A from RESEARCH.md: fast and sufficient for short, well-known SOAP bodies.
pub fn extract_soap_param<'a>(body: &'a str, param: &str) -> Option<&'a str> {
    let open = format!("<{}>", param);
    let close = format!("</{}>", param);
    let start = body.find(&open)? + open.len();
    let end = body[start..].find(&close)? + start;
    Some(&body[start..end])
}

// ── Pagination ────────────────────────────────────────────────────────────────

/// Apply UPnP Browse pagination to a slice.
///
/// - `starting_index` beyond end → empty slice
/// - `requested_count == 0` → ALL items from starting_index onward (UPnP spec: 0 means all)
/// - otherwise → min(requested_count, available) items
///
/// See RESEARCH.md Pattern 9 for exact semantics.
pub fn apply_pagination<T>(
    items: &[T],
    starting_index: u32,
    requested_count: u32,
) -> &[T] {
    let start = (starting_index as usize).min(items.len());
    let slice = &items[start..];
    if requested_count == 0 {
        // UPnP spec: RequestedCount=0 means "return all remaining"
        slice
    } else {
        let count = (requested_count as usize).min(slice.len());
        &slice[..count]
    }
}

// ── protocolInfo construction ─────────────────────────────────────────────────

/// Build the DLNA protocolInfo fourth-field string for a `<res>` element.
///
/// - With profile: `http-get:*:{mime}:DLNA.ORG_PN={profile};DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS={DLNA_FLAGS}`
/// - Without profile: `http-get:*:{mime}:DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS={DLNA_FLAGS}`
///
/// Never uses wildcard `*` as a profile fallback (CONTEXT.md locked decision).
pub fn build_protocol_info(mime: &'static str, dlna_profile: Option<&'static str>) -> String {
    match dlna_profile {
        Some(profile) => format!(
            "http-get:*:{}:DLNA.ORG_PN={};DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS={}",
            mime, profile, DLNA_FLAGS
        ),
        None => format!(
            "http-get:*:{}:DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS={}",
            mime, DLNA_FLAGS
        ),
    }
}

// ── Container UUID derivation ─────────────────────────────────────────────────

/// Derive a stable UUIDv5 for a container by name, using the machine-specific namespace.
///
/// Uses `crate::media::metadata::build_machine_namespace()` — same pattern as `media_item_id()`.
/// Results are deterministic across restarts: same name always yields the same UUID.
pub fn container_uuid(name: &str) -> uuid::Uuid {
    uuid::Uuid::new_v5(&crate::media::metadata::MACHINE_NAMESPACE, name.as_bytes())
}

// ── dc:date formatting ────────────────────────────────────────────────────────

/// Return an ISO 8601 date string (YYYY-MM-DD) from the file modification time.
///
/// Uses `chrono::DateTime::<chrono::Utc>::from(mtime)` for clean calendar conversion.
/// Falls back to `"1970-01-01"` on any error — Samsung requires dc:date; fallback beats omission.
pub fn format_dc_date(path: &std::path::Path) -> String {
    (|| -> Option<String> {
        let meta = std::fs::metadata(path).ok()?;
        let mtime = meta.modified().ok()?;
        let dt: chrono::DateTime<chrono::Utc> = mtime.into();
        Some(dt.format("%Y-%m-%d").to_string())
    })()
    .unwrap_or_else(|| "1970-01-01".to_string())
}

// ── res URL builder ───────────────────────────────────────────────────────────

/// Build the streaming URL for a media item's `<res>` element.
///
/// Reads the Host header from the request; falls back to `localhost:8200` if absent.
/// This is the most portable approach for dual-stack IPv4/IPv6 binds (CONTEXT.md discretion).
pub fn build_res_url(headers: &axum::http::HeaderMap, item_id: &uuid::Uuid) -> String {
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost:8200");
    format!("http://{}/media/{}", host, item_id)
}

// ── XML escaping ──────────────────────────────────────────────────────────────

/// Thin wrapper around `quick_xml::escape::escape`.
///
/// Escapes the five XML special characters (`&`, `<`, `>`, `"`, `'`) so that
/// user-provided strings (titles, URLs) can be safely embedded in XML text nodes
/// and attribute values.
pub fn xml_escape(s: &str) -> Cow<'_, str> {
    quick_xml::escape::escape(s)
}
