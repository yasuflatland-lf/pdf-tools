use std::path::Path;
use std::sync::{Arc, Mutex};

use pdf_tools_core::application::errors::PdfError;
use pdf_tools_core::application::ports::{ComposePlan, ImageDecoder, MergeReport, PdfEngine};
use pdf_tools_core::domain::geometry::{RasterImage, RasterSpec};
use pdf_tools_core::domain::ids::{IdSequence, PageIndex};
use pdf_tools_core::domain::plan::MergePlan;
use pdf_tools_core::domain::source::{DocumentInfo, SourceFile};

/// Everything the commands mutate: the ordered plan, the sources it refers to,
/// and the sequence that hands out their identifiers.
pub struct AppDocument {
    pub plan: MergePlan,
    pub sources: Vec<SourceFile>,
    pub ids: IdSequence,
}

/// The Tauri-managed application state.
///
/// Engines are held behind `Arc<dyn _>` so tests can drive the commands with
/// fakes instead of a real PDFium build. Undo is deliberately absent here; it
/// arrives later, when `AppDocument` is replaced by a session type.
pub struct AppState {
    pub(crate) document: Mutex<AppDocument>,
    pub(crate) pdf: Arc<dyn PdfEngine>,
    pub(crate) images: Arc<dyn ImageDecoder>,
}

impl AppState {
    /// Creates an empty document backed by the given engines.
    pub fn with_engines(pdf: Arc<dyn PdfEngine>, images: Arc<dyn ImageDecoder>) -> Self {
        Self {
            document: Mutex::new(AppDocument {
                plan: MergePlan::default(),
                sources: Vec::new(),
                ids: IdSequence::default(),
            }),
            pdf,
            images,
        }
    }
}

/// Stands in for the PDF engine when the PDFium library failed to load, so the
/// app still starts and reports the reason on every attempted operation.
pub struct UnavailablePdfEngine {
    reason: String,
}

impl UnavailablePdfEngine {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }

    fn error(&self) -> PdfError {
        PdfError::EngineUnavailable(self.reason.clone())
    }
}

impl PdfEngine for UnavailablePdfEngine {
    fn probe(&self, _src: &Path) -> Result<DocumentInfo, PdfError> {
        Err(self.error())
    }

    fn rasterize(
        &self,
        _src: &Path,
        _page: PageIndex,
        _spec: RasterSpec,
    ) -> Result<RasterImage, PdfError> {
        Err(self.error())
    }

    fn compose(&self, _plan: &ComposePlan, _dest: &Path) -> Result<MergeReport, PdfError> {
        Err(self.error())
    }
}
