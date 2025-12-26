//! Windows H.264 encoder using Media Foundation

use super::super::{Encoder, EncoderConfig, Frame, Packet};
use crate::{Error, Result};
use std::ptr;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::*;

/// Media Foundation H.264 encoder
pub struct MediaFoundationEncoder {
    transform: IMFTransform,
    input_type: IMFMediaType,
    output_type: IMFMediaType,
    config: EncoderConfig,
    frame_count: u64,
    initialized: bool,
}

unsafe impl Send for MediaFoundationEncoder {}

impl MediaFoundationEncoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        unsafe {
            // Initialize COM
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| Error::Platform(format!("Failed to initialize COM: {}", e)))?;

            // Initialize Media Foundation
            MFStartup(MF_VERSION, MFSTARTUP_FULL)
                .map_err(|e| Error::Platform(format!("Failed to start MF: {}", e)))?;

            // Find and create H.264 encoder
            let transform = find_h264_encoder()?;

            // Create input media type (NV12)
            let input_type: IMFMediaType = MFCreateMediaType()
                .map_err(|e| Error::Encode(format!("Failed to create input type: {}", e)))?;

            input_type
                .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .map_err(|e| Error::Encode(format!("Failed to set major type: {}", e)))?;

            input_type
                .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_NV12)
                .map_err(|e| Error::Encode(format!("Failed to set subtype: {}", e)))?;

            input_type
                .SetUINT64(
                    &MF_MT_FRAME_SIZE,
                    ((config.width as u64) << 32) | (config.height as u64),
                )
                .map_err(|e| Error::Encode(format!("Failed to set frame size: {}", e)))?;

            input_type
                .SetUINT64(&MF_MT_FRAME_RATE, ((config.fps as u64) << 32) | 1u64)
                .map_err(|e| Error::Encode(format!("Failed to set frame rate: {}", e)))?;

            // Create output media type (H.264)
            let output_type: IMFMediaType = MFCreateMediaType()
                .map_err(|e| Error::Encode(format!("Failed to create output type: {}", e)))?;

            output_type
                .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .map_err(|e| Error::Encode(format!("Failed to set major type: {}", e)))?;

            output_type
                .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)
                .map_err(|e| Error::Encode(format!("Failed to set subtype: {}", e)))?;

            output_type
                .SetUINT64(
                    &MF_MT_FRAME_SIZE,
                    ((config.width as u64) << 32) | (config.height as u64),
                )
                .map_err(|e| Error::Encode(format!("Failed to set frame size: {}", e)))?;

            output_type
                .SetUINT64(&MF_MT_FRAME_RATE, ((config.fps as u64) << 32) | 1u64)
                .map_err(|e| Error::Encode(format!("Failed to set frame rate: {}", e)))?;

            // Calculate bitrate from quality (rough estimate)
            let bitrate = calculate_bitrate(&config);
            output_type
                .SetUINT32(&MF_MT_AVG_BITRATE, bitrate)
                .map_err(|e| Error::Encode(format!("Failed to set bitrate: {}", e)))?;

            // Set interlace mode (progressive scan)
            output_type
                .SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)
                .map_err(|e| Error::Encode(format!("Failed to set interlace mode: {}", e)))?;

            // Set output type
            transform
                .SetOutputType(0, &output_type, 0)
                .map_err(|e| Error::Encode(format!("Failed to set output type: {}", e)))?;

            // Set input type
            transform
                .SetInputType(0, &input_type, 0)
                .map_err(|e| Error::Encode(format!("Failed to set input type: {}", e)))?;

            Ok(Self {
                transform,
                input_type,
                output_type,
                config,
                frame_count: 0,
                initialized: true,
            })
        }
    }

    fn rgba_to_nv12(&self, frame: &Frame) -> Vec<u8> {
        let width = frame.width as usize;
        let height = frame.height as usize;
        let y_size = width * height;
        let uv_size = (width / 2) * (height / 2) * 2;
        let mut nv12 = vec![0u8; y_size + uv_size];

        // Y plane
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 4;
                let r = frame.data[idx] as f32;
                let g = frame.data[idx + 1] as f32;
                let b = frame.data[idx + 2] as f32;

                // BT.601 conversion
                let y_val = (0.299 * r + 0.587 * g + 0.114 * b).clamp(0.0, 255.0) as u8;
                nv12[y * width + x] = y_val;
            }
        }

        // UV plane (interleaved)
        let uv_offset = y_size;
        let uv_width = width / 2;

        for y in 0..(height / 2) {
            for x in 0..(width / 2) {
                let src_x = x * 2;
                let src_y = y * 2;

                // Average 2x2 block
                let mut r_sum = 0u32;
                let mut g_sum = 0u32;
                let mut b_sum = 0u32;

                for dy in 0..2 {
                    for dx in 0..2 {
                        let idx = ((src_y + dy) * width + (src_x + dx)) * 4;
                        r_sum += frame.data[idx] as u32;
                        g_sum += frame.data[idx + 1] as u32;
                        b_sum += frame.data[idx + 2] as u32;
                    }
                }

                let r = (r_sum / 4) as f32;
                let g = (g_sum / 4) as f32;
                let b = (b_sum / 4) as f32;

                let u = ((-0.169 * r - 0.331 * g + 0.500 * b) + 128.0).clamp(0.0, 255.0) as u8;
                let v = ((0.500 * r - 0.419 * g - 0.081 * b) + 128.0).clamp(0.0, 255.0) as u8;

                nv12[uv_offset + y * uv_width * 2 + x * 2] = u;
                nv12[uv_offset + y * uv_width * 2 + x * 2 + 1] = v;
            }
        }

        nv12
    }
}

