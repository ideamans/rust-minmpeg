//! WebM container muxer

use super::{Muxer, MuxerConfig};
use crate::encoder::Packet;
use crate::{Codec, Error, Result};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// WebM muxer using simple EBML writing
pub struct WebmMuxer {
    writer: BufWriter<File>,
    config: MuxerConfig,
    cluster_start: u64,
    timecode: u64,
    frame_duration_ms: u64,
    cluster_open: bool,
    header_written: bool,
}

impl WebmMuxer {
    pub fn new<P: AsRef<Path>>(output_path: P, config: MuxerConfig) -> Result<Self> {
        // WebM only supports AV1 (and VP8/VP9, but we only implement AV1)
        if config.codec != Codec::Av1 {
            return Err(Error::Mux(
                "WebM container only supports AV1 codec".to_string(),
            ));
        }

        let file = File::create(output_path.as_ref()).map_err(Error::Io)?;
        let writer = BufWriter::new(file);

        let frame_duration_ms = 1000 / config.fps as u64;

        let mut muxer = Self {
            writer,
            config,
            cluster_start: 0,
            timecode: 0,
            frame_duration_ms,
            cluster_open: false,
            header_written: false,
        };

        muxer.write_header()?;

        Ok(muxer)
    }

    fn write_header(&mut self) -> Result<()> {
        // EBML Header
        self.write_ebml_element(0x1A45DFA3, &self.create_ebml_header())?;

        // Segment (unknown size)
        self.write_ebml_id(0x18538067)?;
        self.write_ebml_size_unknown()?;

        // Segment Info
        self.write_ebml_element(0x1549A966, &self.create_segment_info())?;

        // Tracks
        self.write_ebml_element(0x1654AE6B, &self.create_tracks())?;

        self.header_written = true;
        Ok(())
    }

    fn create_ebml_header(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // EBMLVersion = 1
        data.extend(encode_ebml_element(0x4286, &[1]));
        // EBMLReadVersion = 1
        data.extend(encode_ebml_element(0x42F7, &[1]));
        // EBMLMaxIDLength = 4
        data.extend(encode_ebml_element(0x42F2, &[4]));
        // EBMLMaxSizeLength = 8
        data.extend(encode_ebml_element(0x42F3, &[8]));
        // DocType = "webm"
        data.extend(encode_ebml_element(0x4282, b"webm"));
        // DocTypeVersion = 4
        data.extend(encode_ebml_element(0x4287, &[4]));
        // DocTypeReadVersion = 2
        data.extend(encode_ebml_element(0x4285, &[2]));

        data
    }

    fn create_segment_info(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // TimestampScale = 1000000 (1ms)
        data.extend(encode_ebml_element(0x2AD7B1, &encode_uint(1_000_000)));
        // MuxingApp
        data.extend(encode_ebml_element(0x4D80, b"minmpeg"));
        // WritingApp
        data.extend(encode_ebml_element(0x5741, b"minmpeg"));

        data
    }

    fn create_tracks(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // TrackEntry
        let track_entry = self.create_track_entry();
        data.extend(encode_ebml_element(0xAE, &track_entry));

        data
    }

    fn create_track_entry(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // TrackNumber = 1
        data.extend(encode_ebml_element(0xD7, &[1]));
        // TrackUID = 1
        data.extend(encode_ebml_element(0x73C5, &encode_uint(1)));
        // TrackType = 1 (video)
        data.extend(encode_ebml_element(0x83, &[1]));
        // CodecID = "V_AV1"
        data.extend(encode_ebml_element(0x86, b"V_AV1"));
        // Video settings
        data.extend(encode_ebml_element(0xE0, &self.create_video_settings()));

        data
    }

    fn create_video_settings(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // PixelWidth
        data.extend(encode_ebml_element(
            0xB0,
            &encode_uint(self.config.width as u64),
        ));
        // PixelHeight
        data.extend(encode_ebml_element(
            0xBA,
            &encode_uint(self.config.height as u64),
        ));

        data
    }

    fn start_cluster(&mut self) -> Result<()> {
        if self.cluster_open {
            return Ok(());
        }

        // Cluster (unknown size for streaming)
        self.write_ebml_id(0x1F43B675)?;
        self.write_ebml_size_unknown()?;

        // Timestamp
        let timestamp_data = encode_ebml_element(0xE7, &encode_uint(self.timecode));
        self.writer.write_all(&timestamp_data).map_err(Error::Io)?;

        self.cluster_start = self.timecode;
        self.cluster_open = true;

        Ok(())
    }

    fn write_simple_block(&mut self, packet: &Packet) -> Result<()> {
        let relative_timecode = (self.timecode - self.cluster_start) as i16;

        let mut block_data = Vec::new();

        // Track number (EBML coded, track 1)
        block_data.push(0x81);

        // Relative timecode (big-endian i16)
        block_data.push((relative_timecode >> 8) as u8);
        block_data.push((relative_timecode & 0xFF) as u8);

        // Flags: keyframe if applicable
        let flags = if packet.is_keyframe { 0x80 } else { 0x00 };
        block_data.push(flags);

        // Frame data
        block_data.extend(&packet.data);

        // SimpleBlock element
        self.write_ebml_element(0xA3, &block_data)?;

        Ok(())
    }

