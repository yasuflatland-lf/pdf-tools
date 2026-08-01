#[path = "support/fixtures.rs"]
mod fixtures;
#[path = "support/phash.rs"]
mod phash;

use fixtures::{engine, fixture};
use image::imageops::rotate90;
use pdf_tools_core::application::ports::{ComposeEntry, ComposePlan, ImageDecoder, PdfEngine};
use pdf_tools_core::domain::geometry::{PageSize, RasterImage, RasterSpec};
use pdf_tools_core::domain::ids::PageIndex;
use pdf_tools_core::domain::plan::Rotation;
use pdf_tools_core::infrastructure::image_decoder::ImageCrateDecoder;
use pdfium_render::prelude::{PdfPageObjectCommon, PdfPageObjectsCommon, PdfPageRenderRotation};
use phash::{average_hash, hamming_distance, render_page};
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
            rotation: Default::default(),
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

/// Reports the filter chain PDFium wrote for the first image object on page 0.
/// Reading it back from the saved file is the only way to tell an embedded
/// stream from a re-encoded bitmap; the compose call itself succeeds either way.
fn image_filters(path: &std::path::Path) -> Vec<String> {
    engine().with_library(|pdfium| {
        let document = pdfium.load_pdf_from_file(path, None).unwrap();
        let pages = document.pages();
        let page = pages.get(0).unwrap();
        let filters = page
            .objects()
            .iter()
            .find_map(|object| {
                object.as_image_object().map(|image| {
                    image
                        .filters()
                        .iter()
                        .map(|f| f.name().to_owned())
                        .collect()
                })
            })
            .expect("the composed page should hold an image object");
        filters
    })
}

fn page_rotation(path: &std::path::Path) -> PdfPageRenderRotation {
    engine().with_library(|pdfium| {
        let document = pdfium.load_pdf_from_file(path, None).unwrap();
        document.pages().get(0).unwrap().rotation().unwrap()
    })
}

/// The pixel dimensions PDFium recorded for the first embedded image on page 0.
fn image_pixels(path: &std::path::Path) -> (i32, i32) {
    engine().with_library(|pdfium| {
        let document = pdfium.load_pdf_from_file(path, None).unwrap();
        let pages = document.pages();
        let page = pages.get(0).unwrap();
        page.objects()
            .iter()
            .find_map(|object| {
                object
                    .as_image_object()
                    .map(|image| (image.width().unwrap(), image.height().unwrap()))
            })
            .expect("the composed page should hold an image object")
    })
}

#[test]
fn an_image_denser_than_the_cap_is_embedded_at_the_cap() {
    // sample.png is 400x400. Fitted onto a one-inch square page it occupies
    // 72x72 pt, which at 200 DPI is 200x200 px -- half its pixel width.
    let out = temp_path("capped.pdf");
    let plan = ComposePlan {
        entries: vec![ComposeEntry::Image {
            path: fixture("sample.png"),
            fit_to: PageSize::new(72.0, 72.0).unwrap(),
            rotation: Default::default(),
        }],
    };
    engine().compose(&plan, &out).unwrap();

    let (width, height) = image_pixels(&out);
    assert_eq!((width, height), (200, 200));
}

#[test]
fn an_image_already_under_the_cap_is_embedded_untouched() {
    // The same 400x400 file on A4 occupies 595 pt, whose 200 DPI budget is
    // 1654 px. Nothing that was lossless may become lossy.
    let out = temp_path("uncapped.pdf");
    engine().compose(&image_plan("sample.png"), &out).unwrap();

    let (width, height) = image_pixels(&out);
    assert_eq!((width, height), (400, 400));
}

#[test]
fn a_passthrough_jpeg_is_never_downscaled() {
    // Passthrough costs nothing regardless of resolution, and capping it would
    // force a lossy decode and re-encode to achieve nothing.
    let out = temp_path("uncapped-jpeg.pdf");
    let plan = ComposePlan {
        entries: vec![ComposeEntry::Image {
            path: fixture("sample.jpg"),
            fit_to: PageSize::new(72.0, 72.0).unwrap(),
            rotation: Default::default(),
        }],
    };
    engine().compose(&plan, &out).unwrap();

    let (width, height) = image_pixels(&out);
    assert_eq!((width, height), (800, 600));
}

