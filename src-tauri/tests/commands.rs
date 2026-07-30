use std::path::PathBuf;
use std::sync::Arc;

use pdf_tools_core::domain::geometry::PageSize;
use pdf_tools_core::domain::source::{DocumentInfo, ImageInfo};
use pdf_tools_core::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};
use pdf_tools_lib::presentation::commands::{
    add_sources_inner, insert_at_inner, rasterize_slot_inner, redo_inner, remove_slots_inner,
    reorder_inner, undo_inner,
};
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
    add_sources_inner(
        state,
        paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    )
    .unwrap()
    .slots
    .last()
    .expect("the source should contribute a slot")
    .id
}

fn state_with_pdf(pages: u32) -> AppState {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new().with_document(
            "/document.pdf",
            DocumentInfo {
                page_count: pages,
                page_sizes: vec![PageSize::A4_PORTRAIT; pages as usize],
                encrypted: false,
            },
        )),
        Arc::new(FakeImageDecoder::new()),
    );
    add_sources_inner(&state, vec!["/document.pdf".into()]).unwrap();
    state
}

fn state_with_pdf_and_image() -> (AppState, u64) {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new().with_document(
            "/document.pdf",
            DocumentInfo {
                page_count: 3,
                page_sizes: vec![PageSize::A4_PORTRAIT; 3],
                encrypted: false,
            },
        )),
        Arc::new(FakeImageDecoder::new().with_image(
            "/image.png",
            ImageInfo {
                width_px: 40,
                height_px: 30,
            },
        )),
    );
    add_sources_inner(&state, vec!["/document.pdf".into()]).unwrap();
    let snapshot = add_sources_inner(&state, vec!["/image.png".into()]).unwrap();
    let image_slot_id = snapshot.slots.last().expect("image slot").id;
    (state, image_slot_id)
}

fn png_width(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(
        bytes[16..20]
            .try_into()
            .expect("a PNG should contain an IHDR width"),
    )
}

#[test]
fn reorder_returns_a_snapshot_in_the_new_order() {
    let state = state_with_pdf(3);
    let snapshot = reorder_inner(&state, 0, 1, 2).unwrap();
    assert_eq!(snapshot.slots[2].page, 0);
}

#[test]
fn undo_flags_reflect_the_history() {
    let state = state_with_pdf(3);
    let snapshot = remove_slots_inner(&state, vec![0]).unwrap();
    assert!(snapshot.can_undo);
    assert!(!snapshot.can_redo);
    let snapshot = undo_inner(&state).unwrap();
    assert!(snapshot.can_redo);
}

#[test]
fn redo_after_undo_restores_the_change() {
    let state = state_with_pdf(3);
    let removed = remove_slots_inner(&state, vec![0]).unwrap();
    undo_inner(&state).unwrap();

    let redone = redo_inner(&state).unwrap();

    assert_eq!(redone.slots, removed.slots);
    assert!(redone.can_undo);
    assert!(!redone.can_redo);
}

#[test]
fn undo_on_a_fresh_state_is_a_no_op() {
    let state = AppState::with_engines(
        Arc::new(FakePdfEngine::new()),
        Arc::new(FakeImageDecoder::new()),
    );

    let snapshot = undo_inner(&state).unwrap();

    assert!(snapshot.slots.is_empty());
    assert!(!snapshot.can_undo);
    assert!(!snapshot.can_redo);
}

#[test]
fn inserting_inside_a_group_marks_the_source_ungrouped_in_the_snapshot() {
    let (state, image_slot_id) = state_with_pdf_and_image();
    let snapshot = insert_at_inner(&state, 1, vec![image_slot_id]).unwrap();
    let pdf_source = snapshot
        .sources
        .iter()
        .find(|source| source.kind == "pdf")
        .unwrap();
    assert_eq!(pdf_source.grouping, "ungrouped");
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
        Arc::new(FakePdfEngine::new().with_document(
            "/known.pdf",
            DocumentInfo {
                page_count: 1,
                page_sizes: vec![],
                encrypted: false,
            },
        )),
        Arc::new(FakeImageDecoder::new()),
    );
    let slot_id = add_sources(&state, &[PathBuf::from("/known.pdf")]);

    let error = rasterize_slot_inner(&state, slot_id, 200).unwrap_err();

    assert_eq!(error, "page 0 is out of range (the document has 1 pages)");
}
