//! Slideshow video generation

use crate::encoder::{create_encoder, EncoderConfig, Frame, Packet};
use crate::image_loader::LoadedImage;
use crate::muxer::{create_muxer, MuxerConfig};
use crate::{EncodeOptions, Error, Result, SlideEntry};

/// Default frame rate for slideshow videos
const DEFAULT_FPS: u32 = 30;

/// Create a slideshow video from a sequence of images
///
/// Each image is displayed for the specified duration (in milliseconds).
/// All images are resized to match the dimensions of the first image.
pub fn slideshow(entries: &[SlideEntry], options: &EncodeOptions) -> Result<()> {
    // Validate options
    options.validate()?;

    if entries.is_empty() {
        return Err(Error::InvalidInput("No slides provided".to_string()));
    }

    // Load and validate all images
    let mut images: Vec<(LoadedImage, u32)> = Vec::new();

    for entry in entries {
        let img = LoadedImage::from_path(&entry.path)?;
        images.push((img, entry.duration_ms));
    }

    // Get target dimensions from the first image
    let (target_width, target_height) = (images[0].0.width, images[0].0.height);

    // Ensure dimensions are even (required for video encoding)
    let target_width = (target_width / 2) * 2;
    let target_height = (target_height / 2) * 2;

    // Resize all images to match the first one
    let images: Vec<(LoadedImage, u32)> = images
        .into_iter()
        .map(|(img, duration)| (img.resize(target_width, target_height), duration))
        .collect();

    // Create encoder
    let encoder_config = EncoderConfig {
        width: target_width,
        height: target_height,
        fps: DEFAULT_FPS,
        quality: options.quality,
    };

    let mut encoder = create_encoder(options.codec, encoder_config.clone())?;

    // Generate all frames and collect packets
    // We need to encode at least one frame before creating the muxer
    // so that H.264 encoders can extract SPS/PPS
    let mut all_packets: Vec<Packet> = Vec::new();
    let mut total_ms: u64 = 0;

    for (image, duration_ms) in &images {
        // Calculate number of frames for this slide
        let frame_count = (*duration_ms as u64 * DEFAULT_FPS as u64) / 1000;
        let frame_count = frame_count.max(1); // At least one frame

        for _ in 0..frame_count {
            let frame = Frame {
                width: image.width,
                height: image.height,
                data: image.data.clone(),
                pts_ms: total_ms,
            };

            let packets = encoder.encode(&frame)?;
            all_packets.extend(packets);

            total_ms += 1000 / DEFAULT_FPS as u64;
        }
    }

    // Flush encoder
    let flush_packets = encoder.flush()?;
    all_packets.extend(flush_packets);

    // Now create muxer with SPS/PPS from encoder (available after encoding)
    let muxer_config = MuxerConfig {
        width: target_width,
        height: target_height,
        fps: DEFAULT_FPS,
        codec: options.codec,
        codec_config: encoder.codec_config(),
        pps: encoder.pps(),
    };

    let mut muxer = create_muxer(options.container, &options.output_path, muxer_config)?;

    // Write all packets
    for packet in all_packets {
        muxer.write_packet(&packet)?;
    }

    // Finalize output
    muxer.finalize()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slideshow_empty_entries() {
        let options = EncodeOptions {
            output_path: "test.mp4".to_string(),
            container: crate::Container::Mp4,
            codec: crate::Codec::Av1,
            quality: 50,
            ffmpeg_path: None,
        };

        let result = slideshow(&[], &options);
        assert!(result.is_err());
    }
}
