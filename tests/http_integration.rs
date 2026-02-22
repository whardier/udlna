use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;
use uuid::Uuid;

use udlna::http::{build_router, state::AppState};
use udlna::media::library::{MediaItem, MediaLibrary, MediaMeta};
use udlna::media::mime::MediaKind;

const TEST_UUID: &str = "550e8400-e29b-41d4-a716-446655440000";
const TEST_NAME: &str = "Test DLNA Server";

fn make_app(items: Vec<MediaItem>) -> axum::Router {
    let mut library = MediaLibrary::new();
    library.items = items;
    let state = AppState {
        library: Arc::new(RwLock::new(library)),
        server_uuid: TEST_UUID.to_string(),
        server_name: TEST_NAME.to_string(),
    };
    build_router(state)
}

fn fake_item() -> MediaItem {
    MediaItem {
        id: Uuid::new_v5(&Uuid::NAMESPACE_DNS, b"test-video-item"),
        path: PathBuf::from("/fake/test.mp4"),
        file_size: 1_048_576,
        mime: "video/mp4",
        kind: MediaKind::Video,
        meta: MediaMeta::default(),
    }
}

async fn body_text(response: axum::response::Response) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// ── GET /device.xml ───────────────────────────────────────────────────────────

#[tokio::test]
async fn device_xml_status_200() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/device.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn device_xml_content_type_is_xml() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/device.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let ct = response.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("text/xml"), "Expected text/xml, got: {ct}");
}

#[tokio::test]
async fn device_xml_contains_root_element() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/device.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(text.contains("<root"), "Expected <root in device.xml:\n{text}");
}

#[tokio::test]
async fn device_xml_contains_server_uuid() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/device.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(text.contains(TEST_UUID), "Expected server UUID in device.xml:\n{text}");
}

#[tokio::test]
async fn device_xml_contains_friendly_name() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/device.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(text.contains(TEST_NAME), "Expected friendly name in device.xml:\n{text}");
}

#[tokio::test]
async fn device_xml_advertises_content_directory_service() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/device.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(
        text.contains("ContentDirectory"),
        "Expected ContentDirectory service in device.xml:\n{text}"
    );
}

// ── GET /cds/scpd.xml ─────────────────────────────────────────────────────────

#[tokio::test]
async fn cds_scpd_status_200() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/cds/scpd.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn cds_scpd_contains_browse_action() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/cds/scpd.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(text.contains("<name>Browse</name>"), "Expected Browse action in CDS SCPD:\n{text}");
}

#[tokio::test]
async fn cds_scpd_contains_get_system_update_id_action() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/cds/scpd.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(
        text.contains("<name>GetSystemUpdateID</name>"),
        "Expected GetSystemUpdateID in CDS SCPD:\n{text}"
    );
}

// ── GET /cms/scpd.xml ─────────────────────────────────────────────────────────

#[tokio::test]
async fn cms_scpd_status_200() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/cms/scpd.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn cms_scpd_contains_get_protocol_info_action() {
    let response = make_app(vec![])
        .oneshot(Request::builder().uri("/cms/scpd.xml").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(
        text.contains("<name>GetProtocolInfo</name>"),
        "Expected GetProtocolInfo in CMS SCPD:\n{text}"
    );
}

// ── POST /cds/control ─────────────────────────────────────────────────────────

const BROWSE_DIRECT_CHILDREN_SOAP: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:Browse xmlns:u="urn:schemas-upnp-org:service:ContentDirectory:1">
      <ObjectID>0</ObjectID>
      <BrowseFlag>BrowseDirectChildren</BrowseFlag>
      <Filter>*</Filter>
      <StartingIndex>0</StartingIndex>
      <RequestedCount>0</RequestedCount>
      <SortCriteria></SortCriteria>
    </u:Browse>
  </s:Body>
</s:Envelope>"#;

fn cds_browse_request(body: &'static str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/cds/control")
        .header(
            "soapaction",
            "\"urn:schemas-upnp-org:service:ContentDirectory:1#Browse\"",
        )
        .header("content-type", "text/xml; charset=\"utf-8\"")
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn cds_browse_root_returns_200() {
    let response = make_app(vec![fake_item()])
        .oneshot(cds_browse_request(BROWSE_DIRECT_CHILDREN_SOAP))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn cds_browse_root_response_contains_didl_lite() {
    let response = make_app(vec![fake_item()])
        .oneshot(cds_browse_request(BROWSE_DIRECT_CHILDREN_SOAP))
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(text.contains("DIDL-Lite"), "Expected DIDL-Lite in Browse response:\n{text}");
}

#[tokio::test]
async fn cds_browse_root_response_is_soap_envelope() {
    let response = make_app(vec![fake_item()])
        .oneshot(cds_browse_request(BROWSE_DIRECT_CHILDREN_SOAP))
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(text.contains("s:Envelope"), "Expected SOAP Envelope in Browse response:\n{text}");
}

#[tokio::test]
async fn cds_unknown_action_returns_soap_fault() {
    let response = make_app(vec![])
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/cds/control")
                .header(
                    "soapaction",
                    "\"urn:schemas-upnp-org:service:ContentDirectory:1#NonExistentAction\"",
                )
                .header("content-type", "text/xml; charset=\"utf-8\"")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // UPnP SOAP faults use HTTP 500 per SOAP 1.1 spec
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// ── POST /cms/control ─────────────────────────────────────────────────────────

#[tokio::test]
async fn cms_get_protocol_info_returns_200() {
    let response = make_app(vec![])
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/cms/control")
                .header(
                    "soapaction",
                    "\"urn:schemas-upnp-org:service:ConnectionManager:1#GetProtocolInfo\"",
                )
                .header("content-type", "text/xml; charset=\"utf-8\"")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn cms_get_protocol_info_response_contains_source_element() {
    let response = make_app(vec![])
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/cms/control")
                .header(
                    "soapaction",
                    "\"urn:schemas-upnp-org:service:ConnectionManager:1#GetProtocolInfo\"",
                )
                .header("content-type", "text/xml; charset=\"utf-8\"")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let text = body_text(response).await;
    assert!(text.contains("<Source>"), "Expected <Source> in GetProtocolInfo response:\n{text}");
}

// ── GET /media/{id} ───────────────────────────────────────────────────────────

#[tokio::test]
async fn media_unknown_id_returns_404() {
    let id = Uuid::new_v4();
    let response = make_app(vec![])
        .oneshot(
            Request::builder()
                .uri(format!("/media/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
