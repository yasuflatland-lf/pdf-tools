use std::path::PathBuf;
use std::sync::Arc;

use pdf_tools_core::application::add_sources::AddSources;
use pdf_tools_core::domain::geometry::PageSize;
use pdf_tools_core::domain::source::{DocumentInfo, ImageInfo};
use pdf_tools_core::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};
use pdf_tools_lib::presentation::commands::{add_sources_inner, rasterize_slot_inner};
use pdf_tools_lib::presentation::state::AppState;

const PNG_MAGIC: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

fn fake_pdf() -> FakePdfEngine {
    let document = DocumentInfo {
        page_count: 3,
        page_sizes: vec![PageSize::A4_PORTRAIT; 3],
        encrypted: false,
    };

    FakePdfEngine::new()
        .with_document("/a.pdf", document.clone())
        .with_document("/deep/dir/invoice.pdf", document)
}

fn fake_images() -> FakeImageDecoder {
    FakeImageDecoder::new()
}

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
fn add_sources_returns_a_snapshot_containing_the_new_slots() {
    let state = AppState::with_engines(Arc::new(fake_pdf()), Arc::new(fake_images()));
    let snapshot = add_sources_inner(&state, vec!["/a.pdf".into()]).unwrap();

    assert_eq!(snapshot.slots.len(), 3);
    assert_eq!(snapshot.sources[0].kind, "pdf");
}

#[test]
fn the_snapshot_exposes_a_display_file_name() {
    let state = AppState::with_engines(Arc::new(fake_pdf()), Arc::new(fake_images()));
    let snapshot = add_sources_inner(&state, vec!["/deep/dir/invoice.pdf".into()]).unwrap();

    assert_eq!(snapshot.sources[0].file_name, "invoice.pdf");
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