    fn write_ebml_id(&mut self, id: u32) -> Result<()> {
        let bytes = encode_ebml_id(id);
        self.writer.write_all(&bytes).map_err(Error::Io)
    }

    fn write_ebml_size_unknown(&mut self) -> Result<()> {
        // Unknown size marker for streaming
        self.writer
            .write_all(&[0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF])
            .map_err(Error::Io)
    }

    fn write_ebml_element(&mut self, id: u32, data: &[u8]) -> Result<()> {
        let bytes = encode_ebml_element(id, data);
        self.writer.write_all(&bytes).map_err(Error::Io)
    }
}

impl Muxer for WebmMuxer {
    fn write_packet(&mut self, packet: &Packet) -> Result<()> {
        // Start a new cluster if needed (e.g., on keyframe or every few seconds)
        if !self.cluster_open || (packet.is_keyframe && self.timecode > self.cluster_start) {
            self.cluster_open = false;
            self.start_cluster()?;
        }

        self.write_simple_block(packet)?;
        self.timecode += self.frame_duration_ms;

        Ok(())
    }

    fn finalize(mut self: Box<Self>) -> Result<()> {
        self.writer.flush().map_err(Error::Io)?;
        Ok(())
    }
}

// EBML encoding helpers

/// Encode an EBML element ID.
///
/// EBML IDs have class markers in their leading bits that indicate the ID length:
/// - Class A (1-byte): 1xxx xxxx (0x80-0xFF)
/// - Class B (2-byte): 01xx xxxx xxxx xxxx (0x4000-0x7FFF)
/// - Class C (3-byte): 001x xxxx ... (0x200000-0x3FFFFF)
/// - Class D (4-byte): 0001 xxxx ... (0x10000000-0x1FFFFFFF)
fn encode_ebml_id(id: u32) -> Vec<u8> {
    // Detect the class based on the ID value's leading bits
    if (0x80..=0xFF).contains(&id) {
        // Class A: 1-byte ID
        vec![id as u8]
    } else if (0x4000..=0x7FFF).contains(&id) {
        // Class B: 2-byte ID
        vec![(id >> 8) as u8, (id & 0xFF) as u8]
    } else if (0x200000..=0x3FFFFF).contains(&id) {
        // Class C: 3-byte ID
        vec![
            (id >> 16) as u8,
            ((id >> 8) & 0xFF) as u8,
            (id & 0xFF) as u8,
        ]
    } else if (0x10000000..=0x1FFFFFFF).contains(&id) {
        // Class D: 4-byte ID
        vec![
            (id >> 24) as u8,
            ((id >> 16) & 0xFF) as u8,
            ((id >> 8) & 0xFF) as u8,
            (id & 0xFF) as u8,
        ]
    } else {
        // Fallback: encode as minimal bytes needed
        // This handles non-standard IDs (if any)
        if id <= 0xFF {
            vec![id as u8]
        } else if id <= 0xFFFF {
            vec![(id >> 8) as u8, (id & 0xFF) as u8]
        } else if id <= 0xFFFFFF {
            vec![
                (id >> 16) as u8,
                ((id >> 8) & 0xFF) as u8,
                (id & 0xFF) as u8,
            ]
        } else {
            vec![
                (id >> 24) as u8,
                ((id >> 16) & 0xFF) as u8,
                ((id >> 8) & 0xFF) as u8,
                (id & 0xFF) as u8,
            ]
        }
    }
}

fn encode_ebml_size(size: u64) -> Vec<u8> {
    if size < 0x7F {
        vec![(size as u8) | 0x80]
    } else if size < 0x3FFF {
        vec![((size >> 8) as u8) | 0x40, (size & 0xFF) as u8]
    } else if size < 0x1FFFFF {
        vec![
            ((size >> 16) as u8) | 0x20,
            ((size >> 8) & 0xFF) as u8,
            (size & 0xFF) as u8,
        ]
    } else if size < 0x0FFFFFFF {
        vec![
            ((size >> 24) as u8) | 0x10,
            ((size >> 16) & 0xFF) as u8,
            ((size >> 8) & 0xFF) as u8,
            (size & 0xFF) as u8,
        ]
    } else {
        // For larger sizes, use 8-byte encoding
        let mut bytes = vec![0x01];
        for i in (0..7).rev() {
            bytes.push(((size >> (i * 8)) & 0xFF) as u8);
        }
        bytes
    }
}

fn encode_ebml_element(id: u32, data: &[u8]) -> Vec<u8> {
    let mut result = encode_ebml_id(id);
    result.extend(encode_ebml_size(data.len() as u64));
    result.extend(data);
    result
}

fn encode_uint(value: u64) -> Vec<u8> {
    if value == 0 {
        return vec![0];
    }

    let mut bytes = Vec::new();
    let mut v = value;

    while v > 0 {
        bytes.insert(0, (v & 0xFF) as u8);
        v >>= 8;
    }

    bytes
}
