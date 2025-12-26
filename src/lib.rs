//! minmpeg - Minimal video generation FFI library
//!
//! This library provides two main functions:
//! - `slideshow`: Create a video from a sequence of images with durations
//! - `juxtapose`: Combine two videos side by side

pub mod encoder;
pub mod error;
pub mod ffi;
pub mod image_loader;
pub mod muxer;

mod juxtapose;
mod slideshow;

pub use error::{Error, Result};
pub use juxtapose::juxtapose;
pub use slideshow::slideshow;

/// Video codec types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum Codec {
    /// AV1 codec (using rav1e/libaom)
    Av1 = 0,
    /// H.264 codec (platform-specific implementation)
    H264 = 1,
}

/// Container format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum Container {
    /// MP4 container (supports AV1 and H.264)
    Mp4 = 0,
    /// WebM container (supports AV1 only)
    WebM = 1,
}

impl Container {
    /// Check if the container supports the given codec
    pub fn supports_codec(&self, codec: Codec) -> bool {
        match (self, codec) {
            (Container::Mp4, _) => true,
            (Container::WebM, Codec::Av1) => true,
            (Container::WebM, Codec::H264) => false,
        }
    }
}

/// RGB color representation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 255,
            g: 255,
            b: 255,
        }
    }
}

/// Slide entry for slideshow creation
#[derive(Debug, Clone)]
pub struct SlideEntry {
    /// Path to the image file
    pub path: String,
    /// Duration to display this image in milliseconds
    pub duration_ms: u32,
}

/// Options for video encoding
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// Output file path
    pub output_path: String,
    /// Container format
    pub container: Container,
    /// Video codec
    pub codec: Codec,
    /// Quality (0-100, where 100 is highest quality)
    pub quality: u8,
    /// Path to ffmpeg executable (for H.264 on Linux)
    pub ffmpeg_path: Option<String>,
}

impl EncodeOptions {
    /// Validate the options
    pub fn validate(&self) -> Result<()> {
        if !self.container.supports_codec(self.codec) {
            return Err(Error::ContainerCodecMismatch {
                container: self.container,
                codec: self.codec,
            });
        }
        Ok(())
    }
}

/// Check if a codec is available on the current system
pub fn available(codec: Codec, ffmpeg_path: Option<&str>) -> Result<()> {
    match codec {
        Codec::Av1 => {
            #[cfg(feature = "av1")]
            {
                Ok(())
            }
            #[cfg(not(feature = "av1"))]
            {
                Err(Error::CodecUnavailable(
                    "AV1 support not compiled in".to_string(),
                ))
            }
        }
        Codec::H264 => encoder::h264::check_available(ffmpeg_path),
    }
}
