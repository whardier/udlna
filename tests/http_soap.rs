use udlna::http::soap::{
    apply_pagination, build_protocol_info, container_uuid, extract_soap_param, soap_response,
    xml_escape, CDS_NAMESPACE, CONTAINER_ALL_MEDIA, CONTAINER_MUSIC, CONTAINER_PHOTOS,
    CONTAINER_VIDEOS,
};

// ── extract_soap_param ────────────────────────────────────────────────────────

fn browse_body() -> &'static str {
    r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
  <s:Body>
    <u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
      <ObjectID>0</ObjectID>
      <BrowseFlag>BrowseDirectChildren</BrowseFlag>
      <Filter>*</Filter>
      <StartingIndex>5</StartingIndex>
      <RequestedCount>0</RequestedCount>
      <SortCriteria></SortCriteria>
    </u:Browse>
  </s:Body>
</s:Envelope>"#
}

#[test]
fn extract_soap_param_object_id() {
    assert_eq!(extract_soap_param(browse_body(), "ObjectID"), Some("0"));
}

#[test]
fn extract_soap_param_browse_flag() {
    assert_eq!(
        extract_soap_param(browse_body(), "BrowseFlag"),
        Some("BrowseDirectChildren")
    );
}

#[test]
fn extract_soap_param_starting_index() {
    assert_eq!(extract_soap_param(browse_body(), "StartingIndex"), Some("5"));
}

#[test]
fn extract_soap_param_requested_count_zero() {
    // RequestedCount=0 is the UPnP "return all" sentinel — must extract as "0" not None
    assert_eq!(
        extract_soap_param(browse_body(), "RequestedCount"),
        Some("0")
    );
}

#[test]
fn extract_soap_param_missing_returns_none() {
    let body = "<u:Browse><ObjectID>42</ObjectID></u:Browse>";
    assert_eq!(extract_soap_param(body, "BrowseFlag"), None);
}

#[test]
fn extract_soap_param_empty_body_returns_none() {
    assert_eq!(extract_soap_param("", "ObjectID"), None);
}

// ── apply_pagination ──────────────────────────────────────────────────────────

#[test]
fn apply_pagination_zero_count_returns_all() {
    // UPnP spec: RequestedCount=0 means "return all remaining from StartingIndex"
    let items = [1u32, 2, 3, 4, 5];
    assert_eq!(apply_pagination(&items, 0, 0), &[1, 2, 3, 4, 5]);
}

#[test]
fn apply_pagination_zero_count_with_offset_returns_remaining() {
    let items = [1u32, 2, 3, 4, 5];
    assert_eq!(apply_pagination(&items, 2, 0), &[3, 4, 5]);
}

#[test]
fn apply_pagination_normal_range() {
    let items = [1u32, 2, 3, 4, 5];
    assert_eq!(apply_pagination(&items, 1, 2), &[2, 3]);
}

#[test]
fn apply_pagination_starting_index_beyond_end_returns_empty() {
    let items = [1u32, 2, 3, 4, 5];
    assert_eq!(apply_pagination(&items, 10, 5), &[] as &[u32]);
}

#[test]
fn apply_pagination_from_start_limited_count() {
    let items = [1u32, 2, 3, 4, 5];
    assert_eq!(apply_pagination(&items, 0, 3), &[1, 2, 3]);
}

#[test]
fn apply_pagination_empty_slice() {
    let items: [u32; 0] = [];
    assert_eq!(apply_pagination(&items, 0, 0), &[] as &[u32]);
}

#[test]
fn apply_pagination_count_exceeds_remaining_clamps() {
    let items = [1u32, 2, 3];
    assert_eq!(apply_pagination(&items, 1, 100), &[2, 3]);
}

// ── build_protocol_info ───────────────────────────────────────────────────────

#[test]
fn build_protocol_info_with_profile_contains_dlna_org_pn() {
    let info = build_protocol_info("audio/mpeg", Some("MP3"));
    assert!(info.contains("DLNA.ORG_PN=MP3"), "Expected DLNA.ORG_PN=MP3 in: {info}");
}

#[test]
fn build_protocol_info_with_profile_contains_dlna_org_op() {
    let info = build_protocol_info("audio/mpeg", Some("MP3"));
    assert!(info.contains("DLNA.ORG_OP=01"), "Expected DLNA.ORG_OP=01 in: {info}");
}

#[test]
fn build_protocol_info_with_profile_contains_dlna_flags() {
    let info = build_protocol_info("audio/mpeg", Some("MP3"));
    assert!(
        info.contains("DLNA.ORG_FLAGS=01700000000000000000000000000000"),
        "Expected DLNA.ORG_FLAGS in: {info}"
    );
}

