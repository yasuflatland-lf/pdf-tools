/// Errors raised by the PDF engine. Later tasks extend this enum; only the
/// variants required by the current milestone are defined so far.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PdfError {
    #[error("PDFium engine is unavailable: {0}")]
    EngineUnavailable(String),
}
