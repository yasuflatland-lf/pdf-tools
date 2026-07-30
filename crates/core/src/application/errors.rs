use std::path::PathBuf;

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum PdfError {
    #[error("PDFium engine is unavailable: {0}")]
    EngineUnavailable(String),
    #[error("the file is password protected: {path}")]
    Encrypted { path: PathBuf },
    #[error("failed to read the PDF at {path}: {reason}")]
    Unreadable { path: PathBuf, reason: String },
    #[error("the file is no longer present: {path}")]
    Missing { path: PathBuf },
    #[error("page {page} is out of range (the document has {count} pages)")]
    PageOutOfRange { page: u32, count: u32 },
    #[error("failed to write the merged PDF to {path}: {reason}")]
    WriteFailed { path: PathBuf, reason: String },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq)]
pub enum ImageError {
    #[error("failed to read the image at {path}: {reason}")]
    Unreadable { path: PathBuf, reason: String },
    #[error("the file is no longer present: {path}")]
    Missing { path: PathBuf },
    #[error("unsupported image format: {path}")]
    UnsupportedFormat { path: PathBuf },
    #[error("failed to encode the image: {reason}")]
    EncodeFailed { reason: String },
}
