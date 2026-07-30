use std::sync::Arc;

use pdf_tools_core::domain::geometry::PageSize;
use pdf_tools_core::domain::source::DocumentInfo;
use pdf_tools_core::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};
use pdf_tools_lib::presentation::commands::add_sources_inner;
use pdf_tools_lib::presentation::state::AppState;

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
