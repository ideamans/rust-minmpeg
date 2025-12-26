//! AV1 encoder using rav1e

use super::{Encoder, EncoderConfig, Frame, Packet};
use crate::{Error, Result};
use rav1e::prelude::*;

/// AV1 encoder using rav1e
pub struct Av1Encoder {
    context: Context<u8>,
    #[allow(dead_code)]
    config: EncoderConfig,
    frame_count: u64,
}

impl Av1Encoder {
    /// Create a new AV1 encoder
    pub fn new(config: EncoderConfig) -> Result<Self> {
        // Map quality (0-100) to quantizer (255-0)
        // Higher quality = lower quantizer
        let quantizer = ((100 - config.quality.min(100)) as usize * 255) / 100;
        let min_quantizer = (quantizer.saturating_sub(10)) as u8;

        let enc_config = rav1e::config::EncoderConfig {
            width: config.width as usize,
            height: config.height as usize,
            speed_settings: SpeedSettings::from_preset(6), // Balance speed/quality
            time_base: Rational::new(1, config.fps as u64),
            sample_aspect_ratio: Rational::new(1, 1),
            bit_depth: 8,
            chroma_sampling: ChromaSampling::Cs420,
            chroma_sample_position: ChromaSamplePosition::Unknown,
            pixel_range: PixelRange::Limited,
            color_description: None,
            mastering_display: None,
            content_light: None,
            enable_timing_info: false,
            still_picture: false,
            error_resilient: false,
            switch_frame_interval: 0,
            min_key_frame_interval: 0,
            max_key_frame_interval: 240,
            reservoir_frame_delay: None,
            low_latency: false,
            quantizer,
            min_quantizer,
            bitrate: 0,
            tune: Tune::Psychovisual,
            tile_cols: 0,
            tile_rows: 0,
            tiles: 0,
            ..Default::default()
        };

        let rav1e_config = Config::new()
            .with_encoder_config(enc_config)
            .with_threads(0);

        let context = rav1e_config
            .new_context()
            .map_err(|e| Error::Encode(format!("Failed to create AV1 context: {}", e)))?;

        Ok(Self {
            context,
            config,
            frame_count: 0,
        })
    }

    /// Convert RGBA frame to YUV420
    fn rgba_to_yuv420(&self, frame: &Frame) -> rav1e::Frame<u8> {
        let mut yuv_frame = self.context.new_frame();

        let width = frame.width as usize;
        let height = frame.height as usize;

        // Y plane
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let r = frame.data[idx] as f32;
                let g = frame.data[idx + 1] as f32;
                let b = frame.data[idx + 2] as f32;

                // BT.601 conversion
                let y_val = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
                yuv_frame.planes[0].data_origin_mut()[y * width + x] = y_val;
            }
        }

        // U and V planes (subsampled 2x2)
        let uv_width = width.div_ceil(2);
        let uv_height = height.div_ceil(2);

        for y in 0..uv_height {
            for x in 0..uv_width {
                let src_x = x * 2;
                let src_y = y * 2;

                // Average 2x2 block
                let mut r_sum = 0u32;
                let mut g_sum = 0u32;
                let mut b_sum = 0u32;
                let mut count = 0u32;

                for dy in 0..2 {
                    for dx in 0..2 {
                        let sx = (src_x + dx).min(width - 1);
                        let sy = (src_y + dy).min(height - 1);
                        let idx = (sy * width + sx) * 4;
                        r_sum += frame.data[idx] as u32;
                        g_sum += frame.data[idx + 1] as u32;
                        b_sum += frame.data[idx + 2] as u32;
                        count += 1;
                    }
                }

                let r = (r_sum / count) as f32;
                let g = (g_sum / count) as f32;
                let b = (b_sum / count) as f32;

                // BT.601 conversion
                let u = ((-0.169 * r - 0.331 * g + 0.500 * b) + 128.0).clamp(0.0, 255.0) as u8;
                let v = ((0.500 * r - 0.419 * g - 0.081 * b) + 128.0).clamp(0.0, 255.0) as u8;

                yuv_frame.planes[1].data_origin_mut()[y * uv_width + x] = u;
                yuv_frame.planes[2].data_origin_mut()[y * uv_width + x] = v;
            }
        }

        yuv_frame
    }

    fn receive_packets(&mut self) -> Result<Vec<Packet>> {
        let mut packets = Vec::new();

        loop {
            match self.context.receive_packet() {
                Ok(pkt) => {
                    packets.push(Packet {
                        data: pkt.data,
                        pts: pkt.input_frameno as i64,
                        dts: pkt.input_frameno as i64,
                        is_keyframe: pkt.frame_type == FrameType::KEY,
                    });
                }
                Err(EncoderStatus::Encoded) => continue,
                Err(EncoderStatus::NeedMoreData) => break,
                Err(EncoderStatus::LimitReached) => break,
                Err(e) => {
                    return Err(Error::Encode(format!("AV1 encoding error: {}", e)));
                }
            }
        }

        Ok(packets)
    }
}

impl Encoder for Av1Encoder {
    fn encode(&mut self, frame: &Frame) -> Result<Vec<Packet>> {
        let yuv_frame = self.rgba_to_yuv420(frame);

        self.context
            .send_frame(yuv_frame)
            .map_err(|e| Error::Encode(format!("Failed to send frame: {}", e)))?;

        self.frame_count += 1;
        self.receive_packets()
    }

    fn flush(&mut self) -> Result<Vec<Packet>> {
        self.context.flush();

        let mut packets = Vec::new();

        loop {
            match self.context.receive_packet() {
                Ok(pkt) => {
                    packets.push(Packet {
                        data: pkt.data,
                        pts: pkt.input_frameno as i64,
                        dts: pkt.input_frameno as i64,
                        is_keyframe: pkt.frame_type == FrameType::KEY,
                    });
                }
                Err(EncoderStatus::Encoded) => continue,
                Err(EncoderStatus::NeedMoreData) => break,
                Err(EncoderStatus::LimitReached) => break,
                Err(_) => break,
            }
        }

        Ok(packets)
    }
}
