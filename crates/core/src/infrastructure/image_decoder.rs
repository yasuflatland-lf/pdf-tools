use std::path::Path;

use image::ImageReader;

use crate::application::errors::ImageError;
use crate::application::ports::ImageDecoder;
use crate::domain::geometry::RasterImage;
use crate::domain::source::ImageInfo;

pub struct ImageCrateDecoder;

impl ImageDecoder for ImageCrateDecoder {
    fn probe(&self, src: &Path) -> Result<ImageInfo, ImageError> {
        validate_source(src)?;

        let (width_px, height_px) = reader(src)?
            .into_dimensions()
            .map_err(|error| unreadable(src, error))?;

        Ok(ImageInfo {
            width_px,
            height_px,
        })
    }

    fn decode_first_frame(&self, src: &Path) -> Result<RasterImage, ImageError> {
        validate_source(src)?;

        // Decoding only the first GIF frame is deliberate; animations are single PDF pages.
        let image = reader(src)?
            .decode()
            .map_err(|error| unreadable(src, error))?
            .to_rgba8();
        let (width, height) = image.dimensions();

        Ok(RasterImage {
            width,
            height,
            rgba: image.into_raw(),
        })
    }
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

    let supported = src
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["jpg", "jpeg", "png", "gif"]
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        });

    if supported {
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
