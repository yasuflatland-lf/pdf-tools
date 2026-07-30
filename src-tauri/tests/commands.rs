use std::path::PathBuf;
use std::sync::Arc;

use pdf_tools_core::application::add_sources::AddSources;
use pdf_tools_core::domain::geometry::PageSize;
use pdf_tools_core::domain::source::{DocumentInfo, ImageInfo};
use pdf_tools_core::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};
use pdf_tools_lib::presentation::commands::rasterize_slot_inner;
use pdf_tools_lib::presentation::state::AppState;

const PNG_MAGIC: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

fn pdf_info() -> DocumentInfo {
    DocumentInfo {
        page_count: 1,
        page_sizes: vec![PageSize::A4_PORTRAIT],
        encrypted: false,
    }
}

fn add_sources(state: &AppState, paths: &[PathBuf]) -> u64 {
    let mut document = state.document();
    let plan = document.plan.clone();
    let sources = document.sources.clone();
    let result = AddSources {
        pdf: state.pdf(),
        images: state.images(),
    }
    .execute(&plan, &sources, &mut document.ids, paths);
    document.plan = result.plan;
    document.sources = result.sources;
    document
        .plan
        .slots()
        .last()
        .expect("the source should contribute a slot")
        .id
        .0
}

fn png_width(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(
        bytes[16..20]
            .try_into()
            .expect("a PNG should contain an IHDR width"),
    )
}

#[test]
fn rasterize_slot_returns_png_bytes_for_a_known_slot() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new().with_document("/known.pdf", pdf_info())),
        Arc::new(FakeImageDecoder::new()),
    );
    let slot_id = add_sources(&state, &[PathBuf::from("/known.pdf")]);

    let bytes = rasterize_slot_inner(&state, slot_id, 200).unwrap();

    assert_eq!(&bytes[..PNG_MAGIC.len()], PNG_MAGIC);
    assert_eq!(png_width(&bytes), 200);
}

#[test]
fn rasterize_slot_reports_an_error_for_an_unknown_slot() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new()),
        Arc::new(FakeImageDecoder::new()),
    );

    assert!(rasterize_slot_inner(&state, 9999, 200).is_err());
}

#[test]
fn rasterize_slot_returns_png_bytes_for_an_image_source() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new()),
        Arc::new(FakeImageDecoder::new().with_image(
            "/known.png",
            ImageInfo {
                width_px: 40,
                height_px: 30,
            },
        )),
    );
    let slot_id = add_sources(&state, &[PathBuf::from("/known.png")]);

    let bytes = rasterize_slot_inner(&state, slot_id, 200).unwrap();

    assert_eq!(&bytes[..PNG_MAGIC.len()], PNG_MAGIC);
    assert_eq!(png_width(&bytes), 40);
}

#[test]
fn rasterize_slot_surfaces_a_rasterize_failure() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new().with_document("/known.pdf", pdf_info())),
        Arc::new(FakeImageDecoder::new()),
    );
    let slot_id = add_sources(&state, &[PathBuf::from("/known.pdf")]);
    state.document().sources[0].path = PathBuf::from("/missing.pdf");

    assert!(rasterize_slot_inner(&state, slot_id, 200).is_err());
}