impl Encoder for MediaFoundationEncoder {
    fn encode(&mut self, frame: &Frame) -> Result<Vec<Packet>> {
        let nv12_data = self.rgba_to_nv12(frame);

        unsafe {
            // Create input sample
            let sample: IMFSample = MFCreateSample()
                .map_err(|e| Error::Encode(format!("Failed to create sample: {}", e)))?;

            let buffer: IMFMediaBuffer = MFCreateMemoryBuffer(nv12_data.len() as u32)
                .map_err(|e| Error::Encode(format!("Failed to create buffer: {}", e)))?;

            // Copy data to buffer
            let mut buffer_ptr: *mut u8 = ptr::null_mut();
            buffer
                .Lock(&mut buffer_ptr, None, None)
                .map_err(|e| Error::Encode(format!("Failed to lock buffer: {}", e)))?;

            ptr::copy_nonoverlapping(nv12_data.as_ptr(), buffer_ptr, nv12_data.len());

            buffer
                .Unlock()
                .map_err(|e| Error::Encode(format!("Failed to unlock buffer: {}", e)))?;

            buffer
                .SetCurrentLength(nv12_data.len() as u32)
                .map_err(|e| Error::Encode(format!("Failed to set length: {}", e)))?;

            sample
                .AddBuffer(&buffer)
                .map_err(|e| Error::Encode(format!("Failed to add buffer: {}", e)))?;

            // Set timestamp
            let timestamp = (self.frame_count as i64 * 10_000_000) / self.config.fps as i64;
            sample
                .SetSampleTime(timestamp)
                .map_err(|e| Error::Encode(format!("Failed to set time: {}", e)))?;

            let duration = 10_000_000 / self.config.fps as i64;
            sample
                .SetSampleDuration(duration)
                .map_err(|e| Error::Encode(format!("Failed to set duration: {}", e)))?;

            // Process input
            self.transform
                .ProcessInput(0, &sample, 0)
                .map_err(|e| Error::Encode(format!("Failed to process input: {}", e)))?;

            self.frame_count += 1;

            // Get output
            self.get_output_packets()
        }
    }

    fn flush(&mut self) -> Result<Vec<Packet>> {
        unsafe {
            self.transform
                .ProcessMessage(MFT_MESSAGE_NOTIFY_END_OF_STREAM, 0)
                .ok();

            self.transform
                .ProcessMessage(MFT_MESSAGE_COMMAND_DRAIN, 0)
                .ok();

            self.get_output_packets()
        }
    }
}

