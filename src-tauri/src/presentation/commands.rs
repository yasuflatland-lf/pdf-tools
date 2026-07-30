use std::sync::Arc;

use pdf_tools_core::domain::geometry::RasterSpec;
use pdf_tools_core::domain::ids::SlotId;
use pdf_tools_core::domain::source::SourceKind;
use pdf_tools_core::infrastructure::pdfium::PdfiumEngine;
use pdf_tools_core::infrastructure::png::encode_png;
use tauri::ipc::Response;
use tauri::State;

use super::state::AppState;

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
