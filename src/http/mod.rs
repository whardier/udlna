pub mod content_directory;
pub mod media;
pub mod soap;
pub mod state;
pub mod description;

use axum::{routing::get, Router};
use tower_http::trace::TraceLayer;
use crate::http::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // Phase 3 — implemented in media.rs
        .route("/media/{id}", get(media::serve_media_get).head(media::serve_media_head))
        // Phase 4 — device and service description XML
        .route("/device.xml", get(description::serve_device_xml))
        .route("/cds/scpd.xml", get(description::serve_cds_scpd))
        .route("/cms/scpd.xml", get(description::serve_cms_scpd))
        // Phase 5 — CDS control endpoint (action dispatch)
        .route("/cds/control", axum::routing::post(content_directory::cds_control))
        .route("/cms/control", axum::routing::post(crate::cms::cms_control))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
