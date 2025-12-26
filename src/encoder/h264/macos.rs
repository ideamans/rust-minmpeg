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
        output_callback: Option<
            extern "C" fn(*mut c_void, *mut c_void, i32, u32, *mut c_void) -> (),
        >,
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

    fn VTSessionSetProperty(session: *mut c_void, key: *const c_void, value: *const c_void) -> i32;
}

#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    fn CMTimeMake(value: i64, timescale: i32) -> CMTime;

    fn CMSampleBufferGetDataBuffer(sample_buffer: *mut c_void) -> *mut c_void;

    fn CMSampleBufferGetFormatDescription(sample_buffer: *mut c_void) -> *mut c_void;

    fn CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
        format_description: *mut c_void,
        parameter_set_index: usize,
        parameter_set_pointer_out: *mut *const u8,
        parameter_set_size_out: *mut usize,
        parameter_set_count_out: *mut usize,
        nal_unit_header_length_out: *mut i32,
    ) -> i32;

    fn CMBlockBufferGetDataLength(block_buffer: *mut c_void) -> usize;

    fn CMBlockBufferCopyDataBytes(
        block_buffer: *mut c_void,
        offset: usize,
        length: usize,
        destination: *mut u8,
    ) -> i32;

    fn CMSampleBufferGetSampleAttachmentsArray(
        sample_buffer: *mut c_void,
        create_if_necessary: bool,
    ) -> *mut c_void;
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

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFDictionaryGetValue(dict: *const c_void, key: *const c_void) -> *const c_void;
    fn CFBooleanGetValue(boolean: *const c_void) -> bool;
    fn CFArrayGetCount(array: *const c_void) -> isize;

    static kCFBooleanTrue: *const c_void;
    static kCFBooleanFalse: *const c_void;

    static kVTCompressionPropertyKey_RealTime: *const c_void;
    static kVTCompressionPropertyKey_ProfileLevel: *const c_void;
    static kVTCompressionPropertyKey_AllowFrameReordering: *const c_void;
    static kVTCompressionPropertyKey_MaxKeyFrameInterval: *const c_void;
    static kVTCompressionPropertyKey_AverageBitRate: *const c_void;

    #[allow(dead_code)]
    static kVTProfileLevel_H264_Baseline_AutoLevel: *const c_void;
    static kVTProfileLevel_H264_Main_AutoLevel: *const c_void;

    static kCMSampleAttachmentKey_NotSync: *const c_void;
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct CMTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}

const K_CM_TIME_FLAGS_VALID: u32 = 1;
const K_CV_PIXEL_FORMAT_TYPE_32_BGRA: u32 = 0x42475241; // 'BGRA'
const K_CMV_VIDEO_CODEC_TYPE_H264: u32 = 0x61766331; // 'avc1'

/// Encoded packet data passed through callback
struct CallbackData {
    packets: Vec<Packet>,
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
    frame_count: u64,
}

/// VideoToolbox H.264 encoder
pub struct VideoToolboxEncoder {
    session: *mut c_void,
    config: EncoderConfig,
    callback_data: Arc<Mutex<CallbackData>>,
    frame_count: u64,
}

unsafe impl Send for VideoToolboxEncoder {}

