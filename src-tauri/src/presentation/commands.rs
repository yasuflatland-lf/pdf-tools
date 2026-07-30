use std::path::PathBuf;
use std::sync::Arc;

use pdf_tools_core::application::add_sources::AddSources;
use pdf_tools_core::domain::geometry::RasterSpec;
use pdf_tools_core::domain::ids::SlotId;
use pdf_tools_core::domain::source::SourceKind;
use pdf_tools_core::infrastructure::pdfium::PdfiumEngine;
use pdf_tools_core::infrastructure::png::encode_png;
use tauri::ipc::Response;
use tauri::State;

use super::dto::PlanSnapshot;
use super::state::{AppDocument, AppState};

/// Managed application state for PDFium startup.
pub struct PdfiumState(Result<Arc<PdfiumEngine>, String>);

impl PdfiumState {
    pub fn new(engine: Result<Arc<PdfiumEngine>, String>) -> Self {
        Self(engine)
    }
}

#[tauri::command]
pub fn pdfium_health(state: State<'_, PdfiumState>) -> Result<String, String> {
    let health = match &state.0 {
        Ok(engine) => Ok(engine.version()),
        Err(error) => Err(error.clone()),
    };
    tracing::debug!(?health, "pdfium_health");
    health
}

/// The body of the `add_sources` command, kept free of Tauri's `State` wrapper
/// so it can be exercised in tests without starting a webview.
///
/// Appends the given files to the document and returns the whole plan, not just
/// the additions: the frontend renders a snapshot rather than applying a diff.
pub fn add_sources_inner(state: &AppState, paths: Vec<String>) -> Result<PlanSnapshot, String> {
    let paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    let mut document = state.document();
    let AppDocument { plan, sources, ids } = &mut *document;
    let result = AddSources {
        pdf: state.pdf(),
        images: state.images(),
    }
    .execute(plan, sources, ids, &paths);

    *plan = result.plan;
    *sources = result.sources;

    Ok(PlanSnapshot::from_document(plan, sources))
}

#[tauri::command]
pub fn add_sources(state: State<'_, AppState>, paths: Vec<String>) -> Result<PlanSnapshot, String> {
    add_sources_inner(&state, paths)
}

/// Renders one plan slot to PNG bytes. Split out of the command so tests can
/// exercise it without a webview.
pub fn rasterize_slot_inner(state: &AppState, slot_id: u64, width: u32) -> Result<Vec<u8>, String> {
    let (path, kind, page) = {
        let document = state.document();
        let slot = document
            .plan
            .slots()
            .iter()
            .find(|slot| slot.id == SlotId(slot_id))
            .ok_or_else(|| format!("slot {slot_id} was not found"))?;
        let source = document
            .sources
            .iter()
            .find(|source| source.id == slot.source)
            .ok_or_else(|| format!("source {} for slot {slot_id} was not found", slot.source.0))?;
        (source.path.clone(), source.kind, slot.page)
    };

    let image = match kind {
        SourceKind::Pdf => state
            .pdf()
            .rasterize(
                &path,
                page,
                RasterSpec {
                    target_width_px: width,
                },
            )
            .map_err(|error| error.to_string())?,
        // Image sources are decoded at their native size because the port takes no target width.
        SourceKind::Image => state
            .images()
            .decode_first_frame(&path)
            .map_err(|error| error.to_string())?,
    };

    encode_png(&image).map_err(|error| error.to_string())
}

/// Returns the slot's thumbnail as a raw binary IPC body, so the PNG never
/// passes through base64 or a JSON array of numbers.
#[tauri::command]
pub fn rasterize_slot(
    state: State<'_, AppState>,
    slot_id: u64,
    width: u32,
) -> Result<Response, String> {
    rasterize_slot_inner(&state, slot_id, width).map(Response::new)
}

#[cfg(test)]
mod tests {
    use pdf_tools_core::application::errors::PdfError;
    use pdf_tools_core::domain::geometry::PageSize;
    use pdf_tools_core::domain::source::DocumentInfo;
    use pdf_tools_core::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};

    use super::super::dto::SourceStatusDto;
    use super::*;

    fn document(pages: u32) -> DocumentInfo {
        DocumentInfo {
            page_count: pages,
            page_sizes: vec![PageSize::A4_PORTRAIT; pages as usize],
            encrypted: false,
        }
    }

    fn state_with(pdf: FakePdfEngine) -> AppState {
        AppState::with_engines(Arc::new(pdf), Arc::new(FakeImageDecoder::new()))
    }

    #[test]
    fn repeated_calls_accumulate_into_a_single_document() {
        let state = state_with(
            FakePdfEngine::new()
                .with_document("/a.pdf", document(2))
                .with_document("/b.pdf", document(1)),
        );

        add_sources_inner(&state, vec!["/a.pdf".into()]).unwrap();
        let snapshot = add_sources_inner(&state, vec!["/b.pdf".into()]).unwrap();

        assert_eq!(snapshot.slots.len(), 3);
        assert_eq!(snapshot.sources.len(), 2);
        // Slot ids stay unique across calls because the sequence is shared.
        let mut ids = snapshot
            .slots
            .iter()
            .map(|slot| slot.id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn an_unreadable_file_is_reported_in_the_snapshot_instead_of_failing() {
        let state = state_with(FakePdfEngine::new().with_failure(
            "/locked.pdf",
            PdfError::Encrypted {
                path: "/locked.pdf".into(),
            },
        ));

        let snapshot = add_sources_inner(&state, vec!["/locked.pdf".into()]).unwrap();

        assert!(snapshot.slots.is_empty());
        assert_eq!(snapshot.sources[0].status, SourceStatusDto::Encrypted);
        assert_eq!(snapshot.sources[0].grouping, "grouped");
    }
}
