//! Error types for minmpeg

use crate::{Codec, Container};
use thiserror::Error;

/// Result type alias for minmpeg operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for minmpeg operations
#[derive(Error, Debug)]
pub enum Error {
    /// Invalid input parameter
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Codec is not available on this system
    #[error("Codec unavailable: {0}")]
    CodecUnavailable(String),

    /// Container and codec combination is not supported
    #[error("Container {container:?} does not support codec {codec:?}")]
    ContainerCodecMismatch { container: Container, codec: Codec },

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Image processing error
    #[error("Image error: {0}")]
    Image(#[from] image::ImageError),

    /// Encoding error
    #[error("Encoding error: {0}")]
    Encode(String),

    /// Decoding error
    #[error("Decoding error: {0}")]
    Decode(String),

    /// Muxing error
    #[error("Muxing error: {0}")]
    Mux(String),

    /// FFmpeg process error
    #[error("FFmpeg error: {0}")]
    Ffmpeg(String),

    /// Platform-specific error
    #[error("Platform error: {0}")]
    Platform(String),
}

/// Error code for FFI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum ErrorCode {
    /// Success
    Ok = 0,
    /// Invalid input parameter
    InvalidInput = 1,
    /// Codec not available
    CodecUnavailable = 2,
    /// Container/codec mismatch
    ContainerCodecMismatch = 3,
    /// I/O error
    IoError = 4,
    /// Encoding error
    EncodeError = 5,
    /// Decoding error
    DecodeError = 6,
}

impl From<&Error> for ErrorCode {
    fn from(err: &Error) -> Self {
        match err {
            Error::InvalidInput(_) => ErrorCode::InvalidInput,
            Error::CodecUnavailable(_) => ErrorCode::CodecUnavailable,
            Error::ContainerCodecMismatch { .. } => ErrorCode::ContainerCodecMismatch,
            Error::Io(_) => ErrorCode::IoError,
            Error::Image(_) => ErrorCode::EncodeError,
            Error::Encode(_) => ErrorCode::EncodeError,
            Error::Decode(_) => ErrorCode::DecodeError,
            Error::Mux(_) => ErrorCode::EncodeError,
            Error::Ffmpeg(_) => ErrorCode::EncodeError,
            Error::Platform(_) => ErrorCode::EncodeError,
        }
    }
}
