//! MP4 container muxer

use super::{Muxer, MuxerConfig};
use crate::encoder::Packet;
use crate::{Codec, Error, Result};
use mp4::{Mp4Config, Mp4Writer, TrackConfig};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// MP4 muxer (H.264 only)
pub struct Mp4Muxer {
    writer: Mp4Writer<BufWriter<File>>,
    #[allow(dead_code)]
    config: MuxerConfig,
    track_id: u32,
    sample_count: u32,
}

impl Mp4Muxer {
    pub fn new<P: AsRef<Path>>(output_path: P, config: MuxerConfig) -> Result<Self> {
        // MP4 with mp4 crate only supports H.264
        // For AV1 in MP4, we would need a different approach
        if config.codec == Codec::Av1 {
            return Err(Error::Mux(
                "MP4 container with AV1 codec requires ffmpeg. Use WebM for AV1 instead."
                    .to_string(),
            ));
        }

        let file = File::create(output_path.as_ref()).map_err(Error::Io)?;
        let writer = BufWriter::new(file);

        let mp4_config = Mp4Config {
            major_brand: str_to_brand("isom"),
            minor_version: 512,
            compatible_brands: vec![
                str_to_brand("isom"),
                str_to_brand("iso2"),
                str_to_brand("avc1"),
                str_to_brand("mp41"),
            ],
            timescale: 1000, // milliseconds
        };

        let mut mp4_writer = Mp4Writer::write_start(writer, &mp4_config)
            .map_err(|e| Error::Mux(format!("Failed to create MP4 writer: {}", e)))?;

        // Add video track for H.264
        let track_config = TrackConfig {
            track_type: mp4::TrackType::Video,
            timescale: config.fps,
            language: String::from("und"),
            media_conf: mp4::MediaConfig::AvcConfig(mp4::AvcConfig {
                width: config.width as u16,
                height: config.height as u16,
                seq_param_set: config.codec_config.clone().unwrap_or_default(),
                pic_param_set: config.pps.clone().unwrap_or_default(),
            }),
        };

        mp4_writer
            .add_track(&track_config)
            .map_err(|e| Error::Mux(format!("Failed to add track: {}", e)))?;

        // Track ID is always 1 for single track
        let track_id = 1;

        Ok(Self {
            writer: mp4_writer,
            config,
            track_id,
            sample_count: 0,
        })
    }
}

impl Muxer for Mp4Muxer {
    fn write_packet(&mut self, packet: &Packet) -> Result<()> {
        let sample = mp4::Mp4Sample {
            start_time: self.sample_count as u64,
            duration: 1,
            rendering_offset: 0,
            is_sync: packet.is_keyframe,
            bytes: mp4::Bytes::copy_from_slice(&packet.data),
        };

        self.writer
            .write_sample(self.track_id, &sample)
            .map_err(|e| Error::Mux(format!("Failed to write sample: {}", e)))?;

        self.sample_count += 1;
        Ok(())
    }

    fn finalize(mut self: Box<Self>) -> Result<()> {
        self.writer
            .write_end()
            .map_err(|e| Error::Mux(format!("Failed to finalize MP4: {}", e)))?;

        Ok(())
    }
}

fn str_to_brand(s: &str) -> mp4::FourCC {
    let bytes = s.as_bytes();
    mp4::FourCC {
        value: [
            bytes.first().copied().unwrap_or(0),
            bytes.get(1).copied().unwrap_or(0),
            bytes.get(2).copied().unwrap_or(0),
            bytes.get(3).copied().unwrap_or(0),
        ],
    }
}
