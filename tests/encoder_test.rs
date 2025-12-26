//! Integration tests for encoder functionality

mod common;

#[allow(unused_imports)]
use common::*;
use minmpeg::{available, Codec};

/// Test AV1 encoder availability
#[test]
fn test_av1_available() {
    let result = available(Codec::Av1, None);
    // AV1 should be available when compiled with the av1 feature
    #[cfg(feature = "av1")]
    assert!(result.is_ok(), "AV1 should be available: {:?}", result);
}

/// Test H.264 encoder availability (platform-specific)
#[test]
#[cfg(target_os = "macos")]
fn test_h264_available_macos() {
    let result = available(Codec::H264, None);
    // H.264 should be available on macOS via VideoToolbox
    assert!(
        result.is_ok(),
        "H.264 should be available on macOS: {:?}",
        result
    );
}

#[test]
#[cfg(target_os = "linux")]
fn test_h264_available_linux() {
    // H.264 on Linux requires ffmpeg
    let result = available(Codec::H264, None);
    // May or may not be available depending on ffmpeg installation
    println!("H.264 availability on Linux: {:?}", result);
}

#[test]
#[cfg(target_os = "windows")]
fn test_h264_available_windows() {
    let result = available(Codec::H264, None);
    // H.264 should be available on Windows via Media Foundation
    assert!(
        result.is_ok(),
        "H.264 should be available on Windows: {:?}",
        result
    );
}
