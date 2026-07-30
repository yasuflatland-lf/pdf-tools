use pdf_tools_core::infrastructure::pdfium::PdfiumEngine;
use tauri::State;

/// Managed application state for PDFium startup.
pub struct PdfiumState(Result<PdfiumEngine, String>);

impl PdfiumState {
    pub fn new(engine: Result<PdfiumEngine, String>) -> Self {
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
