//! Common test utilities

#![allow(dead_code)]

use image::{ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

/// Generate a test image with a solid color and optional gradient
pub fn generate_test_image(width: u32, height: u32, base_color: [u8; 4]) -> RgbaImage {
    let mut img = ImageBuffer::new(width, height);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        // Add subtle gradient to make frames distinguishable
        let r = base_color[0].saturating_add((x % 50) as u8);
        let g = base_color[1].saturating_add((y % 50) as u8);
        let b = base_color[2];
        let a = base_color[3];
        *pixel = Rgba([r, g, b, a]);
    }

    img
}

/// Generate a numbered test image (useful for slideshow testing)
pub fn generate_numbered_image(width: u32, height: u32, number: u32) -> RgbaImage {
    let colors = [
        [255, 100, 100, 255], // Red-ish
        [100, 255, 100, 255], // Green-ish
        [100, 100, 255, 255], // Blue-ish
        [255, 255, 100, 255], // Yellow-ish
        [255, 100, 255, 255], // Magenta-ish
        [100, 255, 255, 255], // Cyan-ish
    ];

    let color = colors[(number as usize) % colors.len()];
    generate_test_image(width, height, color)
}

/// Save a test image as JPEG
pub fn save_jpeg<P: AsRef<Path>>(img: &RgbaImage, path: P, quality: u8) -> std::io::Result<()> {
    // Convert RGBA to RGB for JPEG
    let rgb_img: image::RgbImage = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();

    let file = std::fs::File::create(path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, quality);
    encoder
        .encode_image(&rgb_img)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    Ok(())
}

/// Save a test image as PNG
pub fn save_png<P: AsRef<Path>>(img: &RgbaImage, path: P) -> std::io::Result<()> {
    img.save(path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}

/// Verify that a file exists and has non-zero size
pub fn verify_file_exists_with_size<P: AsRef<Path>>(path: P) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => meta.len() > 0,
        Err(_) => false,
    }
}

/// Parse WebM header to verify it's a valid WebM file
pub fn verify_webm_header<P: AsRef<Path>>(path: P) -> bool {
    use std::io::Read;

    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut header = [0u8; 4];
    if file.read_exact(&mut header).is_err() {
        return false;
    }

    // WebM starts with EBML header: 0x1A 0x45 0xDF 0xA3
    header == [0x1A, 0x45, 0xDF, 0xA3]
}

/// Parse MP4 header to verify it's a valid MP4 file
pub fn verify_mp4_header<P: AsRef<Path>>(path: P) -> bool {
    use std::io::Read;

    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut header = [0u8; 12];
    if file.read_exact(&mut header).is_err() {
        return false;
    }

    // MP4 files have 'ftyp' box at offset 4
    &header[4..8] == b"ftyp"
}

/// Get file size in bytes
pub fn get_file_size<P: AsRef<Path>>(path: P) -> Option<u64> {
    std::fs::metadata(path).ok().map(|m| m.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_test_image() {
        let img = generate_test_image(100, 100, [255, 0, 0, 255]);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }

    #[test]
    fn test_save_jpeg() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.jpg");

        let img = generate_test_image(100, 100, [255, 0, 0, 255]);
        save_jpeg(&img, &path, 85).unwrap();

        assert!(verify_file_exists_with_size(&path));
    }

    #[test]
    fn test_save_png() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.png");

        let img = generate_test_image(100, 100, [255, 0, 0, 255]);
        save_png(&img, &path).unwrap();

        assert!(verify_file_exists_with_size(&path));
    }
}
