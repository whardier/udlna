use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use http_range_header::parse_range_header;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use crate::http::state::AppState;
use crate::media::library::MediaItem;

// DLNA header constants — values per DLNA spec and cross-verified with working DLNA servers
// DLNA.ORG_OP=01: byte seek supported (bit 0), time seek not supported (bit 1)
// DLNA.ORG_CI=0: content is not converted/transcoded
// DLNA.ORG_FLAGS: 32 hex chars (8 significant + 24 zero padding, required length)
//   01700000 = STREAMING_TRANSFER_MODE | BACKGROUND_TRANSFER_MODE | CONNECTION_STALL | DLNA_V15
const DLNA_CONTENT_FEATURES: &str =
    "DLNA.ORG_OP=01;DLNA.ORG_CI=0;DLNA.ORG_FLAGS=01700000000000000000000000000000";
const DLNA_TRANSFER_MODE: &str = "Streaming";

/// Look up MediaItem by UUID string. Returns None if UUID is invalid or item not found.
/// Lock is acquired and released within this function — safe to call before any .await.
fn lookup_item(state: &AppState, id_str: &str) -> Option<MediaItem> {
    let id = Uuid::parse_str(id_str).ok()?;
    let lib = state.library.read().unwrap();
    lib.items.iter().find(|i| i.id == id).cloned()
}

/// Build the standard DLNA response headers present on ALL media responses (GET + HEAD).
/// Returns a HeaderMap with: Content-Type, Content-Length, Accept-Ranges,
/// transferMode.dlna.org, contentFeatures.dlna.org.
fn dlna_headers(item: &MediaItem) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static(item.mime),
    );
    headers.insert(
        axum::http::header::CONTENT_LENGTH,
        HeaderValue::from(item.file_size),
    );
    headers.insert(
        axum::http::header::ACCEPT_RANGES,
        HeaderValue::from_static("bytes"),
    );
    headers.insert(
        HeaderName::from_static("transfermode.dlna.org"),
        HeaderValue::from_static(DLNA_TRANSFER_MODE),
    );
    headers.insert(
        HeaderName::from_static("contentfeatures.dlna.org"),
        HeaderValue::from_static(DLNA_CONTENT_FEATURES),
    );
    headers
}

/// HEAD /media/{id} — returns 200 with all DLNA headers and NO body.
/// Does NOT open the file (avoids unnecessary disk I/O on Samsung TV pre-flight checks).
pub async fn serve_media_head(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
) -> Response {
    let item = match lookup_item(&state, &id_str) {
        Some(i) => i,
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    // Return 200 with all DLNA headers and NO body. Do NOT open the file.
    (StatusCode::OK, dlna_headers(&item)).into_response()
}

/// GET /media/{id} — stream full file or partial content per RFC 7233 Range header.
pub async fn serve_media_get(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    req_headers: HeaderMap,
) -> Response {
    // Lookup (releases lock before any .await — avoids Send issue with RwLock guard)
    let item = match lookup_item(&state, &id_str) {
        Some(i) => i,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    let headers = dlna_headers(&item);

    // Check for Range header
    if let Some(range_val) = req_headers.get(axum::http::header::RANGE) {
        let range_str = match range_val.to_str() {
            Ok(s) => s.to_owned(),
            Err(_) => {
                // Unparseable Range value — treat as unsatisfiable
                return (
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    [(
                        "content-range",
                        format!("bytes */{}", item.file_size),
                    )],
                )
                    .into_response();
            }
        };
        return range_response(&item, &range_str, headers).await;
    }

    // Full GET — stream entire file
    let file = match tokio::fs::File::open(&item.path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to open file {}: {}", item.path.display(), e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    (StatusCode::OK, headers, body).into_response()
}

/// Handle a Range request. Returns 206 Partial Content or 416 Range Not Satisfiable.
/// Decision (CONTEXT.md): multi-part ranges — validate requires non-overlapping but we serve first
/// valid range only by cloning the vec and taking the first element.
async fn range_response(item: &MediaItem, range_str: &str, mut headers: HeaderMap) -> Response {
    // Parse Range header string (e.g., "bytes=0-99", "bytes=-500")
    let parsed = match parse_range_header(range_str) {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [("content-range", format!("bytes */{}", item.file_size))],
            )
                .into_response();
        }
    };

    // Validate against actual file size (resolves suffix ranges to absolute positions).
    // Note: http-range-header rejects overlapping multi-ranges at validate() — we handle all
    // errors uniformly as 416.
    let ranges = match parsed.validate(item.file_size) {
        Ok(r) => r,
        Err(_) => {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [("content-range", format!("bytes */{}", item.file_size))],
            )
                .into_response();
        }
    };

    // Take first range only (CONTEXT.md locked decision: multi-part -> first range only)
    let first = match ranges.into_iter().next() {
        Some(r) => r,
        None => {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [("content-range", format!("bytes */{}", item.file_size))],
            )
                .into_response();
        }
    };

    let start = *first.start();
    let end = *first.end(); // inclusive end byte
    let length = end - start + 1;

    // Open file, seek to start, read exactly `length` bytes
    let mut file = match tokio::fs::File::open(&item.path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Range response: failed to open file {}: {}", item.path.display(), e);
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(e) = file.seek(std::io::SeekFrom::Start(start)).await {
        tracing::error!("Range response: failed to seek in file {}: {}", item.path.display(), e);
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    // Insert Content-Range header for the 206 response
    let content_range = format!("bytes {}-{}/{}", start, end, item.file_size);
    headers.insert(
        axum::http::header::CONTENT_RANGE,
        HeaderValue::from_str(&content_range).unwrap_or_else(|_| {
            HeaderValue::from_static("bytes 0-0/0")
        }),
    );
    // Override Content-Length to the partial length
    headers.insert(
        axum::http::header::CONTENT_LENGTH,
        HeaderValue::from(length),
    );

    let stream = ReaderStream::new(file.take(length));
    let body = Body::from_stream(stream);
    (StatusCode::PARTIAL_CONTENT, headers, body).into_response()
}
