//! macOS H.264 encoder using VideoToolbox

use super::super::{Encoder, EncoderConfig, Frame, Packet};
use crate::{Error, Result};
use std::ffi::c_void;
use std::ptr;
use std::sync::{Arc, Mutex};

// VideoToolbox FFI bindings
#[link(name = "VideoToolbox", kind = "framework")]
extern "C" {
    fn VTCompressionSessionCreate(
        allocator: *const c_void,
        width: i32,
        height: i32,
        codec_type: u32,
        encoder_specification: *const c_void,
        source_image_buffer_attributes: *const c_void,
        compressed_data_allocator: *const c_void,
        output_callback: extern "C" fn(*mut c_void, *mut c_void, i32, u32, *mut c_void),
        output_callback_ref_con: *mut c_void,
        compression_session_out: *mut *mut c_void,
    ) -> i32;

    fn VTCompressionSessionEncodeFrame(
        session: *mut c_void,
        image_buffer: *mut c_void,
        presentation_timestamp: CMTime,
        duration: CMTime,
        frame_properties: *const c_void,
        source_frame_ref_con: *mut c_void,
        info_flags_out: *mut u32,
    ) -> i32;

    fn VTCompressionSessionCompleteFrames(
        session: *mut c_void,
        complete_until_presentation_timestamp: CMTime,
    ) -> i32;

    fn VTCompressionSessionInvalidate(session: *mut c_void);
}

#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    fn CMTimeMake(value: i64, timescale: i32) -> CMTime;
}

#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    fn CVPixelBufferCreate(
        allocator: *const c_void,
        width: usize,
        height: usize,
        pixel_format_type: u32,
        pixel_buffer_attributes: *const c_void,
        pixel_buffer_out: *mut *mut c_void,
    ) -> i32;

    fn CVPixelBufferLockBaseAddress(pixel_buffer: *mut c_void, lock_flags: u64) -> i32;
    fn CVPixelBufferUnlockBaseAddress(pixel_buffer: *mut c_void, unlock_flags: u64) -> i32;
    fn CVPixelBufferGetBaseAddress(pixel_buffer: *mut c_void) -> *mut u8;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: *mut c_void) -> usize;
    fn CVPixelBufferRelease(pixel_buffer: *mut c_void);
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CMTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}

const K_CM_TIME_FLAGS_VALID: u32 = 1;
const K_CV_PIXEL_FORMAT_TYPE_32_BGRA: u32 = 0x42475241; // 'BGRA'
const K_CMV_VIDEO_CODEC_TYPE_H264: u32 = 0x61766331; // 'avc1'

/// VideoToolbox H.264 encoder
pub struct VideoToolboxEncoder {
    session: *mut c_void,
    config: EncoderConfig,
    packets: Arc<Mutex<Vec<Packet>>>,
    frame_count: u64,
}

unsafe impl Send for VideoToolboxEncoder {}

