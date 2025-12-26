//! Integration tests for slideshow functionality

mod common;

use common::*;
use minmpeg::{slideshow, Codec, Container, EncodeOptions, SlideEntry};
use tempfile::TempDir;

/// Test creating a slideshow with JPEG images
#[test]
fn test_slideshow_jpeg_images() {
    let temp_dir = TempDir::new().unwrap();

    // Create test JPEG images
    let image_paths: Vec<_> = (0..3)
        .map(|i| {
            let path = temp_dir.path().join(format!("slide_{}.jpg", i));
            let img = generate_numbered_image(640, 480, i);
            save_jpeg(&img, &path, 85).unwrap();
            path
        })
        .collect();

    // Create slideshow entries
    let entries: Vec<SlideEntry> = image_paths
        .iter()
        .map(|path| SlideEntry {
            path: path.to_string_lossy().to_string(),
            duration_ms: 1000, // 1 second each
        })
        .collect();

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    // Create slideshow
    let result = slideshow(&entries, &options);
    assert!(result.is_ok(), "Slideshow creation failed: {:?}", result);

    // Verify output file
    assert!(
        verify_file_exists_with_size(&output_path),
        "Output file does not exist or is empty"
    );
    assert!(
        verify_webm_header(&output_path),
        "Output file is not a valid WebM"
    );

    // Check file size is reasonable (at least a few KB for 3 seconds of video)
    let size = get_file_size(&output_path).unwrap();
    assert!(size > 1000, "Output file is too small: {} bytes", size);
}

/// Test creating a slideshow with PNG images
#[test]
fn test_slideshow_png_images() {
    let temp_dir = TempDir::new().unwrap();

    // Create test PNG images
    let image_paths: Vec<_> = (0..3)
        .map(|i| {
            let path = temp_dir.path().join(format!("slide_{}.png", i));
            let img = generate_numbered_image(640, 480, i);
            save_png(&img, &path).unwrap();
            path
        })
        .collect();

    // Create slideshow entries
    let entries: Vec<SlideEntry> = image_paths
        .iter()
        .map(|path| SlideEntry {
            path: path.to_string_lossy().to_string(),
            duration_ms: 500, // 0.5 seconds each
        })
        .collect();

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(result.is_ok(), "Slideshow creation failed: {:?}", result);

    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_webm_header(&output_path));
}

/// Test creating a slideshow with mixed JPEG and PNG images
#[test]
fn test_slideshow_mixed_formats() {
    let temp_dir = TempDir::new().unwrap();

    // Create mixed format images
    let jpeg_path = temp_dir.path().join("slide_0.jpg");
    let png_path = temp_dir.path().join("slide_1.png");

    let img1 = generate_numbered_image(640, 480, 0);
    let img2 = generate_numbered_image(640, 480, 1);

    save_jpeg(&img1, &jpeg_path, 85).unwrap();
    save_png(&img2, &png_path).unwrap();

    let entries = vec![
        SlideEntry {
            path: jpeg_path.to_string_lossy().to_string(),
            duration_ms: 1000,
        },
        SlideEntry {
            path: png_path.to_string_lossy().to_string(),
            duration_ms: 1000,
        },
    ];

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(result.is_ok(), "Slideshow creation failed: {:?}", result);

    assert!(verify_file_exists_with_size(&output_path));
}

/// Test slideshow with different image resolutions (should resize to first image size)
#[test]
fn test_slideshow_different_resolutions() {
    let temp_dir = TempDir::new().unwrap();

    // Create images with different sizes
    let sizes = [(640, 480), (800, 600), (320, 240)];
    let image_paths: Vec<_> = sizes
        .iter()
        .enumerate()
        .map(|(i, (w, h))| {
            let path = temp_dir.path().join(format!("slide_{}.png", i));
            let img = generate_numbered_image(*w, *h, i as u32);
            save_png(&img, &path).unwrap();
            path
        })
        .collect();

    let entries: Vec<SlideEntry> = image_paths
        .iter()
        .map(|path| SlideEntry {
            path: path.to_string_lossy().to_string(),
            duration_ms: 500,
        })
        .collect();

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(result.is_ok(), "Slideshow creation failed: {:?}", result);

    assert!(verify_file_exists_with_size(&output_path));
}

