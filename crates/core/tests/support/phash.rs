use crate::fixtures::engine;
use image::{imageops::FilterType, RgbaImage};
use pdf_tools_core::domain::geometry::{RasterImage, RasterSpec};
use pdf_tools_core::domain::ids::PageIndex;
use pdfium_render::prelude::PdfRenderConfig;
use std::path::Path;

/// Renders one page of a PDF to an RGBA raster. This mirrors what
/// `PdfEngine::rasterize` will do once it exists, and keeps the compose tests
/// independent of that separate work item.
pub fn render_page(path: &Path, page: PageIndex, spec: RasterSpec) -> RasterImage {
    engine().with_library(|pdfium| {
        let document = pdfium
            .load_pdf_from_file(path, None)
            .expect("test PDF should load");
        let page = document
            .pages()
            .get(page.0 as i32)
            .expect("test page should exist");
        let bitmap = page
            .render_with_config(
                &PdfRenderConfig::new().set_target_width(spec.target_width_px as i32),
            )
            .expect("test page should render");

        RasterImage {
            width: bitmap.width() as u32,
            height: bitmap.height() as u32,
            rgba: bitmap.as_rgba_bytes(),
        }
    })
}

/// Average hash: downscale to 8x8 grayscale, then set one bit per pixel
/// according to whether it is brighter than the mean. Robust against the small
/// rendering differences that a PDFium version bump introduces, which is why
/// the compose tests compare hashes instead of raw bytes.
pub fn average_hash(image: &RasterImage) -> u64 {
    let source = RgbaImage::from_raw(image.width, image.height, image.rgba.clone())
        .expect("raster dimensions should match its RGBA bytes");
    let thumbnail = image::imageops::resize(&source, 8, 8, FilterType::Triangle);
    let luminances: Vec<u32> = thumbnail
        .pixels()
        .map(|pixel| {
            let [red, green, blue, _alpha] = pixel.0;
            (299 * u32::from(red) + 587 * u32::from(green) + 114 * u32::from(blue)) / 1_000
        })
        .collect();
    let mean = luminances.iter().sum::<u32>() / luminances.len() as u32;

    luminances
        .iter()
        .enumerate()
        .fold(0, |hash, (index, luminance)| {
            hash | (u64::from(*luminance > mean) << index)
        })
}

pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}
