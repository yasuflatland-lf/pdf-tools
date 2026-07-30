use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};

use crate::application::errors::ImageError;
use crate::domain::geometry::RasterImage;

/// Encodes an RGBA8 raster as PNG bytes in memory.
///
/// The pixels are handed to the encoder by reference, so the buffer is never
/// copied. A raster whose buffer length disagrees with its dimensions is
/// rejected instead of being encoded into a corrupt image.
pub fn encode_png(image: &RasterImage) -> Result<Vec<u8>, ImageError> {
    if image.width == 0 || image.height == 0 {
        return Err(ImageError::EncodeFailed {
            reason: "image dimensions must be non-zero".to_owned(),
        });
    }

    let expected_len = u64::from(image.width)
        .checked_mul(u64::from(image.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| ImageError::EncodeFailed {
            reason: "image dimensions exceed the supported buffer size".to_owned(),
        })?;
    if image.rgba.len() != expected_len {
        return Err(ImageError::EncodeFailed {
            reason: format!(
                "RGBA buffer has {} bytes, expected {expected_len}",
                image.rgba.len()
            ),
        });
    }

    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            &image.rgba,
            image.width,
            image.height,
            ExtendedColorType::Rgba8,
        )
        .map_err(|error| ImageError::EncodeFailed {
            reason: error.to_string(),
        })?;

    Ok(bytes)
}
