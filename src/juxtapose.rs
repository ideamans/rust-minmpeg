//! Side-by-side video juxtaposition

use crate::encoder::{create_encoder, EncoderConfig, Frame};
use crate::muxer::{create_muxer, MuxerConfig};
use crate::{Color, EncodeOptions, Error, Result};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

/// Default frame rate for output video
const DEFAULT_FPS: u32 = 30;

/// Video frame from decoded video
struct DecodedFrame {
    width: u32,
    height: u32,
    data: Vec<u8>, // RGBA
}

/// Video decoder using ffmpeg
struct VideoDecoder {
    width: u32,
    height: u32,
    fps: f64,
    frame_count: u64,
    current_frame: u64,
    process: Option<std::process::Child>,
    last_frame: Option<Vec<u8>>,
}

impl VideoDecoder {
    fn new<P: AsRef<Path>>(path: P, ffmpeg_path: Option<&str>) -> Result<Self> {
        let path = path.as_ref();
        let ffmpeg = find_ffmpeg(ffmpeg_path)?;

        // Get video info using ffprobe
        let (width, height, fps, frame_count) = get_video_info(path, &ffmpeg)?;

        Ok(Self {
            width,
            height,
            fps,
            frame_count,
            current_frame: 0,
            process: None,
            last_frame: None,
        })
    }