/// Test slideshow with various durations
#[test]
fn test_slideshow_various_durations() {
    let temp_dir = TempDir::new().unwrap();

    // Create test images with varying durations
    let durations = [100, 500, 1000, 2000]; // ms
    let image_paths: Vec<_> = (0..4)
        .map(|i| {
            let path = temp_dir.path().join(format!("slide_{}.png", i));
            let img = generate_numbered_image(320, 240, i);
            save_png(&img, &path).unwrap();
            path
        })
        .collect();

    let entries: Vec<SlideEntry> = image_paths
        .iter()
        .zip(durations.iter())
        .map(|(path, duration)| SlideEntry {
            path: path.to_string_lossy().to_string(),
            duration_ms: *duration,
        })
        .collect();

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(result.is_ok(), "Slideshow creation failed: {:?}", result);

    assert!(verify_file_exists_with_size(&output_path));
}

/// Test slideshow with empty entries (should fail)
#[test]
fn test_slideshow_empty_entries() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&[], &options);
    assert!(result.is_err(), "Empty slideshow should fail");
}

/// Test slideshow with non-existent image (should fail)
#[test]
fn test_slideshow_nonexistent_image() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output.webm");

    let entries = vec![SlideEntry {
        path: "/nonexistent/path/image.jpg".to_string(),
        duration_ms: 1000,
    }];

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(result.is_err(), "Non-existent image should fail");
}

/// Test slideshow with different quality settings
#[test]
fn test_slideshow_quality_settings() {
    let temp_dir = TempDir::new().unwrap();

    // Create test image
    let path = temp_dir.path().join("slide.png");
    let img = generate_numbered_image(320, 240, 0);
    save_png(&img, &path).unwrap();

    let entries = vec![SlideEntry {
        path: path.to_string_lossy().to_string(),
        duration_ms: 500,
    }];

    // Test different quality levels
    for quality in [10, 50, 90] {
        let output_path = temp_dir.path().join(format!("output_q{}.webm", quality));

        let options = EncodeOptions {
            output_path: output_path.to_string_lossy().to_string(),
            container: Container::WebM,
            codec: Codec::Av1,
            quality,
            ffmpeg_path: None,
        };

        let result = slideshow(&entries, &options);
        assert!(
            result.is_ok(),
            "Slideshow with quality {} failed: {:?}",
            quality,
            result
        );
        assert!(verify_file_exists_with_size(&output_path));
    }
}

/// Test container/codec mismatch (WebM + H.264 should fail)
#[test]
fn test_slideshow_container_codec_mismatch() {
    let temp_dir = TempDir::new().unwrap();

    let path = temp_dir.path().join("slide.png");
    let img = generate_numbered_image(320, 240, 0);
    save_png(&img, &path).unwrap();

    let entries = vec![SlideEntry {
        path: path.to_string_lossy().to_string(),
        duration_ms: 500,
    }];

    let output_path = temp_dir.path().join("output.webm");

    // WebM + H.264 is not supported
    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(result.is_err(), "WebM + H.264 should fail");
}

/// Test large resolution image
#[test]
fn test_slideshow_large_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Create a larger image (1920x1080)
    let path = temp_dir.path().join("slide.png");
    let img = generate_numbered_image(1920, 1080, 0);
    save_png(&img, &path).unwrap();

    let entries = vec![SlideEntry {
        path: path.to_string_lossy().to_string(),
        duration_ms: 500,
    }];

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 30, // Lower quality for faster encoding
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(
        result.is_ok(),
        "Large resolution slideshow failed: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
}

