//! Contest thumbnails: decode upload, resize for UI, store as WebP on disk.
//!
//! Two sizes per contest: thumb (~160px edge) for lists, detail (~512px) for lightbox/hover.

use std::io::{Cursor, Write};
use std::path::PathBuf;

use image::DynamicImage;

/// Max upload body before processing (JPEG/PNG/WebP from client).
pub const CONTEST_IMAGE_UPLOAD_MAX_BYTES: usize = 8 * 1024 * 1024;

/// Longest edge of list thumbnail (2× detail header ~80px).
pub const CONTEST_IMAGE_THUMB_MAX_EDGE: u32 = 160;

/// Longest edge of lightbox / hover preview.
pub const CONTEST_IMAGE_DETAIL_MAX_EDGE: u32 = 512;

const WEBP_QUALITY_THUMB: f32 = 82.0;
const WEBP_QUALITY_DETAIL: f32 = 85.0;

const STORED_EXT: &str = "webp";
const LEGACY_EXT: &str = "png";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageVariant {
    Thumb,
    Detail,
}

pub struct ProcessedContestImages {
    pub thumb: Vec<u8>,
    pub detail: Vec<u8>,
}

/// Directory for contest images (env `CONTEST_IMAGE_DIR`, default `/app/data/contest-images`).
pub fn image_dir() -> PathBuf {
    std::env::var("CONTEST_IMAGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/app/data/contest-images"))
}

pub fn ensure_image_dir() -> std::io::Result<()> {
    std::fs::create_dir_all(image_dir()).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("{}: {}", image_dir().display(), e),
        )
    })
}

pub fn thumb_path_for_key(contest_key: &str) -> PathBuf {
    image_dir().join(format!("{contest_key}.{STORED_EXT}"))
}

pub fn detail_path_for_key(contest_key: &str) -> PathBuf {
    image_dir().join(format!("{contest_key}.detail.{STORED_EXT}"))
}

/// Legacy single-file path (PNG or early WebP thumb only).
pub fn image_path_for_key(contest_key: &str) -> PathBuf {
    thumb_path_for_key(contest_key)
}

fn legacy_png_path(contest_key: &str) -> PathBuf {
    image_dir().join(format!("{contest_key}.{LEGACY_EXT}"))
}

pub fn api_path_for_key(contest_key: &str) -> String {
    shared::dto::contest::contest_image_api_path(contest_key)
}

pub fn api_detail_path_for_key(contest_key: &str) -> String {
    shared::dto::contest::contest_image_detail_api_path(contest_key)
}

fn encode_webp(img: DynamicImage, max_edge: u32, quality: f32) -> Result<Vec<u8>, String> {
    let thumb = img.thumbnail(max_edge, max_edge);
    let rgba = thumb.to_rgba8();
    let (tw, th) = (rgba.width(), rgba.height());
    let encoded = webp::Encoder::from_rgba(rgba.as_raw(), tw, th).encode(quality);
    Ok(encoded.to_vec())
}

/// Decode upload and produce thumb + detail WebP blobs.
pub fn process_image_upload(data: &[u8]) -> Result<ProcessedContestImages, String> {
    if data.is_empty() {
        return Err("empty image".to_string());
    }
    if data.len() > CONTEST_IMAGE_UPLOAD_MAX_BYTES {
        return Err(format!(
            "image exceeds maximum upload size ({} MB)",
            CONTEST_IMAGE_UPLOAD_MAX_BYTES / (1024 * 1024)
        ));
    }

    let img = image::ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| format!("could not read image format: {e}"))?
        .decode()
        .map_err(|e| format!("unsupported or corrupt image: {e}"))?;

    if img.width() == 0 || img.height() == 0 {
        return Err("invalid image dimensions".to_string());
    }

    let thumb = encode_webp(img.clone(), CONTEST_IMAGE_THUMB_MAX_EDGE, WEBP_QUALITY_THUMB)?;
    let detail = encode_webp(img, CONTEST_IMAGE_DETAIL_MAX_EDGE, WEBP_QUALITY_DETAIL)?;
    Ok(ProcessedContestImages { thumb, detail })
}

