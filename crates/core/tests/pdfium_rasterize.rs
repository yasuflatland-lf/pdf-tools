#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{engine, fixture};
use pdf_tools_core::application::errors::PdfError;
use pdf_tools_core::application::ports::PdfEngine;
use pdf_tools_core::domain::geometry::RasterSpec;
use pdf_tools_core::domain::ids::PageIndex;

#[test]
fn rasterize_honours_the_requested_width_and_keeps_the_aspect_ratio() {
    let img = engine()
        .rasterize(
            &fixture("multi_page.pdf"),
            PageIndex(0),
            RasterSpec {
                target_width_px: 200,
            },
        )
        .unwrap();
    assert_eq!(img.width, 200);
    // A4 portrait: height/width == 841.89/595.276 == 1.414...
    let ratio = img.height as f32 / img.width as f32;
    assert!(
        (ratio - 1.414).abs() < 0.02,
        "unexpected aspect ratio: {ratio}"
    );
    assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
}

#[test]
fn rasterize_rejects_pages_beyond_the_document() {
    let err = engine()
        .rasterize(
            &fixture("multi_page.pdf"),
            PageIndex(99),
            RasterSpec {
                target_width_px: 100,
            },
        )
        .unwrap_err();
    assert!(matches!(err, PdfError::PageOutOfRange { count: 3, .. }));
}

#[test]
fn rasterized_pages_are_not_blank() {
    // The fixture has visible content; a fully white buffer means rendering
    // silently did nothing.
    let img = engine()
        .rasterize(
            &fixture("multi_page.pdf"),
            PageIndex(0),
            RasterSpec {
                target_width_px: 200,
            },
        )
        .unwrap();
    assert!(img
        .rgba
        .chunks(4)
        .any(|px| px[0] < 250 || px[1] < 250 || px[2] < 250));
}
