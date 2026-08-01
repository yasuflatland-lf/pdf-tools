use super::probe::map_load_error;
use super::PdfiumEngine;
use crate::application::errors::{ImageError, PdfError};
use crate::application::ports::{ComposeEntry, ComposePlan, ImageDecoder, MergeReport};
use crate::domain::geometry::PageSize;
use crate::domain::plan::Rotation;
use crate::infrastructure::image_decoder::{probe_with_orientation, ImageCrateDecoder};
use crate::infrastructure::jpeg::{self, Passthrough};
use image::imageops::FilterType;
use image::metadata::Orientation;
use image::{DynamicImage, RgbaImage};
use pdfium_render::prelude::{
    PdfColor, PdfDocument, PdfMatrix, PdfPageImageObject, PdfPageObjectsCommon, PdfPagePaperSize,
    PdfPagePathObject, PdfPageRenderRotation, PdfPoints, PdfRect,
};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

/// The highest pixel density an image is embedded at, measured against the space
/// it occupies on the page.
///
/// PDFium deflates the raw pixel buffer when the document is saved -- 2.41 s of a
/// measured 2.50 s merge -- so the embedded pixel count is what a merge costs. It
/// exposes no API for embedding a pre-compressed non-JPEG stream, which makes
/// this the only lever available for PNG and GIF. At the scale the 10 s objective
/// names, 100 photo-like pages measured 7.48 s through the capped raster path
/// (43.8 MB of PNG in, 111.7 MB out). The cap earns its keep past that scale: on
/// 900 MB of deliberately noisy PNG it cut output size by about 37% and wall clock
/// by about 12%, and what is left there is PNG decode, which no cap can reduce.
/// Screenshots and line art are already below the cap and remain untouched.
const MAX_EMBED_DPI: f32 = 200.0;

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
                ComposeEntry::PdfPage {
                    path,
                    page,
                    rotation,
                } => {
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
                    let mut copied_page =
                        destination
                            .pages()
                            .get(page_count as i32)
                            .map_err(|error| PdfError::Unreadable {
                                path: path.clone(),
                                reason: format!(
                                    "PDFium could not access imported page {}: {error}",
                                    page.0 + 1
                                ),
                            })?;
                    let existing_rotation =
                        copied_page
                            .rotation()
                            .map_err(|error| PdfError::Unreadable {
                                path: path.clone(),
                                reason: format!(
                                    "PDFium could not read imported page {} rotation: {error}",
                                    page.0 + 1
                                ),
                            })?;
                    copied_page.set_rotation(add_rotation(existing_rotation, *rotation));
                }
                ComposeEntry::Image {
                    path,
                    fit_to,
                    rotation,
                } => {
                    let image_object = image_object(&destination, path, *fit_to)?;

                    let mut page = destination
                        .pages_mut()
                        .create_page_at_end(PdfPagePaperSize::Custom(
                            PdfPoints::new(fit_to.width_pt),
                            PdfPoints::new(fit_to.height_pt),
                        ))
                        .map_err(|error| image_composition_error(path, error))?;
                    let background = PdfPagePathObject::new_rect(
                        &destination,
                        PdfRect::new(
                            PdfPoints::ZERO,
                            PdfPoints::ZERO,
                            PdfPoints::new(fit_to.height_pt),
                            PdfPoints::new(fit_to.width_pt),
                        ),
                        None,
                        None,
                        Some(PdfColor::WHITE),
                    )
                    .map_err(|error| image_composition_error(path, error))?;
                    page.objects_mut()
                        .add_path_object(background)
                        .map_err(|error| image_composition_error(path, error))?;

                    page.objects_mut()
                        .add_image_object(image_object)
                        .map_err(|error| image_composition_error(path, error))?;
                    page.regenerate_content()
                        .map_err(|error| image_composition_error(path, error))?;
                    page.set_rotation(pdfium_rotation(*rotation));
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

fn add_rotation(existing: PdfPageRenderRotation, added: Rotation) -> PdfPageRenderRotation {
    let existing_quarter_turns = match existing {
        PdfPageRenderRotation::None => 0,
        PdfPageRenderRotation::Degrees90 => 1,
        PdfPageRenderRotation::Degrees180 => 2,
        PdfPageRenderRotation::Degrees270 => 3,
    };
    let quarter_turns = (existing_quarter_turns + added.quarter_turns()) % 4;

    match quarter_turns {
        0 => PdfPageRenderRotation::None,
        1 => PdfPageRenderRotation::Degrees90,
        2 => PdfPageRenderRotation::Degrees180,
        3 => PdfPageRenderRotation::Degrees270,
        _ => unreachable!("a value modulo four is always in 0..4"),
    }
}

fn pdfium_rotation(rotation: Rotation) -> PdfPageRenderRotation {
    add_rotation(PdfPageRenderRotation::None, rotation)
}

/// Where a fitted image sits on its page, in points.
struct Placement {
    left: f32,
    bottom: f32,
    width: f32,
    height: f32,
}

/// Scales an image's intrinsic pixel size to fit the page, preserving the aspect
/// ratio and centring the result.
fn place(fit_to: PageSize, width_px: u32, height_px: u32) -> Placement {
    let scale = (fit_to.width_pt / width_px as f32).min(fit_to.height_pt / height_px as f32);
    let width = width_px as f32 * scale;
    let height = height_px as f32 * scale;

    Placement {
        left: (fit_to.width_pt - width) / 2.0,
        bottom: (fit_to.height_pt - height) / 2.0,
        width,
        height,
    }
}

/// Shrinks an image that would be embedded at more than `MAX_EMBED_DPI` for the
/// space it occupies. An image already at or under the cap is returned unchanged,
/// so nothing that was lossless becomes lossy.
///
/// `resize` fits inside the box while preserving the aspect ratio, so the
/// placement computed from the original dimensions stays correct.
fn cap_resolution(image: DynamicImage, placement: &Placement) -> DynamicImage {
    let max_width = (placement.width / 72.0 * MAX_EMBED_DPI).round().max(1.0) as u32;
    let max_height = (placement.height / 72.0 * MAX_EMBED_DPI).round().max(1.0) as u32;

    if image.width() <= max_width && image.height() <= max_height {
        return image;
    }

    image.resize(max_width, max_height, FilterType::Triangle)
}

/// Returns the file's bytes when it is a JPEG whose stream PDF can carry
/// unchanged, and None when the image has to be decoded instead.
///
/// A file that cannot be read returns None rather than an error: the raster path
/// reports missing and unreadable sources, and reporting them here as well would
/// give one failure two different messages.
fn passthrough_bytes(path: &Path) -> Option<Vec<u8>> {
    let bytes = std::fs::read(path).ok()?;
    matches!(jpeg::passthrough(&bytes), Passthrough::Eligible).then_some(bytes)
}

/// Builds the image object for one entry together with the space it occupies.
///
/// An eligible JPEG is handed to PDFium as its original stream, which PDFium
/// stores as DCTDecode: no decode, no re-encode, and the bytes on disk are the
/// bytes that came in. Everything else is decoded and embedded as a bitmap,
/// which PDFium deflates at save time.
fn image_object<'a>(
    document: &PdfDocument<'a>,
    path: &Path,
    fit_to: PageSize,
) -> Result<PdfPageImageObject<'a>, PdfError> {
    if let Some(bytes) = passthrough_bytes(path) {
        // Only dimensions and EXIF metadata are needed; this reads headers, not pixels.
        let oriented =
            probe_with_orientation(path).map_err(|error| map_image_error(path, error))?;
        if !is_mirrored(oriented.orientation) {
            let placement = place(fit_to, oriented.info.width_px, oriented.info.height_px);
            let mut object = PdfPageImageObject::new_from_jpeg_reader(document, Cursor::new(bytes))
                .map_err(|error| image_composition_error(path, error))?;
            object
                .reset_matrix(placement_matrix(&placement, oriented.orientation))
                .map_err(|error| image_composition_error(path, error))?;
            return Ok(object);
        }
    }

    let raster = ImageCrateDecoder
        .decode_first_frame(path)
        .map_err(|error| map_image_error(path, error))?;
    let placement = place(fit_to, raster.width, raster.height);
    let image = RgbaImage::from_raw(raster.width, raster.height, raster.rgba)
        .map(DynamicImage::ImageRgba8)
        .ok_or_else(|| PdfError::Unreadable {
            path: path.to_path_buf(),
            reason: "decoded image buffer has invalid dimensions".into(),
        })?;
    let image = cap_resolution(image, &placement);
    let mut object = PdfPageImageObject::new_with_size(
        document,
        &image,
        PdfPoints::new(placement.width),
        PdfPoints::new(placement.height),
    )
    .map_err(|error| image_composition_error(path, error))?;
    object
        .translate(
            PdfPoints::new(placement.left),
            PdfPoints::new(placement.bottom),
        )
        .map_err(|error| image_composition_error(path, error))?;

    Ok(object)
}

