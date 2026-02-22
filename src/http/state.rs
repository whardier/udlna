use std::sync::{Arc, RwLock};
use crate::media::library::MediaLibrary;

/// Shared application state injected into all route handlers via axum::extract::State.
/// Arc provides cheap clone; RwLock provides thread-safe read access.
/// Write-once at startup (scan), then read-only for server lifetime.
#[derive(Clone)]
pub struct AppState {
    pub library: Arc<RwLock<MediaLibrary>>,
    pub server_uuid: String,   // Stable UUID v5 derived from hostname (Phase 8)
    pub server_name: String,   // Friendly name from --name / config / default (Phase 8)
}