/// Test small resolution image
#[test]
fn test_slideshow_small_resolution() {
    let temp_dir = TempDir::new().unwrap();

    // Create a small image (64x64)
    let path = temp_dir.path().join("slide.png");
    let img = generate_numbered_image(64, 64, 0);
    save_png(&img, &path).unwrap();

    let entries = vec![SlideEntry {
        path: path.to_string_lossy().to_string(),
        duration_ms: 500,
    }];

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(
        result.is_ok(),
        "Small resolution slideshow failed: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
}

// ============================================================================
// Container/Codec combination tests
// Supported combinations:
// - WebM + AV1 (all platforms, using rav1e)
// - MP4 + H.264 (macOS: VideoToolbox, Windows: MediaFoundation, Linux: ffmpeg)
// ============================================================================

/// Test MP4 container with H.264 codec (platform-specific)
#[test]
#[cfg(target_os = "macos")]
fn test_slideshow_mp4_h264_macos() {
    let temp_dir = TempDir::new().unwrap();

    let path = temp_dir.path().join("slide.png");
    let img = generate_numbered_image(320, 240, 0);
    save_png(&img, &path).unwrap();

    let entries = vec![SlideEntry {
        path: path.to_string_lossy().to_string(),
        duration_ms: 500,
    }];

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(
        result.is_ok(),
        "MP4+H.264 slideshow failed on macOS: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(
        verify_mp4_header(&output_path),
        "Output file is not a valid MP4"
    );
}

/// Test MP4 container with H.264 codec on Windows
#[test]
#[cfg(target_os = "windows")]
fn test_slideshow_mp4_h264_windows() {
    let temp_dir = TempDir::new().unwrap();

    let path = temp_dir.path().join("slide.png");
    let img = generate_numbered_image(320, 240, 0);
    save_png(&img, &path).unwrap();

    let entries = vec![SlideEntry {
        path: path.to_string_lossy().to_string(),
        duration_ms: 500,
    }];

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(
        result.is_ok(),
        "MP4+H.264 slideshow failed on Windows: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(
        verify_mp4_header(&output_path),
        "Output file is not a valid MP4"
    );
}

/// Test MP4 container with H.264 codec on Linux (requires ffmpeg)
#[test]
#[cfg(target_os = "linux")]
fn test_slideshow_mp4_h264_linux() {
    use minmpeg::available;

    // First check if H.264 is available (requires ffmpeg)
    if available(Codec::H264, None).is_err() {
        println!("Skipping MP4+H.264 test on Linux: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    let path = temp_dir.path().join("slide.png");
    let img = generate_numbered_image(320, 240, 0);
    save_png(&img, &path).unwrap();

    let entries = vec![SlideEntry {
        path: path.to_string_lossy().to_string(),
        duration_ms: 500,
    }];

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(
        result.is_ok(),
        "MP4+H.264 slideshow failed on Linux: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(
        verify_mp4_header(&output_path),
        "Output file is not a valid MP4"
    );
}

/// Test WebM container with AV1 codec (multiple slides to ensure encoding works)
#[test]
fn test_slideshow_webm_av1_multiple() {
    let temp_dir = TempDir::new().unwrap();

    // Create 5 slides with different colors
    let image_paths: Vec<_> = (0..5)
        .map(|i| {
            let path = temp_dir.path().join(format!("slide_{}.png", i));
            let img = generate_numbered_image(320, 240, i);
            save_png(&img, &path).unwrap();
            path
        })
        .collect();

    let entries: Vec<SlideEntry> = image_paths
        .iter()
        .map(|path| SlideEntry {
            path: path.to_string_lossy().to_string(),
            duration_ms: 200,
        })
        .collect();

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(
        result.is_ok(),
        "WebM+AV1 multiple slides failed: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_webm_header(&output_path));

    // Verify file has reasonable size for 5 slides
    let size = get_file_size(&output_path).unwrap();
    assert!(size > 2000, "Output file too small for 5 slides: {} bytes", size);
}

/// Test MP4 container with H.264 codec (multiple slides, macOS)
#[test]
#[cfg(target_os = "macos")]
fn test_slideshow_mp4_h264_multiple_macos() {
    let temp_dir = TempDir::new().unwrap();

    // Create 5 slides with different colors
    let image_paths: Vec<_> = (0..5)
        .map(|i| {
            let path = temp_dir.path().join(format!("slide_{}.png", i));
            let img = generate_numbered_image(320, 240, i);
            save_png(&img, &path).unwrap();
            path
        })
        .collect();

    let entries: Vec<SlideEntry> = image_paths
        .iter()
        .map(|path| SlideEntry {
            path: path.to_string_lossy().to_string(),
            duration_ms: 200,
        })
        .collect();

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(
        result.is_ok(),
        "MP4+H.264 multiple slides failed on macOS: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_mp4_header(&output_path));

    // Verify file has reasonable size for 5 slides
    let size = get_file_size(&output_path).unwrap();
    assert!(size > 2000, "Output file too small for 5 slides: {} bytes", size);
}

/// Test MP4 container with H.264 codec (multiple slides, Windows)
#[test]
#[cfg(target_os = "windows")]
fn test_slideshow_mp4_h264_multiple_windows() {
    let temp_dir = TempDir::new().unwrap();

    // Create 5 slides with different colors
    let image_paths: Vec<_> = (0..5)
        .map(|i| {
            let path = temp_dir.path().join(format!("slide_{}.png", i));
            let img = generate_numbered_image(320, 240, i);
            save_png(&img, &path).unwrap();
            path
        })
        .collect();

    let entries: Vec<SlideEntry> = image_paths
        .iter()
        .map(|path| SlideEntry {
            path: path.to_string_lossy().to_string(),
            duration_ms: 200,
        })
        .collect();

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = slideshow(&entries, &options);
    assert!(
        result.is_ok(),
        "MP4+H.264 multiple slides failed on Windows: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_mp4_header(&output_path));

    // Verify file has reasonable size for 5 slides
    let size = get_file_size(&output_path).unwrap();
    assert!(size > 2000, "Output file too small for 5 slides: {} bytes", size);
}
