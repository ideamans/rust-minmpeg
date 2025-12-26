//! FFI (Foreign Function Interface) for C/Go interoperability

use crate::error::ErrorCode;
use crate::{available, juxtapose, slideshow, Codec, Color, Container, EncodeOptions, SlideEntry};
use libc::{c_char, size_t};
use std::ffi::{CStr, CString};
use std::ptr;
use std::slice;

/// FFI result structure
#[repr(C)]
pub struct FfiResult {
    pub code: ErrorCode,
    pub message: *mut c_char,
}

impl FfiResult {
    fn ok() -> Self {
        Self {
            code: ErrorCode::Ok,
            message: ptr::null_mut(),
        }
    }

    fn error(code: ErrorCode, message: &str) -> Self {
        let c_message =
            CString::new(message).unwrap_or_else(|_| CString::new("Unknown error").unwrap());
        Self {
            code,
            message: c_message.into_raw(),
        }
    }
}

/// FFI slide entry structure
#[repr(C)]
pub struct FfiSlideEntry {
    pub path: *const c_char,
    pub duration_ms: u32,
}

/// FFI color structure
#[repr(C)]
pub struct FfiColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Check if a codec is available
///
/// # Safety
/// - `ffmpeg_path` must be a valid null-terminated string or null
#[no_mangle]
pub unsafe extern "C" fn minmpeg_available(codec: Codec, ffmpeg_path: *const c_char) -> FfiResult {
    let ffmpeg_path = if ffmpeg_path.is_null() {
        None
    } else {
        match CStr::from_ptr(ffmpeg_path).to_str() {
            Ok(s) => Some(s),
            Err(_) => return FfiResult::error(ErrorCode::InvalidInput, "Invalid ffmpeg path"),
        }
    };

    match available(codec, ffmpeg_path) {
        Ok(_) => FfiResult::ok(),
        Err(e) => FfiResult::error(ErrorCode::from(&e), &e.to_string()),
    }
}

/// Create a slideshow video from images
///
/// # Safety
/// - `entries` must point to a valid array of `FfiSlideEntry` with `entry_count` elements
/// - `output_path` must be a valid null-terminated string
/// - `ffmpeg_path` must be a valid null-terminated string or null
#[no_mangle]
pub unsafe extern "C" fn minmpeg_slideshow(
    entries: *const FfiSlideEntry,
    entry_count: size_t,
    output_path: *const c_char,
    container: Container,
    codec: Codec,
    quality: u8,
    ffmpeg_path: *const c_char,
) -> FfiResult {
    // Validate inputs
    if entries.is_null() || entry_count == 0 {
        return FfiResult::error(ErrorCode::InvalidInput, "No slides provided");
    }

    if output_path.is_null() {
        return FfiResult::error(ErrorCode::InvalidInput, "Output path is null");
    }

    // Convert output path
    let output_path = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return FfiResult::error(ErrorCode::InvalidInput, "Invalid output path"),
    };

    // Convert ffmpeg path
    let ffmpeg_path = if ffmpeg_path.is_null() {
        None
    } else {
        match CStr::from_ptr(ffmpeg_path).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => return FfiResult::error(ErrorCode::InvalidInput, "Invalid ffmpeg path"),
        }
    };

    // Convert slide entries
    let ffi_entries = slice::from_raw_parts(entries, entry_count);
    let mut slide_entries: Vec<SlideEntry> = Vec::with_capacity(entry_count);

    for entry in ffi_entries {
        if entry.path.is_null() {
            return FfiResult::error(ErrorCode::InvalidInput, "Slide path is null");
        }

        let path = match CStr::from_ptr(entry.path).to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return FfiResult::error(ErrorCode::InvalidInput, "Invalid slide path"),
        };

        slide_entries.push(SlideEntry {
            path,
            duration_ms: entry.duration_ms,
        });
    }

    // Create encode options
    let options = EncodeOptions {
        output_path,
        container,
        codec,
        quality,
        ffmpeg_path,
    };

    // Run slideshow
    match slideshow(&slide_entries, &options) {
        Ok(_) => FfiResult::ok(),
        Err(e) => FfiResult::error(ErrorCode::from(&e), &e.to_string()),
    }
}

/// Combine two videos side by side
///
/// # Safety
/// - `left_path`, `right_path`, and `output_path` must be valid null-terminated strings
/// - `background` can be null (defaults to white)
/// - `ffmpeg_path` can be null
#[no_mangle]
pub unsafe extern "C" fn minmpeg_juxtapose(
    left_path: *const c_char,
    right_path: *const c_char,
    output_path: *const c_char,
    container: Container,
    codec: Codec,
    quality: u8,
    background: *const FfiColor,
    ffmpeg_path: *const c_char,
) -> FfiResult {
    // Validate inputs
    if left_path.is_null() {
        return FfiResult::error(ErrorCode::InvalidInput, "Left video path is null");
    }

    if right_path.is_null() {
        return FfiResult::error(ErrorCode::InvalidInput, "Right video path is null");
    }

    if output_path.is_null() {
        return FfiResult::error(ErrorCode::InvalidInput, "Output path is null");
    }

    // Convert paths
    let left_path = match CStr::from_ptr(left_path).to_str() {
        Ok(s) => s,
        Err(_) => return FfiResult::error(ErrorCode::InvalidInput, "Invalid left video path"),
    };

    let right_path = match CStr::from_ptr(right_path).to_str() {
        Ok(s) => s,
        Err(_) => return FfiResult::error(ErrorCode::InvalidInput, "Invalid right video path"),
    };

    let output_path = match CStr::from_ptr(output_path).to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return FfiResult::error(ErrorCode::InvalidInput, "Invalid output path"),
    };

    // Convert ffmpeg path
    let ffmpeg_path = if ffmpeg_path.is_null() {
        None
    } else {
        match CStr::from_ptr(ffmpeg_path).to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => return FfiResult::error(ErrorCode::InvalidInput, "Invalid ffmpeg path"),
        }
    };

    // Convert background color
    let bg_color = if background.is_null() {
        None
    } else {
        let bg = &*background;
        Some(Color {
            r: bg.r,
            g: bg.g,
            b: bg.b,
        })
    };

    // Create encode options
    let options = EncodeOptions {
        output_path,
        container,
        codec,
        quality,
        ffmpeg_path,
    };

    // Run juxtapose
    match juxtapose(left_path, right_path, &options, bg_color) {
        Ok(_) => FfiResult::ok(),
        Err(e) => FfiResult::error(ErrorCode::from(&e), &e.to_string()),
    }
}

/// Free a result's message string
///
/// # Safety
/// - `result` must point to a valid `FfiResult` that was returned by a minmpeg function
#[no_mangle]
pub unsafe extern "C" fn minmpeg_free_result(result: *mut FfiResult) {
    if result.is_null() {
        return;
    }

    let result = &mut *result;
    if !result.message.is_null() {
        // Reclaim the CString and let it drop
        let _ = CString::from_raw(result.message);
        result.message = ptr::null_mut();
    }
}

/// Get version string
#[no_mangle]
pub extern "C" fn minmpeg_version() -> *const c_char {
    static VERSION: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();
    VERSION.as_ptr() as *const c_char
}
