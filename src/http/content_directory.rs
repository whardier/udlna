use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    response::Response,
};
use crate::http::soap::{self, soap_response, soap_fault, extract_soap_param, apply_pagination};
use crate::http::state::AppState;
use crate::media::library::MediaItem;
use crate::media::mime::MediaKind;

// ── Helper ────────────────────────────────────────────────────────────────────

/// Wrap a SOAP response body string into an HTTP 200 response with correct XML content-type.
fn ok_xml(body: String) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/xml; charset=\"utf-8\"")],
        body,
    )
        .into_response()
}

// ── Main handler ──────────────────────────────────────────────────────────────

/// Main CDS control handler: extracts the SOAP action and dispatches to the
/// appropriate action handler.
pub async fn cds_control(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    // Extract action name from SOAPAction header.
    // axum HeaderMap is case-insensitive, so "soapaction" matches "SOAPAction".
    let action_from_header = headers
        .get("soapaction")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split('#').nth(1))
        .map(|s| s.trim_matches('"').to_string());

    // Fall back to parsing the SOAP body for the action element name if the
    // SOAPAction header is absent or empty (RESEARCH.md Pitfall 3).
    let action = match action_from_header {
        Some(ref s) if !s.is_empty() => action_from_header,
        _ => {
            // Fallback: parse body for <u:ActionName ...>
            body.find("<u:")
                .map(|pos| {
                    let rest = &body[pos + 3..];
                    let end = rest
                        .find([' ', '>', '/'])
                        .unwrap_or(rest.len());
                    rest[..end].to_string()
                })
        }
    };

    match action.as_deref() {
        Some("Browse") => handle_browse(&state, &headers, &body).await,
        Some("GetSearchCapabilities") => handle_get_search_capabilities(),
        Some("GetSortCapabilities") => handle_get_sort_capabilities(),
        Some("GetSystemUpdateID") => handle_get_system_update_id(),
        _ => {
            tracing::warn!("Unknown CDS action: {:?}", action);
            soap_fault(402, "InvalidArgs").into_response()
        }
    }
}

// ── Stub actions ──────────────────────────────────────────────────────────────

/// GetSearchCapabilities: this server exposes no search capabilities.
fn handle_get_search_capabilities() -> Response {
    ok_xml(soap_response(
        "GetSearchCapabilities",
        "<SearchCaps></SearchCaps>",
    ))
}

/// GetSortCapabilities: this server exposes no sort capabilities.
fn handle_get_sort_capabilities() -> Response {
    ok_xml(soap_response(
        "GetSortCapabilities",
        "<SortCaps></SortCaps>",
    ))
}

/// GetSystemUpdateID: returns a fixed counter of 1.
/// The element name is `Id` (capital I, lowercase d) per the UPnP CDS spec.
fn handle_get_system_update_id() -> Response {
    ok_xml(soap_response("GetSystemUpdateID", "<Id>1</Id>"))
}

// ── DIDL-Lite generation helpers ──────────────────────────────────────────────

/// Wrap inner XML content in a DIDL-Lite root element with all four required namespaces.
///
/// CRITICAL: All four namespaces are required. Samsung TVs reject missing xmlns:dlna silently.
fn didl_lite_wrap(inner: &str) -> String {
    format!(
        r#"<DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/" xmlns:dlna="urn:schemas-dlna-org:metadata-1-0/">{inner}</DIDL-Lite>"#,
        inner = inner,
    )
}

/// Generate a single DIDL-Lite <container> element.
fn container_element(id: &str, parent_id: &str, title: &str, child_count: usize) -> String {
    format!(
        r#"<container id="{id}" parentID="{parent_id}" restricted="1" childCount="{child_count}"><dc:title>{title}</dc:title><upnp:class>object.container.storageFolder</upnp:class></container>"#,
        id = id,
        parent_id = parent_id,
        title = soap::xml_escape(title),
        child_count = child_count,
    )
}