#[test]
fn a_baseline_jpeg_is_embedded_as_its_original_stream() {
    let out = temp_path("baseline-passthrough.pdf");
    engine().compose(&image_plan("sample.jpg"), &out).unwrap();
    assert_eq!(image_filters(&out), vec!["DCTDecode".to_owned()]);
}

#[test]
fn an_identity_exif_jpeg_is_embedded_as_its_original_stream() {
    let out = temp_path("identity-exif-passthrough.pdf");
    engine()
        .compose(&image_plan("exif-orientation-1.jpg"), &out)
        .unwrap();
    assert_eq!(image_filters(&out), vec!["DCTDecode".to_owned()]);
}

/// Renders the composed page of an EXIF-oriented JPEG next to the page composed
/// from the raster the decoder has already oriented, both at the same width.
fn oriented_jpeg_and_reference(value: u8) -> (std::path::PathBuf, RasterImage, RasterImage) {
    let jpeg = format!("exif-orientation-{value}.jpg");
    let reference_png = temp_path(&format!("exif-orientation-{value}-reference.png"));
    let raster = ImageCrateDecoder
        .decode_first_frame(&fixture(&jpeg))
        .unwrap();
    image::save_buffer(
        &reference_png,
        &raster.rgba,
        raster.width,
        raster.height,
        image::ColorType::Rgba8,
    )
    .unwrap();

    let passthrough_out = temp_path(&format!("exif-orientation-{value}-passthrough.pdf"));
    let reference_out = temp_path(&format!("exif-orientation-{value}-reference.pdf"));
    engine()
        .compose(&image_plan(&jpeg), &passthrough_out)
        .unwrap();
    engine()
        .compose(
            &ComposePlan {
                entries: vec![ComposeEntry::Image {
                    path: reference_png,
                    fit_to: PageSize::A4_PORTRAIT,
                    rotation: Default::default(),
                }],
            },
            &reference_out,
        )
        .unwrap();

    let spec = RasterSpec {
        target_width_px: 128,
    };
    let actual = render_page(&passthrough_out, PageIndex(0), spec);
    let expected = render_page(&reference_out, PageIndex(0), spec);
    (passthrough_out, actual, expected)
}

/// The pixels whose channels differ by more than JPEG quantisation explains.
/// One page carries the original DCT stream and the other a deflated raster of
/// the same decoded pixels, so a handful of levels of difference is expected and
/// a wrong orientation is a whole block of colour in the wrong place.
fn mismatched_pixels(actual: &RasterImage, expected: &RasterImage) -> usize {
    const TOLERANCE: u8 = 24;

    if (actual.width, actual.height) != (expected.width, expected.height) {
        return usize::MAX;
    }

    actual
        .rgba
        .chunks_exact(4)
        .zip(expected.rgba.chunks_exact(4))
        .filter(|(left, right)| {
            left[..3]
                .iter()
                .zip(right[..3].iter())
                .any(|(a, b)| a.abs_diff(*b) > TOLERANCE)
        })
        .count()
}

