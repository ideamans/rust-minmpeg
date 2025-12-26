//! Video encoders

#[cfg(feature = "av1")]
pub mod av1;

pub mod h264;

use crate::{Codec, Result};

/// Raw video frame in RGBA format
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// RGBA pixel data (width * height * 4 bytes)
    pub data: Vec<u8>,
    /// Presentation timestamp in milliseconds
    pub pts_ms: u64,
}

/// Encoded video packet
#[derive(Debug, Clone)]
pub struct Packet {
    /// Encoded data
    pub data: Vec<u8>,
    /// Presentation timestamp in time_base units
    pub pts: i64,
    /// Decoding timestamp in time_base units
    pub dts: i64,
    /// Is this a keyframe?
    pub is_keyframe: bool,
}

/// Video encoder trait
pub trait Encoder: Send {
    /// Encode a frame
    fn encode(&mut self, frame: &Frame) -> Result<Vec<Packet>>;

    /// Flush remaining packets
    fn flush(&mut self) -> Result<Vec<Packet>>;

    /// Get the codec-specific configuration data (SPS for H.264)
    fn codec_config(&self) -> Option<Vec<u8>> {
        None
    }

    /// Get the Picture Parameter Set (PPS for H.264)
    fn pps(&self) -> Option<Vec<u8>> {
        None
    }
}

/// Encoder configuration
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Frame rate (frames per second)
    pub fps: u32,
    /// Quality (0-100)
    pub quality: u8,
}

/// Create an encoder for the specified codec
pub fn create_encoder(codec: Codec, config: EncoderConfig) -> Result<Box<dyn Encoder>> {
    match codec {
        #[cfg(feature = "av1")]
        Codec::Av1 => Ok(Box::new(av1::Av1Encoder::new(config)?)),
        #[cfg(not(feature = "av1"))]
        Codec::Av1 => Err(crate::Error::CodecUnavailable(
            "AV1 support not compiled in".to_string(),
        )),
        Codec::H264 => h264::create_encoder(config),
    }
}