/// Generate a single DIDL-Lite <item> element for a MediaItem.
///
/// - dc:title uses file_stem() not file_name() (no extension) — RESEARCH.md Pitfall 8
/// - dc:date is always present — RESEARCH.md Pitfall 5
/// - protocolInfo uses DLNA.ORG_PN when dlna_profile is Some, omits when None
/// - res URL is built from Host header
fn item_element(item: &MediaItem, parent_id: &str, headers: &HeaderMap) -> String {
    let title = item
        .path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let upnp_class = match item.kind {
        MediaKind::Video => "object.item.videoItem",
        MediaKind::Audio => "object.item.audioItem.musicTrack",
        MediaKind::Image => "object.item.imageItem.photo",
        _ => "object.item",
    };
    let dc_date = soap::format_dc_date(&item.path);
    let protocol_info = soap::build_protocol_info(item.mime, item.meta.dlna_profile);
    let res_url = soap::build_res_url(headers, &item.id);

    // Build optional res attributes
    let duration_attr = if let Some(ref d) = item.meta.duration {
        format!(r#" duration="{}""#, d)
    } else {
        String::new()
    };
    let resolution_attr = if let Some(ref r) = item.meta.resolution {
        format!(r#" resolution="{}""#, r)
    } else {
        String::new()
    };
    let bitrate_attr = if let Some(b) = item.meta.bitrate {
        format!(r#" bitrate="{}""#, b)
    } else {
        String::new()
    };

    format!(
        r#"<item id="{id}" parentID="{parent_id}" restricted="1"><dc:title>{title}</dc:title><upnp:class>{upnp_class}</upnp:class><dc:date>{dc_date}</dc:date><res protocolInfo="{protocol_info}" size="{size}"{duration_attr}{resolution_attr}{bitrate_attr}>{res_url}</res></item>"#,
        id = item.id,
        parent_id = parent_id,
        title = soap::xml_escape(title),
        upnp_class = upnp_class,
        dc_date = dc_date,
        protocol_info = protocol_info,
        size = item.file_size,
        duration_attr = duration_attr,
        resolution_attr = resolution_attr,
        bitrate_attr = bitrate_attr,
        res_url = soap::xml_escape(&res_url),
    )
}

// ── Browse helpers ─────────────────────────────────────────────────────────────

/// Build a Browse response for a list of media items with pagination.
fn browse_items_response(
    items: &[&MediaItem],
    parent_id: &str,
    headers: &HeaderMap,
    starting_index: u32,
    requested_count: u32,
) -> Response {
    let total_matches = items.len();
    let paged = apply_pagination(items, starting_index, requested_count);
    let number_returned = paged.len();
    let elements: String = paged
        .iter()
        .map(|item| item_element(item, parent_id, headers))
        .collect();
    let didl_xml = didl_lite_wrap(&elements);
    let inner = format!(
        "<Result>{}</Result><NumberReturned>{}</NumberReturned><TotalMatches>{}</TotalMatches><UpdateID>1</UpdateID>",
        soap::xml_escape(&didl_xml),
        number_returned,
        total_matches,
    );
    ok_xml(soap_response("Browse", &inner))
}

/// Build a BrowseMetadata response for a single container element.
fn browse_metadata_container(id: &str, parent_id: &str, title: &str, child_count: usize) -> Response {
    let element = container_element(id, parent_id, title, child_count);
    let didl_xml = didl_lite_wrap(&element);
    let inner = format!(
        "<Result>{}</Result><NumberReturned>1</NumberReturned><TotalMatches>1</TotalMatches><UpdateID>1</UpdateID>",
        soap::xml_escape(&didl_xml),
    );
    ok_xml(soap_response("Browse", &inner))
}

// ── Browse handler ────────────────────────────────────────────────────────────

/// Full Browse handler implementing BrowseDirectChildren and BrowseMetadata
/// with pagination, four-container hierarchy, and 701 fault for unknown ObjectIDs.
async fn handle_browse(state: &AppState, headers: &HeaderMap, body: &str) -> Response {
    // Derive container UUIDs (deterministic, cheap)
    let videos_id = soap::container_uuid(soap::CONTAINER_VIDEOS);
    let music_id  = soap::container_uuid(soap::CONTAINER_MUSIC);
    let photos_id = soap::container_uuid(soap::CONTAINER_PHOTOS);
    let all_id    = soap::container_uuid(soap::CONTAINER_ALL_MEDIA);

    // Parse Browse parameters from SOAP body
    let object_id: &str = match extract_soap_param(body, "ObjectID") {
        Some(v) => v,
        None => return soap_fault(402, "InvalidArgs").into_response(),
    };
    let browse_flag: &str = match extract_soap_param(body, "BrowseFlag") {
        Some(v) => v,
        None => return soap_fault(402, "InvalidArgs").into_response(),
    };
    let starting_index: u32 = extract_soap_param(body, "StartingIndex")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let requested_count: u32 = extract_soap_param(body, "RequestedCount")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Acquire library read guard
    let lib = state.library.read().expect("library lock poisoned");

    // Pre-compute filtered item lists for use in both BrowseDirectChildren and BrowseMetadata
    let video_items: Vec<&MediaItem> = lib.items.iter()
        .filter(|it| it.kind == MediaKind::Video)
        .collect();
    let audio_items: Vec<&MediaItem> = lib.items.iter()
        .filter(|it| it.kind == MediaKind::Audio)
        .collect();
    let image_items: Vec<&MediaItem> = lib.items.iter()
        .filter(|it| it.kind == MediaKind::Image)
        .collect();
    let all_items: Vec<&MediaItem> = lib.items.iter().collect();

    let videos_id_str = videos_id.to_string();
    let music_id_str = music_id.to_string();
    let photos_id_str = photos_id.to_string();
    let all_id_str = all_id.to_string();

    match browse_flag {
        "BrowseDirectChildren" => {
            // Build the four containers definition (for root browsing)
            let containers: Vec<(String, String, usize)> = vec![
                (videos_id_str.clone(), "Videos".to_string(), video_items.len()),
                (music_id_str.clone(),  "Music".to_string(),  audio_items.len()),
                (photos_id_str.clone(), "Photos".to_string(), image_items.len()),
                (all_id_str.clone(),    "All Media".to_string(), all_items.len()),
            ];

            match object_id {
                "0" => {
                    // Root: return four containers with pagination
                    let total_matches = 4;
                    let paged = apply_pagination(&containers, starting_index, requested_count);
                    let number_returned = paged.len();
                    let elements: String = paged
                        .iter()
                        .map(|(id, title, count)| container_element(id, "0", title, *count))
                        .collect();
                    let didl_xml = didl_lite_wrap(&elements);
                    let inner = format!(
                        "<Result>{}</Result><NumberReturned>{}</NumberReturned><TotalMatches>{}</TotalMatches><UpdateID>1</UpdateID>",
                        soap::xml_escape(&didl_xml),
                        number_returned,
                        total_matches,
                    );
                    ok_xml(soap_response("Browse", &inner))
                }
                id if id == videos_id_str => {
                    browse_items_response(&video_items, &videos_id_str, headers, starting_index, requested_count)
                }
                id if id == music_id_str => {
                    browse_items_response(&audio_items, &music_id_str, headers, starting_index, requested_count)
                }
                id if id == photos_id_str => {
                    browse_items_response(&image_items, &photos_id_str, headers, starting_index, requested_count)
                }
                id if id == all_id_str => {
                    browse_items_response(&all_items, &all_id_str, headers, starting_index, requested_count)
                }
                _ => {
                    tracing::debug!("Browse unknown ObjectID: {}", object_id);
                    soap_fault(701, "No such object").into_response()
                }
            }
        }
        "BrowseMetadata" => {
            match object_id {
                "0" => browse_metadata_container("0", "-1", "Root", 4),
                id if id == videos_id_str => {
                    browse_metadata_container(&videos_id_str, "0", "Videos", video_items.len())
                }
                id if id == music_id_str => {
                    browse_metadata_container(&music_id_str, "0", "Music", audio_items.len())
                }
                id if id == photos_id_str => {
                    browse_metadata_container(&photos_id_str, "0", "Photos", image_items.len())
                }
                id if id == all_id_str => {
                    browse_metadata_container(&all_id_str, "0", "All Media", all_items.len())
                }
                _ => {
                    // Search lib.items for a media item with matching UUID
                    if let Some(item) = lib.items.iter().find(|it| it.id.to_string() == object_id) {
                        // Determine parent container for this item
                        let parent_id = match item.kind {
                            MediaKind::Video => &videos_id_str,
                            MediaKind::Audio => &music_id_str,
                            MediaKind::Image => &photos_id_str,
                            _ => &all_id_str,
                        };
                        let element = item_element(item, parent_id, headers);
                        let didl_xml = didl_lite_wrap(&element);
                        let inner = format!(
                            "<Result>{}</Result><NumberReturned>1</NumberReturned><TotalMatches>1</TotalMatches><UpdateID>1</UpdateID>",
                            soap::xml_escape(&didl_xml),
                        );
                        ok_xml(soap_response("Browse", &inner))
                    } else {
                        tracing::debug!("BrowseMetadata unknown ObjectID: {}", object_id);
                        soap_fault(701, "No such object").into_response()
                    }
                }
            }
        }
        _ => {
            tracing::warn!("Unknown BrowseFlag: {}", browse_flag);
            soap_fault(402, "InvalidArgs").into_response()
        }
    }
}
