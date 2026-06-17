//! Ephemeral sell-listing photo storage (WebP thumb + detail, like contest images).

use crate::contest::image::process_image_upload;
use std::io::Write;
use std::path::{Path, PathBuf};

const STORED_EXT: &str = "webp";

/// Max raw upload per photo (default 8 MiB).
pub fn max_upload_bytes() -> usize {
    std::env::var("SELL_IMAGE_MAX_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8 * 1024 * 1024)
}

/// Max photos per listing.
pub fn max_photo_count() -> u32 {
    std::env::var("SELL_IMAGE_MAX_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8)
}

/// TTL in hours for listing media.
pub fn ttl_hours() -> i64 {
    std::env::var("SELL_IMAGE_TTL_HOURS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24)
}

pub fn image_dir() -> PathBuf {
    std::env::var("SELL_IMAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/app/data/sell-images"))
}

pub fn ensure_image_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(image_dir()).map_err(|e| {
        std::io::Error::new(e.kind(), format!("{}: {}", image_dir().display(), e))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SellPhotoVariant {
    Thumb,
    Detail,
}

pub fn thumb_path(listing_key: &str, photo_key: &str) -> PathBuf {
    image_dir().join(format!("{listing_key}_{photo_key}.thumb.{STORED_EXT}"))
}

pub fn detail_path(listing_key: &str, photo_key: &str) -> PathBuf {
    image_dir().join(format!("{listing_key}_{photo_key}.detail.{STORED_EXT}"))
}

/// Legacy raw upload path (pre-WebP); kept for read fallback.
pub fn legacy_bin_path(listing_key: &str, photo_key: &str) -> PathBuf {
    image_dir().join(format!("{listing_key}_{photo_key}.bin"))
}

/// Full-quality path for BGG export / Playwright upload.
pub fn photo_path(listing_key: &str, photo_key: &str) -> PathBuf {
    detail_path(listing_key, photo_key)
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn write_photo_atomic(listing_key: &str, photo_key: &str, data: &[u8]) -> Result<(), String> {
    if data.is_empty() {
        return Err("empty image".to_string());
    }
    if data.len() > max_upload_bytes() {
        return Err(format!(
            "image exceeds maximum upload size ({} MB)",
            max_upload_bytes() / (1024 * 1024)
        ));
    }
    ensure_image_dir().map_err(|e| e.to_string())?;
    let processed = process_image_upload(data)?;
    write_file_atomic(&thumb_path(listing_key, photo_key), &processed.thumb)?;
    write_file_atomic(&detail_path(listing_key, photo_key), &processed.detail)?;
    let legacy = legacy_bin_path(listing_key, photo_key);
    if legacy.is_file() {
        let _ = std::fs::remove_file(legacy);
    }
    Ok(())
}

pub fn read_photo_variant(
    listing_key: &str,
    photo_key: &str,
    variant: SellPhotoVariant,
) -> Option<(Vec<u8>, &'static str)> {
    match variant {
        SellPhotoVariant::Detail => {
            let detail = detail_path(listing_key, photo_key);
            if detail.is_file() {
                return std::fs::read(detail).ok().map(|b| (b, "image/webp"));
            }
            let legacy = legacy_bin_path(listing_key, photo_key);
            if legacy.is_file() {
                return std::fs::read(legacy).ok().map(|b| (b, "image/jpeg"));
            }
            read_photo_variant(listing_key, photo_key, SellPhotoVariant::Thumb)
        }
        SellPhotoVariant::Thumb => {
            let thumb = thumb_path(listing_key, photo_key);
            if thumb.is_file() {
                return std::fs::read(thumb).ok().map(|b| (b, "image/webp"));
            }
            let legacy = legacy_bin_path(listing_key, photo_key);
            if legacy.is_file() {
                return std::fs::read(legacy).ok().map(|b| (b, "image/jpeg"));
            }
            let detail = detail_path(listing_key, photo_key);
            if detail.is_file() {
                return std::fs::read(detail).ok().map(|b| (b, "image/webp"));
            }
            None
        }
    }
}

pub fn delete_photo_file(listing_key: &str, photo_key: &str) {
    for path in [
        thumb_path(listing_key, photo_key),
        detail_path(listing_key, photo_key),
        legacy_bin_path(listing_key, photo_key),
    ] {
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub fn delete_all_listing_photos(listing_key: &str, photo_keys: &[String]) {
    for key in photo_keys {
        delete_photo_file(listing_key, key);
    }
}

pub fn upload_content_type_allowed(content_type: &str) -> bool {
    let ct = content_type.split(';').next().unwrap_or(content_type).trim();
    ct.starts_with("image/")
        || ct.eq_ignore_ascii_case("application/octet-stream")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_use_listing_and_photo_keys() {
        assert!(thumb_path("listing-a", "photo-b")
            .to_string_lossy()
            .contains("listing-a_photo-b.thumb.webp"));
        assert!(detail_path("listing-a", "photo-b")
            .to_string_lossy()
            .contains("listing-a_photo-b.detail.webp"));
    }
}