/// Every EXIF orientation, including the four mirrored ones, must reach the page
/// through the passthrough path: the acceptance list says passthrough JPEGs are
/// still `DCTDecode` after this change, and a PDF image matrix expresses a mirror
/// as readily as a rotation.
///
/// Each page is compared pixel by pixel against the page composed from the raster
/// `decode_first_frame` has already oriented -- an average hash is too coarse
/// here, because orientation 1 and orientation 3 of this fixture are only four
/// bits apart, exactly the tolerance the rotation tests allow.
#[test]
fn every_exif_orientation_is_oriented_without_reencoding_or_turning_its_sheet() {
    let (_, identity, _) = oriented_jpeg_and_reference(1);

    for value in 1..=8 {
        let (out, actual, expected) = oriented_jpeg_and_reference(value);

        assert_eq!(
            image_filters(&out),
            vec!["DCTDecode".to_owned()],
            "EXIF orientation {value} should keep JPEG passthrough"
        );
        assert_eq!(
            page_rotation(&out),
            PdfPageRenderRotation::None,
            "EXIF orientation {value} should orient the image, not the sheet"
        );
        assert_eq!(
            mismatched_pixels(&actual, &expected),
            0,
            "EXIF orientation {value} did not render as the oriented raster"
        );
        // Without this the loop would pass on a fixture too symmetric to tell
        // one orientation from another.
        if value != 1 {
            assert_ne!(
                mismatched_pixels(&actual, &identity),
                0,
                "EXIF orientation {value} is indistinguishable from orientation 1"
            );
        }
    }
}

#[test]
fn a_rotated_jpeg_is_still_embedded_as_its_original_stream() {
    let out = temp_path("rotated-passthrough.pdf");
    let plan = ComposePlan {
        entries: vec![ComposeEntry::Image {
            path: fixture("sample.jpg"),
            fit_to: PageSize::A4_PORTRAIT,
            rotation: Rotation::from_quarter_turns(1),
        }],
    };

    engine().compose(&plan, &out).unwrap();

    assert_eq!(page_rotation(&out), PdfPageRenderRotation::Degrees90);
    assert_eq!(image_filters(&out), vec!["DCTDecode".to_owned()]);
}

#[test]
fn a_progressive_jpeg_is_embedded_as_its_original_stream() {
    let out = temp_path("progressive-passthrough.pdf");
    engine()
        .compose(&image_plan("progressive.jpg"), &out)
        .unwrap();
    assert_eq!(image_filters(&out), vec!["DCTDecode".to_owned()]);
}

#[test]
fn a_cmyk_jpeg_is_decoded_rather_than_embedded() {
    // PDFium accepts a four-component stream and renders it with the wrong
    // colours, so this one must not take the passthrough path.
    let out = temp_path("cmyk-fallback.pdf");
    engine().compose(&image_plan("cmyk.jpg"), &out).unwrap();
    assert_eq!(image_filters(&out), vec!["FlateDecode".to_owned()]);
}

#[test]
fn a_png_is_decoded_rather_than_embedded() {
    let out = temp_path("png-fallback.pdf");
    engine().compose(&image_plan("sample.png"), &out).unwrap();
    assert_eq!(image_filters(&out), vec!["FlateDecode".to_owned()]);
}

#[test]
fn an_embedded_stream_lands_where_a_decoded_one_would() {
    // A passthrough page that is scaled or positioned differently from the
    // raster page would be a silent regression: it still merges, still opens,
    // and still reports the right page count.
    let passthrough = temp_path("bounds-jpeg.pdf");
    let raster = temp_path("bounds-png.pdf");
    engine()
        .compose(&image_plan("sample.jpg"), &passthrough)
        .unwrap();
    engine()
        .compose(&image_plan("sample.png"), &raster)
        .unwrap();

    let bounds = |path: &std::path::Path| {
        engine().with_library(|pdfium| {
            let document = pdfium.load_pdf_from_file(path, None).unwrap();
            let pages = document.pages();
            let page = pages.get(0).unwrap();
            let rect = page
                .objects()
                .iter()
                .find_map(|object| object.as_image_object().map(|i| i.bounds().unwrap()))
                .unwrap();
            (rect.left().value, rect.right().value)
        })
    };

    // sample.jpg is 800x600 and sample.png is 400x400; both are wider than tall
    // or square, so both fit to the full page width on A4 portrait.
    let (jpeg_left, jpeg_right) = bounds(&passthrough);
    let (png_left, png_right) = bounds(&raster);
    assert!((jpeg_left - png_left).abs() < 0.5);
    assert!((jpeg_right - png_right).abs() < 0.5);
}