fn write_file_atomic(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    let mut tmp_str = path.to_string_lossy().to_string();
    tmp_str.push_str(".tmp");
    let tmp = PathBuf::from(tmp_str);
    {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| format!("create {}: {e}", tmp.display()))?;
        f.write_all(bytes)
            .map_err(|e| format!("write {}: {e}", tmp.display()))?;
        f.sync_all()
            .map_err(|e| format!("sync {}: {e}", tmp.display()))?;
    }
    std::fs::rename(&tmp, path)
        .map_err(|e| format!("rename {} -> {}: {e}", tmp.display(), path.display()))
}

pub fn write_image_atomic(contest_key: &str, upload: &[u8]) -> Result<(), String> {
    let processed = process_image_upload(upload)?;
    ensure_image_dir().map_err(|e| format!("contest image directory: {e}"))?;

    write_file_atomic(&thumb_path_for_key(contest_key), &processed.thumb)?;
    write_file_atomic(&detail_path_for_key(contest_key), &processed.detail)?;

    let legacy = legacy_png_path(contest_key);
    if legacy.is_file() {
        let _ = std::fs::remove_file(legacy);
    }
    Ok(())
}

pub fn delete_image_file(contest_key: &str) {
    for path in [
        thumb_path_for_key(contest_key),
        detail_path_for_key(contest_key),
        legacy_png_path(contest_key),
    ] {
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub fn image_file_exists(contest_key: &str) -> bool {
    thumb_path_for_key(contest_key).is_file()
        || detail_path_for_key(contest_key).is_file()
        || legacy_png_path(contest_key).is_file()
}

/// Stored WebP preferred; legacy PNG on thumb path only. Detail falls back to thumb when missing.
pub fn read_image_file(contest_key: &str, variant: ImageVariant) -> Option<(Vec<u8>, &'static str)> {
    match variant {
        ImageVariant::Detail => {
            let detail = detail_path_for_key(contest_key);
            if detail.is_file() {
                return std::fs::read(detail).ok().map(|b| (b, "image/webp"));
            }
            read_image_file(contest_key, ImageVariant::Thumb)
        }
        ImageVariant::Thumb => {
            let webp = thumb_path_for_key(contest_key);
            if webp.is_file() {
                return std::fs::read(webp).ok().map(|b| (b, "image/webp"));
            }
            let png = legacy_png_path(contest_key);
            std::fs::read(png).ok().map(|b| (b, "image/png"))
        }
    }
}

pub fn parse_has_image_from_json(v: &serde_json::Value) -> bool {
    v.get("has_image")
        .or_else(|| v.get("hasImage"))
        .and_then(|x| x.as_bool())
        .unwrap_or(false)
}

pub fn enrich_contest_dto(dto: &mut shared::dto::contest::ContestDto) {
    let key = crate::surreal_helpers::record_id_to_key(&dto.id, "contest");
    if dto.has_image || image_file_exists(&key) {
        dto.has_image = true;
        dto.image_url = Some(api_path_for_key(&key));
        dto.image_detail_url = Some(api_detail_path_for_key(&key));
    } else {
        dto.has_image = false;
        dto.image_url = None;
        dto.image_detail_url = None;
    }
}

/// Build an in-memory PNG (integration tests and unit tests).
pub fn sample_png_bytes(w: u32, h: u32) -> Vec<u8> {
    use image::RgbaImage;
    let img = RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
    });
    let mut buf = Vec::new();
    img.write_to(
        &mut Cursor::new(&mut buf),
        image::ImageFormat::Png,
    )
    .expect("png encode");
    buf
}

/// Build an in-memory JPEG.
pub fn sample_jpeg_bytes(w: u32, h: u32) -> Vec<u8> {
    use image::RgbImage;
    let img = RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([200, (x % 256) as u8, (y % 256) as u8])
    });
    let mut buf = Vec::new();
    img.write_to(
        &mut Cursor::new(&mut buf),
        image::ImageFormat::Jpeg,
    )
    .expect("jpeg encode");
    buf
}

