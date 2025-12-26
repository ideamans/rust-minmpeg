//! Integration tests for juxtapose functionality

mod common;

use common::*;
use minmpeg::{juxtapose, slideshow, Codec, Color, Container, EncodeOptions, SlideEntry};
use std::process::Command;
use tempfile::TempDir;

/// Check if ffmpeg is available
fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Create a test video using slideshow (helper function)
fn create_test_video(
    temp_dir: &TempDir,
    name: &str,
    width: u32,
    height: u32,
    slides: u32,
    container: Container,
    codec: Codec,
) -> String {
    let image_paths: Vec<_> = (0..slides)
        .map(|i| {
            let path = temp_dir.path().join(format!("{}_{}.png", name, i));
            let img = generate_numbered_image(width, height, i);
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

    let ext = match container {
        Container::WebM => "webm",
        Container::Mp4 => "mp4",
    };

    let output_path = temp_dir.path().join(format!("{}.{}", name, ext));

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container,
        codec,
        quality: 50,
        ffmpeg_path: None,
    };

    slideshow(&entries, &options).expect("Failed to create test video");

    output_path.to_string_lossy().to_string()
}

// ============================================================================
// Same size composition tests (WebM + AV1)
// ============================================================================

/// Test juxtapose with two same-size videos (WebM + AV1)
#[test]
fn test_juxtapose_same_size_webm_av1() {
    if !ffmpeg_available() {
        println!("Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create two test videos with the same dimensions (small for fast testing)
    let left_video = create_test_video(&temp_dir, "left", 160, 120, 2, Container::WebM, Codec::Av1);
    let right_video =
        create_test_video(&temp_dir, "right", 160, 120, 2, Container::WebM, Codec::Av1);

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = juxtapose(&left_video, &right_video, &options, None);
    assert!(
        result.is_ok(),
        "Juxtapose same size WebM+AV1 failed: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_webm_header(&output_path));
}

// ============================================================================
// Same size composition tests (MP4 + H.264) - Platform specific
// ============================================================================

/// Test juxtapose with two same-size videos (MP4 + H.264, macOS)
#[test]
#[cfg(target_os = "macos")]
fn test_juxtapose_same_size_mp4_h264_macos() {
    if !ffmpeg_available() {
        println!("Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create two test videos with the same dimensions (small for fast testing)
    let left_video = create_test_video(&temp_dir, "left", 160, 120, 2, Container::Mp4, Codec::H264);
    let right_video =
        create_test_video(&temp_dir, "right", 160, 120, 2, Container::Mp4, Codec::H264);

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = juxtapose(&left_video, &right_video, &options, None);
    assert!(
        result.is_ok(),
        "Juxtapose same size MP4+H.264 failed on macOS: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_mp4_header(&output_path));
}

/// Test juxtapose with two same-size videos (MP4 + H.264, Windows)
#[test]
#[cfg(target_os = "windows")]
fn test_juxtapose_same_size_mp4_h264_windows() {
    if !ffmpeg_available() {
        println!("Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create two test videos with the same dimensions (small for fast testing)
    let left_video = create_test_video(&temp_dir, "left", 160, 120, 2, Container::Mp4, Codec::H264);
    let right_video =
        create_test_video(&temp_dir, "right", 160, 120, 2, Container::Mp4, Codec::H264);

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = juxtapose(&left_video, &right_video, &options, None);
    assert!(
        result.is_ok(),
        "Juxtapose same size MP4+H.264 failed on Windows: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_mp4_header(&output_path));
}

// ============================================================================
// Different size composition tests (WebM + AV1)
// ============================================================================

/// Test juxtapose with two different-size videos (WebM + AV1)
#[test]
fn test_juxtapose_different_size_webm_av1() {
    if !ffmpeg_available() {
        println!("Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create two test videos with different dimensions (small for fast testing)
    let left_video = create_test_video(&temp_dir, "left", 160, 120, 2, Container::WebM, Codec::Av1);
    let right_video =
        create_test_video(&temp_dir, "right", 200, 150, 2, Container::WebM, Codec::Av1);

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    // Use a custom background color
    let bg = Color {
        r: 128,
        g: 128,
        b: 128,
    };

    let result = juxtapose(&left_video, &right_video, &options, Some(bg));
    assert!(
        result.is_ok(),
        "Juxtapose different size WebM+AV1 failed: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_webm_header(&output_path));
}

// ============================================================================
// Different size composition tests (MP4 + H.264) - Platform specific
// ============================================================================

/// Test juxtapose with two different-size videos (MP4 + H.264, macOS)
#[test]
#[cfg(target_os = "macos")]
fn test_juxtapose_different_size_mp4_h264_macos() {
    if !ffmpeg_available() {
        println!("Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create two test videos with different dimensions (small for fast testing)
    let left_video = create_test_video(&temp_dir, "left", 160, 120, 2, Container::Mp4, Codec::H264);
    let right_video =
        create_test_video(&temp_dir, "right", 200, 150, 2, Container::Mp4, Codec::H264);

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    // Use a custom background color
    let bg = Color {
        r: 64,
        g: 64,
        b: 64,
    };

    let result = juxtapose(&left_video, &right_video, &options, Some(bg));
    assert!(
        result.is_ok(),
        "Juxtapose different size MP4+H.264 failed on macOS: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_mp4_header(&output_path));
}

/// Test juxtapose with two different-size videos (MP4 + H.264, Windows)
#[test]
#[cfg(target_os = "windows")]
fn test_juxtapose_different_size_mp4_h264_windows() {
    if !ffmpeg_available() {
        println!("Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create two test videos with different dimensions (small for fast testing)
    let left_video = create_test_video(&temp_dir, "left", 160, 120, 2, Container::Mp4, Codec::H264);
    let right_video =
        create_test_video(&temp_dir, "right", 200, 150, 2, Container::Mp4, Codec::H264);

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let bg = Color {
        r: 64,
        g: 64,
        b: 64,
    };

    let result = juxtapose(&left_video, &right_video, &options, Some(bg));
    assert!(
        result.is_ok(),
        "Juxtapose different size MP4+H.264 failed on Windows: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_mp4_header(&output_path));
}

// ============================================================================
// Cross-format tests (different input formats)
// ============================================================================

/// Test juxtapose with mixed input formats (WebM left, MP4 right) -> WebM output
#[test]
#[cfg(target_os = "macos")]
fn test_juxtapose_mixed_formats_to_webm_macos() {
    if !ffmpeg_available() {
        println!("Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create videos in different formats (small for fast testing)
    let left_video = create_test_video(&temp_dir, "left", 160, 120, 2, Container::WebM, Codec::Av1);
    let right_video =
        create_test_video(&temp_dir, "right", 160, 120, 2, Container::Mp4, Codec::H264);

    let output_path = temp_dir.path().join("output.webm");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::WebM,
        codec: Codec::Av1,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = juxtapose(&left_video, &right_video, &options, None);
    assert!(
        result.is_ok(),
        "Juxtapose mixed formats to WebM failed on macOS: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_webm_header(&output_path));
}

/// Test juxtapose with mixed input formats (WebM left, MP4 right) -> MP4 output
#[test]
#[cfg(target_os = "macos")]
fn test_juxtapose_mixed_formats_to_mp4_macos() {
    if !ffmpeg_available() {
        println!("Skipping test: ffmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create videos in different formats (small for fast testing)
    let left_video = create_test_video(&temp_dir, "left", 160, 120, 2, Container::WebM, Codec::Av1);
    let right_video =
        create_test_video(&temp_dir, "right", 160, 120, 2, Container::Mp4, Codec::H264);

    let output_path = temp_dir.path().join("output.mp4");

    let options = EncodeOptions {
        output_path: output_path.to_string_lossy().to_string(),
        container: Container::Mp4,
        codec: Codec::H264,
        quality: 50,
        ffmpeg_path: None,
    };

    let result = juxtapose(&left_video, &right_video, &options, None);
    assert!(
        result.is_ok(),
        "Juxtapose mixed formats to MP4 failed on macOS: {:?}",
        result
    );
    assert!(verify_file_exists_with_size(&output_path));
    assert!(verify_mp4_header(&output_path));
}