impl MediaFoundationEncoder {
    unsafe fn get_output_packets(&mut self) -> Result<Vec<Packet>> {
        let mut packets = Vec::new();

        loop {
            let mut output_info = MFT_OUTPUT_DATA_BUFFER::default();
            let mut status = 0u32;

            // Create output sample
            let output_sample: IMFSample = match MFCreateSample() {
                Ok(s) => s,
                Err(_) => break,
            };

            // Get buffer requirements
            let stream_info = match self.transform.GetOutputStreamInfo(0) {
                Ok(info) => info,
                Err(_) => break,
            };

            let output_buffer: IMFMediaBuffer = match MFCreateMemoryBuffer(stream_info.cbSize) {
                Ok(b) => b,
                Err(_) => break,
            };

            if output_sample.AddBuffer(&output_buffer).is_err() {
                break;
            }

            let sample_clone = output_sample.clone();
            output_info.pSample = std::mem::ManuallyDrop::new(Some(output_sample));

            let result = self
                .transform
                .ProcessOutput(0, &mut [output_info], &mut status);

            if result.is_err() {
                break;
            }

            // Extract data from sample (use clone since output_info was moved)
            {
                let sample = sample_clone;
                if let Ok(buffer) = sample.GetBufferByIndex(0) {
                    let mut data_ptr: *mut u8 = ptr::null_mut();
                    let mut length = 0u32;

                    if buffer.Lock(&mut data_ptr, None, Some(&mut length)).is_ok() {
                        let data = std::slice::from_raw_parts(data_ptr, length as usize).to_vec();
                        buffer.Unlock().ok();

                        packets.push(Packet {
                            data,
                            pts: self.frame_count as i64 - 1,
                            dts: self.frame_count as i64 - 1,
                            is_keyframe: packets.is_empty(), // First packet is keyframe
                        });
                    }
                }
            }
        }

        Ok(packets)
    }
}

impl Drop for MediaFoundationEncoder {
    fn drop(&mut self) {
        unsafe {
            MFShutdown().ok();
            CoUninitialize();
        }
    }
}

fn find_h264_encoder() -> Result<IMFTransform> {
    unsafe {
        let mut count = 0u32;
        let mut activates: *mut Option<IMFActivate> = ptr::null_mut();

        let input_type = MFT_REGISTER_TYPE_INFO {
            guidMajorType: MFMediaType_Video,
            guidSubtype: MFVideoFormat_NV12,
        };

        let output_type = MFT_REGISTER_TYPE_INFO {
            guidMajorType: MFMediaType_Video,
            guidSubtype: MFVideoFormat_H264,
        };

        MFTEnumEx(
            MFT_CATEGORY_VIDEO_ENCODER,
            MFT_ENUM_FLAG_SYNCMFT | MFT_ENUM_FLAG_ASYNCMFT | MFT_ENUM_FLAG_HARDWARE,
            Some(&input_type),
            Some(&output_type),
            &mut activates,
            &mut count,
        )
        .map_err(|e| Error::CodecUnavailable(format!("Failed to enumerate encoders: {}", e)))?;

        if count == 0 || activates.is_null() {
            return Err(Error::CodecUnavailable(
                "No H.264 encoder found".to_string(),
            ));
        }

        // Get the first activate object
        let activate_slice = std::slice::from_raw_parts(activates, count as usize);
        let activate = activate_slice[0]
            .as_ref()
            .ok_or_else(|| Error::CodecUnavailable("Invalid activate object".to_string()))?;

        // Create transform from activate
        let transform: IMFTransform = activate
            .ActivateObject()
            .map_err(|e| Error::CodecUnavailable(format!("Failed to activate encoder: {}", e)))?;

        // Free the activate array
        for i in 0..count as usize {
            drop(activate_slice[i].clone());
        }
        CoTaskMemFree(Some(activates as *const _));

        Ok(transform)
    }
}

fn calculate_bitrate(config: &EncoderConfig) -> u32 {
    // Rough bitrate calculation based on resolution, fps, and quality
    let pixels = config.width * config.height;
    let base_bitrate = (pixels * config.fps) / 100;
    let quality_factor = (config.quality as u32 + 10) / 10;
    base_bitrate * quality_factor
}

/// Check if Media Foundation H.264 encoder is available
pub fn check_available() -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| Error::Platform(format!("Failed to initialize COM: {}", e)))?;

        MFStartup(MF_VERSION, MFSTARTUP_FULL)
            .map_err(|e| Error::Platform(format!("Failed to start MF: {}", e)))?;

        // Convert Result<IMFTransform, Error> to Result<(), Error>
        // We need to drop the transform BEFORE shutting down COM/MF to avoid access violation
        let result = match find_h264_encoder() {
            Ok(_transform) => {
                // Transform is dropped here, before MFShutdown
                Ok(())
            }
            Err(e) => Err(e),
        };

        MFShutdown().ok();
        CoUninitialize();

        result
    }
}