#[test]
fn an_image_becomes_a_page_of_the_requested_size() {
    let out = temp_path("img.pdf");
    let plan = ComposePlan {
        entries: vec![ComposeEntry::Image {
            path: fixture("sample.jpg"),
            fit_to: PageSize::A4_PORTRAIT,
            rotation: Default::default(),
        }],
    };
    engine().compose(&plan, &out).unwrap();
    let info = engine().probe(&out).unwrap();
    assert_eq!(info.page_count, 1);
    assert_eq!(
        info.page_sizes[0].size_class(),
        PageSize::A4_PORTRAIT.size_class()
    );
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
fn a_rotated_image_page_renders_as_a_clockwise_turn() {
    let unrotated_out = temp_path("unrotated-render.pdf");
    let rotated_out = temp_path("rotated-render.pdf");
    engine()
        .compose(&image_plan("sample.jpg"), &unrotated_out)
        .unwrap();
    let rotated_plan = ComposePlan {
        entries: vec![ComposeEntry::Image {
            path: fixture("sample.jpg"),
            fit_to: PageSize::A4_PORTRAIT,
            rotation: Rotation::from_quarter_turns(1),
        }],
    };
    engine().compose(&rotated_plan, &rotated_out).unwrap();

    let baseline = render_page(
        &unrotated_out,
        PageIndex(0),
        RasterSpec {
            target_width_px: 128,
        },
    );
    let baseline_buffer =
        image::RgbaImage::from_raw(baseline.width, baseline.height, baseline.rgba.clone())
            .expect("the rendered baseline should have valid dimensions");
    let expected_buffer = rotate90(&baseline_buffer);
    let expected = RasterImage {
        width: expected_buffer.width(),
        height: expected_buffer.height(),
        rgba: expected_buffer.into_raw(),
    };
    let actual = render_page(
        &rotated_out,
        PageIndex(0),
        RasterSpec {
            target_width_px: expected.width,
        },
    );
    let baseline_hash = average_hash(&baseline);
    let expected_hash = average_hash(&expected);
    let actual_hash = average_hash(&actual);

    assert!(
        hamming_distance(baseline_hash, expected_hash) > 4,
        "the fixture is too symmetric to prove that the page turned"
    );
    assert!(
        hamming_distance(actual_hash, expected_hash) <= 4,
        "the rendered image page did not turn clockwise"
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
                rotation: Default::default(),
            },
            ComposeEntry::Image {
                path: fixture("sample.jpg"),
                fit_to: PageSize::A4_PORTRAIT,
                rotation: Default::default(),
            },
            ComposeEntry::PdfPage {
                path: fixture("multi_page.pdf"),
                page: PageIndex(1),
                rotation: Default::default(),
            },
        ],
    };
    engine().compose(&plan, &out).unwrap();
    assert_eq!(engine().probe(&out).unwrap().page_count, 3);
}

#[test]
fn a_turned_image_page_and_a_turned_pdf_page_agree() {
    let out = temp_path("turned-pages.pdf");
    let rotation = Rotation::from_quarter_turns(1);
    let plan = ComposePlan {
        entries: vec![
            ComposeEntry::PdfPage {
                path: fixture("multi_page.pdf"),
                page: PageIndex(0),
                rotation,
            },
            ComposeEntry::Image {
                path: fixture("sample.jpg"),
                fit_to: PageSize::A4_PORTRAIT,
                rotation,
            },
        ],
    };
    engine().compose(&plan, &out).unwrap();

    engine().with_library(|pdfium| {
        let document = pdfium.load_pdf_from_file(&out, None).unwrap();
        let pages = document.pages();
        let pdf_page = pages.get(0).unwrap();
        let image_page = pages.get(1).unwrap();
        assert!((pdf_page.width().value - image_page.width().value).abs() < 0.01);
        assert!((pdf_page.height().value - image_page.height().value).abs() < 0.01);
        // Both must be the turned A4 sheet, so agreeing on portrait cannot pass.
        assert!(pdf_page.width().value > pdf_page.height().value);
    });
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
