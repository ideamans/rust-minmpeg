//! Video container muxers

pub mod mp4;
pub mod webm;

use crate::encoder::Packet;
use crate::{Codec, Container, Result};
use std::path::Path;

/// Video muxer trait
pub trait Muxer: Send {
    /// Write a video packet
    fn write_packet(&mut self, packet: &Packet) -> Result<()>;

    /// Finalize and close the output file
    fn finalize(self: Box<Self>) -> Result<()>;
}

/// Muxer configuration
#[derive(Debug, Clone)]
pub struct MuxerConfig {
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Frame rate (fps)
    pub fps: u32,
    /// Video codec
    pub codec: Codec,
    /// Codec-specific configuration data (e.g., SPS/PPS for H.264)
    pub codec_config: Option<Vec<u8>>,
}

/// Create a muxer for the specified container format
pub fn create_muxer<P: AsRef<Path>>(
    container: Container,
    output_path: P,
    config: MuxerConfig,
) -> Result<Box<dyn Muxer>> {
    match container {
        Container::Mp4 => Ok(Box::new(mp4::Mp4Muxer::new(output_path, config)?)),
        Container::WebM => Ok(Box::new(webm::WebmMuxer::new(output_path, config)?)),
    }
}
