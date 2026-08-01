use std::path::Path;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder as _, ImageReader};

use crate::application::errors::ImageError;
use crate::application::ports::ImageDecoder;
use crate::domain::geometry::RasterImage;
use crate::domain::source::ImageInfo;
use crate::domain::source::SourceKind;

pub struct ImageCrateDecoder;

pub(crate) struct OrientedImageInfo {
    pub info: ImageInfo,
    pub orientation: Orientation,
}

impl ImageDecoder for ImageCrateDecoder {
    fn probe(&self, src: &Path) -> Result<ImageInfo, ImageError> {
        Ok(probe_with_orientation(src)?.info)
    }

    fn decode_first_frame(&self, src: &Path) -> Result<RasterImage, ImageError> {
        validate_source(src)?;

        // Decoding only the first GIF frame is deliberate; animations are single PDF pages.
        let mut decoder = reader(src)?
            .into_decoder()
            .map_err(|error| unreadable(src, error))?;
        // Metadata is advisory. A damaged EXIF block must not make otherwise
        // decodable pixels fail to import.
        let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
        let mut image =
            DynamicImage::from_decoder(decoder).map_err(|error| unreadable(src, error))?;
        image.apply_orientation(orientation);
        let image = image.to_rgba8();
        let (width, height) = image.dimensions();

        Ok(RasterImage {
            width,
            height,
            rgba: image.into_raw(),
        })
    }
}

pub(crate) fn probe_with_orientation(src: &Path) -> Result<OrientedImageInfo, ImageError> {
    validate_source(src)?;

    let mut decoder = reader(src)?
        .into_decoder()
        .map_err(|error| unreadable(src, error))?;
    let (mut width_px, mut height_px) = decoder.dimensions();
    // Keep a usable image usable even when only its optional EXIF metadata is
    // malformed or unreadable.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    if swaps_axes(orientation) {
        std::mem::swap(&mut width_px, &mut height_px);
    }

    Ok(OrientedImageInfo {
        info: ImageInfo {
            width_px,
            height_px,
        },
        orientation,
    })
}

fn swaps_axes(orientation: Orientation) -> bool {
    matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

fn validate_source(src: &Path) -> Result<(), ImageError> {
    match src.try_exists() {
        Ok(false) => {
            return Err(ImageError::Missing {
                path: src.to_path_buf(),
            });
        }
        Err(error) => return Err(unreadable(src, error)),
        Ok(true) => {}
    }

    if SourceKind::from_extension(src) == Some(SourceKind::Image) {
        Ok(())
    } else {
        Err(ImageError::UnsupportedFormat {
            path: src.to_path_buf(),
        })
    }
}

fn reader(src: &Path) -> Result<ImageReader<std::io::BufReader<std::fs::File>>, ImageError> {
    ImageReader::open(src)
        .map_err(|error| unreadable(src, error))?
        .with_guessed_format()
        .map_err(|error| unreadable(src, error))
}

fn unreadable(src: &Path, error: impl std::fmt::Display) -> ImageError {
    ImageError::Unreadable {
        path: src.to_path_buf(),
        reason: error.to_string(),
    }
}
