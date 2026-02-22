use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use crate::http::soap::{soap_response_ns, soap_fault, CMS_NAMESPACE};
use crate::http::state::AppState;
use crate::media::mime::SUPPORTED_MIMES;

fn ok_xml(body: String) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        body,
    )
        .into_response()
}

pub async fn cms_control(
    State(_state): State<AppState>,
    headers: HeaderMap,
    _body: String,
) -> Response {
    let action = headers
        .get("soapaction")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('#').nth(1))
        .map(|s| s.trim_matches('"').to_string());

    match action.as_deref() {
        Some("GetProtocolInfo") => handle_get_protocol_info(),
        Some("GetCurrentConnectionIDs") => handle_get_current_connection_ids(),
        Some("GetCurrentConnectionInfo") => handle_get_current_connection_info(),
        _ => {
            tracing::warn!("Unknown CMS action: {:?}", action);
            // UPnP 1.0 error 401 = Invalid Action (not 402 InvalidArgs)
            soap_fault(401, "Invalid Action").into_response()
        }
    }
}

fn handle_get_protocol_info() -> Response {
    let source: String = SUPPORTED_MIMES
        .iter()
        .map(|mime| format!("http-get:*:{}:*", mime))
        .collect::<Vec<_>>()
        .join(",");
    let inner = format!("<Source>{}</Source><Sink></Sink>", source);
    ok_xml(soap_response_ns("GetProtocolInfo", &inner, CMS_NAMESPACE))
}

fn handle_get_current_connection_ids() -> Response {
    ok_xml(soap_response_ns(
        "GetCurrentConnectionIDs",
        "<ConnectionIDs>0</ConnectionIDs>",
        CMS_NAMESPACE,
    ))
}

fn handle_get_current_connection_info() -> Response {
    let inner = concat!(
        "<RcsID>-1</RcsID>",
        "<AVTransportID>-1</AVTransportID>",
        "<ProtocolInfo></ProtocolInfo>",
        "<PeerConnectionManager></PeerConnectionManager>",
        "<PeerConnectionID>-1</PeerConnectionID>",
        "<Direction>Output</Direction>",
        "<Status>OK</Status>",
    );
    ok_xml(soap_response_ns(
        "GetCurrentConnectionInfo",
        inner,
        CMS_NAMESPACE,
    ))
}
