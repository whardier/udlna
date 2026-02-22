use std::path::PathBuf;
use uuid::Uuid;
use udlna::media::metadata::{
    build_machine_namespace, dlna_profile_for, extract_metadata, format_upnp_duration, media_item_id,
};
use udlna::media::mime::MediaKind;

// ── format_upnp_duration ─────────────────────────────────────────────────────

#[test]
fn duration_zero() {
    assert_eq!(format_upnp_duration(0, 0.0), "00:00:00.000");
}

#[test]
fn duration_one_minute() {
    assert_eq!(format_upnp_duration(60, 0.0), "00:01:00.000");
}

#[test]
fn duration_one_hour_one_min_one_sec_half() {
    assert_eq!(format_upnp_duration(3661, 0.5), "01:01:01.500");
}

#[test]
fn duration_272_sec_frac_841() {
    assert_eq!(format_upnp_duration(272, 0.841), "00:04:32.841");
}

#[test]
fn duration_5025_sec() {
    assert_eq!(format_upnp_duration(5025, 0.0), "01:23:45.000");
}

#[test]
fn duration_zero_frac_999() {
    assert_eq!(format_upnp_duration(0, 0.999), "00:00:00.999");
}

#[test]
fn duration_max_realistic() {
    let secs = 3600 * 99 + 59 * 60 + 59;
    assert_eq!(format_upnp_duration(secs, 0.999), "99:59:59.999");
}

// ── dlna_profile_for ─────────────────────────────────────────────────────────

#[test]
fn profile_audio_mpeg_is_mp3() {
    assert_eq!(dlna_profile_for("audio/mpeg"), Some("MP3"));
}

#[test]
fn profile_audio_mp4_is_aac() {
    assert_eq!(dlna_profile_for("audio/mp4"), Some("AAC_ISO_320"));
}

#[test]
fn profile_image_jpeg_is_jpeg_lrg() {
    assert_eq!(dlna_profile_for("image/jpeg"), Some("JPEG_LRG"));
}

#[test]
fn profile_image_png_is_png_lrg() {
    assert_eq!(dlna_profile_for("image/png"), Some("PNG_LRG"));
}

#[test]
fn profile_audio_ogg_is_none() {
    assert_eq!(dlna_profile_for("audio/ogg"), None);
}

#[test]
fn profile_audio_flac_is_none() {
    assert_eq!(dlna_profile_for("audio/flac"), None);
}

#[test]
fn profile_audio_wav_is_none() {
    assert_eq!(dlna_profile_for("audio/wav"), None);
}

#[test]
fn profile_audio_wma_is_none() {
    assert_eq!(dlna_profile_for("audio/x-ms-wma"), None);
}

#[test]
fn profile_video_mp4_is_none_phase2() {
    assert_eq!(dlna_profile_for("video/mp4"), None);
}

#[test]
fn profile_video_mkv_is_none() {
    assert_eq!(dlna_profile_for("video/x-matroska"), None);
}

#[test]
fn profile_video_avi_is_none() {
    assert_eq!(dlna_profile_for("video/x-msvideo"), None);
}

#[test]
fn profile_video_ts_is_none() {
    assert_eq!(dlna_profile_for("video/MP2T"), None);
}

#[test]
fn profile_image_gif_is_none() {
    assert_eq!(dlna_profile_for("image/gif"), None);
}

#[test]
fn profile_application_unknown_is_none() {
    assert_eq!(dlna_profile_for("application/unknown"), None);
}

#[test]
fn profile_arbitrary_string_is_none() {
    assert_eq!(dlna_profile_for("totally/made-up"), None);
}

// ── build_machine_namespace / media_item_id ───────────────────────────────────

#[test]
fn machine_namespace_is_non_nil() {
    let ns = build_machine_namespace();
    assert_ne!(ns, Uuid::nil());
}

#[test]
fn machine_namespace_is_deterministic() {
    let ns1 = build_machine_namespace();
    let ns2 = build_machine_namespace();
    assert_eq!(ns1, ns2);
}

#[test]
fn media_item_id_is_non_nil() {
    let ns = build_machine_namespace();
    let path = PathBuf::from("/some/canonical/path.mp3");
    let id = media_item_id(&ns, &path);
    assert_ne!(id, Uuid::nil());
}

#[test]
fn media_item_id_is_deterministic() {
    let ns = build_machine_namespace();
    let path = PathBuf::from("/some/canonical/path.mp3");
    let id1 = media_item_id(&ns, &path);
    let id2 = media_item_id(&ns, &path);
    assert_eq!(id1, id2);
}

#[test]
fn media_item_id_differs_for_different_paths() {
    let ns = build_machine_namespace();
    let path_a = PathBuf::from("/media/file_a.mp3");
    let path_b = PathBuf::from("/media/file_b.mp3");
    let id_a = media_item_id(&ns, &path_a);
    let id_b = media_item_id(&ns, &path_b);
    assert_ne!(id_a, id_b);
}

// ── extract_metadata ──────────────────────────────────────────────────────────

#[test]
fn extract_metadata_audio_nonexistent_returns_none() {
    let path = PathBuf::from("/nonexistent/file.mp3");
    let result = extract_metadata(&path, MediaKind::Audio, "audio/mpeg");
    assert!(result.is_none());
}

#[test]
fn extract_metadata_video_nonexistent_returns_none() {
    let path = PathBuf::from("/nonexistent/file.mp4");
    let result = extract_metadata(&path, MediaKind::Video, "video/mp4");
    assert!(result.is_none());
}

#[test]
fn extract_metadata_image_nonexistent_returns_none() {
    let path = PathBuf::from("/nonexistent/file.jpg");
    let result = extract_metadata(&path, MediaKind::Image, "image/jpeg");
    assert!(result.is_none());
}

#[test]
fn extract_metadata_subtitle_returns_none() {
    let path = PathBuf::from("/any/file.srt");
    let result = extract_metadata(&path, MediaKind::Subtitle, "text/srt");
    assert!(result.is_none());
}
