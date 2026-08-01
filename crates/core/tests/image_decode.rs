#[path = "support/fixtures.rs"]
mod fixtures;

use std::path::Path;

use fixtures::image_fixture as fixture;
use pdf_tools_core::application::add_sources::AddSources;
use pdf_tools_core::application::errors::ImageError;
use pdf_tools_core::application::ports::ImageDecoder;
use pdf_tools_core::domain::document::MergeDocument;
use pdf_tools_core::domain::ids::IdSequence;
use pdf_tools_core::domain::source::SourceStatus;
use pdf_tools_core::infrastructure::fake_engine::FakePdfEngine;
use pdf_tools_core::infrastructure::image_decoder::ImageCrateDecoder;

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
fn all_exif_orientations_decode_to_the_displayed_pixels() {
    let baseline = image::open(fixture("exif-absent.jpg")).unwrap();

    for value in 1..=8 {
        let orientation = image::metadata::Orientation::from_exif(value).unwrap();
        let mut expected = baseline.clone();
        expected.apply_orientation(orientation);
        let expected = expected.to_rgba8();
        let actual = ImageCrateDecoder
            .decode_first_frame(&fixture(&format!("exif-orientation-{value}.jpg")))
            .unwrap();

        assert_eq!(
            (actual.width, actual.height),
            expected.dimensions(),
            "EXIF orientation {value} returned the wrong dimensions"
        );
        assert_eq!(
            actual.rgba,
            expected.into_raw(),
            "EXIF orientation {value} returned the wrong pixels"
        );
    }
}

#[test]
fn probe_reports_dimensions_after_exif_orientation() {
    // The fixture is stored 48x32. Four of the eight orientations exchange the
    // axes, and the dominant-page-size census is computed from what `probe`
    // reports, so leaving any of them out lands an image on a sheet of the wrong
    // shape.
    for value in [1, 2, 3, 4] {
        let info = ImageCrateDecoder
            .probe(&fixture(&format!("exif-orientation-{value}.jpg")))
            .unwrap();
        assert_eq!(
            (info.width_px, info.height_px),
            (48, 32),
            "EXIF orientation {value} should keep the stored axes"
        );
    }

    for value in [5, 6, 7, 8] {
        let info = ImageCrateDecoder
            .probe(&fixture(&format!("exif-orientation-{value}.jpg")))
            .unwrap();
        assert_eq!(
            (info.width_px, info.height_px),
            (32, 48),
            "EXIF orientation {value} should report exchanged axes"
        );
    }
}

#[test]
fn absent_or_unparseable_exif_yields_no_orientation_and_remains_ready() {
    let paths = [fixture("exif-absent.jpg"), fixture("exif-corrupt.jpg")];
    let document = AddSources {
        pdf: &FakePdfEngine::new(),
        images: &ImageCrateDecoder,
    }
    .execute(
        &MergeDocument::default(),
        &mut IdSequence::default(),
        &paths,
    );

    assert_eq!(document.plan().len(), 2);
    assert!(document
        .sources()
        .iter()
        .all(|source| source.status == SourceStatus::Ready));
    let absent = ImageCrateDecoder.decode_first_frame(&paths[0]).unwrap();
    let unparseable = ImageCrateDecoder.decode_first_frame(&paths[1]).unwrap();
    assert_eq!(
        (unparseable.width, unparseable.height),
        (absent.width, absent.height)
    );
    assert_eq!(unparseable.rgba, absent.rgba);
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