#[test]
fn build_protocol_info_with_profile_starts_with_http_get_prefix() {
    let info = build_protocol_info("audio/mpeg", Some("MP3"));
    assert!(info.starts_with("http-get:*:audio/mpeg:"), "Expected http-get:*:audio/mpeg: in: {info}");
}

#[test]
fn build_protocol_info_none_profile_omits_dlna_org_pn() {
    // LOCKED DECISION: None profile must NOT include DLNA.ORG_PN at all (no wildcard "*")
    let info = build_protocol_info("video/x-matroska", None);
    assert!(!info.contains("DLNA.ORG_PN"), "Expected NO DLNA.ORG_PN in: {info}");
}

#[test]
fn build_protocol_info_none_profile_still_contains_dlna_org_op() {
    let info = build_protocol_info("video/x-matroska", None);
    assert!(info.contains("DLNA.ORG_OP=01"), "Expected DLNA.ORG_OP=01 in: {info}");
}

#[test]
fn build_protocol_info_none_profile_starts_with_http_get_prefix() {
    let info = build_protocol_info("video/x-matroska", None);
    assert!(
        info.starts_with("http-get:*:video/x-matroska:"),
        "Expected http-get:*:video/x-matroska: in: {info}"
    );
}

// ── container_uuid ────────────────────────────────────────────────────────────

#[test]
fn container_uuid_videos_is_non_nil() {
    assert_ne!(container_uuid(CONTAINER_VIDEOS), uuid::Uuid::nil());
}

#[test]
fn container_uuid_is_deterministic() {
    assert_eq!(container_uuid(CONTAINER_VIDEOS), container_uuid(CONTAINER_VIDEOS));
}

#[test]
fn container_uuid_videos_differs_from_music() {
    assert_ne!(container_uuid(CONTAINER_VIDEOS), container_uuid(CONTAINER_MUSIC));
}

#[test]
fn container_uuid_photos_differs_from_all_media() {
    assert_ne!(container_uuid(CONTAINER_PHOTOS), container_uuid(CONTAINER_ALL_MEDIA));
}

#[test]
fn container_uuid_all_four_are_distinct() {
    let videos = container_uuid(CONTAINER_VIDEOS);
    let music = container_uuid(CONTAINER_MUSIC);
    let photos = container_uuid(CONTAINER_PHOTOS);
    let all_media = container_uuid(CONTAINER_ALL_MEDIA);
    assert_ne!(videos, music);
    assert_ne!(videos, photos);
    assert_ne!(videos, all_media);
    assert_ne!(music, photos);
    assert_ne!(music, all_media);
    assert_ne!(photos, all_media);
}

// ── xml_escape ────────────────────────────────────────────────────────────────

#[test]
fn xml_escape_ampersand() {
    let result = xml_escape("hello & world");
    assert!(result.contains("&amp;"), "Expected &amp; in: {result}");
}

#[test]
fn xml_escape_less_than() {
    let result = xml_escape("<title>");
    assert!(result.contains("&lt;"), "Expected &lt; in: {result}");
}

#[test]
fn xml_escape_no_special_chars_unchanged() {
    let input = "normal text";
    assert_eq!(xml_escape(input).as_ref(), input);
}

// ── soap_response ─────────────────────────────────────────────────────────────

#[test]
fn soap_response_contains_xml_declaration() {
    let out = soap_response("Browse", "<Result></Result>");
    assert!(out.contains(r#"<?xml version="1.0""#), "Expected XML declaration in: {out}");
}

#[test]
fn soap_response_contains_closing_envelope() {
    let out = soap_response("Browse", "<Result></Result>");
    assert!(out.contains("</s:Envelope>"), "Expected </s:Envelope> in: {out}");
}

#[test]
fn soap_response_contains_action_response_tag() {
    let out = soap_response("Browse", "<Result></Result>");
    assert!(out.contains("<u:BrowseResponse"), "Expected <u:BrowseResponse in: {out}");
    assert!(out.contains("</u:BrowseResponse>"), "Expected </u:BrowseResponse> in: {out}");
}

#[test]
fn soap_response_contains_inner_xml_verbatim() {
    let inner = "<Result>some content</Result>";
    let out = soap_response("Browse", inner);
    assert!(out.contains(inner), "Expected inner XML verbatim in: {out}");
}

#[test]
fn soap_response_contains_cds_namespace() {
    let out = soap_response("Browse", "");
    assert!(out.contains(CDS_NAMESPACE), "Expected CDS_NAMESPACE in: {out}");
}