impl VideoToolboxEncoder {
    pub fn new(config: EncoderConfig) -> Result<Self> {
        let callback_data = Arc::new(Mutex::new(CallbackData {
            packets: Vec::new(),
            sps: None,
            pps: None,
            frame_count: 0,
        }));

        let callback_data_ptr = Arc::into_raw(Arc::clone(&callback_data)) as *mut c_void;

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
                Some(compression_output_callback),
                callback_data_ptr,
                &mut session,
            )
        };

        if status != 0 {
            // Clean up the Arc we created
            unsafe {
                let _ = Arc::from_raw(callback_data_ptr as *const Mutex<CallbackData>);
            }
            return Err(Error::Encode(format!(
                "Failed to create VideoToolbox session: {}",
                status
            )));
        }

        // Configure encoder properties
        unsafe {
            // Use Main profile for better compatibility
            VTSessionSetProperty(
                session,
                kVTCompressionPropertyKey_ProfileLevel,
                kVTProfileLevel_H264_Main_AutoLevel,
            );

            // Disable frame reordering for simpler output (no B-frames)
            VTSessionSetProperty(
                session,
                kVTCompressionPropertyKey_AllowFrameReordering,
                kCFBooleanFalse,
            );

            // Set keyframe interval
            let keyframe_interval = config.fps; // Keyframe every second
            let cf_number = create_cf_number(keyframe_interval as i64);
            if !cf_number.is_null() {
                VTSessionSetProperty(
                    session,
                    kVTCompressionPropertyKey_MaxKeyFrameInterval,
                    cf_number,
                );
                CFRelease(cf_number);
            }

            // Set bitrate based on quality
            let bitrate = calculate_bitrate(&config);
            let cf_bitrate = create_cf_number(bitrate as i64);
            if !cf_bitrate.is_null() {
                VTSessionSetProperty(
                    session,
                    kVTCompressionPropertyKey_AverageBitRate,
                    cf_bitrate,
                );
                CFRelease(cf_bitrate);
            }

            // Enable real-time encoding
            VTSessionSetProperty(session, kVTCompressionPropertyKey_RealTime, kCFBooleanTrue);
        }

        Ok(Self {
            session,
            config,
            callback_data,
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

    /// Get SPS/PPS for MP4 muxer configuration
    pub fn get_codec_config(&self) -> Option<Vec<u8>> {
        let data = self.callback_data.lock().ok()?;
        data.sps.clone()
    }

    /// Get PPS for MP4 muxer
    pub fn get_pps(&self) -> Option<Vec<u8>> {
        let data = self.callback_data.lock().ok()?;
        data.pps.clone()
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

    // Get callback data
    let callback_data = unsafe {
        let ptr = output_callback_ref_con as *const Mutex<CallbackData>;
        // Don't take ownership - just borrow
        &*ptr
    };

    let mut data = match callback_data.lock() {
        Ok(d) => d,
        Err(_) => return,
    };

    // Extract SPS/PPS on first frame
    if data.sps.is_none() {
        unsafe {
            let format_desc = CMSampleBufferGetFormatDescription(sample_buffer);
            if !format_desc.is_null() {
                // Get SPS
                let mut sps_ptr: *const u8 = ptr::null();
                let mut sps_size: usize = 0;
                let mut param_count: usize = 0;
                let mut nal_header_len: i32 = 0;

                let sps_status = CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
                    format_desc,
                    0, // SPS index
                    &mut sps_ptr,
                    &mut sps_size,
                    &mut param_count,
                    &mut nal_header_len,
                );

                if sps_status == 0 && !sps_ptr.is_null() && sps_size > 0 {
                    let sps_data = std::slice::from_raw_parts(sps_ptr, sps_size);
                    data.sps = Some(sps_data.to_vec());
                }

                // Get PPS
                let mut pps_ptr: *const u8 = ptr::null();
                let mut pps_size: usize = 0;

                let pps_status = CMVideoFormatDescriptionGetH264ParameterSetAtIndex(
                    format_desc,
                    1, // PPS index
                    &mut pps_ptr,
                    &mut pps_size,
                    &mut param_count,
                    &mut nal_header_len,
                );

                if pps_status == 0 && !pps_ptr.is_null() && pps_size > 0 {
                    let pps_data = std::slice::from_raw_parts(pps_ptr, pps_size);
                    data.pps = Some(pps_data.to_vec());
                }
            }
        }
    }

    // Get encoded data from CMBlockBuffer
    unsafe {
        let block_buffer = CMSampleBufferGetDataBuffer(sample_buffer);
        if block_buffer.is_null() {
            return;
        }

        let data_length = CMBlockBufferGetDataLength(block_buffer);
        if data_length == 0 {
            return;
        }

        let mut buffer = vec![0u8; data_length];
        let copy_status =
            CMBlockBufferCopyDataBytes(block_buffer, 0, data_length, buffer.as_mut_ptr());

        if copy_status != 0 {
            return;
        }

        // Convert AVCC format (length-prefixed) to Annex B (start code prefixed)
        let annex_b_data = convert_avcc_to_annex_b(&buffer);

        // Check if this is a keyframe
        let is_keyframe = is_sample_keyframe(sample_buffer);

        let frame_count = data.frame_count;
        data.frame_count += 1;

        data.packets.push(Packet {
            data: annex_b_data,
            pts: frame_count as i64,
            dts: frame_count as i64,
            is_keyframe,
        });
    }
}

/// Convert AVCC format (4-byte length prefix) to Annex B format (start codes)
fn convert_avcc_to_annex_b(avcc_data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(avcc_data.len() + 32);
    let mut offset = 0;

    while offset + 4 <= avcc_data.len() {
        // Read 4-byte length prefix (big endian)
        let nal_length = u32::from_be_bytes([
            avcc_data[offset],
            avcc_data[offset + 1],
            avcc_data[offset + 2],
            avcc_data[offset + 3],
        ]) as usize;

        offset += 4;

        if offset + nal_length > avcc_data.len() {
            break;
        }

        // Add Annex B start code
        result.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);

        // Add NAL unit data
        result.extend_from_slice(&avcc_data[offset..offset + nal_length]);

        offset += nal_length;
    }

    result
}