fn is_mirrored(orientation: Orientation) -> bool {
    matches!(
        orientation,
        Orientation::FlipHorizontal
            | Orientation::FlipVertical
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

fn placement_matrix(placement: &Placement, orientation: Orientation) -> PdfMatrix {
    let Placement {
        left,
        bottom,
        width,
        height,
    } = *placement;

    match orientation {
        Orientation::NoTransforms => PdfMatrix::new(width, 0.0, 0.0, height, left, bottom),
        Orientation::Rotate90 => PdfMatrix::new(0.0, -height, width, 0.0, left, bottom + height),
        Orientation::Rotate180 => {
            PdfMatrix::new(-width, 0.0, 0.0, -height, left + width, bottom + height)
        }
        Orientation::Rotate270 => PdfMatrix::new(0.0, height, -width, 0.0, left + width, bottom),
        Orientation::FlipHorizontal
        | Orientation::FlipVertical
        | Orientation::Rotate90FlipH
        | Orientation::Rotate270FlipH => {
            unreachable!("mirrored orientations use the raster path")
        }
    }
}

fn write_error(path: &Path, reason: String) -> PdfError {
    PdfError::WriteFailed {
        path: path.to_path_buf(),
        reason,
    }
}

/// Translates a decoder failure into a compose failure. `source` names the file
/// being decoded, so variants that carry no path of their own stay attributable.
fn map_image_error(source: &Path, error: ImageError) -> PdfError {
    match error {
        ImageError::Missing { path } => PdfError::Missing { path },
        ImageError::Unreadable { path, reason } => PdfError::Unreadable { path, reason },
        ImageError::UnsupportedFormat { path } => PdfError::Unreadable {
            path,
            reason: "unsupported image format".into(),
        },
        ImageError::EncodeFailed { reason } => PdfError::Unreadable {
            path: source.to_path_buf(),
            reason,
        },
    }
}

fn image_composition_error(path: &Path, error: impl std::fmt::Display) -> PdfError {
    PdfError::Unreadable {
        path: path.to_path_buf(),
        reason: format!("PDFium could not compose image: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_image_fills_the_page_width_and_is_centred_vertically() {
        let placement = place(PageSize::A4_PORTRAIT, 800, 600);
        assert!((placement.width - PageSize::A4_PORTRAIT.width_pt).abs() < 0.01);
        assert!((placement.left - 0.0).abs() < 0.01);
        assert!(placement.bottom > 0.0);
    }

    #[test]
    fn a_tall_image_fills_the_page_height_and_is_centred_horizontally() {
        let placement = place(PageSize::A4_PORTRAIT, 300, 900);
        assert!((placement.height - PageSize::A4_PORTRAIT.height_pt).abs() < 0.01);
        assert!((placement.bottom - 0.0).abs() < 0.01);
        assert!(placement.left > 0.0);
    }

    #[test]
    fn the_aspect_ratio_survives_the_fit() {
        let placement = place(PageSize::A4_PORTRAIT, 800, 600);
        assert!(((placement.width / placement.height) - 800.0 / 600.0).abs() < 0.001);
    }

    #[test]
    fn a_dense_image_is_capped_to_its_placement_at_200_dpi() {
        let placement = place(
            PageSize {
                width_pt: 72.0,
                height_pt: 72.0,
            },
            400,
            400,
        );
        let capped = cap_resolution(DynamicImage::new_rgba8(400, 400), &placement);
        assert_eq!((capped.width(), capped.height()), (200, 200));
    }

    #[test]
    fn an_image_under_the_cap_is_returned_unchanged() {
        let placement = place(PageSize::A4_PORTRAIT, 400, 400);
        let capped = cap_resolution(DynamicImage::new_rgba8(400, 400), &placement);
        assert_eq!((capped.width(), capped.height()), (400, 400));
    }

    #[test]
    fn capping_preserves_the_aspect_ratio() {
        let placement = place(
            PageSize {
                width_pt: 72.0,
                height_pt: 72.0,
            },
            800,
            600,
        );
        let capped = cap_resolution(DynamicImage::new_rgba8(800, 600), &placement);
        assert!(((capped.width() as f32 / capped.height() as f32) - 800.0 / 600.0).abs() < 0.01);
    }
}
