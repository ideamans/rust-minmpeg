//! H.264 encoder with platform-specific implementations

use super::{Encoder, EncoderConfig};
use crate::Result;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

/// Check if H.264 encoding is available
#[allow(unused_variables)]
pub fn check_available(ffmpeg_path: Option<&str>) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        macos::check_available()
    }

    #[cfg(target_os = "windows")]
    {
        windows::check_available()
    }

    #[cfg(target_os = "linux")]
    {
        linux::check_available(ffmpeg_path)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Err(crate::Error::CodecUnavailable(
            "H.264 not supported on this platform".to_string(),
        ))
    }
}

/// Create an H.264 encoder for the current platform
pub fn create_encoder(config: EncoderConfig) -> Result<Box<dyn Encoder>> {
    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::VideoToolboxEncoder::new(config)?))
    }

    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::MediaFoundationEncoder::new(config)?))
    }

    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::FfmpegEncoder::new(config, None)?))
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = config;
        Err(Error::CodecUnavailable(
            "H.264 not supported on this platform".to_string(),
        ))
    }
}

/// Create an H.264 encoder with custom ffmpeg path (Linux only)
#[allow(dead_code)]
pub fn create_encoder_with_ffmpeg(
    config: EncoderConfig,
    ffmpeg_path: Option<&str>,
) -> Result<Box<dyn Encoder>> {
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::FfmpegEncoder::new(config, ffmpeg_path)?))
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = ffmpeg_path;
        create_encoder(config)
    }
}
