use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use pdf_tools_core::application::errors::PdfError;
use pdf_tools_core::application::ports::{ComposePlan, ImageDecoder, MergeReport, PdfEngine};
use pdf_tools_core::domain::geometry::{RasterImage, RasterSpec};
use pdf_tools_core::domain::ids::{IdSequence, PageIndex};
use pdf_tools_core::domain::plan::MergePlan;
use pdf_tools_core::domain::source::{DocumentInfo, SourceFile};

/// The mutable document every command operates on.
#[derive(Default)]
pub struct AppDocument {
    pub plan: MergePlan,
    pub sources: Vec<SourceFile>,
    pub ids: IdSequence,
}

/// Managed application state shared by every command.
pub struct AppState {
    document: Mutex<AppDocument>,
    pdf: Arc<dyn PdfEngine>,
    images: Arc<dyn ImageDecoder>,
}

impl AppState {
    /// Builds the state around the given engines so tests can substitute fakes.
    pub fn with_engines(pdf: Arc<dyn PdfEngine>, images: Arc<dyn ImageDecoder>) -> Self {
        Self {
            document: Mutex::new(AppDocument::default()),
            pdf,
            images,
        }
    }

    /// Returns exclusive access to the document.
    pub fn document(&self) -> MutexGuard<'_, AppDocument> {
        self.document
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn pdf(&self) -> &dyn PdfEngine {
        self.pdf.as_ref()
    }

    pub fn images(&self) -> &dyn ImageDecoder {
        self.images.as_ref()
    }
}

/// Stands in for PDFium when the library could not be loaded, so every command
/// reports the load failure instead of the state being absent entirely.
pub struct UnavailablePdfEngine {
    reason: String,
}

impl UnavailablePdfEngine {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

impl PdfEngine for UnavailablePdfEngine {
    fn probe(&self, _src: &Path) -> Result<DocumentInfo, PdfError> {
        Err(PdfError::EngineUnavailable(self.reason.clone()))
    }

    fn rasterize(
        &self,
        _src: &Path,
        _page: PageIndex,
        _spec: RasterSpec,
    ) -> Result<RasterImage, PdfError> {
        Err(PdfError::EngineUnavailable(self.reason.clone()))
    }

    fn compose(&self, _plan: &ComposePlan, _dest: &Path) -> Result<MergeReport, PdfError> {
        Err(PdfError::EngineUnavailable(self.reason.clone()))
    }
}
