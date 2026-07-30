use std::path::{Path, PathBuf};

use pdf_tools_core::application::errors::ImageError;
use pdf_tools_core::application::ports::ImageDecoder;
use pdf_tools_core::infrastructure::image_decoder::ImageCrateDecoder;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures")).join(name)
}

#[test]
fn probe_reports_pixel_dimensions() {
    let info = ImageCrateDecoder.probe(&fixture("sample.jpg")).unwrap();
    assert_eq!((info.width_px, info.height_px), (800, 600));
}

#[test]
fn animated_gifs_decode_only_their_first_frame() {
    let img = ImageCrateDecoder
        .decode_first_frame(&fixture("animated.gif"))
        .unwrap();
    // The first frame is solid red; a second-frame leak would show blue.
    assert_eq!(&img.rgba[0..4], &[255, 0, 0, 255]);
}

#[test]
fn decoding_produces_a_buffer_matching_the_reported_dimensions() {
    let img = ImageCrateDecoder
        .decode_first_frame(&fixture("sample.png"))
        .unwrap();
    assert_eq!(img.rgba.len(), (img.width * img.height * 4) as usize);
}

#[test]
fn corrupt_images_report_unreadable() {
    let err = ImageCrateDecoder
        .decode_first_frame(&fixture("corrupt.png"))
        .unwrap_err();
    assert!(matches!(err, ImageError::Unreadable { .. }));
}

#[test]
fn absent_files_report_missing() {
    let err = ImageCrateDecoder.probe(Path::new("/nope.png")).unwrap_err();
    assert!(matches!(err, ImageError::Missing { .. }));
}
