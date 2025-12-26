//! Linux H.264 encoder using ffmpeg external process

use super::super::{Encoder, EncoderConfig, Frame, Packet};
use crate::{Error, Result};
use std::io::Write;
use std::process::{Child, Command, Stdio};

/// FFmpeg-based H.264 encoder for Linux
pub struct FfmpegEncoder {
    process: Child,
    #[allow(dead_code)]
    config: EncoderConfig,
    frame_count: u64,
    #[allow(dead_code)]
    output_buffer: Vec<u8>,
}

impl FfmpegEncoder {
    pub fn new(config: EncoderConfig, ffmpeg_path: Option<&str>) -> Result<Self> {
        let ffmpeg = find_ffmpeg(ffmpeg_path)?;

        // Map quality (0-100) to CRF (51-0)
        let crf = ((100 - config.quality.min(100)) as u32 * 51) / 100;

        let process = Command::new(&ffmpeg)
            .args([
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "-s",
                &format!("{}x{}", config.width, config.height),
                "-r",
                &config.fps.to_string(),
                "-i",
                "pipe:0",
                "-c:v",
                "libx264",
                "-preset",
                "medium",
                "-crf",
                &crf.to_string(),
                "-pix_fmt",
                "yuv420p",
                "-f",
                "h264",
                "pipe:1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Ffmpeg(format!("Failed to start ffmpeg: {}", e)))?;

        Ok(Self {
            process,
            config,
            frame_count: 0,
            output_buffer: Vec::new(),
        })
    }

    fn read_available_output(&mut self) -> Result<Vec<u8>> {
        use std::io::Read;

        let stdout = self
            .process
            .stdout
            .as_mut()
            .ok_or_else(|| Error::Ffmpeg("FFmpeg stdout not available".to_string()))?;

        let mut buffer = vec![0u8; 65536];
        let mut result = Vec::new();

        // Non-blocking read - this is a simplified approach
        // In production, you might want to use async I/O
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => result.extend_from_slice(&buffer[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        Ok(result)
    }
}

impl Encoder for FfmpegEncoder {
    fn encode(&mut self, frame: &Frame) -> Result<Vec<Packet>> {
        let stdin = self
            .process
            .stdin
            .as_mut()
            .ok_or_else(|| Error::Ffmpeg("FFmpeg stdin not available".to_string()))?;

        // Write raw RGBA frame data
        stdin
            .write_all(&frame.data)
            .map_err(|e| Error::Ffmpeg(format!("Failed to write frame: {}", e)))?;

        self.frame_count += 1;

        // Try to read any available output
        let output = self.read_available_output()?;

        if output.is_empty() {
            return Ok(Vec::new());
        }

        // Parse H.264 NAL units from output
        let packets = parse_h264_packets(&output, self.frame_count - 1);
        Ok(packets)
    }

    fn flush(&mut self) -> Result<Vec<Packet>> {
        // Close stdin to signal end of input
        drop(self.process.stdin.take());

        // Wait for process to finish and read remaining output
        use std::io::Read;

        let mut output = Vec::new();
        if let Some(ref mut stdout) = self.process.stdout {
            stdout
                .read_to_end(&mut output)
                .map_err(|e| Error::Ffmpeg(format!("Failed to read output: {}", e)))?;
        }

        // Wait for process to exit
        self.process
            .wait()
            .map_err(|e| Error::Ffmpeg(format!("FFmpeg process error: {}", e)))?;

        // Parse remaining packets
        let packets = parse_h264_packets(&output, self.frame_count);
        Ok(packets)
    }
}

impl Drop for FfmpegEncoder {
    fn drop(&mut self) {
        // Kill the process if it's still running
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Parse H.264 NAL units from raw H.264 stream
fn parse_h264_packets(data: &[u8], base_pts: u64) -> Vec<Packet> {
    let mut packets = Vec::new();
    let mut start = 0;
    let mut pts = base_pts as i64;

    // Simple NAL unit parsing (looking for start codes)
    while start < data.len() {
        // Find start code (0x00 0x00 0x01 or 0x00 0x00 0x00 0x01)
        let nal_start = find_start_code(data, start);
        if nal_start.is_none() {
            break;
        }

        let (nal_start, start_code_len) = nal_start.unwrap();

        // Find next start code or end of data
        let nal_end = find_start_code(data, nal_start + start_code_len)
            .map(|(pos, _)| pos)
            .unwrap_or(data.len());

        let nal_data = data[nal_start + start_code_len..nal_end].to_vec();

        if !nal_data.is_empty() {
            let nal_type = nal_data[0] & 0x1F;
            let is_keyframe = nal_type == 5; // IDR slice

            packets.push(Packet {
                data: nal_data,
                pts,
                dts: pts,
                is_keyframe,
            });

            pts += 1;
        }

        start = nal_end;
    }

    packets
}

/// Find H.264 start code in data
fn find_start_code(data: &[u8], start: usize) -> Option<(usize, usize)> {
    if start + 3 > data.len() {
        return None;
    }

    for i in start..data.len() - 2 {
        if data[i] == 0x00 && data[i + 1] == 0x00 {
            if data[i + 2] == 0x01 {
                return Some((i, 3));
            }
            if i + 3 < data.len() && data[i + 2] == 0x00 && data[i + 3] == 0x01 {
                return Some((i, 4));
            }
        }
    }

    None
}

/// Find ffmpeg executable
fn find_ffmpeg(custom_path: Option<&str>) -> Result<String> {
    if let Some(path) = custom_path {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
        return Err(Error::Ffmpeg(format!("FFmpeg not found at: {}", path)));
    }

    // Try to find ffmpeg in PATH
    let paths = ["ffmpeg", "/usr/bin/ffmpeg", "/usr/local/bin/ffmpeg"];

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

    Err(Error::CodecUnavailable(
        "FFmpeg not found in PATH".to_string(),
    ))
}

/// Check if ffmpeg with H.264 support is available
pub fn check_available(ffmpeg_path: Option<&str>) -> Result<()> {
    let ffmpeg = find_ffmpeg(ffmpeg_path)?;

    // Check if ffmpeg has libx264 support
    let output = Command::new(&ffmpeg)
        .args(["-encoders"])
        .output()
        .map_err(|e| Error::Ffmpeg(format!("Failed to run ffmpeg: {}", e)))?;

    let encoders = String::from_utf8_lossy(&output.stdout);
    if encoders.contains("libx264") {
        Ok(())
    } else {
        Err(Error::CodecUnavailable(
            "FFmpeg does not have libx264 support".to_string(),
        ))
    }
}
