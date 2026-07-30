#[path = "support/fixtures.rs"]
mod fixtures;

use fixtures::{engine, fixture};
use pdf_tools_core::application::ports::{ComposeEntry, ComposePlan, PdfEngine};
use pdf_tools_core::domain::geometry::{PageSize, RasterImage, RasterSpec};
use pdf_tools_core::domain::ids::PageIndex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

fn temp_path(name: &str) -> PathBuf {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    std::env::temp_dir().join(format!(
        "pdf-tools-compose-image-{}-{}-{name}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ))
}

fn image_plan(name: &str) -> ComposePlan {
    ComposePlan {
        entries: vec![ComposeEntry::Image {
            path: fixture(name),
            fit_to: PageSize::A4_PORTRAIT,
        }],
    }
}

fn is_white_row(image: &RasterImage, row: u32) -> bool {
    let start = (row * image.width * 4) as usize;
    image.rgba[start..start + (image.width * 4) as usize]
        .chunks_exact(4)
        .all(is_white)
}

fn is_white_column(image: &RasterImage, column: u32) -> bool {
    (0..image.height).all(|row| {
        let offset = ((row * image.width + column) * 4) as usize;
        is_white(&image.rgba[offset..offset + 4])
    })
}

fn is_white(pixel: &[u8]) -> bool {
    pixel[0] >= 250 && pixel[1] >= 250 && pixel[2] >= 250
}

fn center_pixel(image: &RasterImage) -> &[u8] {
    let offset = (((image.height / 2) * image.width + image.width / 2) * 4) as usize;
    &image.rgba[offset..offset + 4]
}

#[test]
fn an_image_becomes_a_page_of_the_requested_size() {
    let out = temp_path("img.pdf");
    let plan = ComposePlan {
        entries: vec![ComposeEntry::Image {
            path: fixture("sample.jpg"),
            fit_to: PageSize::A4_PORTRAIT,
        }],
    };
    engine().compose(&plan, &out).unwrap();
    let info = engine().probe(&out).unwrap();
    assert_eq!(info.page_count, 1);
    assert!(info.page_sizes[0].approx_eq(&PageSize::A4_PORTRAIT));
}

#[test]
fn a_landscape_image_is_letterboxed_top_and_bottom() {
    let out = temp_path("landscape.pdf");
    engine().compose(&image_plan("sample.jpg"), &out).unwrap();
    let image = engine()
        .rasterize(
            &out,
            PageIndex(0),
            RasterSpec {
                target_width_px: 200,
            },
        )
        .unwrap();
    assert!(is_white_row(&image, 2), "top band should be white");
    assert!(
        !is_white_row(&image, image.height / 2),
        "middle row should contain image content"
    );
}

#[test]
fn a_portrait_image_is_pillarboxed_left_and_right() {
    let out = temp_path("portrait.pdf");
    engine().compose(&image_plan("tall.png"), &out).unwrap();
    let image = engine()
        .rasterize(
            &out,
            PageIndex(0),
            RasterSpec {
                target_width_px: 200,
            },
        )
        .unwrap();
    assert!(is_white_column(&image, 2), "left band should be white");
    assert!(
        !is_white_column(&image, image.width / 2),
        "middle column should contain image content"
    );
}

#[test]
fn images_and_pdf_pages_can_be_interleaved() {
    let out = temp_path("mixed.pdf");
    let plan = ComposePlan {
        entries: vec![
            ComposeEntry::PdfPage {
                path: fixture("multi_page.pdf"),
                page: PageIndex(0),
            },
            ComposeEntry::Image {
                path: fixture("sample.jpg"),
                fit_to: PageSize::A4_PORTRAIT,
            },
            ComposeEntry::PdfPage {
                path: fixture("multi_page.pdf"),
                page: PageIndex(1),
            },
        ],
    };
    engine().compose(&plan, &out).unwrap();
    assert_eq!(engine().probe(&out).unwrap().page_count, 3);
}

#[test]
fn an_animated_gif_contributes_only_its_first_frame() {
    let out = temp_path("gif.pdf");
    engine().compose(&image_plan("animated.gif"), &out).unwrap();
    let image = engine()
        .rasterize(
            &out,
            PageIndex(0),
            RasterSpec {
                target_width_px: 100,
            },
        )
        .unwrap();
    let center = center_pixel(&image);
    assert!(
        center[0] > center[2],
        "expected a red-dominant page, got {center:?}"
    );
}
