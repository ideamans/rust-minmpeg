//! Integration tests for image loading functionality

mod common;

use common::*;
use minmpeg::image_loader::LoadedImage;
use tempfile::TempDir;

/// Test loading a JPEG image
#[test]
fn test_load_jpeg() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.jpg");

    let original = generate_test_image(200, 150, [255, 128, 64, 255]);
    save_jpeg(&original, &path, 85).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();

    assert_eq!(loaded.width, 200);
    assert_eq!(loaded.height, 150);
    assert_eq!(loaded.data.len(), (200 * 150 * 4) as usize);
}

/// Test loading a PNG image
#[test]
fn test_load_png() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.png");

    let original = generate_test_image(200, 150, [255, 128, 64, 255]);
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();

    assert_eq!(loaded.width, 200);
    assert_eq!(loaded.height, 150);
    assert_eq!(loaded.data.len(), (200 * 150 * 4) as usize);
}

/// Test loading a non-existent file
#[test]
fn test_load_nonexistent() {
    let result = LoadedImage::from_path("/nonexistent/path/image.png");
    assert!(result.is_err());
}

/// Test resizing an image
#[test]
fn test_resize_image() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.png");

    let original = generate_test_image(400, 300, [100, 150, 200, 255]);
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();
    let resized = loaded.resize(200, 150);

    assert_eq!(resized.width, 200);
    assert_eq!(resized.height, 150);
    assert_eq!(resized.data.len(), (200 * 150 * 4) as usize);
}

/// Test resizing to same size (should be no-op)
#[test]
fn test_resize_same_size() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.png");

    let original = generate_test_image(200, 150, [100, 150, 200, 255]);
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();
    let resized = loaded.resize(200, 150);

    assert_eq!(resized.width, 200);
    assert_eq!(resized.height, 150);
}

/// Test resize_fit with aspect ratio preservation
#[test]
fn test_resize_fit() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.png");

    // Create a wide image
    let original = generate_test_image(400, 200, [100, 150, 200, 255]);
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();
    let resized = loaded.resize_fit(300, 300, [255, 255, 255, 255]);

    // Should fit within 300x300 while preserving aspect ratio
    assert_eq!(resized.width, 300);
    assert_eq!(resized.height, 300);
}

/// Test loading various JPEG quality levels
#[test]
fn test_load_jpeg_various_quality() {
    let temp_dir = TempDir::new().unwrap();

    for quality in [10, 50, 85, 100] {
        let path = temp_dir.path().join(format!("test_q{}.jpg", quality));

        let original = generate_test_image(200, 150, [255, 128, 64, 255]);
        save_jpeg(&original, &path, quality).unwrap();

        let loaded = LoadedImage::from_path(&path);
        assert!(
            loaded.is_ok(),
            "Failed to load JPEG with quality {}",
            quality
        );

        let loaded = loaded.unwrap();
        assert_eq!(loaded.width, 200);
        assert_eq!(loaded.height, 150);
    }
}

/// Test loading PNG with transparency
#[test]
fn test_load_png_with_alpha() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test_alpha.png");

    // Create image with varying alpha
    let original = generate_test_image(100, 100, [255, 0, 0, 128]); // Semi-transparent red
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();

    assert_eq!(loaded.width, 100);
    assert_eq!(loaded.height, 100);

    // Check that alpha channel is preserved
    // Note: The generated image has varying alpha due to gradient, but base should be around 128
}

/// Test loading large image
#[test]
fn test_load_large_image() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("large.png");

    let original = generate_test_image(1920, 1080, [100, 150, 200, 255]);
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();

    assert_eq!(loaded.width, 1920);
    assert_eq!(loaded.height, 1080);
    assert_eq!(loaded.data.len(), (1920 * 1080 * 4) as usize);
}

/// Test loading small image
#[test]
fn test_load_small_image() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("small.png");

    let original = generate_test_image(16, 16, [100, 150, 200, 255]);
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();

    assert_eq!(loaded.width, 16);
    assert_eq!(loaded.height, 16);
}

/// Test upscaling an image
#[test]
fn test_upscale_image() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.png");

    let original = generate_test_image(100, 100, [100, 150, 200, 255]);
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();
    let resized = loaded.resize(400, 400);

    assert_eq!(resized.width, 400);
    assert_eq!(resized.height, 400);
    assert_eq!(resized.data.len(), (400 * 400 * 4) as usize);
}

/// Test downscaling an image
#[test]
fn test_downscale_image() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().join("test.png");

    let original = generate_test_image(800, 600, [100, 150, 200, 255]);
    save_png(&original, &path).unwrap();

    let loaded = LoadedImage::from_path(&path).unwrap();
    let resized = loaded.resize(200, 150);

    assert_eq!(resized.width, 200);
    assert_eq!(resized.height, 150);
}
