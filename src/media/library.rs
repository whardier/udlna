use std::path::PathBuf;
use uuid::Uuid;
use crate::media::mime::MediaKind;

/// Metadata extracted from file headers at scan time.
/// All fields are Option — extraction may fail for any given file.
#[derive(Debug, Clone, Default)]
pub struct MediaMeta {
    /// UPnP duration format: "HH:MM:SS.mmm" (INDX-02). None if extraction failed.
    pub duration: Option<String>,
    /// Pixel dimensions: "WxH" (INDX-03). None if not applicable or extraction failed.
    pub resolution: Option<String>,
    /// Bitrate in bits per second (INDX-01). None if not available.
    pub bitrate: Option<u32>,
    /// DLNA profile name e.g. "MP3", "AVC_MP4_MP_HD_720p_AAC" (INDX-04).
    /// None means omit DLNA.ORG_PN= from protocolInfo entirely — do NOT use wildcard.
    pub dlna_profile: Option<&'static str>,
}


/// A single discovered media file with all metadata extracted at scan time.
#[derive(Debug, Clone)]
pub struct MediaItem {
    /// Stable UUIDv5: uuid5(machine_namespace, canonical_path_bytes). Same file on same
    /// machine always produces the same ID across server restarts (CONTEXT.md locked).
    pub id: Uuid,
    /// Canonical absolute path (via std::fs::canonicalize — resolves symlinks and .././).
    pub path: PathBuf,
    /// File size in bytes, used in DIDL-Lite <res size="..."> (STRM-07).
    pub file_size: u64,
    /// MIME type string from Phase 1 classify() — static str, e.g. "video/mp4".
    pub mime: &'static str,
    /// Media kind from Phase 1 classify() — Video, Audio, or Image (not Subtitle).
    pub kind: MediaKind,
    /// Extracted metadata. Fields are None when extraction failed or not applicable.
    pub meta: MediaMeta,
}

/// Flat in-memory media library built synchronously at startup.
/// Wrapped in Arc<RwLock<MediaLibrary>> in main.rs for thread-safe sharing.
/// Library is write-once at startup, then read-only for the server lifetime.
#[derive(Debug, Default)]
pub struct MediaLibrary {
    /// All discovered media items. No subtitle items — subtitles are filtered at scan time.
    pub items: Vec<MediaItem>,
}

impl MediaLibrary {
    pub fn new() -> Self {
        Self::default()
    }
}
