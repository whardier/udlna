use std::io::BufReader;
use std::path::Path;
use uuid::Uuid;

use crate::media::library::MediaMeta;
use crate::media::mime::MediaKind;

/// Format a duration for UPnP/DLNA. `total_seconds` is the whole-second count;
/// `frac` is the sub-second fraction in [0.0, 1.0).
/// Returns a string in "HH:MM:SS.mmm" format (hours zero-padded to at least 2 digits).
pub fn format_upnp_duration(total_seconds: u64, frac: f64) -> String {
    let h = total_seconds / 3600;
    let m = (total_seconds % 3600) / 60;
    let s = total_seconds % 60;
    let ms = (frac * 1000.0).round() as u32;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}

/// Return the static DLNA profile name for a given MIME type, or None when the
/// type has no assigned profile (Phase 2 scope).
/// video/mp4 returns None — DLNA profile deferred to Phase 5 per RESEARCH.md.
pub fn dlna_profile_for(mime: &str) -> Option<&'static str> {
    match mime {
        "audio/mpeg" => Some("MP3"),
        "audio/mp4" => Some("AAC_ISO_320"),
        "image/jpeg" => Some("JPEG_LRG"),
        "image/png" => Some("PNG_LRG"),
        _ => None,
    }
}

/// Derive a machine-specific UUID namespace by seeding UUIDv5 from the machine UID.
/// Always returns the same value on the same machine (deterministic).
/// Falls back to "unknown" if machine_uid::get() fails.
pub fn build_machine_namespace() -> Uuid {
    let machine_id = machine_uid::get().unwrap_or_else(|_| "unknown".to_string());
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, machine_id.as_bytes())
}

/// Cached machine-specific UUID namespace. Computed once at first access.
pub static MACHINE_NAMESPACE: std::sync::LazyLock<Uuid> =
    std::sync::LazyLock::new(build_machine_namespace);

/// Derive a stable UUIDv5 for a media file using `namespace` (from
/// `build_machine_namespace`) and the file's canonical path bytes.
pub fn media_item_id(namespace: &Uuid, canonical_path: &Path) -> Uuid {
    Uuid::new_v5(namespace, canonical_path.as_os_str().as_encoded_bytes())
}

/// Extract metadata from a media file by reading its container headers.
/// Returns None if extraction fails entirely — partial data is never returned.
/// Per LOCKED decision: skip files that fail extraction entirely (not include with null fields).
pub fn extract_metadata(path: &Path, kind: MediaKind, mime: &'static str) -> Option<MediaMeta> {
    match kind {
        MediaKind::Audio => extract_audio_meta(path, mime),
        MediaKind::Video => extract_video_meta(path, mime),
        MediaKind::Image => extract_image_meta(path, mime),
        MediaKind::Subtitle => None, // Subtitles are filtered before this is called
    }
}

/// Extract metadata from an audio file using symphonia.
/// Returns None if the file cannot be opened or probed — never panics.
fn extract_audio_meta(path: &Path, mime: &'static str) -> Option<MediaMeta> {
    use symphonia::core::codecs::CODEC_TYPE_NULL;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .ok()?;

    let format = probed.format;

    // Select first non-null track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;

    // Duration via n_frames × time_base (works for FLAC, WAV, OGG, M4A with known frame count)
    // For VBR MP3 without Xing header, n_frames may be None — log and leave duration as None
    let duration = track
        .codec_params
        .time_base
        .and_then(|tb| {
            track.codec_params.n_frames.map(|n| {
                let t = tb.calc_time(n);
                format_upnp_duration(t.seconds, t.frac)
            })
        });

    if duration.is_none() {
        tracing::debug!("No duration for {} (n_frames unavailable)", path.display());
    }

    // Bitrate from bits_per_coded_sample if available
    let bitrate = track
        .codec_params
        .bits_per_coded_sample;

    Some(MediaMeta {
        duration,
        resolution: None,
        bitrate,
        dlna_profile: dlna_profile_for(mime),
    })
}

/// Extract metadata from a video file.
/// For MP4/M4V, uses the mp4 crate for width/height/duration.
/// For other video formats (MKV, AVI, etc.), uses symphonia for audio track duration;
/// resolution is left as None since symphonia does not expose video frame dimensions.
fn extract_video_meta(path: &Path, mime: &'static str) -> Option<MediaMeta> {
    match mime {
        "video/mp4" | "video/x-m4v" => extract_mp4_video_meta(path, mime),
        _ => extract_non_mp4_video_meta(path, mime),
    }
}

/// Extract MP4/M4V video metadata using the mp4 crate.
fn extract_mp4_video_meta(path: &Path, mime: &'static str) -> Option<MediaMeta> {
    use mp4::TrackType;

    let file = std::fs::File::open(path).ok()?;
    let file_len = file.metadata().ok()?.len();
    let reader = BufReader::new(file);

    let mp4 = mp4::Mp4Reader::read_header(reader, file_len).ok()?;

    // Duration in milliseconds from the container
    let duration_ms = mp4.duration().as_millis() as u64;
    let duration = if duration_ms > 0 {
        Some(format_upnp_duration(
            duration_ms / 1000,
            (duration_ms % 1000) as f64 / 1000.0,
        ))
    } else {
        None
    };

    // Find first video track for resolution and bitrate
    let mut resolution = None;
    let mut bitrate = None;

    for track in mp4.tracks().values() {
        if matches!(track.track_type(), Ok(TrackType::Video)) {
            let w = track.width();
            let h = track.height();
            if w > 0 && h > 0 {
                resolution = Some(format!("{}x{}", w, h));
            }
            let bps = track.bitrate();
            if bps > 0 {
                bitrate = Some(bps);
            }
            break;
        }
    }

    Some(MediaMeta {
        duration,
        resolution,
        bitrate,
        dlna_profile: dlna_profile_for(mime),
    })
}

/// Extract non-MP4 video metadata (MKV, AVI, etc.) using symphonia for audio track duration.
/// Resolution is not available — symphonia does not expose video frame dimensions (RESEARCH.md Pitfall 2).
fn extract_non_mp4_video_meta(path: &Path, mime: &'static str) -> Option<MediaMeta> {
    use symphonia::core::codecs::CODEC_TYPE_NULL;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .ok()?;

    let format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)?;

    let duration = track
        .codec_params
        .time_base
        .and_then(|tb| {
            track.codec_params.n_frames.map(|n| {
                let t = tb.calc_time(n);
                format_upnp_duration(t.seconds, t.frac)
            })
        });

    tracing::debug!("No video resolution for {} (non-MP4)", path.display());

    Some(MediaMeta {
        duration,
        resolution: None,
        bitrate: None,
        dlna_profile: dlna_profile_for(mime),
    })
}

/// Extract image metadata (dimensions only) using imagesize.
/// Uses header-only reads (~16 bytes) — never fully decodes the image.
/// Returns None if imagesize fails — LOCKED: skip file on extraction failure.
fn extract_image_meta(path: &Path, mime: &'static str) -> Option<MediaMeta> {
    match imagesize::size(path) {
        Ok(dim) => {
            let resolution = format!("{}x{}", dim.width, dim.height);
            Some(MediaMeta {
                duration: None,
                resolution: Some(resolution),
                bitrate: None,
                dlna_profile: dlna_profile_for(mime),
            })
        }
        Err(e) => {
            tracing::warn!("Cannot read image dimensions for {}: {}", path.display(), e);
            None
        }
    }
}
