//! Image loading utilities

use crate::{Error, Result};
use image::{DynamicImage, GenericImageView, ImageReader};
use std::path::Path;

/// Loaded image in RGBA format
#[derive(Debug, Clone)]
pub struct LoadedImage {
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// RGBA pixel data
    pub data: Vec<u8>,
}

impl LoadedImage {
    /// Load an image from a file path
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        let img = ImageReader::open(path).map_err(Error::Io)?.decode()?;

        Ok(Self::from_dynamic_image(img))
    }

    /// Create from a DynamicImage
    pub fn from_dynamic_image(img: DynamicImage) -> Self {
        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();
        let data = rgba.into_raw();

        Self {
            width,
            height,
            data,
        }
    }

    /// Resize the image to fit within the given dimensions
    pub fn resize(&self, target_width: u32, target_height: u32) -> Self {
        if self.width == target_width && self.height == target_height {
            return self.clone();
        }

        let img = image::RgbaImage::from_raw(self.width, self.height, self.data.clone())
            .expect("Invalid image data");

        let dynamic = DynamicImage::ImageRgba8(img);
        let resized = dynamic.resize_exact(
            target_width,
            target_height,
            image::imageops::FilterType::Lanczos3,
        );

        Self::from_dynamic_image(resized)
    }

    /// Resize the image to fit within the given dimensions while preserving aspect ratio
    /// Pads with the specified background color if needed
    pub fn resize_fit(&self, target_width: u32, target_height: u32, bg_color: [u8; 4]) -> Self {
        if self.width == target_width && self.height == target_height {
            return self.clone();
        }

        // Calculate scaling factor to fit within target dimensions
        let scale_x = target_width as f64 / self.width as f64;
        let scale_y = target_height as f64 / self.height as f64;
        let scale = scale_x.min(scale_y);

        let new_width = (self.width as f64 * scale).round() as u32;
        let new_height = (self.height as f64 * scale).round() as u32;

        // Resize the image
        let img = image::RgbaImage::from_raw(self.width, self.height, self.data.clone())
            .expect("Invalid image data");

        let dynamic = DynamicImage::ImageRgba8(img);
        let resized =
            dynamic.resize_exact(new_width, new_height, image::imageops::FilterType::Lanczos3);

        // Create output image with background color
        let mut output = vec![0u8; (target_width * target_height * 4) as usize];

        // Fill with background color
        for i in 0..(target_width * target_height) as usize {
            output[i * 4] = bg_color[0];
            output[i * 4 + 1] = bg_color[1];
            output[i * 4 + 2] = bg_color[2];
            output[i * 4 + 3] = bg_color[3];
        }

        // Calculate offset to center the image
        let offset_x = (target_width - new_width) / 2;
        let offset_y = (target_height - new_height) / 2;

        // Copy resized image to output
        let resized_rgba = resized.to_rgba8();
        for y in 0..new_height {
            for x in 0..new_width {
                let dst_idx = (((offset_y + y) * target_width + (offset_x + x)) * 4) as usize;

                output[dst_idx] = resized_rgba[(x, y)][0];
                output[dst_idx + 1] = resized_rgba[(x, y)][1];
                output[dst_idx + 2] = resized_rgba[(x, y)][2];
                output[dst_idx + 3] = resized_rgba[(x, y)][3];
            }
        }

        Self {
            width: target_width,
            height: target_height,
            data: output,
        }
    }
}

/// Load multiple images and normalize them to the same size
pub fn load_and_normalize_images<P: AsRef<Path>>(paths: &[P]) -> Result<Vec<LoadedImage>> {
    if paths.is_empty() {
        return Err(Error::InvalidInput("No images provided".to_string()));
    }

    // Load all images
    let images: Vec<LoadedImage> = paths
        .iter()
        .map(LoadedImage::from_path)
        .collect::<Result<Vec<_>>>()?;

    // Use the first image's dimensions as the target
    let target_width = images[0].width;
    let target_height = images[0].height;

    // Resize all images to match
    let normalized: Vec<LoadedImage> = images
        .into_iter()
        .map(|img| img.resize(target_width, target_height))
        .collect();

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize() {
        // Create a simple 2x2 image
        let img = LoadedImage {
            width: 2,
            height: 2,
            data: vec![
                255, 0, 0, 255, // Red
                0, 255, 0, 255, // Green
                0, 0, 255, 255, // Blue
                255, 255, 0, 255, // Yellow
            ],
        };

        let resized = img.resize(4, 4);
        assert_eq!(resized.width, 4);
        assert_eq!(resized.height, 4);
        assert_eq!(resized.data.len(), 4 * 4 * 4);
    }
}
