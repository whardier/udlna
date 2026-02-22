use std::path::Path;
use udlna::media::mime::{classify, MediaKind};

#[test]
fn test_mp4_classified_as_video() {
    let (kind, mime) = classify(Path::new("movie.mp4")).unwrap();
    assert_eq!(kind, MediaKind::Video);
    assert_eq!(mime, "video/mp4");
}

#[test]
fn test_srt_classified_as_subtitle() {
    let (kind, mime) = classify(Path::new("movie.srt")).unwrap();
    assert_eq!(kind, MediaKind::Subtitle);
    assert_eq!(mime, "text/srt");
}

#[test]
fn test_txt_returns_none() {
    assert!(classify(Path::new("readme.txt")).is_none());
}

#[test]
fn test_no_extension_returns_none() {
    assert!(classify(Path::new("Makefile")).is_none());
}

#[test]
fn test_case_insensitive() {
    // Extensions should be lowercased before matching
    let result = classify(Path::new("MOVIE.MP4"));
    assert!(result.is_some());
}

#[test]
fn test_mp3_classified_as_audio() {
    let (kind, mime) = classify(Path::new("song.mp3")).unwrap();
    assert_eq!(kind, MediaKind::Audio);
    assert_eq!(mime, "audio/mpeg");
}

#[test]
fn test_jpeg_classified_as_image() {
    let (kind, mime) = classify(Path::new("photo.jpg")).unwrap();
    assert_eq!(kind, MediaKind::Image);
    assert_eq!(mime, "image/jpeg");
}

#[test]
fn test_mkv_mime_is_matroska() {
    let (_, mime) = classify(Path::new("video.mkv")).unwrap();
    assert_eq!(mime, "video/x-matroska");
}
