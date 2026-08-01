use std::path::Path;

use crate::application::errors::PdfError;
use crate::application::ports::{ComposePlan, MergeReport, PdfEngine};
use crate::domain::geometry::{RasterImage, RasterSpec};
use crate::domain::ids::PageIndex;
use crate::domain::source::DocumentInfo;

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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::UnavailablePdfEngine;
    use crate::application::errors::PdfError;
    use crate::application::ports::{ComposePlan, PdfEngine};
    use crate::domain::geometry::RasterSpec;
    use crate::domain::ids::PageIndex;

    const REASON: &str = "PDFium failed to load";

    #[test]
    fn probe_reports_the_engine_unavailable_reason() {
        let engine = UnavailablePdfEngine::new(REASON.to_owned());

        assert_eq!(
            engine.probe(Path::new("document.pdf")),
            Err(PdfError::EngineUnavailable(REASON.to_owned()))
        );
    }

    #[test]
    fn rasterize_reports_the_engine_unavailable_reason() {
        let engine = UnavailablePdfEngine::new(REASON.to_owned());

        assert_eq!(
            engine.rasterize(
                Path::new("document.pdf"),
                PageIndex(0),
                RasterSpec {
                    target_width_px: 200,
                },
            ),
            Err(PdfError::EngineUnavailable(REASON.to_owned()))
        );
    }

    #[test]
    fn compose_reports_the_engine_unavailable_reason() {
        let engine = UnavailablePdfEngine::new(REASON.to_owned());
        let plan = ComposePlan {
            entries: Vec::new(),
        };

        assert_eq!(
            engine.compose(&plan, Path::new("output.pdf")),
            Err(PdfError::EngineUnavailable(REASON.to_owned()))
        );
    }
}
