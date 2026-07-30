use super::probe::map_load_error;
use super::PdfiumEngine;
use crate::application::errors::{ImageError, PdfError};
use crate::application::ports::{ComposeEntry, ComposePlan, ImageDecoder, MergeReport};
use crate::infrastructure::image_decoder::ImageCrateDecoder;
use image::{DynamicImage, RgbaImage};
use pdfium_render::prelude::{
    PdfColor, PdfPageImageObject, PdfPageObjectsCommon, PdfPagePaperSize, PdfPagePathObject,
    PdfPoints, PdfRect,
};
use std::collections::HashMap;
use std::path::Path;

/// Builds a new PDF holding the plan's entries, in plan order.
///
/// Each distinct source path is opened once and kept for the duration of the
/// call, so a plan that draws many pages from one file -- or repeats the same
/// page -- never reopens it.
pub(super) fn compose(
    engine: &PdfiumEngine,
    plan: &ComposePlan,
    dest: &Path,
) -> Result<MergeReport, PdfError> {
    engine.with_library(|pdfium| {
        let mut destination = pdfium
            .create_new_pdf()
            .map_err(|error| write_error(dest, format!("failed to create document: {error}")))?;
        let mut sources = HashMap::new();
        let mut page_count = 0_u32;

        for entry in &plan.entries {
            match entry {
                ComposeEntry::PdfPage { path, page } => {
                    if !path.is_file() {
                        return Err(PdfError::Missing { path: path.clone() });
                    }

                    if !sources.contains_key(path) {
                        let source = pdfium
                            .load_pdf_from_file(path, None)
                            .map_err(|error| map_load_error(path, error))?;
                        sources.insert(path.clone(), source);
                    }
                    let source = sources
                        .get(path)
                        .expect("source was inserted immediately above");
                    let source_page_count = source.pages().len() as u32;

                    if page.0 >= source_page_count {
                        return Err(PdfError::PageOutOfRange {
                            page: page.0,
                            count: source_page_count,
                        });
                    }

                    destination
                        .pages_mut()
                        .copy_page_from_document(source, page.0 as i32, page_count as i32)
                        .map_err(|error| PdfError::Unreadable {
                            path: path.clone(),
                            reason: format!("PDFium could not import page {}: {error}", page.0 + 1),
                        })?;
                }
                ComposeEntry::Image { path, fit_to } => {
                    let raster = ImageCrateDecoder
                        .decode_first_frame(path)
                        .map_err(map_image_error)?;
                    let image = RgbaImage::from_raw(raster.width, raster.height, raster.rgba)
                        .map(DynamicImage::ImageRgba8)
                        .ok_or_else(|| PdfError::Unreadable {
                            path: path.clone(),
                            reason: "decoded image buffer has invalid dimensions".into(),
                        })?;

                    let page_width = fit_to.width_pt;
                    let page_height = fit_to.height_pt;
                    let scale =
                        (page_width / raster.width as f32).min(page_height / raster.height as f32);
                    let image_width = raster.width as f32 * scale;
                    let image_height = raster.height as f32 * scale;
                    let image_left = (page_width - image_width) / 2.0;
                    let image_bottom = (page_height - image_height) / 2.0;

                    let mut page = destination
                        .pages_mut()
                        .create_page_at_end(PdfPagePaperSize::Custom(
                            PdfPoints::new(page_width),
                            PdfPoints::new(page_height),
                        ))
                        .map_err(|error| image_composition_error(path, error))?;
                    let background = PdfPagePathObject::new_rect(
                        &destination,
                        PdfRect::new(
                            PdfPoints::ZERO,
                            PdfPoints::ZERO,
                            PdfPoints::new(page_height),
                            PdfPoints::new(page_width),
                        ),
                        None,
                        None,
                        Some(PdfColor::WHITE),
                    )
                    .map_err(|error| image_composition_error(path, error))?;
                    page.objects_mut()
                        .add_path_object(background)
                        .map_err(|error| image_composition_error(path, error))?;

                    let mut image_object = PdfPageImageObject::new_with_size(
                        &destination,
                        &image,
                        PdfPoints::new(image_width),
                        PdfPoints::new(image_height),
                    )
                    .map_err(|error| image_composition_error(path, error))?;
                    image_object
                        .translate(PdfPoints::new(image_left), PdfPoints::new(image_bottom))
                        .map_err(|error| image_composition_error(path, error))?;
                    page.objects_mut()
                        .add_image_object(image_object)
                        .map_err(|error| image_composition_error(path, error))?;
                    page.regenerate_content()
                        .map_err(|error| image_composition_error(path, error))?;
                }
            }
            page_count += 1;
        }

        destination
            .save_to_file(dest)
            .map_err(|error| write_error(dest, error.to_string()))?;
        let bytes_written = std::fs::metadata(dest)
            .map_err(|error| write_error(dest, error.to_string()))?
            .len();

        Ok(MergeReport {
            page_count,
            bytes_written,
        })
    })
}

fn write_error(path: &Path, reason: String) -> PdfError {
    PdfError::WriteFailed {
        path: path.to_path_buf(),
        reason,
    }
}

fn map_image_error(error: ImageError) -> PdfError {
    match error {
        ImageError::Missing { path } => PdfError::Missing { path },
        ImageError::Unreadable { path, reason } => PdfError::Unreadable { path, reason },
        ImageError::UnsupportedFormat { path } => PdfError::Unreadable {
            path,
            reason: "unsupported image format".into(),
        },
    }
}

fn image_composition_error(path: &Path, error: impl std::fmt::Display) -> PdfError {
    PdfError::Unreadable {
        path: path.to_path_buf(),
        reason: format!("PDFium could not compose image: {error}"),
    }
}