    fn start_decode<P: AsRef<Path>>(&mut self, path: P, ffmpeg_path: Option<&str>) -> Result<()> {
        let ffmpeg = find_ffmpeg(ffmpeg_path)?;

        let process = Command::new(&ffmpeg)
            .args([
                "-i",
                path.as_ref().to_str().unwrap(),
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-r",
                &DEFAULT_FPS.to_string(),
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Ffmpeg(format!("Failed to start ffmpeg: {}", e)))?;

        self.process = Some(process);
        Ok(())
    }

    fn read_frame(&mut self) -> Result<Option<DecodedFrame>> {
        let process = match self.process.as_mut() {
            Some(p) => p,
            None => return Ok(None),
        };

        let stdout = match process.stdout.as_mut() {
            Some(s) => s,
            None => return Ok(None),
        };

        let frame_size = (self.width * self.height * 4) as usize;
        let mut buffer = vec![0u8; frame_size];

        match stdout.read_exact(&mut buffer) {
            Ok(_) => {
                self.current_frame += 1;
                self.last_frame = Some(buffer.clone());
                Ok(Some(DecodedFrame {
                    width: self.width,
                    height: self.height,
                    data: buffer,
                }))
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                // End of video - return last frame if available
                if let Some(ref last) = self.last_frame {
                    Ok(Some(DecodedFrame {
                        width: self.width,
                        height: self.height,
                        data: last.clone(),
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(Error::Decode(format!("Failed to read frame: {}", e))),
        }
    }

    fn duration_frames(&self) -> u64 {
        ((self.frame_count as f64 * DEFAULT_FPS as f64) / self.fps).ceil() as u64
    }
}

impl Drop for VideoDecoder {
    fn drop(&mut self) {
        if let Some(ref mut process) = self.process {
            let _ = process.kill();
            let _ = process.wait();
        }
    }
}

/// Combine two videos side by side
///
/// The output video will have:
/// - Width = left video width + right video width
/// - Height = max(left video height, right video height)
/// - Duration = max(left video duration, right video duration)
///
/// If heights differ, videos are aligned to the top with the background color filling the bottom.
/// If durations differ, the shorter video continues showing its last frame.
pub fn juxtapose<P: AsRef<Path>>(
    left_path: P,
    right_path: P,
    options: &EncodeOptions,
    background: Option<Color>,
) -> Result<()> {
    // Validate options
    options.validate()?;

    let bg = background.unwrap_or_default();
    let ffmpeg_path = options.ffmpeg_path.as_deref();

    // Open both video decoders
    let mut left_decoder = VideoDecoder::new(&left_path, ffmpeg_path)?;
    let mut right_decoder = VideoDecoder::new(&right_path, ffmpeg_path)?;

    // Calculate output dimensions
    let output_width = left_decoder.width + right_decoder.width;
    let output_height = left_decoder.height.max(right_decoder.height);

    // Ensure dimensions are even
    let output_width = (output_width / 2) * 2;
    let output_height = (output_height / 2) * 2;

    // Calculate total frames (longer video duration)
    let total_frames = left_decoder
        .duration_frames()
        .max(right_decoder.duration_frames());

    // Start decoding
    left_decoder.start_decode(&left_path, ffmpeg_path)?;
    right_decoder.start_decode(&right_path, ffmpeg_path)?;

    // Create encoder
    let encoder_config = EncoderConfig {
        width: output_width,
        height: output_height,
        fps: DEFAULT_FPS,
        quality: options.quality,
    };

    let mut encoder = create_encoder(options.codec, encoder_config.clone())?;

    // Create muxer
    let muxer_config = MuxerConfig {
        width: output_width,
        height: output_height,
        fps: DEFAULT_FPS,
        codec: options.codec,
        codec_config: encoder.codec_config(),
    };

    let mut muxer = create_muxer(options.container, &options.output_path, muxer_config)?;

    // Process frames
    for frame_idx in 0..total_frames {
        // Read frames from both videos
        let left_frame = left_decoder.read_frame()?;
        let right_frame = right_decoder.read_frame()?;

        // Combine frames
        let combined = combine_frames(
            left_frame.as_ref(),
            right_frame.as_ref(),
            output_width,
            output_height,
            &bg,
        );

        let frame = Frame {
            width: output_width,
            height: output_height,
            data: combined,
            pts_ms: frame_idx * 1000 / DEFAULT_FPS as u64,
        };

        let packets = encoder.encode(&frame)?;
        for packet in packets {
            muxer.write_packet(&packet)?;
        }
    }

    // Flush encoder
    let packets = encoder.flush()?;
    for packet in packets {
        muxer.write_packet(&packet)?;
    }

    // Finalize output
    muxer.finalize()?;

    Ok(())
}

/// Combine two frames side by side
fn combine_frames(
    left: Option<&DecodedFrame>,
    right: Option<&DecodedFrame>,
    output_width: u32,
    output_height: u32,
    bg: &Color,
) -> Vec<u8> {
    let mut output = vec![0u8; (output_width * output_height * 4) as usize];

    // Fill with background color
    for i in 0..(output_width * output_height) as usize {
        output[i * 4] = bg.r;
        output[i * 4 + 1] = bg.g;
        output[i * 4 + 2] = bg.b;
        output[i * 4 + 3] = 255;
    }

    // Copy left frame (top-aligned)
    if let Some(left) = left {
        for y in 0..left.height.min(output_height) {
            for x in 0..left.width {
                let src_idx = ((y * left.width + x) * 4) as usize;
                let dst_idx = ((y * output_width + x) * 4) as usize;

                output[dst_idx] = left.data[src_idx];
                output[dst_idx + 1] = left.data[src_idx + 1];
                output[dst_idx + 2] = left.data[src_idx + 2];
                output[dst_idx + 3] = left.data[src_idx + 3];
            }
        }
    }

    // Copy right frame (top-aligned, offset by left width)
    if let Some(right) = right {
        let left_width = left.map(|l| l.width).unwrap_or(0);

        for y in 0..right.height.min(output_height) {
            for x in 0..right.width {
                let src_idx = ((y * right.width + x) * 4) as usize;
                let dst_idx = ((y * output_width + left_width + x) * 4) as usize;

                if dst_idx + 3 < output.len() {
                    output[dst_idx] = right.data[src_idx];
                    output[dst_idx + 1] = right.data[src_idx + 1];
                    output[dst_idx + 2] = right.data[src_idx + 2];
                    output[dst_idx + 3] = right.data[src_idx + 3];
                }
            }
        }
    }

    output
}

/// Find ffmpeg executable
fn find_ffmpeg(custom_path: Option<&str>) -> Result<String> {
    if let Some(path) = custom_path {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
        return Err(Error::Ffmpeg(format!("FFmpeg not found at: {}", path)));
    }

    // Try common paths
    let paths = [
        "ffmpeg",
        "/usr/bin/ffmpeg",
        "/usr/local/bin/ffmpeg",
        "/opt/homebrew/bin/ffmpeg",
    ];

    for path in paths {
        if Command::new(path)
            .arg("-version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Ok(path.to_string());
        }
    }

    Err(Error::Ffmpeg("FFmpeg not found in PATH".to_string()))
}

/// Get video information using ffprobe
fn get_video_info<P: AsRef<Path>>(path: P, ffmpeg: &str) -> Result<(u32, u32, f64, u64)> {
    // Derive ffprobe path from ffmpeg path
    let ffprobe = if ffmpeg.ends_with("ffmpeg") {
        ffmpeg.replace("ffmpeg", "ffprobe")
    } else {
        "ffprobe".to_string()
    };

    let output = Command::new(&ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,r_frame_rate,nb_frames",
            "-of",
            "csv=p=0",
            path.as_ref().to_str().unwrap(),
        ])
        .output()
        .map_err(|e| Error::Ffmpeg(format!("Failed to run ffprobe: {}", e)))?;

    let info = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = info.trim().split(',').collect();

    if parts.len() < 3 {
        return Err(Error::Decode(format!(
            "Failed to parse video info: {}",
            info
        )));
    }

    let width: u32 = parts[0]
        .parse()
        .map_err(|_| Error::Decode("Failed to parse width".to_string()))?;

    let height: u32 = parts[1]
        .parse()
        .map_err(|_| Error::Decode("Failed to parse height".to_string()))?;

    // Parse frame rate (e.g., "30/1" or "30000/1001")
    let fps: f64 = if parts[2].contains('/') {
        let fps_parts: Vec<&str> = parts[2].split('/').collect();
        let num: f64 = fps_parts[0].parse().unwrap_or(30.0);
        let den: f64 = fps_parts[1].parse().unwrap_or(1.0);
        num / den
    } else {
        parts[2].parse().unwrap_or(30.0)
    };

    let frame_count: u64 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);

    // If frame count is not available, estimate from duration
    let frame_count = if frame_count == 0 {
        // Try to get duration
        let duration_output = Command::new(&ffprobe)
            .args([
                "-v",
                "error",
                "-show_entries",
                "format=duration",
                "-of",
                "csv=p=0",
                path.as_ref().to_str().unwrap(),
            ])
            .output()
            .ok();

        if let Some(output) = duration_output {
            let duration_str = String::from_utf8_lossy(&output.stdout);
            let duration: f64 = duration_str.trim().parse().unwrap_or(0.0);
            (duration * fps).ceil() as u64
        } else {
            0
        }
    } else {
        frame_count
    };

    Ok((width, height, fps, frame_count))
}
