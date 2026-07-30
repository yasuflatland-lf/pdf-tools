use super::{probe::map_load_error, PdfiumEngine};
use crate::application::errors::PdfError;
use crate::domain::geometry::{RasterImage, RasterSpec};
use crate::domain::ids::PageIndex;
use pdfium_render::prelude::{PdfColor, PdfPageIndex, PdfRenderConfig, PdfiumError, Pixels};
use std::path::Path;

pub(super) fn rasterize_page(
    engine: &PdfiumEngine,
    src: &Path,
    page: PageIndex,
    spec: RasterSpec,
) -> Result<RasterImage, PdfError> {
    if !src.is_file() {
        return Err(PdfError::Missing {
            path: src.to_path_buf(),
        });
    }

    if spec.target_width_px == 0 {
        return Err(invalid_raster_dimensions(
            src,
            "target width must be greater than zero",
        ));
    }

    engine.with_library(|pdfium| {
        let document = pdfium
            .load_pdf_from_file(src, None)
            .map_err(|error| map_load_error(src, error))?;
        let pages = document.pages();
        // A negative count only happens when PDFium reports a failure, and an
        // empty document makes every page index out of range.
        let page_count = u32::try_from(pages.len()).unwrap_or(0);

        if page.0 >= page_count {
            return Err(PdfError::PageOutOfRange {
                page: page.0,
                count: page_count,
            });
        }

        let pdf_page = pages
            .get(page.0 as PdfPageIndex)
            .map_err(|error| unreadable(src, "failed to load the requested page", error))?;
        let page_width_pt = pdf_page.width().value;
        let page_height_pt = pdf_page.height().value;
        let (width, height) =
            raster_dimensions(src, spec.target_width_px, page_width_pt, page_height_pt)?;
        let render_config = PdfRenderConfig::new()
            .set_target_size(width, height)
            .set_clear_color(PdfColor::WHITE);
        let bitmap = pdf_page
            .render_with_config(&render_config)
            .map_err(|error| unreadable(src, "failed to render the requested page", error))?;

        if bitmap.width() != width || bitmap.height() != height {
            return Err(invalid_raster_dimensions(
                src,
                "PDFium returned an image with unexpected dimensions",
            ));
        }

        let mut rgba = bitmap.as_rgba_bytes();
        composite_over_white(&mut rgba);

        let width = width as u32;
        let height = height as u32;
        let expected_len = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| invalid_raster_dimensions(src, "raster buffer size overflowed"))?;

        if rgba.len() != expected_len {
            return Err(invalid_raster_dimensions(
                src,
                "PDFium returned an image buffer with an unexpected length",
            ));
        }

        Ok(RasterImage {
            width,
            height,
            rgba,
        })
    })
}

fn raster_dimensions(
    src: &Path,
    target_width_px: u32,
    page_width_pt: f32,
    page_height_pt: f32,
) -> Result<(Pixels, Pixels), PdfError> {
    if !page_width_pt.is_finite()
        || !page_height_pt.is_finite()
        || page_width_pt <= 0.0
        || page_height_pt <= 0.0
    {
        return Err(invalid_raster_dimensions(
            src,
            "page dimensions must be finite and greater than zero",
        ));
    }

    let width = Pixels::try_from(target_width_px).map_err(|_| {
        invalid_raster_dimensions(src, "target width exceeds PDFium's supported range")
    })?;
    let height = (f64::from(target_width_px) * f64::from(page_height_pt)
        / f64::from(page_width_pt))
    .round()
    .max(1.0);

    if !height.is_finite() || height > f64::from(Pixels::MAX) {
        return Err(invalid_raster_dimensions(
            src,
            "target height exceeds PDFium's supported range",
        ));
    }

    Ok((width, height as Pixels))
}

fn composite_over_white(rgba: &mut [u8]) {
    for pixel in rgba.chunks_exact_mut(4) {
        let alpha = u16::from(pixel[3]);

        if alpha < 255 {
            for channel in &mut pixel[..3] {
                let foreground = u16::from(*channel) * alpha;
                let background = 255 * (255 - alpha);
                *channel = ((foreground + background + 127) / 255) as u8;
            }
            pixel[3] = 255;
        }
    }
}

fn unreadable(src: &Path, context: &str, error: PdfiumError) -> PdfError {
    PdfError::Unreadable {
        path: src.to_path_buf(),
        reason: format!("{context}: {error}"),
    }
}

fn invalid_raster_dimensions(src: &Path, reason: &str) -> PdfError {
    PdfError::Unreadable {
        path: src.to_path_buf(),
        reason: reason.to_owned(),
    }
}
