//! Contest thumbnails: decode upload, resize for UI, store as WebP on disk.

use std::io::{Cursor, Write};
use std::path::PathBuf;


/// Max upload body before processing (JPEG/PNG/WebP from client).
pub const CONTEST_IMAGE_UPLOAD_MAX_BYTES: usize = 8 * 1024 * 1024;

/// Longest edge of stored thumbnail (detail header is 80px; 2× for retina).
pub const CONTEST_IMAGE_OUTPUT_MAX_EDGE: u32 = 160;

/// WebP quality (0–100). Balance of size vs clarity for small thumbnails.
const WEBP_QUALITY: f32 = 82.0;

const STORED_EXT: &str = "webp";
const LEGACY_EXT: &str = "png";

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

pub fn image_path_for_key(contest_key: &str) -> PathBuf {
    image_dir().join(format!("{contest_key}.{STORED_EXT}"))
}

fn legacy_png_path(contest_key: &str) -> PathBuf {
    image_dir().join(format!("{contest_key}.{LEGACY_EXT}"))
}

pub fn api_path_for_key(contest_key: &str) -> String {
    shared::dto::contest::contest_image_api_path(contest_key)
}

/// Decode, fit within `CONTEST_IMAGE_OUTPUT_MAX_EDGE`, encode WebP.
pub fn process_image_upload(data: &[u8]) -> Result<Vec<u8>, String> {
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

    let (w, h) = (img.width(), img.height());
    if w == 0 || h == 0 {
        return Err("invalid image dimensions".to_string());
    }

    let thumb = img.thumbnail(CONTEST_IMAGE_OUTPUT_MAX_EDGE, CONTEST_IMAGE_OUTPUT_MAX_EDGE);
    let rgba = thumb.to_rgba8();
    let (tw, th) = (rgba.width(), rgba.height());

    let encoded = webp::Encoder::from_rgba(rgba.as_raw(), tw, th).encode(WEBP_QUALITY);
    Ok(encoded.to_vec())
}

pub fn write_image_atomic(contest_key: &str, upload: &[u8]) -> Result<(), String> {
    let webp = process_image_upload(upload)?;
    ensure_image_dir().map_err(|e| format!("contest image directory: {e}"))?;
    let path = image_path_for_key(contest_key);
    let tmp = path.with_extension(format!("{STORED_EXT}.tmp"));
    {
        let mut f = std::fs::File::create(&tmp)
            .map_err(|e| format!("create {}: {e}", tmp.display()))?;
        f.write_all(&webp)
            .map_err(|e| format!("write {}: {e}", tmp.display()))?;
        f.sync_all()
            .map_err(|e| format!("sync {}: {e}", tmp.display()))?;
    }
    std::fs::rename(&tmp, &path)
        .map_err(|e| format!("rename {} -> {}: {e}", tmp.display(), path.display()))?;
    // Drop legacy PNG from earlier versions.
    let legacy = legacy_png_path(contest_key);
    if legacy.is_file() {
        let _ = std::fs::remove_file(legacy);
    }
    Ok(())
}

pub fn delete_image_file(contest_key: &str) {
    for path in [image_path_for_key(contest_key), legacy_png_path(contest_key)] {
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
}

pub fn image_file_exists(contest_key: &str) -> bool {
    image_path_for_key(contest_key).is_file() || legacy_png_path(contest_key).is_file()
}

/// Stored WebP preferred; legacy PNG still served for old uploads.
pub fn read_image_file(contest_key: &str) -> Option<(Vec<u8>, &'static str)> {
    let webp = image_path_for_key(contest_key);
    if webp.is_file() {
        return std::fs::read(webp)
            .ok()
            .map(|b| (b, "image/webp"));
    }
    let png = legacy_png_path(contest_key);
    std::fs::read(png).ok().map(|b| (b, "image/png"))
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
    } else {
        dto.has_image = false;
        dto.image_url = None;
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
    fn process_resizes_large_upload_to_max_edge() {
        let input = sample_png_bytes(800, 600);
        let out = process_image_upload(&input).expect("process");
        assert!(!out.is_empty());
        let decoded = image::load_from_memory(&out).expect("webp decode");
        assert!(decoded.width() <= CONTEST_IMAGE_OUTPUT_MAX_EDGE);
        assert!(decoded.height() <= CONTEST_IMAGE_OUTPUT_MAX_EDGE);
        assert!(out.len() < input.len());
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
        assert!(is_webp(&out));
        let decoded = image::load_from_memory(&out).expect("webp decode");
        assert!(decoded.width() <= CONTEST_IMAGE_OUTPUT_MAX_EDGE);
        assert!(decoded.height() <= CONTEST_IMAGE_OUTPUT_MAX_EDGE);
    }

    #[test]
    #[serial_test::serial]
    fn write_read_roundtrip_stores_webp_on_disk() {
        let _guard = ImageDirGuard::new();
        let png = sample_png_bytes(400, 300);
        write_image_atomic("roundtrip-key", &png).expect("write");
        let (bytes, mime) = read_image_file("roundtrip-key").expect("read");
        assert_eq!(mime, "image/webp");
        assert!(is_webp(&bytes));
        assert!(image_path_for_key("roundtrip-key").is_file());
    }

    #[test]
    #[serial_test::serial]
    fn delete_image_file_removes_webp() {
        let _guard = ImageDirGuard::new();
        write_image_atomic("del-key", &sample_png_bytes(32, 32)).expect("write");
        assert!(image_file_exists("del-key"));
        delete_image_file("del-key");
        assert!(!image_file_exists("del-key"));
    }
}
