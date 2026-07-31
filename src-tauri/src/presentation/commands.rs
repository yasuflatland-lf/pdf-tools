use std::path::{Path, PathBuf};

use pdf_tools_core::application::compose::{Compose, ComposeError, ProgressSink};
use pdf_tools_core::domain::geometry::RasterSpec;
use pdf_tools_core::domain::ids::SlotId;
use pdf_tools_core::domain::source::SourceKind;
use pdf_tools_core::infrastructure::png::encode_png;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, State};

use super::dto::{ComposeProgressDto, MergeReportDto, PlanSnapshot};
use super::state::AppState;

/// The body of the `add_sources` command, kept free of Tauri's `State` wrapper
/// so it can be exercised in tests without starting a webview.
///
/// Appends the given files to the document and returns the whole plan, not just
/// the additions: the frontend renders a snapshot rather than applying a diff.
pub fn add_sources_inner(state: &AppState, paths: Vec<String>) -> Result<PlanSnapshot, String> {
    let paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    let mut session = state.session();
    session.add_sources(state.pdf(), state.images(), &paths);
    Ok(PlanSnapshot::from_session(&session))
}

#[tauri::command]
pub fn add_sources(state: State<'_, AppState>, paths: Vec<String>) -> Result<PlanSnapshot, String> {
    add_sources_inner(&state, paths)
}

pub fn reorder_inner(
    state: &AppState,
    from_start: usize,
    from_end: usize,
    to: usize,
) -> Result<PlanSnapshot, String> {
    let mut session = state.session();
    session.reorder(from_start..from_end, to);
    Ok(PlanSnapshot::from_session(&session))
}

#[tauri::command]
pub fn reorder(
    state: State<'_, AppState>,
    from_start: usize,
    from_end: usize,
    to: usize,
) -> Result<PlanSnapshot, String> {
    reorder_inner(&state, from_start, from_end, to)
}

pub fn remove_slots_inner(state: &AppState, slot_ids: Vec<u64>) -> Result<PlanSnapshot, String> {
    let slot_ids = slot_ids.into_iter().map(SlotId).collect::<Vec<_>>();
    let mut session = state.session();
    session.remove(&slot_ids);
    Ok(PlanSnapshot::from_session(&session))
}

#[tauri::command]
pub fn remove_slots(
    state: State<'_, AppState>,
    slot_ids: Vec<u64>,
) -> Result<PlanSnapshot, String> {
    remove_slots_inner(&state, slot_ids)
}

pub fn rotate_slots_inner(
    state: &AppState,
    slot_ids: Vec<u64>,
    delta: i32,
) -> Result<PlanSnapshot, String> {
    let slot_ids = slot_ids.into_iter().map(SlotId).collect::<Vec<_>>();
    let delta = delta.rem_euclid(4) as i8;
    let mut session = state.session();
    session.rotate(&slot_ids, delta);
    Ok(PlanSnapshot::from_session(&session))
}

#[tauri::command]
pub fn rotate_slots(
    state: State<'_, AppState>,
    slot_ids: Vec<u64>,
    delta: i32,
) -> Result<PlanSnapshot, String> {
    rotate_slots_inner(&state, slot_ids, delta)
}

pub fn undo_inner(state: &AppState) -> Result<PlanSnapshot, String> {
    let mut session = state.session();
    // Empty history is a successful no-op; the unchanged snapshot is returned.
    session.undo();
    Ok(PlanSnapshot::from_session(&session))
}

#[tauri::command]
pub fn undo(state: State<'_, AppState>) -> Result<PlanSnapshot, String> {
    undo_inner(&state)
}

pub fn redo_inner(state: &AppState) -> Result<PlanSnapshot, String> {
    let mut session = state.session();
    // Empty history is a successful no-op; the unchanged snapshot is returned.
    session.redo();
    Ok(PlanSnapshot::from_session(&session))
}

#[tauri::command]
pub fn redo(state: State<'_, AppState>) -> Result<PlanSnapshot, String> {
    redo_inner(&state)
}

/// The body of the `compose` command. Split out of the command so tests can
/// exercise it without a webview.
pub fn compose_inner(
    state: &AppState,
    dest: &Path,
    progress: &dyn ProgressSink,
) -> Result<MergeReportDto, String> {
    // The document is cloned out of the session so the lock is released before
    // the engine runs: a merge must never block the commands the UI issues
    // meanwhile, and it must never mutate the document.
    let document = {
        let session = state.session();
        session.document().clone()
    };

    Compose { pdf: state.pdf() }
        .execute(&document, dest, progress)
        .map(MergeReportDto::from)
        .map_err(|error| match error {
            ComposeError::EmptyPlan => {
                "there is nothing to merge: add at least one file first".to_owned()
            }
            ComposeError::NoUsableSources => {
                "there is nothing to merge: the document has no usable pages".to_owned()
            }
            other => other.to_string(),
        })
}

/// Forwards every tick the `Compose` use case reports as a `compose-progress`
/// event. `Compose` currently reports all ticks before it calls the engine, so
/// the frontend sees the whole ramp up front rather than page-by-page.
struct EventProgress {
    app: AppHandle,
}

impl ProgressSink for EventProgress {
    fn report(&self, done: u32, total: u32) {
        // A failed emit must not abort the merge; it only costs a progress tick.
        if let Err(error) = self
            .app
            .emit("compose-progress", ComposeProgressDto { done, total })
        {
            tracing::warn!(%error, "failed to emit compose-progress");
        }
    }
}

#[tauri::command]
pub async fn compose(app: AppHandle, dest: String) -> Result<MergeReportDto, String> {
    let dest = PathBuf::from(dest);
    tauri::async_runtime::spawn_blocking(move || {
        let progress = EventProgress { app: app.clone() };
        compose_inner(&app.state::<AppState>(), &dest, &progress)
    })
    .await
    .map_err(|error| format!("the merge task could not be started: {error}"))?
}

/// Renders one plan slot to PNG bytes. Split out of the command so tests can
/// exercise it without a webview.
pub fn rasterize_slot_inner(state: &AppState, slot_id: u64, width: u32) -> Result<Vec<u8>, String> {
    let (path, kind, page) = {
        let session = state.session();
        let slot = session
            .plan()
            .slots()
            .iter()
            .find(|slot| slot.id == SlotId(slot_id))
            .ok_or_else(|| format!("slot {slot_id} was not found"))?;
        let source = session.document().source_of(slot);
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
    use std::sync::Arc;

    use pdf_tools_core::application::errors::PdfError;
    use pdf_tools_core::domain::geometry::PageSize;
    use pdf_tools_core::domain::source::DocumentInfo;
    use pdf_tools_core::infrastructure::fake_engine::{FakeImageDecoder, FakePdfEngine};

    use super::super::dto::{GroupingDto, SourceStatusDto};
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
        assert_eq!(snapshot.sources[0].grouping, GroupingDto::Grouped);
    }
}