impl VideoToolboxEncoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        let packets = Arc::new(Mutex::new(Vec::new()));
        let packets_clone = Arc::clone(&packets);

        let mut session: *mut c_void = ptr::null_mut();

        // Create compression session
        let status = unsafe {
            VTCompressionSessionCreate(
                ptr::null(),
                config.width as i32,
                config.height as i32,
                K_CMV_VIDEO_CODEC_TYPE_H264,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                compression_output_callback,
                Box::into_raw(Box::new(packets_clone)) as *mut c_void,
                &mut session,
            )
        };

        if status != 0 {
            return Err(Error::Encode(format!(
                "Failed to create VideoToolbox session: {}",
                status
            )));
        }

        Ok(Self {
            session,
            config,
            packets,
            frame_count: 0,
        })
    }

    fn create_pixel_buffer(&self, frame: &Frame) -> Result<*mut c_void> {
        let mut pixel_buffer: *mut c_void = ptr::null_mut();

        let status = unsafe {
            CVPixelBufferCreate(
                ptr::null(),
                frame.width as usize,
                frame.height as usize,
                K_CV_PIXEL_FORMAT_TYPE_32_BGRA,
                ptr::null(),
                &mut pixel_buffer,
            )
        };

        if status != 0 {
            return Err(Error::Encode(format!(
                "Failed to create pixel buffer: {}",
                status
            )));
        }

        // Lock and copy data
        unsafe {
            CVPixelBufferLockBaseAddress(pixel_buffer, 0);
            let base_address = CVPixelBufferGetBaseAddress(pixel_buffer);
            let bytes_per_row = CVPixelBufferGetBytesPerRow(pixel_buffer);

            // Convert RGBA to BGRA and copy
            for y in 0..frame.height as usize {
                for x in 0..frame.width as usize {
                    let src_idx = (y * frame.width as usize + x) * 4;
                    let dst_idx = y * bytes_per_row + x * 4;

                    *base_address.add(dst_idx) = frame.data[src_idx + 2]; // B
                    *base_address.add(dst_idx + 1) = frame.data[src_idx + 1]; // G
                    *base_address.add(dst_idx + 2) = frame.data[src_idx]; // R
                    *base_address.add(dst_idx + 3) = frame.data[src_idx + 3]; // A
                }
            }

            CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);
        }

        Ok(pixel_buffer)
    }
}

extern "C" fn compression_output_callback(
    output_callback_ref_con: *mut c_void,
    _source_frame_ref_con: *mut c_void,
    status: i32,
    _info_flags: u32,
    sample_buffer: *mut c_void,
) {
    if status != 0 || sample_buffer.is_null() {
        return;
    }

    // This is a simplified implementation
    // In a real implementation, we would extract the encoded data from CMSampleBuffer
    let packets = unsafe { &*(output_callback_ref_con as *const Arc<Mutex<Vec<Packet>>>) };

    // TODO: Extract actual encoded data from CMSampleBuffer
    // For now, this is a placeholder
    if let Ok(mut pkts) = packets.lock() {
        // Placeholder - actual implementation would extract data
        pkts.push(Packet {
            data: Vec::new(),
            pts: 0,
            dts: 0,
            is_keyframe: true,
        });
    }
}

impl Encoder for VideoToolboxEncoder {
    fn encode(&mut self, frame: &Frame) -> Result<Vec<Packet>> {
        let pixel_buffer = self.create_pixel_buffer(frame)?;

        let pts = unsafe { CMTimeMake(self.frame_count as i64, self.config.fps as i32) };
        let duration = unsafe { CMTimeMake(1, self.config.fps as i32) };

        let status = unsafe {
            VTCompressionSessionEncodeFrame(
                self.session,
                pixel_buffer,
                pts,
                duration,
                ptr::null(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };

        unsafe {
            CVPixelBufferRelease(pixel_buffer);
        }

        if status != 0 {
            return Err(Error::Encode(format!("Failed to encode frame: {}", status)));
        }

        self.frame_count += 1;

        // Get encoded packets
        let mut packets = self.packets.lock().unwrap();
        let result = std::mem::take(&mut *packets);
        Ok(result)
    }

    fn flush(&mut self) -> Result<Vec<Packet>> {
        let complete_time = CMTime {
            value: i64::MAX,
            timescale: 1,
            flags: K_CM_TIME_FLAGS_VALID,
            epoch: 0,
        };

        unsafe {
            VTCompressionSessionCompleteFrames(self.session, complete_time);
        }

        let mut packets = self.packets.lock().unwrap();
        Ok(std::mem::take(&mut *packets))
    }
}

impl Drop for VideoToolboxEncoder {
    fn drop(&mut self) {
        if !self.session.is_null() {
            unsafe {
                VTCompressionSessionInvalidate(self.session);
            }
        }
    }
}

/// Check if VideoToolbox is available
pub fn check_available() -> Result<()> {
    // VideoToolbox is always available on macOS 10.8+
    Ok(())
}