fn is_webp(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP"
}

fn decoded_max_edge(bytes: &[u8]) -> u32 {
    image::load_from_memory(bytes)
        .map(|i| i.width().max(i.height()))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ImageDirGuard {
        previous: Option<String>,
        _temp: tempfile::TempDir,
    }

    impl ImageDirGuard {
        fn new() -> Self {
            let temp = tempfile::tempdir().expect("tempdir");
            let previous = std::env::var("CONTEST_IMAGE_DIR").ok();
            std::env::set_var(
                "CONTEST_IMAGE_DIR",
                temp.path().to_string_lossy().as_ref(),
            );
            Self {
                previous,
                _temp: temp,
            }
        }
    }

    impl Drop for ImageDirGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(v) => std::env::set_var("CONTEST_IMAGE_DIR", v),
                None => std::env::remove_var("CONTEST_IMAGE_DIR"),
            }
        }
    }

    #[test]
    fn process_resizes_large_upload_to_thumb_and_detail() {
        let input = sample_png_bytes(1200, 900);
        let out = process_image_upload(&input).expect("process");
        assert!(!out.thumb.is_empty());
        assert!(!out.detail.is_empty());
        assert!(decoded_max_edge(&out.thumb) <= CONTEST_IMAGE_THUMB_MAX_EDGE);
        assert!(decoded_max_edge(&out.detail) <= CONTEST_IMAGE_DETAIL_MAX_EDGE);
        assert!(out.detail.len() > out.thumb.len());
    }

    #[test]
    fn process_rejects_garbage() {
        assert!(process_image_upload(b"not an image").is_err());
    }

    #[test]
    fn process_rejects_oversized_upload() {
        let huge = vec![0u8; CONTEST_IMAGE_UPLOAD_MAX_BYTES + 1];
        assert!(process_image_upload(&huge).is_err());
    }

    #[test]
    #[serial_test::serial]
    fn process_accepts_jpeg_and_outputs_webp() {
        let input = sample_jpeg_bytes(640, 480);
        let out = process_image_upload(&input).expect("jpeg process");
        assert!(is_webp(&out.thumb));
        assert!(is_webp(&out.detail));
    }

    #[test]
    #[serial_test::serial]
    fn write_read_roundtrip_stores_both_sizes() {
        let _guard = ImageDirGuard::new();
        let png = sample_png_bytes(800, 600);
        write_image_atomic("roundtrip-key", &png).expect("write");
        let (thumb, _) = read_image_file("roundtrip-key", ImageVariant::Thumb).expect("thumb");
        let (detail, _) = read_image_file("roundtrip-key", ImageVariant::Detail).expect("detail");
        assert!(is_webp(&thumb));
        assert!(is_webp(&detail));
        assert!(detail_path_for_key("roundtrip-key").is_file());
        assert!(decoded_max_edge(&detail) >= decoded_max_edge(&thumb));
    }

    #[test]
    #[serial_test::serial]
    fn detail_read_falls_back_to_thumb_when_only_legacy_thumb() {
        let _guard = ImageDirGuard::new();
        let png = sample_png_bytes(400, 300);
        let processed = process_image_upload(&png).expect("process");
        std::fs::write(thumb_path_for_key("legacy-fallback"), &processed.thumb).expect("write");
        let (bytes, _) = read_image_file("legacy-fallback", ImageVariant::Detail).expect("read");
        assert!(is_webp(&bytes));
    }

    #[test]
    #[serial_test::serial]
    fn delete_image_file_removes_both_sizes() {
        let _guard = ImageDirGuard::new();
        write_image_atomic("del-key", &sample_png_bytes(32, 32)).expect("write");
        assert!(image_file_exists("del-key"));
        delete_image_file("del-key");
        assert!(!image_file_exists("del-key"));
    }
}
