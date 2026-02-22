use std::path::Path;

/// All MIME types this server can serve. Used by CMS GetProtocolInfo.
/// Only video, audio, and image types — subtitle types (text/srt, text/vtt)
/// are intentionally excluded per DLNA ConnectionManager spec.
pub const SUPPORTED_MIMES: &[&str] = &[
    // Video
    "video/mp4",
    "video/x-matroska",
    "video/x-msvideo",
    "video/quicktime",
    "video/MP2T",
    "video/mpeg",
    "video/x-ms-wmv",
    "video/x-flv",
    "video/ogg",
    "video/webm",
    "video/3gpp",
    // Audio
    "audio/mpeg",
    "audio/flac",
    "audio/wav",
    "audio/mp4",
    "audio/aac",
    "audio/ogg",
    "audio/x-ms-wma",
    "audio/aiff",
    // Image
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/bmp",
    "image/tiff",
];

/// Media kind classification for discovered files.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MediaKind {
    Video,
    Audio,
    Image,
    Subtitle,
}

/// Classify a file path by its extension into a (MediaKind, MIME type) pair.
///
/// Returns `None` for unrecognized extensions (silent skip — no logging at this layer).
/// Extensions are matched case-insensitively.
///
/// MIME strings use DLNA-correct values (e.g. "video/x-matroska" for .mkv,
/// "video/MP2T" for .ts/.m2ts).
pub fn classify(path: &Path) -> Option<(MediaKind, &'static str)> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();

    let result = match ext.as_str() {
        // Video
        "mp4" => (MediaKind::Video, "video/mp4"),
        "m4v" => (MediaKind::Video, "video/mp4"),
        "mkv" => (MediaKind::Video, "video/x-matroska"),
        "avi" => (MediaKind::Video, "video/x-msvideo"),
        "mov" => (MediaKind::Video, "video/quicktime"),
        "ts" => (MediaKind::Video, "video/MP2T"),
        "m2ts" => (MediaKind::Video, "video/MP2T"),
        "mts" => (MediaKind::Video, "video/MP2T"),
        "mpg" => (MediaKind::Video, "video/mpeg"),
        "mpeg" => (MediaKind::Video, "video/mpeg"),
        "wmv" => (MediaKind::Video, "video/x-ms-wmv"),
        "flv" => (MediaKind::Video, "video/x-flv"),
        "ogv" => (MediaKind::Video, "video/ogg"),
        "webm" => (MediaKind::Video, "video/webm"),
        "3gp" => (MediaKind::Video, "video/3gpp"),

        // Audio
        "mp3" => (MediaKind::Audio, "audio/mpeg"),
        "flac" => (MediaKind::Audio, "audio/flac"),
        "wav" => (MediaKind::Audio, "audio/wav"),
        "m4a" => (MediaKind::Audio, "audio/mp4"),
        "aac" => (MediaKind::Audio, "audio/aac"),
        "ogg" => (MediaKind::Audio, "audio/ogg"),
        "oga" => (MediaKind::Audio, "audio/ogg"),
        "wma" => (MediaKind::Audio, "audio/x-ms-wma"),
        "opus" => (MediaKind::Audio, "audio/ogg"),
        "aiff" => (MediaKind::Audio, "audio/aiff"),
        "aif" => (MediaKind::Audio, "audio/aiff"),

        // Image
        "jpg" => (MediaKind::Image, "image/jpeg"),
        "jpeg" => (MediaKind::Image, "image/jpeg"),
        "png" => (MediaKind::Image, "image/png"),
        "gif" => (MediaKind::Image, "image/gif"),
        "webp" => (MediaKind::Image, "image/webp"),
        "bmp" => (MediaKind::Image, "image/bmp"),
        "tiff" => (MediaKind::Image, "image/tiff"),
        "tif" => (MediaKind::Image, "image/tiff"),

        // Subtitle — LOCKED DECISION: must be recognized, classified as Subtitle, NOT skipped
        "srt" => (MediaKind::Subtitle, "text/srt"),
        "vtt" => (MediaKind::Subtitle, "text/vtt"),

        // Everything else: silent skip
        _ => return None,
    };

    Some(result)
}