/// Check if sample is a keyframe
fn is_sample_keyframe(sample_buffer: *mut c_void) -> bool {
    unsafe {
        let attachments = CMSampleBufferGetSampleAttachmentsArray(sample_buffer, false);
        if attachments.is_null() {
            return true; // Assume keyframe if no attachments
        }

        let count = CFArrayGetCount(attachments);
        if count == 0 {
            return true;
        }

        // Get first attachment dictionary
        let dict = CFArrayGetValueAtIndex(attachments, 0);
        if dict.is_null() {
            return true;
        }

        // Check kCMSampleAttachmentKey_NotSync
        let not_sync = CFDictionaryGetValue(dict, kCMSampleAttachmentKey_NotSync);
        if not_sync.is_null() {
            return true; // No NotSync key means it's a sync frame (keyframe)
        }

        // If NotSync is true, it's not a keyframe
        !CFBooleanGetValue(not_sync)
    }
}

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFNumberCreate(
        allocator: *const c_void,
        the_type: i32,
        value_ptr: *const c_void,
    ) -> *mut c_void;
    fn CFRelease(cf: *mut c_void);
    fn CFArrayGetValueAtIndex(array: *const c_void, index: isize) -> *const c_void;
}

const K_CF_NUMBER_INT64_TYPE: i32 = 4;

fn create_cf_number(value: i64) -> *mut c_void {
    unsafe {
        CFNumberCreate(
            ptr::null(),
            K_CF_NUMBER_INT64_TYPE,
            &value as *const _ as *const c_void,
        )
    }
}

fn calculate_bitrate(config: &EncoderConfig) -> u32 {
    // Base bitrate calculation based on resolution and quality
    let pixels = config.width * config.height;
    let base_bitrate = match pixels {
        p if p <= 320 * 240 => 500_000,     // QVGA: 500 kbps
        p if p <= 640 * 480 => 1_000_000,   // VGA: 1 Mbps
        p if p <= 1280 * 720 => 2_500_000,  // 720p: 2.5 Mbps
        p if p <= 1920 * 1080 => 5_000_000, // 1080p: 5 Mbps
        _ => 8_000_000,                     // 4K+: 8 Mbps
    };

    // Adjust by quality (0-100)
    let quality_factor = (config.quality as u32 + 50) / 100; // 0.5x to 1.5x
    base_bitrate * quality_factor.max(1)
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
        let mut data = self.callback_data.lock().unwrap();
        let result = std::mem::take(&mut data.packets);
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

        let mut data = self.callback_data.lock().unwrap();
        Ok(std::mem::take(&mut data.packets))
    }

    fn codec_config(&self) -> Option<Vec<u8>> {
        self.get_codec_config()
    }

    fn pps(&self) -> Option<Vec<u8>> {
        self.get_pps()
    }
}

impl Drop for VideoToolboxEncoder {
    fn drop(&mut self) {
        if !self.session.is_null() {
            unsafe {
                VTCompressionSessionInvalidate(self.session);
            }
        }
        // Note: callback_data Arc will be properly dropped when all references are gone
    }
}

/// Check if VideoToolbox is available
pub fn check_available() -> Result<()> {
    // VideoToolbox is always available on macOS 10.8+
    Ok(())
}
