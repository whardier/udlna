use std::path::{Path, PathBuf};
use std::time::Instant;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::media::library::{MediaItem, MediaLibrary};
use crate::media::metadata::{extract_metadata, media_item_id, MACHINE_NAMESPACE};
use crate::media::mime::{classify, MediaKind};

/// Statistics collected during scanning for the startup summary line.
/// Reserved for Phase 3+ callers that need scan statistics.
pub struct ScanStats {
    pub total: usize,
    pub video: usize,
    pub audio: usize,
    pub image: usize,
    pub elapsed_secs: f64,
}

/// Scan all provided paths and return a MediaLibrary with all discovered media items.
/// Symlinks are followed. Missing/unreadable paths log warn and continue.
/// Per LOCKED decision: MediaKind::Subtitle items are excluded from library.items.
pub fn scan(paths: &[PathBuf]) -> MediaLibrary {
    let start = Instant::now();
    let machine_ns = *MACHINE_NAMESPACE;
    let mut library = MediaLibrary::new();
    let mut video_count = 0usize;
    let mut audio_count = 0usize;
    let mut image_count = 0usize;

    for root in paths {
        // LOCKED: warn and continue if directory is missing — do not abort startup
        if !root.exists() {
            tracing::warn!("Scan path does not exist, skipping: {}", root.display());
            continue;
        }
        for entry in WalkDir::new(root).follow_links(true) {
            match entry {
                Err(e) => {
                    // LOCKED: log warn for unreadable files / broken symlinks, continue
                    tracing::warn!("Cannot access entry: {}", e);
                }
                Ok(entry) if entry.file_type().is_file() => {
                    process_file(
                        entry.path(),
                        &machine_ns,
                        &mut library,
                        &mut video_count,
                        &mut audio_count,
                        &mut image_count,
                    );
                }
                Ok(_) => {} // directory entries — walkdir handles recursion
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total = library.items.len();
    tracing::info!(
        "Scanned {} files ({} video, {} audio, {} image) in {:.1}s",
        total,
        video_count,
        audio_count,
        image_count,
        elapsed
    );

    library
}

fn process_file(
    path: &Path,
    machine_ns: &Uuid,
    library: &mut MediaLibrary,
    video_count: &mut usize,
    audio_count: &mut usize,
    image_count: &mut usize,
) {
    // classify() returns None for non-media files (silently skipped) and for unrecognized extensions
    let Some((kind, mime)) = classify(path) else {
        return;
    };

    // LOCKED: Subtitle files must NOT appear as media items in the library.
    // They are recognized by classify() but excluded here (handled in Phase 3/5).
    if kind == MediaKind::Subtitle {
        tracing::debug!(
            "Subtitle file recognized but excluded from library: {}",
            path.display()
        );
        return;
    }

    // LOCKED: canonicalize path for UUID stability (Pitfall 5 in RESEARCH.md)
    let canonical = match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("Cannot canonicalize {}: {}", path.display(), e);
            return;
        }
    };

    let file_size = match std::fs::metadata(&canonical) {
        Ok(m) => m.len(),
        Err(e) => {
            tracing::warn!("Cannot stat {}: {}", canonical.display(), e);
            return;
        }
    };

    // LOCKED: skip file entirely on metadata extraction failure (not include with null fields)
    let Some(meta) = extract_metadata(&canonical, kind, mime) else {
        tracing::warn!(
            "Skipping {} — metadata extraction failed",
            canonical.display()
        );
        return;
    };

    let id = media_item_id(machine_ns, &canonical);

    match kind {
        MediaKind::Video => *video_count += 1,
        MediaKind::Audio => *audio_count += 1,
        MediaKind::Image => *image_count += 1,
        MediaKind::Subtitle => unreachable!("filtered above"),
    }

    let item = MediaItem {
        id,
        path: canonical,
        file_size,
        mime,
        kind,
        meta,
    };
    tracing::debug!("indexed {} -> {}", item.id, item.path.display());
    library.items.push(item);
}
